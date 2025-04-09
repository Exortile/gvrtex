use crate::formats::{DataFlags, DataFormat, PixelFormat, TextureType};
use crate::pixel_codecs::*;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use image::{ImageError, ImageReader, ImageResult, RgbaImage};
use std::error::Error;
use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub mod formats;
mod iter;
mod pixel_codecs;

#[derive(Debug)]
pub enum TextureEncodeError {
    EncodeError(ImageError),
    PaletteError(imagequant::Error),
}

impl Error for TextureEncodeError {}

impl fmt::Display for TextureEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EncodeError(err) => write!(f, "{err}"),
            Self::PaletteError(err) => write!(f, "{err}"),
        }
    }
}

impl From<ImageError> for TextureEncodeError {
    fn from(value: ImageError) -> Self {
        Self::EncodeError(value)
    }
}

impl From<imagequant::Error> for TextureEncodeError {
    fn from(value: imagequant::Error) -> Self {
        Self::PaletteError(value)
    }
}

impl From<std::io::Error> for TextureEncodeError {
    fn from(value: std::io::Error) -> Self {
        Self::EncodeError(ImageError::IoError(value))
    }
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
            data_flags: DataFlags::InternalPalette,
        }
    }

    pub fn new_gcix(data_format: DataFormat) -> Self {
        Self {
            texture_type: TextureType::GCIX,
            data_format,
            ..Default::default()
        }
    }

    pub fn new_gbix_palettized(pixel_format: PixelFormat, data_format: DataFormat) -> Self {
        Self {
            texture_type: TextureType::GBIX,
            pixel_format,
            data_format,
            data_flags: DataFlags::InternalPalette,
        }
    }

    pub fn new_gbix(data_format: DataFormat) -> Self {
        Self {
            texture_type: TextureType::GBIX,
            data_format,
            ..Default::default()
        }
    }

    pub fn encode(&mut self, img_path: &str) -> Result<Vec<u8>, TextureEncodeError> {
        let mut result = Vec::new();
        let img = ImageReader::open(img_path)?.decode()?;
        let rgba_img = img.into_rgba8();

        let encoded = match self.data_format {
            DataFormat::Rgb565 => encode_pixels_rgb565(&rgba_img),
            DataFormat::Rgb5a3 => encode_pixels_rgb5a3(&rgba_img),
            DataFormat::Argb8888 => encode_pixels_argb8888(&rgba_img),
            DataFormat::IntensityA4 => encode_pixels_intensity_alpha4(&rgba_img),
            DataFormat::IntensityA8 => encode_pixels_intensity_alpha8(&rgba_img),
            DataFormat::Intensity4 => encode_pixels_intensity_4(&rgba_img),
            DataFormat::Intensity8 => encode_pixels_intensity_8(&rgba_img),
            DataFormat::Index8 => encode_pixels_with_palette_index8(&rgba_img, self.pixel_format)?,
            DataFormat::Index4 => encode_pixels_with_palette_index4(&rgba_img, self.pixel_format)?,
            DataFormat::Dxt1 => encode_pixels_dxt1(&rgba_img),
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
        if self.texture_type == TextureType::GCIX {
            buf.write_all(b"GCIX")?;
        } else {
            buf.write_all(b"GBIX")?;
        }
        buf.write_u32::<LittleEndian>(8)?;
        buf.resize(0x10, 0); // padding

        buf.write_all(b"GVRT")?;
        buf.write_u32::<LittleEndian>((encoded.len() + 8).try_into().unwrap())?;
        buf.write_u16::<LittleEndian>(0)?; // padding

        let pixel_format = (self.pixel_format as u8) << 4;
        let data_flags: u8 = self.data_flags.into();
        let flags = pixel_format | data_flags;

        buf.write_u8(flags)?;
        buf.write_u8(self.data_format.into())?;
        buf.write_u16::<BigEndian>(image.width().try_into().unwrap())?;
        buf.write_u16::<BigEndian>(image.height().try_into().unwrap())?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum TextureDecodeError {
    InvalidFile,
    ParseError(&'static str),
    IoError(std::io::Error),
}

impl Error for TextureDecodeError {}

impl fmt::Display for TextureDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFile => write!(f, "The given file is an invalid GVR texture file."),
            Self::IoError(err) => write!(f, "{err}"),
            Self::ParseError(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<std::io::Error> for TextureDecodeError {
    fn from(value: std::io::Error) -> Self {
        TextureDecodeError::IoError(value)
    }
}

impl From<&'static str> for TextureDecodeError {
    fn from(value: &'static str) -> Self {
        TextureDecodeError::ParseError(value)
    }
}

#[derive(Default)]
pub struct TextureDecoder {
    cursor: Cursor<Vec<u8>>,
    image: RgbaImage,
}

impl TextureDecoder {
    pub fn new(gvr_path: &str) -> Result<Self, std::io::Error> {
        Ok(Self {
            cursor: Cursor::new(std::fs::read(gvr_path)?),
            ..Default::default()
        })
    }

    pub fn decode(&mut self) -> Result<(), TextureDecodeError> {
        self.is_valid_gvr()?;

        self.cursor.seek(SeekFrom::Start(0x14))?;
        let data_len = (self.cursor.read_u32::<LittleEndian>()? - 8)
            .try_into()
            .unwrap();

        self.cursor.seek(SeekFrom::Start(0x1B))?;
        let data_format: DataFormat = DataFormat::try_from(self.cursor.read_u8()?)?;
        let width = self.cursor.read_u16::<BigEndian>()?;
        let height = self.cursor.read_u16::<BigEndian>()?;

        let mut data: Vec<u8> = Vec::with_capacity(data_len);
        let read_size = self.cursor.read_to_end(&mut data)?;
        if read_size != data_len {
            return Err(TextureDecodeError::InvalidFile);
        }

        self.image = match data_format {
            DataFormat::Rgb5a3 => decode_pixels_rgb5a3(&data, width.into(), height.into())?,
            _ => unimplemented!(),
        };

        Ok(())
    }

    pub fn save(&self, path: &str) -> ImageResult<()> {
        self.image.save(path)
    }

    fn read_string(&mut self, len: usize) -> Result<String, std::io::Error> {
        let mut buf = vec![0; len];
        self.cursor.read_exact(&mut buf)?;

        let char_buf: Vec<char> = buf.into_iter().map(|e| e as char).collect();
        let result: String = char_buf.into_iter().collect();
        Ok(result)
    }

    fn is_valid_gvr(&mut self) -> Result<(), TextureDecodeError> {
        let type_magic = self.read_string(4)?;
        if type_magic != "GCIX" && type_magic != "GBIX" {
            return Err(TextureDecodeError::InvalidFile);
        }

        self.cursor.seek(SeekFrom::Start(0x10))?;
        let tex_magic = self.read_string(4)?;
        if tex_magic != "GVRT" {
            return Err(TextureDecodeError::InvalidFile);
        }
        Ok(())
    }
}
