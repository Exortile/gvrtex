# gvrtex

gvrtex is a Rust library for interfacing with the GVR texture format used in GameCube/Wii games, for example Sonic Riders. It's essentially the same as a regular TPL texture file (the official texture file format for GameCube/Wii), but with GVR headers instead. The image data in the file remains the same as it is for TPL files, so that the game console can read it.

More details on the GVR texture format can be found on the [PuyoTools Wiki](https://code.google.com/archive/p/puyotools/wikis/GVRTexture.wiki).

## Examples

To encode an image into a GVR file (the supported image formats can be found [here](https://github.com/image-rs/image/blob/main/README.md#supported-image-formats)):

```rust
use gvrtex::error::TextureEncodeError;
use gvrtex::formats::DataFormat;
use gvrtex::TextureEncoder;

fn example(img_path: &str) -> Result<Vec<u8>, TextureEncodeError> {
    let mut encoder = TextureEncoder::new_gcix(DataFormat::Dxt1)?;
    let encoded_file = encoder.encode(img_path)?;
    Ok(encoded_file)
}
```

To encode a GVR file with a quantized color palette (can be 4-bit indexed or 8-bit indexed):

```rust
use gvrtex::error::TextureEncodeError;
use gvrtex::formats::{DataFormat, PixelFormat};
use gvrtex::TextureEncoder;

fn example(img_path: &str) -> Result<Vec<u8>, TextureEncodeError> {
    let mut encoder = TextureEncoder::new_gcix_palettized(PixelFormat::RGB5A3, DataFormat::Index8)?;
    let encoded_file = encoder.encode(img_path)?;
    Ok(encoded_file)
}
```

To decode a GVR file into the respective image file:

```rust
use gvrtex::error::TextureDecodeError;
use gvrtex::formats::{DataFormat, PixelFormat};
use gvrtex::TextureDecoder;

fn example(gvr_path: &str, save_path: &str) -> Result<(), TextureDecodeError> {
    // Reads the contents of the file in gvr_path, but doesn't decode it yet.
    let mut decoder = TextureDecoder::new(gvr_path)?;

    // Decode file, saving the result in the decoder
    decoder.decode()?;

    // Save the decoded image to the given path. The image format is derived from the file
    // extension in the path.
    decoder.save(save_path)?;

    Ok(())
}
```

## Credits

- [PuyoTools](https://github.com/nickworonekin/puyotools) for the internal encoding and decoding algorithms, as well as information on the GVR file format.
