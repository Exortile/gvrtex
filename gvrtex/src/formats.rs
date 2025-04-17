//! Contains all the possible formats a GVR texture can be.
//!
//! This module is to be used for specifying which format you want to encode your GVR texture in.
//!
//! Each format has their own use case. For example, DXT1 compressed textures don't look too great
//! when it's in a very easily viewable place (like a 2D menu), you're better off using something
//! that doesn't look that bad with all the compression artifacts. Same applies vice versa, using
//! something like an ARGB8888 encoding (has the largest filesize), for something that isn't easily
//! viewable (like a 3D model) is overkill. You would be better off using a compressed texture
//! format.
//!
//! See [`crate::TextureEncoder`] for where these are used.

use crate::TextureDecodeError;
use bitflags::bitflags;

#[derive(Default, PartialEq, Eq)]
pub(crate) enum TextureType {
    #[default]
    Gcix,
    Gbix,
}

/// This enum specifies the format the color palette for a palettized GVR texture will be encoded
/// in.
///
/// This means the image will be rendered using whichever format is specified, as this is only used
/// in GVR textures with the formats [`DataFormat::Index4`] and [`DataFormat::Index8`] (these data
/// formats don't inherently store the format of the color data, rather the format of the indices
/// to refer to the color palette).
///
/// See [`crate::TextureEncoder::new_gcix_palettized()`] and [`crate::TextureEncoder::new_gbix_palettized()`]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PixelFormat {
    /// See [`DataFormat::IntensityA8`]
    #[default]
    IntensityA8,
    /// See [`DataFormat::Rgb565`]
    RGB565,
    /// See [`DataFormat::Rgb5a3`]
    RGB5A3,
}

impl From<PixelFormat> for u8 {
    fn from(value: PixelFormat) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for PixelFormat {
    type Error = TextureDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::IntensityA8),
            1 => Ok(Self::RGB565),
            2 => Ok(Self::RGB5A3),
            _ => Err(TextureDecodeError::InvalidFile),
        }
    }
}

/// This enum specifies the format for which a GVR texture should be encoded in.
///
/// Most formats are to be used in accordance with [`crate::TextureEncoder::new_gcix()`] or
/// [`crate::TextureEncoder::new_gbix()`]. If you wish to use [`DataFormat::Index4`] or
/// [`DataFormat::Index8`], then use [`crate::TextureEncoder::new_gcix_palettized()`] or
/// [`crate::TextureEncoder::new_gbix_palettized()`]. That way you can specify the color format for
/// the color palette alongside the data format.
#[derive(Default, Clone, Copy)]
#[repr(u8)]
pub enum DataFormat {
    /// Stores 4-bit intensity values (each pixel is composed of just one value). This makes the
    /// image look grayscale. This format stores no alpha channel.
    Intensity4 = 0x00,
    /// Stores 8-bit intensity values (each pixel is composed of just one value). This makes the
    /// image look grayscale. This format stores no alpha channel.
    Intensity8 = 0x01,
    /// Stores 4-bit intensity values (each pixel is composed of just one value) along with an
    /// alpha channel. This makes the image look grayscale.
    IntensityA4 = 0x02,
    /// Stores 8-bit intensity values (each pixel is composed of just one value) along with an
    /// alpha channel. This makes the image look grayscale.
    IntensityA8 = 0x03,
    /// Stores 16-bit color values, but does not save an alpha channel.
    Rgb565 = 0x04,
    /// Stores 16-bit color values, but saves the alpha channel as well.
    #[default]
    Rgb5a3 = 0x05,
    /// Stores 24-bit depth true color (1 byte per color). It also stores an 8-bit alpha channel.
    ///
    /// This format is by far the one with the largest filesize, although the most accurate in terms of
    /// color.
    Argb8888 = 0x06,
    /// Stores 4-bit indices into a quantized color palette.
    ///
    /// The color palette can only encode a maximum of 16 colors, which means images with a larger
    /// variety of colors will not look that great.
    Index4 = 0x08,
    /// Stores 8-bit indices into a quantized color palette.
    ///
    /// The color palette can encode a maximum of 256 colors, which means images preserve a decent
    /// amount of their color quality, as opposed to the [`DataFormat::Index4`] format.
    Index8 = 0x09,
    /// Encodes the image using a DXT1 compression algorithm, also known as BC1 (Block Compression 1).
    ///
    /// Works well in environments where the texture cannot be easily viewed (like a 3D model in
    /// motion), but not that well in other cases (like on a 2D menu), as the compression artifacts
    /// can be quite visible at times.
    Dxt1 = 0x0E,
}

impl From<DataFormat> for u8 {
    fn from(value: DataFormat) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for DataFormat {
    type Error = TextureDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Intensity4),
            0x01 => Ok(Self::Intensity8),
            0x02 => Ok(Self::IntensityA4),
            0x03 => Ok(Self::IntensityA8),
            0x04 => Ok(Self::Rgb565),
            0x05 => Ok(Self::Rgb5a3),
            0x06 => Ok(Self::Argb8888),
            0x08 => Ok(Self::Index4),
            0x09 => Ok(Self::Index8),
            0x0E => Ok(Self::Dxt1),
            _ => Err(TextureDecodeError::InvalidFile),
        }
    }
}

impl From<PixelFormat> for DataFormat {
    fn from(value: PixelFormat) -> Self {
        match value {
            PixelFormat::RGB565 => Self::Rgb565,
            PixelFormat::RGB5A3 => Self::Rgb5a3,
            PixelFormat::IntensityA8 => Self::IntensityA8,
        }
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(crate) struct DataFlags: u8 {
        const None = 0;
        const Mipmaps = 0x1;
        const ExternalPalette = 0x2;
        const InternalPalette = 0x8;
        const Palette = Self::ExternalPalette.bits() | Self::InternalPalette.bits();
    }
}

impl From<DataFlags> for u8 {
    fn from(val: DataFlags) -> Self {
        val.bits()
    }
}
