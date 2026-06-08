use std::io::Cursor;
use std::path::PathBuf;
use image::{RgbImage, ImageFormat};

mod util;
pub use util::{pixel_index, rgb_index};

pub struct LibraryConfig {
    library_path: PathBuf,
    search_subdirectories: bool,
    supported_formats: Vec<String>,
}

pub fn load_image_library(config: &LibraryConfig) -> Result<Vec<PathBuf>, String> {
    if !config.library_path.exists() { return Err("Library path doesn't exist!".to_string()) }
    if !config.library_path.is_dir() { return Err("Library path is not a directory!".to_string()) }

    let library_children = config.library_path
        .read_dir()
        .map_err(|_| "Failed to read directory entries!".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Failed to read directory entries!".to_string())?;

    let mut images = Vec::new();
    for entry in &library_children {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| config.supported_formats.contains(&ext.to_string_lossy().into_owned())) {
            images.push(path);
        } else if path.is_dir() && config.search_subdirectories {
            if let Ok(mut sub_images) = load_image_library(config) {
                images.append(&mut sub_images);
            }
        }
    }

    Ok(images)
}

pub struct ComparisonScore {
    pub hue_similarity: f64,
    pub saturation_similarity: f64,
    pub brightness_similarity: f64,
    pub edge_similarity: f64,
}

pub fn compare_images(image1: &RgbImage, image2: &RgbImage) -> Result<ComparisonScore, String> {
    let (h1, s1, v1) = util::hsv_histograms(image1);
    let (h2, s2, v2) = util::hsv_histograms(image2);
    let edges1 = util::sobel_edge_histogram(image1);
    let edges2 = util::sobel_edge_histogram(image2);
    Ok(ComparisonScore {
        hue_similarity: util::histogram_intersection(&h1, &h2),
        saturation_similarity: util::histogram_intersection(&s1, &s2),
        brightness_similarity: util::histogram_intersection(&v1, &v2),
        edge_similarity: util::histogram_intersection(&edges1, &edges2),
    })
}

pub fn load_image(path: &PathBuf) -> Result<RgbImage, String> {
    image::open(path)
        .map_err(|e| e.to_string())
        .map(|img| img.into_rgb8())
}

pub fn save_image(image: &RgbImage, path: &PathBuf) -> Result<(), String> {
    image.save(path).map_err(|e| e.to_string())
}

pub struct CompressedImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub trait ImageCompressor {
    fn compress(&self, image: &RgbImage) -> Result<CompressedImage, String>;
    fn decompress(&self, compressed: &CompressedImage) -> Result<RgbImage, String>;
}

pub struct CompressionStats {
    pub raw_size: usize,
    pub compressed_size: usize,
    pub png_size: Option<usize>,
    pub lossless: bool,
    pub accuracy: Option<ComparisonScore>,
    pub compress_ms: f64,
    pub decompress_ms: f64,
}

impl CompressionStats {
    pub fn print(&self) {
        let ratio = self.compressed_size as f64 / self.raw_size as f64 * 100.0;
        println!("Raw size:        {} bytes", self.raw_size);
        println!("Compressed size: {} bytes ({:.1}%)", self.compressed_size, ratio);
        if let Some(png) = self.png_size {
            println!("PNG size:        {} bytes ({:.1}%)", png, png as f64 / self.raw_size as f64 * 100.0);
        }
        println!("Compress time:   {:.2}ms", self.compress_ms);
        println!("Decompress time: {:.2}ms", self.decompress_ms);
        if self.lossless {
            println!("Lossless:        yes");
        } else if let Some(ref s) = self.accuracy {
            println!("Lossless:        no");
            println!("  Hue:        {:.1}%", s.hue_similarity * 100.0);
            println!("  Saturation: {:.1}%", s.saturation_similarity * 100.0);
            println!("  Brightness: {:.1}%", s.brightness_similarity * 100.0);
            println!("  Edges:      {:.1}%", s.edge_similarity * 100.0);
        }
    }

    pub fn print_average(stats: &[CompressionStats]) {
        let n = stats.len() as f64;
        let ratio = stats.iter().map(|s| s.compressed_size as f64 / s.raw_size as f64 * 100.0).sum::<f64>() / n;
        let compress_ms = stats.iter().map(|s| s.compress_ms).sum::<f64>() / n;
        let decompress_ms = stats.iter().map(|s| s.decompress_ms).sum::<f64>() / n;
        let lossless_count = stats.iter().filter(|s| s.lossless).count();
        println!("Images:          {}", stats.len());
        println!("Avg compression: {:.1}%", ratio);
        println!("Avg compress:    {:.2}ms", compress_ms);
        println!("Avg decompress:  {:.2}ms", decompress_ms);
        println!("Lossless:        {}/{}", lossless_count, stats.len());
        let lossy: Vec<_> = stats.iter().filter_map(|s| s.accuracy.as_ref()).collect();
        if !lossy.is_empty() {
            let m = lossy.len() as f64;
            println!("Avg accuracy (lossy images):");
            println!("  Hue:        {:.1}%", lossy.iter().map(|s| s.hue_similarity).sum::<f64>() / m * 100.0);
            println!("  Saturation: {:.1}%", lossy.iter().map(|s| s.saturation_similarity).sum::<f64>() / m * 100.0);
            println!("  Brightness: {:.1}%", lossy.iter().map(|s| s.brightness_similarity).sum::<f64>() / m * 100.0);
            println!("  Edges:      {:.1}%", lossy.iter().map(|s| s.edge_similarity).sum::<f64>() / m * 100.0);
        }
    }
}

pub fn compression_stats(
    original: &RgbImage,
    compressed: &CompressedImage,
    decompressed: &RgbImage,
    compress_ms: f64,
    decompress_ms: f64,
) -> CompressionStats {
    let raw_size = original.as_raw().len();
    let compressed_size = compressed.data.len();
    let mut png_bytes: Vec<u8> = Vec::new();
    let png_size = original
        .write_to(&mut Cursor::new(&mut png_bytes), ImageFormat::Png)
        .ok()
        .map(|_| png_bytes.len());
    let lossless = original.as_raw() == decompressed.as_raw();
    let accuracy = if lossless { None } else { compare_images(original, decompressed).ok() };
    CompressionStats { raw_size, compressed_size, png_size, lossless, accuracy, compress_ms, decompress_ms }
}