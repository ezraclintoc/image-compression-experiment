use compression_core::{CompressedImage, ImageCompressor};
use image::RgbImage;

pub struct RunLengthCompressor {}

impl RunLengthCompressor {
    pub fn new() -> Self {
        Self {}
    }
}

impl ImageCompressor for RunLengthCompressor {
    fn compress(&self, image: &RgbImage) -> Result<CompressedImage, String> {
        let width = image.width();
        let height = image.height();
        let raw: &[u8] = image.as_raw();

        let mut data: Vec<u8> = Vec::with_capacity(raw.len() / 3 * 4);
        let mut i = 0;
        while i < raw.len() {
            let pixel = &raw[i..i + 3];
            let mut run = 1usize;
            while i + run * 3 < raw.len()
                && &raw[i + run * 3..i + run * 3 + 3] == pixel
                && run < 255
            {
                run += 1;
            }

            data.extend_from_slice(&[run as u8, pixel[0], pixel[1], pixel[2]]);
            i += run * 3;
        }

        Ok(CompressedImage {
            data,
            width,
            height,
        })
    }

    fn decompress(&self, compressed: &CompressedImage) -> Result<RgbImage, String> {
        let mut raw = Vec::with_capacity(
            (compressed.width * compressed.height * 3)
                .try_into()
                .unwrap(),
        );
        let mut i: usize = 0;

        while i < compressed.data.len() {
            let count = compressed.data[i] as usize;
            let pixel = &compressed.data[i + 1..i + 4];
            for _ in 0..count {
                raw.extend_from_slice(pixel);
            }
            i += 4;
        }

        RgbImage::from_raw(compressed.width, compressed.height, raw)
            .ok_or_else(|| "Buffer too small for image dimensions".to_string())
    }
}
