use std::io::Cursor;

use crate::{
    formats::PixelFormat,
    iter::{DxtBlockIterator, PixelBlockIterator, PixelBlockIteratorExt},
};
use byteorder::{BigEndian, ReadBytesExt};
use image::{Pixel, Rgba, RgbaImage};

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

    for block in DxtBlockIterator::new(image) {
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

    let (palette, indices) = palettize_image(image, 256, palette_pixel_format)?;
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

    let (palette, indices) = palettize_image(image, 16, palette_pixel_format)?;
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

pub fn decode_pixels_rgb5a3(
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<RgbaImage, std::io::Error> {
    let mut image = RgbaImage::new(width, height);
    let mut cursor = Cursor::new(data);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let pixel = cursor.read_u16::<BigEndian>()?;

        if (pixel & 0x8000) != 0 {
            // Rgb555
            let r = ((((pixel >> 10) & 0x1F) as f32) * 255. / 31.) as u8;
            let g = ((((pixel >> 5) & 0x1F) as f32) * 255. / 31.) as u8;
            let b = (((pixel & 0x1F) as f32) * 255. / 31.) as u8;
            image.put_pixel(x, y, [r, g, b, 0xFF].into());
        } else {
            // Argb3444
            let r = ((((pixel >> 8) & 0x0F) as f32) * 255. / 15.) as u8;
            let g = ((((pixel >> 4) & 0x0F) as f32) * 255. / 15.) as u8;
            let b = (((pixel & 0x0F) as f32) * 255. / 15.) as u8;
            let a = ((((pixel >> 12) & 0x07) as f32) * 255. / 7.) as u8;
            image.put_pixel(x, y, [r, g, b, a].into());
        }
    }

    Ok(image)
}
