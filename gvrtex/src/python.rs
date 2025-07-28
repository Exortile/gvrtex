use pyo3::prelude::*;

use crate::formats::{DataFormat, PixelFormat};
use crate::{PyTextureDecodeError, PyTextureEncodeError, TextureDecoder, TextureEncoder};

#[allow(dead_code)]
#[pyclass]
pub struct DecodedGVR {
    #[pyo3(get)]
    pub width: u32,

    #[pyo3(get)]
    pub height: u32,

    /// In RGBA format
    #[pyo3(get)]
    pub data: Vec<u8>,
}

fn decode_internal(mut decoder: TextureDecoder) -> PyResult<DecodedGVR> {
    if let Err(err) = decoder.decode() {
        return Err(PyTextureDecodeError::new_err(err.to_string()));
    }

    let Ok(img) = decoder.into_decoded() else {
        return Err(PyTextureDecodeError::new_err("Something went wrong."));
    };

    Ok(DecodedGVR {
        width: img.width(),
        height: img.height(),
        data: img.into_vec(),
    })
}

#[pyfunction]
pub fn decode_from_path(gvr_path: &str) -> PyResult<DecodedGVR> {
    let decoder = TextureDecoder::new(gvr_path)?;
    decode_internal(decoder)
}

#[pyfunction]
pub fn decode_from_buffer(buffer: Vec<u8>) -> PyResult<DecodedGVR> {
    let decoder = TextureDecoder::new_from_buffer(buffer);
    decode_internal(decoder)
}

#[pyfunction]
pub fn encode_pixel_buffer_gcix(
    format: &str,
    buffer: Vec<u8>,
    width: u32,
    height: u32,
) -> PyResult<Vec<u8>> {
    let mut encoder = match format {
        "intensity4" => match TextureEncoder::new_gcix(DataFormat::Intensity4) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },
        "intensity8" => match TextureEncoder::new_gcix(DataFormat::Intensity8) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },
        "intensity_a4" => match TextureEncoder::new_gcix(DataFormat::IntensityA4) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },
        "intensity_a8" => match TextureEncoder::new_gcix(DataFormat::IntensityA8) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },
        "rgb565" | "rgb565_mipmaps" => match TextureEncoder::new_gcix(DataFormat::Rgb565) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },
        "rgb5a3" | "rgb5a3_mipmaps" => match TextureEncoder::new_gcix(DataFormat::Rgb5a3) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },
        "argb8888" => match TextureEncoder::new_gcix(DataFormat::Argb8888) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },
        "index4" => {
            match TextureEncoder::new_gcix_palettized(PixelFormat::RGB5A3, DataFormat::Index4) {
                Ok(enc) => enc,
                Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
            }
        }
        "index8" => {
            match TextureEncoder::new_gcix_palettized(PixelFormat::RGB5A3, DataFormat::Index8) {
                Ok(enc) => enc,
                Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
            }
        }
        "dxt1" | "dxt1_mipmaps" => match TextureEncoder::new_gcix(DataFormat::Dxt1) {
            Ok(enc) => enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },

        _ => return Err(PyTextureEncodeError::new_err("Unknown format specified.")),
    };

    match format {
        "rgb5a3_mipmaps" | "rgb565_mipmaps" | "dxt1_mipmaps" => match encoder.with_mipmaps() {
            Ok(enc) => encoder = enc,
            Err(err) => return Err(PyTextureEncodeError::new_err(err.to_string())),
        },

        _ => {}
    }

    match encoder.encode_pixel_buffer(buffer, width, height) {
        Ok(data) => Ok(data),
        Err(err) => Err(PyTextureEncodeError::new_err(err.to_string())),
    }
}
