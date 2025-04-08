use bitflags::bitflags;

#[derive(Default, PartialEq, Eq)]
pub enum TextureType {
    #[default]
    GCIX,
    GBIX,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PixelFormat {
    #[default]
    IntensityA8,
    RGB565,
    RGB5A3,
}

impl From<PixelFormat> for u8 {
    fn from(value: PixelFormat) -> Self {
        value as u8
    }
}

#[derive(Default, Clone, Copy)]
#[repr(u8)]
pub enum DataFormat {
    Intensity4 = 0x00,
    Intensity8 = 0x01,
    IntensityA4 = 0x02,
    IntensityA8 = 0x03,
    Rgb565 = 0x04,
    #[default]
    Rgb5a3 = 0x05,
    Argb8888 = 0x06,
    Index4 = 0x08,
    Index8 = 0x09,
    Dxt1 = 0x0E,
}

impl From<DataFormat> for u8 {
    fn from(value: DataFormat) -> Self {
        value as u8
    }
}

impl TryFrom<u8> for DataFormat {
    type Error = &'static str;

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
            _ => Err("Invalid value for DataFormat enum"),
        }
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct DataFlags: u8 {
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
