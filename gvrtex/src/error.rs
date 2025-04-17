//! Contains all the possible custom error types from encoding and decoding textures.

use image::ImageError;
use std::error::Error;
use std::fmt;

/// Contains all the possible errors that can occur during encoding textures via
/// [`crate::TextureEncoder::encode()`], or during the instantation of a [`crate::TextureEncoder`].
#[derive(Debug)]
pub enum TextureEncodeError {
    /// Something went wrong opening the source image file.
    Encode(ImageError),
    /// Something went wrong when trying to construct a color palette during encoding a texture via
    /// [`crate::TextureEncoder::new_gcix_palettized()`].
    Palette(imagequant::Error),
    /// If the given [`crate::DataFormat`] doesn't support encoding mipmaps along with it.
    Mipmap,
    /// If a wrong [`crate::DataFormat`] is used in the instantation of a [`crate::TextureEncoder`].
    ///
    /// This means you either tried to use [`crate::DataFormat::Index4`] or [`crate::DataFormat::Index8`]
    /// format with [`crate::TextureEncoder::new_gcix()`] or [`crate::TextureEncoder::new_gbix()`],
    /// or you tried to use data formats *other than the two aforementioned formats* when
    /// instantiating with [`crate::TextureEncoder::new_gcix_palettized()`] or
    /// [`crate::TextureEncoder::new_gbix_palettized()`].
    Format,
    /// The given source image file has dimensions that are too small for the given [`crate::DataFormat`].
    SmallDimensions(u32, u32, u32, u32),
    /// The given source image file has dimensions that are invalid for the given [`crate::DataFormat`].
    ///
    /// This usually means that your source image dimensions are not a multiple of the block size
    /// that the data format needs to properly encode the image.
    ///
    /// Easiest way to fix this is by keeping your image dimensions as powers of 2 (for example:
    /// 64x64, 128x64, 512x256, etc).
    InvalidDimensions(u32, u32, u32),
}

impl Error for TextureEncodeError {}

impl fmt::Display for TextureEncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(err) => write!(f, "{err}"),
            Self::Palette(err) => write!(f, "{err}"),
            Self::Mipmap => {
                write!(f, "The given texture format type doesn't support mipmaps.")
            }
            Self::Format => write!(
                f,
                "Incorrect or incompatible formats supplied for texture encoding."
            ),
            Self::SmallDimensions(width, height, x_block, y_block) => write!(f, "The dimensions for the input image ({width}x{height}) are too small! Dimensions have to be at least {x_block}x{y_block}."),
            Self::InvalidDimensions(width, height, block_size) => write!(f, "The dimensions for the input image ({width}x{height}) are invalid! Dimensions have to be a multiple of {block_size}."),
        }
    }
}

impl From<ImageError> for TextureEncodeError {
    fn from(value: ImageError) -> Self {
        Self::Encode(value)
    }
}

impl From<imagequant::Error> for TextureEncodeError {
    fn from(value: imagequant::Error) -> Self {
        Self::Palette(value)
    }
}

impl From<std::io::Error> for TextureEncodeError {
    fn from(value: std::io::Error) -> Self {
        Self::Encode(ImageError::IoError(value))
    }
}

/// Contains all the possible errors that can occur during the use of a [`crate::TextureDecoder`].
#[derive(Debug)]
pub enum TextureDecodeError {
    /// The input file that was given was not a valid GVR texture file.
    ///
    /// This can be because of many things, but it all stems from invalid data used in the header
    /// of the file. This can be because the size of the texture portrayed in the file header
    /// doesn't match the actual filesize, there are invalid flags set in the header, invalid data
    /// formats used, or the header is missing the required magic strings.
    ///
    /// The latter option is the most common reason, with the other options only really being possible
    /// if the file was corrupted in some way or the encoder that encoded said file has a bug in it.
    InvalidFile,
    /// Returned when attempting to access the decoded image before decoding has started,
    /// or after decoding has failed.
    Undecoded,
    /// A standard IO error has occurred.
    Io(std::io::Error),
    /// Something went wrong saving the decoded image.
    ///
    /// This error can only be encountered when using [`crate::TextureDecoder::save()`].
    Image(ImageError),
}

impl Error for TextureDecodeError {}

impl fmt::Display for TextureDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFile => write!(f, "The given file is an invalid GVR texture file."),
            Self::Undecoded => write!(f, "This texture has not been decoded successfully."),
            Self::Io(err) => write!(f, "{err}"),
            Self::Image(err) => write!(f, "{err}"),
        }
    }
}

impl From<std::io::Error> for TextureDecodeError {
    fn from(value: std::io::Error) -> Self {
        TextureDecodeError::Io(value)
    }
}

impl From<ImageError> for TextureDecodeError {
    fn from(value: ImageError) -> Self {
        TextureDecodeError::Image(value)
    }
}
