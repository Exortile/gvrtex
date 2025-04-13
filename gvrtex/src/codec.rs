use crate::formats::PixelFormat;
use crate::TextureEncodeError;
use image::RgbaImage;

pub trait GvrBase {
    fn get_block_size(&self) -> (u32, u32);
}

pub trait GvrEncoderBase: GvrBase {
    fn validate_input(&self, image: &RgbaImage) -> Result<(), TextureEncodeError> {
        let (x_block_size, y_block_size) = self.get_block_size();
        let biggest_block = x_block_size.max(y_block_size);

        let width = image.width();
        let height = image.height();

        if width < x_block_size || height < y_block_size {
            return Err(TextureEncodeError::SmallDimensions(
                width,
                height,
                x_block_size,
                y_block_size,
            ));
        }

        if width % biggest_block != 0 || height % biggest_block != 0 {
            return Err(TextureEncodeError::InvalidDimensions(
                width,
                height,
                biggest_block,
            ));
        }

        Ok(())
    }
}

pub trait GvrEncoder: GvrEncoderBase {
    fn encode(&self, image: &RgbaImage) -> Vec<u8>;
}

pub trait GvrEncoderPalette: GvrEncoderBase {
    fn encode(
        &self,
        image: &RgbaImage,
        palette_pixel_format: PixelFormat,
    ) -> Result<Vec<u8>, imagequant::Error>;
}

pub trait GvrDecoder: GvrBase {
    fn decode(&self, data: &[u8], width: u32, height: u32) -> Result<RgbaImage, std::io::Error>;
}

pub trait GvrDecoderPalette: GvrBase {
    fn decode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        palette_pixel_format: PixelFormat,
    ) -> Result<RgbaImage, std::io::Error>;
}
