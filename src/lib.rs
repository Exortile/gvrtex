use crate::formats::{DataFlags, DataFormat, PixelFormat, TextureType};
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use image::{ImageError, ImageReader, RgbaImage};
use std::io::Write;

pub mod formats;

/// Provides the internal implementation for a [`Iterator::next()`] function, catered to the pixel
/// block iterators.
///
/// This macro allows adding a block of statements on each iteration of a full block, which is
/// needed in [`BlockPixelIteratorExt`].
///
/// # Metavariables
///
/// * `$iter` - The iterator data. Should be a binding to [`BlockPixelIterator`]
/// * `$next_point` - The expression to use for returning the next point out of the iterator.
/// * `$each_block` - The block of statements that gets run on each full block iteration.
macro_rules! impl_blockpixeliterator {
    ($iter:ident, $next_point:expr, $each_block:block) => {
        {
            if $iter.y_block >= $iter.height {
                return None;
            }

            let next_point = $next_point;

            $iter.x += 1;
            if $iter.x == $iter.x_block_size {
                $iter.x = 0;
                $iter.y += 1;
            } else {
                return Some(next_point);
            }

            if $iter.y == $iter.y_block_size {
                $iter.y = 0;

                $each_block

                $iter.x_block += $iter.x_block_size;
            } else {
                return Some(next_point);
            }

            if $iter.x_block >= $iter.width {
                $iter.x_block = 0;
                $iter.y_block += $iter.y_block_size;
            }

            Some(next_point)
        }
    };
}

/// Iterates through an image of the given width and height in 4x4 blocks instead of singular
/// pixels. The iterator returns the x and y coordinate as a tuple on each iteration.
///
/// It works by iterating through a block row by row, before moving on to the next block, which it
/// also iterates through row by row until the end of the image.
struct BlockPixelIterator {
    width: u32,
    height: u32,
    x_block_size: u32,
    y_block_size: u32,

    x_block: u32,
    y_block: u32,
    x: u32,
    y: u32,
}

impl BlockPixelIterator {
    pub fn new(width: u32, height: u32, x_block_size: u32, y_block_size: u32) -> Self {
        Self {
            width,
            height,
            x_block_size,
            y_block_size,

            x_block: 0,
            y_block: 0,
            x: 0,
            y: 0,
        }
    }
}

impl Iterator for BlockPixelIterator {
    type Item = (u32, u32);

    /// Iterates over each pixel, returning the x and y coordinate of the next pixel as a tuple.
    fn next(&mut self) -> Option<Self::Item> {
        impl_blockpixeliterator!(self, (self.x_block + self.x, self.y_block + self.y), {})
    }
}

/// See [`BlockPixelIterator`] for specifics on how this iterator works.
///
/// This is an extension upon that iterator, that also returns the amount of blocks that have been
/// processed thus far, which some encodings need.
struct BlockPixelIteratorExt {
    iterator: BlockPixelIterator,
    blocks: u32,
}

impl BlockPixelIteratorExt {
    pub fn new(width: u32, height: u32, x_block_size: u32, y_block_size: u32) -> Self {
        Self {
            iterator: BlockPixelIterator::new(width, height, x_block_size, y_block_size),
            blocks: 0,
        }
    }
}

impl Iterator for BlockPixelIteratorExt {
    type Item = (u32, u32, u32);

    /// Iterates over each pixel, returning the x and y coordinate of the next pixel as a tuple.
    fn next(&mut self) -> Option<Self::Item> {
        let iter = &mut self.iterator;
        impl_blockpixeliterator!(
            iter,
            (self.blocks, iter.x_block + iter.x, iter.y_block + iter.y),
            {
                self.blocks += 1;
            }
        )
    }
}

fn encode_pixels_rgb5a3(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height * 2).try_into().unwrap();
    let mut dest: Vec<u8> = Vec::with_capacity(dest_size);

    for (x, y) in BlockPixelIterator::new(width, height, 4, 4) {
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

fn encode_pixels_argb8888(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let dest_size = (width * height * 4).try_into().unwrap();
    let mut dest = vec![0u8; dest_size];

    let mut dest_idx = 0;

    for (block, x, y) in BlockPixelIteratorExt::new(width, height, 4, 4) {
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

#[derive(Default)]
pub struct TextureEncoder {
    texture_type: TextureType,
    pixel_format: PixelFormat,
    data_format: DataFormat,
    data_flags: DataFlags,
}

impl TextureEncoder {
    pub fn new_gcix_palettized(pixel_format: PixelFormat, data_format: DataFormat) -> Self {
        Self {
            texture_type: TextureType::GCIX,
            pixel_format,
            data_format,
            data_flags: DataFlags::None,
        }
    }

    pub fn new_gcix(data_format: DataFormat) -> Self {
        Self {
            texture_type: TextureType::GCIX,
            data_format,
            data_flags: DataFlags::None,
            ..Default::default()
        }
    }

    pub fn encode(&mut self, img_path: &str) -> Result<Vec<u8>, ImageError> {
        let mut result = Vec::new();
        let img = ImageReader::open(img_path)?.decode()?;
        let rgba_img = img.into_rgba8();

        let encoded = match self.data_format {
            DataFormat::Rgb5a3 => encode_pixels_rgb5a3(&rgba_img),
            DataFormat::Argb8888 => encode_pixels_argb8888(&rgba_img),
            _ => unimplemented!(),
        };

        self.write_header(&rgba_img, &encoded, &mut result)?;
        result.write_all(&encoded)?;

        Ok(result)
    }

    fn write_header(
        &self,
        image: &RgbaImage,
        encoded: &[u8],
        buf: &mut Vec<u8>,
    ) -> std::io::Result<()> {
        buf.write_all(b"GCIX")?;
        buf.write_u32::<LittleEndian>(8)?;
        buf.resize(0x10, 0); // padding

        buf.write_all(b"GVRT")?;
        buf.write_u32::<LittleEndian>((encoded.len() + 8).try_into().unwrap())?;
        buf.write_u16::<LittleEndian>(0)?; // padding

        buf.write_u8(0)?;
        buf.write_u8(self.data_format.into())?;
        buf.write_u16::<BigEndian>(image.width().try_into().unwrap())?;
        buf.write_u16::<BigEndian>(image.height().try_into().unwrap())?;

        Ok(())
    }
}
