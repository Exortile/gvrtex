use image::ImageError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum TextureEncodeError {
    Encode(ImageError),
    Palette(imagequant::Error),
    Mipmap,
    Format,
    SmallDimensions(u32, u32, u32, u32),
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

#[derive(Debug)]
pub enum TextureDecodeError {
    InvalidFile,
    Undecoded,
    Parsing(&'static str),
    Io(std::io::Error),
    Image(ImageError),
}

impl Error for TextureDecodeError {}

impl fmt::Display for TextureDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFile => write!(f, "The given file is an invalid GVR texture file."),
            Self::Undecoded => write!(f, "This texture has not been decoded successfully."),
            Self::Io(err) => write!(f, "{err}"),
            Self::Parsing(msg) => write!(f, "{msg}"),
            Self::Image(err) => write!(f, "{err}"),
        }
    }
}

impl From<std::io::Error> for TextureDecodeError {
    fn from(value: std::io::Error) -> Self {
        TextureDecodeError::Io(value)
    }
}

impl From<&'static str> for TextureDecodeError {
    fn from(value: &'static str) -> Self {
        TextureDecodeError::Parsing(value)
    }
}

impl From<ImageError> for TextureDecodeError {
    fn from(value: ImageError) -> Self {
        TextureDecodeError::Image(value)
    }
}
