//! gvrtex is a Rust library for interfacing with the GVR texture format used
//! in GameCube/Wii games, for example Sonic Riders.
//!
//! It's essentially the same as a regular TPL texture file (the official texture
//! file format for GameCube/Wii), but with GVR headers instead. The image data
//! in the file remains the same as it is for TPL files, so that the game console can read it.
//!
//! # Examples
//!
//! Here's a few examples on how to encode and decode GVR texture files.
//!
//! Encoding an image into a GVR file:
//!
//! ```no_run
//! use gvrtex::error::TextureEncodeError;
//! use gvrtex::formats::DataFormat;
//! use gvrtex::TextureEncoder;
//!
//! # fn main() -> Result<Vec<u8>, TextureEncodeError> {
//! # let img_path: &str = "";
//! let mut encoder = TextureEncoder::new_gcix(DataFormat::Dxt1)?;
//! let encoded_file = encoder.encode(img_path)?;
//! # Ok(encoded_file)
//! # }
//! ```
//!
//! Decoding a GVR file:
//!
//! ```no_run
//! use gvrtex::error::TextureDecodeError;
//! use gvrtex::TextureDecoder;
//!
//! # fn main() -> Result<(), TextureDecodeError> {
//! # let gvr_path: &str = "";
//! # let save_path: &str = "";
//! // Reads the contents of the file in gvr_path, but doesn't decode it yet.
//! let mut decoder = TextureDecoder::new(gvr_path)?;
//!
//! // Decode file, saving the result in the decoder
//! decoder.decode()?;
//!
//! // Save the decoded image to the given path. The image format is derived from the file
//! // extension in the path.
//! decoder.save(save_path)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Hints
//!
//! Easiest place to start off is to look at [`TextureEncoder`] for encoding GVR textures and
//! [`TextureDecoder`] for decoding GVR textures.

#![warn(missing_docs)]

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

/// Provides all the functionality needed to encode a GVR texture file.
///
/// The encoder doesn't inherently provide a method to save the texture into a file, you will be
/// given a [`Vec`] of bytes from [`Self::encode()`], which you can use and save all the bytes to a
/// file yourself.
///
/// For examples, see the documentation on the root of the [`crate`]
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

    /// Creates a new encoder, that encodes palettized GVR texture files using the given `data_format`
    /// and `pixel_format`.
    ///
    /// This specific function sets the magic strings in the header of the encoded texture file to
    /// "GCIX".
    ///
    /// # Errors
    ///
    /// This function will return a [`TextureEncodeError::Format`] if you pass in a data format
    /// that isn't [`DataFormat::Index4`] or [`DataFormat::Index8`]. If you want to encode textures
    /// that you don't want to generate a color palette for, see [`Self::new_gcix()`].
    pub fn new_gcix_palettized(
        pixel_format: PixelFormat,
        data_format: DataFormat,
    ) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats_palettized(data_format)?;

        Ok(Self {
            texture_type: TextureType::Gcix,
            pixel_format,
            data_format,
            data_flags: DataFlags::InternalPalette,
            ..Default::default()
        })
    }

    /// Creates a new encoder, that encodes GVR texture files using the given `data_format`.
    ///
    /// This specific function sets the magic strings in the header of the encoded texture file to
    /// "GCIX".
    ///
    /// # Errors
    ///
    /// This function will return a [`TextureEncodeError::Format`] if you pass in a data format
    /// that is [`DataFormat::Index4`] or [`DataFormat::Index8`]. If you want to encode textures
    /// that you want to generate a color palette for, see [`Self::new_gcix_palettized()`], as that
    /// allows you to set the data format for the color palette as well.
    pub fn new_gcix(data_format: DataFormat) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats(data_format)?;

        Ok(Self {
            texture_type: TextureType::Gcix,
            data_format,
            ..Default::default()
        })
    }

    /// Creates a new encoder, that encodes palettized GVR texture files using the given `data_format`
    /// and `pixel_format`.
    ///
    /// This specific function sets the magic strings in the header of the encoded texture file to
    /// "GBIX".
    ///
    /// # Errors
    ///
    /// This function will return a [`TextureEncodeError::Format`] if you pass in a data format
    /// that isn't [`DataFormat::Index4`] or [`DataFormat::Index8`]. If you want to encode textures
    /// that you don't want to generate a color palette for, see [`Self::new_gbix()`].
    pub fn new_gbix_palettized(
        pixel_format: PixelFormat,
        data_format: DataFormat,
    ) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats_palettized(data_format)?;

        Ok(Self {
            texture_type: TextureType::Gbix,
            pixel_format,
            data_format,
            data_flags: DataFlags::InternalPalette,
            ..Default::default()
        })
    }

    /// Creates a new encoder, that encodes GVR texture files using the given `data_format`.
    ///
    /// This specific function sets the magic strings in the header of the encoded texture file to
    /// "GBIX".
    ///
    /// # Errors
    ///
    /// This function will return a [`TextureEncodeError::Format`] if you pass in a data format
    /// that is [`DataFormat::Index4`] or [`DataFormat::Index8`]. If you want to encode textures
    /// that you want to generate a color palette for, see [`Self::new_gbix_palettized()`], as that
    /// allows you to set the data format for the color palette as well.
    pub fn new_gbix(data_format: DataFormat) -> Result<Self, TextureEncodeError> {
        Self::check_given_formats(data_format)?;

        Ok(Self {
            texture_type: TextureType::Gbix,
            data_format,
            ..Default::default()
        })
    }

    /// Instructs the encoder to also generate mipmaps alongside the original texture.
    ///
    /// <div class="warning">
    ///
    /// The only data formats that support mipmaps are [`DataFormat::Dxt1`],
    /// [`DataFormat::Rgb565`], and [`DataFormat::Rgb5a3`].
    ///
    /// </div>
    ///
    /// # Errors
    ///
    /// If you try to enable mipmaps on data formats that aren't listed above, a
    /// [`TextureEncodeError::Mipmap`] error is returned.
    pub fn with_mipmaps(mut self) -> Result<Self, TextureEncodeError> {
        match self.data_format {
            DataFormat::Dxt1 | DataFormat::Rgb565 | DataFormat::Rgb5a3 => {
                self.data_flags.set(DataFlags::Mipmaps, true);
                Ok(self)
            }
            _ => Err(TextureEncodeError::Mipmap),
        }
    }

    /// Sets the global index in the header of the encoded GVR texture file.
    ///
    /// Most GameCube and Wii games don't really use this but some games do. If this method is not
    /// used in the process of instantiating the encoder, then the global index will default to 0.
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

    /// Encodes the image file given in `img_path` into a GVR texture.
    ///
    /// This method returns an in-memory representation of the file as a [`Vec`] of bytes.
    ///
    /// # Errors
    ///
    /// If anything goes wrong in the encoding process, a [`TextureEncodeError`] is returned
    /// instead.
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
        if self.texture_type == TextureType::Gcix {
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

/// Provides all the functionality needed to decode a GVR texture file.
///
/// When the file is decoded using [`Self::decode()`], the image is not given to you from that
/// method. You can retrieve it via [`Self::as_decoded()`] or [`Self::into_decoded()`], or if you
/// don't want an in-memory representation of the file, you can immediately save the file via
/// [`Self::save()`].
///
/// For examples, see the documentation on the root of the [`crate`]
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
    ///
    /// # Errors
    ///
    /// An IO error will be returned if the given `gvr_path` is invalid in any way.
    pub fn new(gvr_path: &str) -> Result<Self, std::io::Error> {
        Ok(Self {
            cursor: Cursor::new(std::fs::read(gvr_path)?),
            ..Default::default()
        })
    }

    /// Decodes the given image from [`Self::new()`].
    ///
    /// # Errors
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
    /// # Errors
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
    /// This does not consume the decoder, so you can save the same image file as many times as you
    /// want.
    ///
    /// # Errors
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
