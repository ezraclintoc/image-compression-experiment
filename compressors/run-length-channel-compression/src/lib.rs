use compression_core::{CompressedImage, ImageCompressor};
use image::RgbImage;

pub struct RunLengthChannelCompressor {}

impl RunLengthChannelCompressor {
    pub fn new() -> Self {
        Self {}
    }
}

impl ImageCompressor for RunLengthChannelCompressor {
    fn compress(&self, image: &RgbImage) -> Result<CompressedImage, String> {
        let width = image.width();
        let height = image.height();
        let raw: &[u8] = image.as_raw();

        let mut data: Vec<u8> = Vec::with_capacity(raw.len() * 2);
        let mut c = 0;
        while c < 3 {
            let mut i = c;
            while i < raw.len() {
                let color = &raw[i];
                let mut run = 1usize;
                while i + run * 3 < raw.len() && &raw[i + run * 3] == color && run < 255 {
                    run += 1;
                }
                data.extend_from_slice(&[run as u8, *color]);
                i += run * 3;
            }
            c += 1;
        }

        Ok(CompressedImage {
            data,
            width,
            height,
            metadata: compression_core::CompressionMetadata::None,
        })
    }

    fn decompress(&self, compressed: &CompressedImage) -> Result<RgbImage, String> {
        let mut raw: Vec<u8> = vec![
            0 as u8;
            (compressed.width * compressed.height * 3)
                .try_into()
                .unwrap()
        ];

        let mut c = 0;
        let mut i: usize = 0; //Input Index
        while c < 3 {
            let mut j: usize = 0; //Output Index
            while j < (compressed.width * compressed.height).try_into().unwrap() {
                let count = compressed.data[i] as usize;
                let color = &compressed.data[i + 1];
                for k in 0..count {
                    raw[(j + k) * 3 + c] = *color;
                }
                j += count;
                i += 2;
            }
            c += 1;
        }

        RgbImage::from_raw(compressed.width, compressed.height, raw)
            .ok_or_else(|| "Buffer too small for image dimensions".to_string())
    }
}
