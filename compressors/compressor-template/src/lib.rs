use image::RgbImage;
use compression_core::{CompressedImage, ImageCompressor};

pub struct MyCompressor {
    // Add configuration fields here (e.g. quality, block size)
}

impl MyCompressor {
    pub fn new() -> Self {
        Self {}
    }
}

impl ImageCompressor for MyCompressor {
    fn compress(&self, image: &RgbImage) -> Result<CompressedImage, String> {
        let width = image.width();
        let height = image.height();
        let raw: &[u8] = image.as_raw();

        // TODO: compress `raw` into `data`
        let data = raw.to_vec();

        Ok(CompressedImage { data, width, height, metadata: compression_core::CompressionMetadata::None })
    }

    fn decompress(&self, compressed: &CompressedImage) -> Result<RgbImage, String> {
        // TODO: decompress `compressed.data` back into raw RGB bytes
        let raw = compressed.data.clone();

        RgbImage::from_raw(compressed.width, compressed.height, raw)
            .ok_or_else(|| "Buffer too small for image dimensions".to_string())
    }
}
