use crate::formats::PixelFormat;
use crate::TextureEncodeError;
use image::RgbaImage;

pub trait GvrEncoder {
    fn validate_input(&self, image: &RgbaImage) -> Result<(), TextureEncodeError>;
    fn encode(&self, image: &RgbaImage) -> Vec<u8>;
}

pub trait GvrEncoderPalette {
    fn validate_input(&self, image: &RgbaImage) -> Result<(), TextureEncodeError>;
    fn encode(&self, image: &RgbaImage, palette_pixel_format: PixelFormat) -> Vec<u8>;
}

pub trait GvrDecoder {
    fn decode(&self, data: &[u8], width: u32, height: u32) -> Result<RgbaImage, std::io::Error>;
}

pub trait GvrDecoderPalette {
    fn decode(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        palette_pixel_format: PixelFormat,
    ) -> Result<RgbaImage, std::io::Error>;
}
