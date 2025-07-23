use pyo3::prelude::*;

use crate::{PyTextureDecodeError, TextureDecoder};

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
