use crate::error::*;
use crate::formats::{DataFlags, DataFormat, PixelFormat, TextureType};
use crate::pixel_codecs::*;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use codec::GvrEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageReader, RgbaImage};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Not;

mod codec;
pub mod error;
pub mod formats;
mod iter;
mod pixel_codecs;

#[derive(Default)]
pub struct TextureEncoder {
    texture_type: TextureType,
    pixel_format: PixelFormat,
    data_format: DataFormat,
    data_flags: DataFlags,
    global_index: u32,
}

impl TextureEncoder {
    fn check_given_formats(data_format: DataFormat) -> Result<(), TextureEncodeError> {
        match data_format {
            DataFormat::Index4 | DataFormat::Index8 => Err(TextureEncodeError::Format),
            _ => Ok(()),
        }
    }

    fn check_given_formats_palettized(data_format: DataFormat) -> Result<(), TextureEncodeError> {
        match data_format {
            DataFormat::Index4 | DataFormat::Index8 => Ok(()),
            _ => Err(TextureEncodeError::Format),
        }
    }

    pub fn new_gcix_palettized(
        pixel_format: PixelFormat,
        data_format: DataFormat,
    ) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats_palettized(data_format)?;

        Ok(Self {
            texture_type: TextureType::GCIX,
            pixel_format,
            data_format,
            data_flags: DataFlags::InternalPalette,
            ..Default::default()
        })
    }

    pub fn new_gcix(data_format: DataFormat) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats(data_format)?;

        Ok(Self {
            texture_type: TextureType::GCIX,
            data_format,
            ..Default::default()
        })
    }

    pub fn new_gbix_palettized(
        pixel_format: PixelFormat,
        data_format: DataFormat,
    ) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats_palettized(data_format)?;

        Ok(Self {
            texture_type: TextureType::GBIX,
            pixel_format,
            data_format,
            data_flags: DataFlags::InternalPalette,
            ..Default::default()
        })
    }

    pub fn new_gbix(data_format: DataFormat) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats(data_format)?;

        Ok(Self {
            texture_type: TextureType::GBIX,
            data_format,
            ..Default::default()
        })
    }

    pub fn with_mipmaps(mut self) -> Result<Self, TextureEncodeError> {
        match self.data_format {
            DataFormat::Dxt1 | DataFormat::Rgb565 | DataFormat::Rgb5a3 => {
                self.data_flags.set(DataFlags::Mipmaps, true);
                Ok(self)
            }
            _ => Err(TextureEncodeError::Mipmap),
        }
    }

    pub fn with_global_index(mut self, global_index: u32) -> Self {
        self.global_index = global_index;
        self
    }

    fn encode_mipmaps(&self, img: &RgbaImage, encoder: &dyn GvrEncoder) -> Vec<u8> {
        let mut mipmaps: Vec<u8> = vec![];
        let mipmap_count = img.width().ilog2();
        let mut tex_size = img.width() / 2;

        for _ in 0..mipmap_count {
            if tex_size < 1 {
                break;
            }

            let mipmap = DynamicImage::ImageRgba8(img.clone()).resize_exact(
                tex_size,
                tex_size,
                FilterType::Triangle,
            );

            let mut encoded = encoder.encode(&mipmap.into_rgba8());

            if encoded.len() < 32 {
                encoded.resize(32, 0);
            }

            mipmaps.append(&mut encoded);
            tex_size /= 2;
        }

        mipmaps
    }

    pub fn encode(&mut self, img_path: &str) -> Result<Vec<u8>, TextureEncodeError> {
        let mut result = Vec::new();
        let img = ImageReader::open(img_path)?.decode()?;
        let rgba_img = img.into_rgba8();

        let mut encoded;
        if self.data_flags.intersects(DataFlags::InternalPalette) {
            let encoder = create_new_encoder_with_palette(self.data_format);
            encoder.validate_input(&rgba_img)?;
            encoded = encoder.encode(&rgba_img, self.pixel_format)?;
        } else {
            let encoder = create_new_encoder(self.data_format);
            encoder.validate_input(&rgba_img)?;
            encoded = encoder.encode(&rgba_img);

            if self.data_flags.intersects(DataFlags::Mipmaps) {
                let mut encoded_mipmaps = self.encode_mipmaps(&rgba_img, &*encoder);
                encoded.append(&mut encoded_mipmaps);
            }
        }

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
        buf.write_u32::<BigEndian>(self.global_index)?;
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

#[derive(Default)]
pub struct TextureDecoder {
    cursor: Cursor<Vec<u8>>,
    image: Option<RgbaImage>,
}

impl TextureDecoder {
    /// Instantiate a new [`TextureDecoder`], that can decode the file in the given `gvr_path`,
    /// reading the file's contents.
    ///
    /// This function doesn't decode the file by itself, [`Self::decode()`] must be called.
    pub fn new(gvr_path: &str) -> Result<Self, std::io::Error> {
        Ok(Self {
            cursor: Cursor::new(std::fs::read(gvr_path)?),
            ..Default::default()
        })
    }

    /// Decodes the given image from [`Self::new()`].
    ///
    /// If something goes wrong while decoding, or the given file is not a valid GVR texture file,
    /// a [`TextureDecodeError`] is returned.
    pub fn decode(&mut self) -> Result<(), TextureDecodeError> {
        self.is_valid_gvr()?;

        self.cursor.seek(SeekFrom::Start(0x14))?;
        let data_len = (self.cursor.read_u32::<LittleEndian>()? - 8)
            .try_into()
            .unwrap();

        self.cursor.seek(SeekFrom::Start(0x1A))?;

        let flags = self.cursor.read_u8()?;
        let Some(data_flags) = DataFlags::from_bits(flags & 0xF) else {
            return Err(TextureDecodeError::InvalidFile);
        };
        let Ok(palette_format) = PixelFormat::try_from((flags >> 4) & 0xF) else {
            return Err(TextureDecodeError::InvalidFile);
        };

        let data_format: DataFormat = DataFormat::try_from(self.cursor.read_u8()?)?;

        if data_flags.intersects(DataFlags::ExternalPalette) {
            unimplemented!();
        }

        // Check if data format is matching if a palette is included
        if data_flags.intersects(DataFlags::InternalPalette)
            && matches!(data_format, DataFormat::Index4 | DataFormat::Index8).not()
        {
            return Err(TextureDecodeError::InvalidFile);
        }

        let width = self.cursor.read_u16::<BigEndian>()?;
        let height = self.cursor.read_u16::<BigEndian>()?;

        let mut data: Vec<u8> = Vec::with_capacity(data_len);
        let read_size = self.cursor.read_to_end(&mut data)?;
        if read_size != data_len {
            return Err(TextureDecodeError::InvalidFile);
        }

        if data_flags.intersects(DataFlags::InternalPalette) {
            let decoder = create_new_decoder_with_palette(data_format);
            self.image =
                Some(decoder.decode(&data, width.into(), height.into(), palette_format)?);
        } else {
            let decoder = create_new_decoder(data_format);
            self.image = Some(decoder.decode(&data, width.into(), height.into())?);
        }

        Ok(())
    }

    /// Checks if the decode process has concluded successfully.
    pub fn is_decoded(&self) -> bool {
        self.image.is_some()
    }

    /// Borrows the decoded image, if [`Self::decode()`] has ran successfully.
    pub fn as_decoded(&self) -> &Option<RgbaImage> {
        &self.image
    }

    /// Returns the decoded image, if [`Self::decode()`] has ran successfully, consuming `self`.
    ///
    /// If the image hasn't been decoded yet, a [`TextureDecodeError::Undecoded`] is returned.
    pub fn into_decoded(self) -> Result<RgbaImage, TextureDecodeError> {
        if let Some(image) = self.image {
            Ok(image)
        } else {
            Err(TextureDecodeError::Undecoded)
        }
    }

    /// Saves the currently decoded image into a file, with a format of your choice.
    /// The format the file is saved in is derived from the file extension (.png, .jpg, etc.)
    /// in the given `path`.
    ///
    /// If the image hasn't been decoded yet, a [`TextureDecodeError::Undecoded`] is returned.
    pub fn save(&self, path: &str) -> Result<(), TextureDecodeError> {
        if self.image.is_none() {
            return Err(TextureDecodeError::Undecoded);
        }
        self.image.as_ref().unwrap().save(path)?;
        Ok(())
    }

    fn read_string(&mut self, len: usize) -> Result<String, std::io::Error> {
        let mut buf = vec![0; len];
        self.cursor.read_exact(&mut buf)?;

        let char_buf: Vec<char> = buf.into_iter().map(|e| e as char).collect();
        let result: String = char_buf.into_iter().collect();
        Ok(result)
    }

    /// This function checks if the magic strings "GCIX" and "GVRT" in the file match.
    /// It doesn't check the actual validity of the data in the headers, that's done in
    /// [`Self::decode()`]
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
