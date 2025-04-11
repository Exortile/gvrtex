use std::io::{Cursor, Seek};

use crate::{
    formats::PixelFormat,
    iter::{
        DecodeDxtBlockIterator, EncodeDxtBlockIterator, PixelBlockIterator, PixelBlockIteratorExt,
    },
};
use byteorder::{BigEndian, ReadBytesExt};
use image::{Pixel, Rgba, RgbaImage};

const INDEX4_PALETTE_SIZE: u32 = 16;
const INDEX8_PALETTE_SIZE: u32 = 256;

/// Returns a copy of the given RGBA `image` as a vector of pixels that's suitable
/// for in use with [`imagequant`].
fn as_imagequant_vec(
    image: &RgbaImage,
    palette_pixel_format: PixelFormat,
) -> Vec<imagequant::RGBA> {
    image
        .as_raw()
        .chunks(4)
        .map(|pixel| {
            if palette_pixel_format == PixelFormat::RGB565 {
                imagequant::RGBA::new(pixel[0], pixel[1], pixel[2], 0xFF)
            } else {
                imagequant::RGBA::new(pixel[0], pixel[1], pixel[2], pixel[3])
            }
        })
        .collect()
}

/// Uses [`imagequant`] to turn the given `image` into a color palette with each pixel mapped to an
/// index into the palette.
///
/// `max_colors` determines how many colors the palette should consist of. If there isn't enough
/// colors in the provided image (less than `max_colors`), the resulting palette gets padded with
/// transparent values instead.
fn palettize_image(
    image: &RgbaImage,
    max_colors: u32,
    palette_pixel_format: PixelFormat,
) -> Result<(Vec<imagequant::RGBA>, Vec<u8>), imagequant::Error> {
    let mut attr = imagequant::new();
    attr.set_max_colors(max_colors)?;
    let mut imagequant_img = attr.new_image(
        as_imagequant_vec(image, palette_pixel_format),
        image.width() as usize,
        image.height() as usize,
        0.,
    )?;

    let mut quantized = attr.quantize(&mut imagequant_img)?;
    let (mut palette, indices) = quantized.remapped(&mut imagequant_img)?;

    if palette.len() != max_colors as usize {
        log::warn!(
            "Constructed palette only has {} colors (needs {max_colors}). Padding with transparent color.",
            palette.len()
        );

        palette.resize(max_colors as usize, imagequant::RGBA::new(0, 0, 0, 0));
    }

    Ok((palette, indices))
}

/// Encodes the given `palette` into the suitable [`PixelFormat`], returning a [`Vec`] of bytes.
fn encode_palette(palette: Vec<imagequant::RGBA>, palette_pixel_format: PixelFormat) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();

    for color in palette {
        match palette_pixel_format {
            PixelFormat::RGB5A3 => {
                let color_slice = [color.r, color.g, color.b, color.a];
                let p = Rgba::from_slice(&color_slice);
                let pixel = encode_pixel_rgb5a3(p);
                result.push(((pixel >> 8) & 0xFF).try_into().unwrap());
                result.push((pixel & 0xFF).try_into().unwrap());
            }
            PixelFormat::RGB565 => {
                let color_slice = [color.r, color.g, color.b, color.a];
                let p = Rgba::from_slice(&color_slice);
                let pixel = encode_pixel_rgb565(p);
                result.push(((pixel >> 8) & 0xFF).try_into().unwrap());
                result.push((pixel & 0xFF).try_into().unwrap());
            }
            PixelFormat::IntensityA8 => {
                let color_slice = [color.r, color.g, color.b, color.a];
                let p = Rgba::from_slice(&color_slice);
                let (pixel, alpha) = encode_pixel_intensity_alpha8(p);
                result.push(alpha);
                result.push(pixel);
            }
        }
    }

    result
}

fn decode_palette(
    cursor: &mut Cursor<&[u8]>,
    palette_pixel_format: PixelFormat,
    palette_size: u32,
) -> Result<Vec<Rgba<u8>>, std::io::Error> {
    let mut result = Vec::with_capacity(palette_size as usize);

    for _ in 0..palette_size {
        match palette_pixel_format {
            PixelFormat::IntensityA8 => {
                let alpha = cursor.read_u8()?;
                let pixel = cursor.read_u8()?;
                result.push(decode_pixel_intensity_alpha8(pixel, alpha));
            }
            PixelFormat::RGB565 => {
                let color = cursor.read_u16::<BigEndian>()?;
                result.push(decode_pixel_rgb565(color));
            }
            PixelFormat::RGB5A3 => {
                let color = cursor.read_u16::<BigEndian>()?;
                result.push(decode_pixel_rgb5a3(color));
            }
        }
    }

    Ok(result)
}

////////////////////////
// Encoding Functions //
////////////////////////

fn encode_pixel_rgb5a3(p: &Rgba<u8>) -> u16 {
    let mut pixel: u16 = 0;
    if p.0[3] <= 0xDA {
        // Argb3444
        pixel |= ((p.0[0] >> 4) as u16) << 8;
        pixel |= ((p.0[1] >> 4) as u16) << 4;
        pixel |= (p.0[2] >> 4) as u16;
        pixel |= ((p.0[3] >> 5) as u16) << 12;
    } else {
        // Rgb555
        pixel |= ((p.0[0] >> 3) as u16) << 10;
        pixel |= ((p.0[1] >> 3) as u16) << 5;
        pixel |= (p.0[2] >> 3) as u16;
        pixel |= 0x8000;
    }
    pixel
}

fn encode_pixel_rgb565(p: &Rgba<u8>) -> u16 {
    let mut pixel: u16 = 0x0000;
    pixel |= ((p.0[0] >> 3) as u16) << 11;
    pixel |= ((p.0[1] >> 2) as u16) << 5;
    pixel |= (p.0[2] >> 3) as u16;
    pixel
}

fn encode_pixel_intensity_alpha8(p: &Rgba<u8>) -> (u8, u8) {
    let pixel = (0.30 * p.0[0] as f32 + 0.59 * p.0[1] as f32 + 0.11 * p.0[2] as f32) as u8;
    (pixel, p.0[3])
}

fn compress_block_to_bc1(block: &[u8]) -> Vec<u8> {
    let mut dist: Option<i32> = None;
    let mut col_1 = 0;
    let mut col_2 = 0;
    let mut alpha = false;
    let mut result = vec![0u8; 8];

    for i in 0..15 {
        if block[i * 4 + 3] < 16 {
            alpha = true;
        } else {
            for j in (i + 1)..16 {
                let temp = distance_bc1(block, i * 4, block, j * 4);

                if temp > dist.unwrap_or(-1) {
                    dist = Some(temp);
                    col_1 = i;
                    col_2 = j;
                }
            }
        }
    }

    let mut palette: Vec<Vec<u8>> = Vec::with_capacity(4);

    if dist.is_none() {
        palette.push(vec![0, 0, 0, 0xff]);
        palette.push(vec![0xff, 0xff, 0xff, 0xff]);
    } else {
        let color1_idx = col_1 * 4;
        let color2_idx = col_2 * 4;

        palette.push(vec![
            block[color1_idx],
            block[color1_idx + 1],
            block[color1_idx + 2],
            0xff,
        ]);

        palette.push(vec![
            block[color2_idx],
            block[color2_idx + 1],
            block[color2_idx + 2],
            0xff,
        ]);

        if palette[0][0] >> 3 == palette[1][0] >> 3
            && palette[0][1] >> 2 == palette[1][1] >> 2
            && palette[0][2] >> 3 == palette[1][2] >> 3
        {
            if palette[0][0] >> 3 == 0 && palette[0][1] >> 2 == 0 && palette[0][2] >> 3 == 0 {
                palette[1][0] = 0xff;
                palette[1][1] = 0xff;
                palette[1][2] = 0xff;
            } else {
                palette[1][0] = 0x0;
                palette[1][1] = 0x0;
                palette[1][2] = 0x0;
            }
        }
    }

    palette.resize(4, vec![]);

    result[0] = palette[0][2] & 0xf8 | palette[0][1] >> 5;
    result[1] = palette[0][1] << 3 & 0xe0 | palette[0][0] >> 3;
    result[2] = palette[1][2] & 0xf8 | palette[1][1] >> 5;
    result[3] = palette[1][1] << 3 & 0xe0 | palette[1][0] >> 3;

    if (result[0] > result[2] || (result[0] == result[2] && result[1] >= result[3])) == alpha {
        result[4] = result[0];
        result[5] = result[1];
        result[0] = result[2];
        result[1] = result[3];
        result[2] = result[4];
        result[3] = result[5];

        palette[2] = palette[0].clone();
        palette[0] = palette[1].clone();
        palette[1] = palette[2].clone();
    }

    if !alpha {
        palette[2] = vec![
            ((((palette[0][0] as u32) << 1) + palette[1][0] as u32) / 3) as u8,
            ((((palette[0][1] as u32) << 1) + palette[1][1] as u32) / 3) as u8,
            ((((palette[0][2] as u32) << 1) + palette[1][2] as u32) / 3) as u8,
            0xff,
        ];

        palette[3] = vec![
            ((palette[0][0] as u32 + ((palette[1][0] as u32) << 1)) / 3) as u8,
            ((palette[0][1] as u32 + ((palette[1][1] as u32) << 1)) / 3) as u8,
            ((palette[0][2] as u32 + ((palette[1][2] as u32) << 1)) / 3) as u8,
            0xff,
        ];
    } else {
        palette[2] = vec![
            ((palette[0][0] as u32 + palette[1][0] as u32) >> 1) as u8,
            ((palette[0][1] as u32 + palette[1][1] as u32) >> 1) as u8,
            ((palette[0][2] as u32 + palette[1][2] as u32) >> 1) as u8,
            0xff,
        ];

        palette[3] = vec![0, 0, 0, 0];
    }

    for i in 0..(block.len() / 16) {
        result[4 + i] = (least_distance_bc1(&palette, block, i * 16) << 6
            | least_distance_bc1(&palette, block, i * 16 + 4) << 4
            | least_distance_bc1(&palette, block, i * 16 + 8) << 2
            | least_distance_bc1(&palette, block, i * 16 + 12)) as u8;
    }

    result
}

fn least_distance_bc1(palette: &[Vec<u8>], color: &[u8], offset: usize) -> usize {
    if color[offset + 3] < 8 {
        return 3;
    }

    let mut dist: i32 = i32::MAX;
    let mut best = 0;

    for (i, c) in palette.iter().enumerate() {
        if c[3] != 0xff {
            break;
        }

        let temp = distance_bc1(c, 0, color, offset);

        if temp < dist {
            if temp == 0 {
                return i;
            }

            dist = temp;
            best = i;
        }
    }

    best
}

fn distance_bc1(color_1: &[u8], offset_1: usize, color_2: &[u8], offset_2: usize) -> i32 {
    let mut temp: i32 = 0;

    for i in 0..3 {
        let val: i32 = color_1[offset_1 + i] as i32 - color_2[offset_2 + i] as i32;
        temp += val * val;
    }

    temp
}

pub fn encode_pixels_dxt1(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height / 2).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for block in EncodeDxtBlockIterator::new(image) {
        dest.append(&mut compress_block_to_bc1(&block));
    }

    // Pad the data if needed
    if dest.len() < 32 {
        dest.resize(32, 0);
    }

    dest
}

pub fn encode_pixels_rgb5a3(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height * 2).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let p = image.get_pixel(x, y);
        let pixel = encode_pixel_rgb5a3(p);

        dest.push(((pixel >> 8) & 0xFF).try_into().unwrap());
        dest.push((pixel & 0xFF).try_into().unwrap());
    }

    dest
}

pub fn encode_pixels_argb8888(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height * 4).try_into().unwrap();
    let mut dest = vec![0u8; dest_size];

    let mut dest_idx = 0;

    for (block, _, x, y) in PixelBlockIteratorExt::new(width, height, 4, 4) {
        let p = image.get_pixel(x, y);
        let cur_idx = (block * 32) + dest_idx;
        let cur_dest_idx = cur_idx as usize;

        dest[cur_dest_idx] = p.0[3];
        dest[cur_dest_idx + 1] = p.0[0];
        dest[cur_dest_idx + 32] = p.0[1];
        dest[cur_dest_idx + 33] = p.0[2];

        dest_idx += 2;
    }

    dest
}

pub fn encode_pixels_rgb565(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height * 2).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let p = image.get_pixel(x, y);

        let pixel = encode_pixel_rgb565(p);

        dest.push(((pixel >> 8) & 0xFF).try_into().unwrap());
        dest.push((pixel & 0xFF).try_into().unwrap());
    }

    dest
}

pub fn encode_pixels_intensity_alpha4(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for (x, y) in PixelBlockIterator::new(width, height, 8, 4) {
        let p = image.get_pixel(x, y);

        let mut pixel: u8 = 0;
        pixel |= (((0.30 * p.0[0] as f32 + 0.59 * p.0[1] as f32 + 0.11 * p.0[2] as f32) * 15.
            / 255.) as u8)
            & 0xF;
        pixel |= (((p.0[3] as f32 * 15. / 255.) as u8) & 0xF) << 4;

        dest.push(pixel);
    }

    dest
}

pub fn encode_pixels_intensity_alpha8(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height * 2).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let p = image.get_pixel(x, y);

        let (pixel, alpha) = encode_pixel_intensity_alpha8(p);

        dest.push(alpha);
        dest.push(pixel);
    }

    dest
}

pub fn encode_pixels_intensity_4(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height / 2).try_into().unwrap();
    let mut dest = vec![0u8; dest_size];

    for (idx, (_, col, x, y)) in PixelBlockIteratorExt::new(width, height, 8, 8).enumerate() {
        let p = image.get_pixel(x, y);

        let pixel = ((0.30 * p.0[0] as f32 + 0.59 * p.0[1] as f32 + 0.11 * p.0[2] as f32) * 15.
            / 255.) as u8;

        dest[idx / 2] |= (pixel & 0xF) << ((!col & 0x1) * 4);
    }

    dest
}

pub fn encode_pixels_intensity_8(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for (x, y) in PixelBlockIterator::new(width, height, 8, 4) {
        let p = image.get_pixel(x, y);

        let pixel = (0.30 * p.0[0] as f32 + 0.59 * p.0[1] as f32 + 0.11 * p.0[2] as f32) as u8;

        dest.push(pixel);
    }

    dest
}

pub fn encode_pixels_with_palette_index8(
    image: &RgbaImage,
    palette_pixel_format: PixelFormat,
) -> Result<Vec<u8>, imagequant::Error> {
    let width = image.width();
    let height = image.height();

    let (palette, indices) = palettize_image(image, INDEX8_PALETTE_SIZE, palette_pixel_format)?;
    let mut result = encode_palette(palette, palette_pixel_format);

    for (x, y) in PixelBlockIterator::new(width, height, 8, 4) {
        let src_idx = y * width + x;
        result.push(indices[src_idx as usize]);
    }

    Ok(result)
}

pub fn encode_pixels_with_palette_index4(
    image: &RgbaImage,
    palette_pixel_format: PixelFormat,
) -> Result<Vec<u8>, imagequant::Error> {
    let width = image.width();
    let height = image.height();

    let (palette, indices) = palettize_image(image, INDEX4_PALETTE_SIZE, palette_pixel_format)?;
    let mut result = encode_palette(palette, palette_pixel_format);

    // Resize vec to fill entire image data size (with palette)
    let cur_len = result.len();
    result.resize(cur_len + (width * height / 2) as usize, 0);

    for (dest_idx, (_, col, x, y)) in PixelBlockIteratorExt::new(width, height, 8, 8).enumerate() {
        let src_idx = y * width + x;
        result[cur_len + dest_idx / 2] |= (indices[src_idx as usize] & 0xF) << ((!col & 0x1) * 4);
    }

    Ok(result)
}

////////////////////////
// Decoding Functions //
////////////////////////

fn decode_pixel_rgb5a3(pixel: u16) -> Rgba<u8> {
    if (pixel & 0x8000) != 0 {
        // Rgb555
        let r = ((((pixel >> 10) & 0x1F) as f32) * 255. / 31.) as u8;
        let g = ((((pixel >> 5) & 0x1F) as f32) * 255. / 31.) as u8;
        let b = (((pixel & 0x1F) as f32) * 255. / 31.) as u8;
        [r, g, b, 0xFF].into()
    } else {
        // Argb3444
        let r = ((((pixel >> 8) & 0x0F) as f32) * 255. / 15.) as u8;
        let g = ((((pixel >> 4) & 0x0F) as f32) * 255. / 15.) as u8;
        let b = (((pixel & 0x0F) as f32) * 255. / 15.) as u8;
        let a = ((((pixel >> 12) & 0x07) as f32) * 255. / 7.) as u8;
        [r, g, b, a].into()
    }
}

fn decode_pixel_rgb565(pixel: u16) -> Rgba<u8> {
    let r = ((((pixel >> 11) & 0x1F) as f32) * 255. / 31.) as u8;
    let g = ((((pixel >> 5) & 0x3F) as f32) * 255. / 63.) as u8;
    let b = (((pixel & 0x1F) as f32) * 255. / 31.) as u8;
    [r, g, b, 0xFF].into()
}

fn decode_pixel_intensity_alpha8(pixel: u8, alpha: u8) -> Rgba<u8> {
    [pixel, pixel, pixel, alpha].into()
}

pub fn decode_pixels_rgb5a3(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let pixel = cursor.read_u16::<BigEndian>()?;
        image.put_pixel(x, y, decode_pixel_rgb5a3(pixel));
    }

    Ok(image)
}

pub fn decode_pixels_rgb565(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let pixel = cursor.read_u16::<BigEndian>()?;
        image.put_pixel(x, y, decode_pixel_rgb565(pixel));
    }

    Ok(image)
}

pub fn decode_pixels_argb8888(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);

    let mut src_idx = 0;

    for (block, _, x, y) in PixelBlockIteratorExt::new(width, height, 4, 4) {
        let cur_idx = (src_idx + block * 32) as usize;

        let a = data[cur_idx];
        let r = data[cur_idx + 1];
        let g = data[cur_idx + 32];
        let b = data[cur_idx + 33];

        image.put_pixel(x, y, [r, g, b, a].into());

        src_idx += 2;
    }

    Ok(image)
}

pub fn decode_pixels_intensity_alpha8(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let alpha = cursor.read_u8()?;
        let pixel = cursor.read_u8()?;
        image.put_pixel(x, y, decode_pixel_intensity_alpha8(pixel, alpha));
    }

    Ok(image)
}

pub fn decode_pixels_intensity_alpha4(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    for (x, y) in PixelBlockIterator::new(width, height, 8, 4) {
        let pixel = cursor.read_u8()?;

        let c = ((pixel & 0x0F) as f32 * 255. / 15.) as u8;
        let a = (((pixel >> 4) & 0x0F) as f32 * 255. / 15.) as u8;

        image.put_pixel(x, y, [c, c, c, a].into());
    }

    Ok(image)
}

pub fn decode_pixels_intensity_8(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    for (x, y) in PixelBlockIterator::new(width, height, 8, 4) {
        let c = cursor.read_u8()?;
        image.put_pixel(x, y, [c, c, c, 0xFF].into());
    }

    Ok(image)
}

pub fn decode_pixels_intensity_4(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);

    for (idx, (_, col, x, y)) in PixelBlockIteratorExt::new(width, height, 8, 8).enumerate() {
        let pixel = (data[idx / 2] >> ((!col & 0x1) * 4)) & 0x0F;
        let c = (pixel as f32 * 255. / 15.) as u8;
        image.put_pixel(x, y, [c, c, c, 0xFF].into());
    }

    Ok(image)
}

pub fn decode_pixels_with_palette_index8(
    data: &[u8],
    width: u32,
    height: u32,
    palette_pixel_format: PixelFormat,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    let palette = decode_palette(&mut cursor, palette_pixel_format, INDEX8_PALETTE_SIZE)?;

    for (x, y) in PixelBlockIterator::new(width, height, 8, 4) {
        let palette_idx = cursor.read_u8()?;
        image.put_pixel(x, y, palette[palette_idx as usize]);
    }

    Ok(image)
}

pub fn decode_pixels_with_palette_index4(
    data: &[u8],
    width: u32,
    height: u32,
    palette_pixel_format: PixelFormat,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    let palette = decode_palette(&mut cursor, palette_pixel_format, INDEX4_PALETTE_SIZE)?;
    const PALETTE_SIZE_BYTES: usize = INDEX4_PALETTE_SIZE as usize * size_of::<u16>();

    for (idx, (_, col, x, y)) in PixelBlockIteratorExt::new(width, height, 8, 8).enumerate() {
        let palette_idx =
            (data[PALETTE_SIZE_BYTES + (idx / 2)] >> ((col % 2 == 0) as u8 * 4)) & 0x0F;
        image.put_pixel(x, y, palette[palette_idx as usize]);
    }

    Ok(image)
}

pub fn decode_pixels_dxt1(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);
    let mut src_idx = 0;
    let colors: &mut [Rgba<u8>] = &mut [[0, 0, 0, 0].into(); 4];

    for (x, y) in DecodeDxtBlockIterator::new(width, height) {
        cursor.seek(std::io::SeekFrom::Start(src_idx))?;
        let encoded_1 = cursor.read_u16::<BigEndian>()?;
        let encoded_2 = cursor.read_u16::<BigEndian>()?;

        colors[0] = decode_pixel_rgb565(encoded_1);
        colors[1] = decode_pixel_rgb565(encoded_2);

        if encoded_1 > encoded_2 {
            colors[2] = [
                ((colors[0].0[0] as u32 * 2 + colors[1].0[0] as u32) / 3) as u8,
                ((colors[0].0[1] as u32 * 2 + colors[1].0[1] as u32) / 3) as u8,
                ((colors[0].0[2] as u32 * 2 + colors[1].0[2] as u32) / 3) as u8,
                0xFF,
            ]
            .into();

            colors[3] = [
                ((colors[1].0[0] as u32 * 2 + colors[0].0[0] as u32) / 3) as u8,
                ((colors[1].0[1] as u32 * 2 + colors[0].0[1] as u32) / 3) as u8,
                ((colors[1].0[2] as u32 * 2 + colors[0].0[2] as u32) / 3) as u8,
                0xFF,
            ]
            .into();
        } else {
            colors[2] = [
                ((colors[0].0[0] as u32 + colors[1].0[0] as u32) / 2) as u8,
                ((colors[0].0[1] as u32 + colors[1].0[1] as u32) / 2) as u8,
                ((colors[0].0[2] as u32 + colors[1].0[2] as u32) / 2) as u8,
                0xFF,
            ]
            .into();

            colors[3] = [0, 0, 0, 0].into();
        }

        src_idx += 4;

        for y2 in (0..4).take_while(|i| y + i < height) {
            for x2 in (0..4).take_while(|i| x + i < width) {
                let color_idx = (data[(src_idx + y2 as u64) as usize] >> (6 - x2 * 2)) & 0x3;
                image.put_pixel(x + x2, y + y2, colors[color_idx as usize]);
            }
        }

        src_idx += 4;
    }

    Ok(image)
}
