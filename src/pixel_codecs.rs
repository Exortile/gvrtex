use std::io::Cursor;

use crate::iter::{PixelBlockIterator, PixelBlockIteratorExt};
use byteorder::{BigEndian, ReadBytesExt};
use image::RgbaImage;

pub fn encode_pixels_rgb5a3(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height * 2).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for (x, y) in PixelBlockIterator::new(width, height, 4, 4) {
        let p = image.get_pixel(x, y);

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

        let mut pixel: u16 = 0x0000;
        pixel |= ((p.0[0] >> 3) as u16) << 11;
        pixel |= ((p.0[1] >> 2) as u16) << 5;
        pixel |= (p.0[2] >> 3) as u16;

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

        let pixel = (0.30 * p.0[0] as f32 + 0.59 * p.0[1] as f32 + 0.11 * p.0[2] as f32) as u8;

        dest.push(p.0[3]);
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
