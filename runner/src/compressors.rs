//! The single source of truth for which compressors exist.
//!
//! To add a compressor:
//!   1. add its crate as a path dependency in `runner/Cargo.toml`
//!   2. add a `use` line below
//!   3. add one entry to the vec in `all_compressors()`
//!
//! Everything else — the CLI `--compressor` flag, CSV export, and the TUI
//! picker — reads from `all_compressors()`, so there is nothing else to wire up.

use compression_core::{CompressedImage, CompressionMetadata, ImageCompressor};
use huffman_compressor::HuffmanCompressor;
use image::{ImageFormat, RgbImage};
use run_length_channel_compression::RunLengthChannelCompressor;
use run_length_compression::RunLengthCompressor;
use std::io::Cursor;

/// Baseline reference codec: the `image` crate's PNG encoder. Lives here rather
/// than in its own crate because it is just a thin wrapper used as a yardstick.
pub struct PngCompressor;

impl ImageCompressor for PngCompressor {
    fn compress(&self, image: &RgbImage) -> Result<CompressedImage, String> {
        let mut buf = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(CompressedImage {
            data: buf,
            width: image.width(),
            height: image.height(),
            metadata: CompressionMetadata::None,
        })
    }

    fn decompress(&self, compressed: &CompressedImage) -> Result<RgbImage, String> {
        image::load_from_memory(&compressed.data)
            .map_err(|e| e.to_string())
            .map(|img| img.into_rgb8())
    }
}

/// Every compressor known to the runner, in display order.
pub fn all_compressors() -> Vec<(&'static str, Box<dyn ImageCompressor>)> {
    vec![
        ("PNG", Box::new(PngCompressor)),
        ("RLE", Box::new(RunLengthCompressor::new())),
        ("RLCE", Box::new(RunLengthChannelCompressor::new())),
        ("Huffman", Box::new(HuffmanCompressor::new())),
    ]
}

/// Names of all compressors, for usage messages and the picker.
pub fn compressor_names() -> Vec<&'static str> {
    all_compressors().into_iter().map(|(n, _)| n).collect()
}

/// Resolve a `--compressor` spec into concrete compressors.
///
/// `"all"` (case-insensitive) returns every compressor. Otherwise the spec is a
/// comma-separated list of names matched case-insensitively. An unknown name is
/// returned as an `Err` so the caller can report it.
pub fn select_compressors(
    spec: &str,
) -> Result<Vec<(&'static str, Box<dyn ImageCompressor>)>, String> {
    let all = all_compressors();
    if spec.eq_ignore_ascii_case("all") {
        return Ok(all);
    }
    let wanted: Vec<&str> = spec
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    for w in &wanted {
        if !all.iter().any(|(n, _)| n.eq_ignore_ascii_case(w)) {
            return Err((*w).to_string());
        }
    }
    Ok(all
        .into_iter()
        .filter(|(n, _)| wanted.iter().any(|w| n.eq_ignore_ascii_case(w)))
        .collect())
}
