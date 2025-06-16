use ffi::DecodedGVRInfo;

use crate::{error::TextureDecodeError, TextureDecoder};

#[cxx::bridge]
mod ffi {
    struct DecodedGVRInfo {
        width: u32,
        height: u32,
        data: Vec<u8>,
    }

    #[namespace = "gvr_decoder"]
    extern "Rust" {
        fn decode_from_path(gvr_path: &str) -> Result<DecodedGVRInfo>;
        fn decode_from_buffer(buffer: Vec<u8>) -> Result<DecodedGVRInfo>;
    }
}

fn decode_from_path(gvr_path: &str) -> Result<DecodedGVRInfo, TextureDecodeError> {
    let mut decoder = TextureDecoder::new(gvr_path)?;
    decoder.decode()?;
    let img = decoder.into_decoded()?;
    Ok(DecodedGVRInfo {
        width: img.width(),
        height: img.height(),
        data: img.into_vec(),
    })
}

fn decode_from_buffer(buffer: Vec<u8>) -> Result<DecodedGVRInfo, TextureDecodeError> {
    let mut decoder = TextureDecoder::new_from_buffer(buffer);
    decoder.decode()?;
    let img = decoder.into_decoded()?;
    Ok(DecodedGVRInfo {
        width: img.width(),
        height: img.height(),
        data: img.into_vec(),
    })
}
