use bitflags::bitflags;

#[derive(Default)]
pub enum TextureType {
    #[default]
    GCIX,
    GBIX,
}

#[derive(Default, Clone, Copy)]
#[repr(u8)]
pub enum PixelFormat {
    IntensityA8,
    RGB565,
    #[default]
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
