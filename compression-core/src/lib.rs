use std::path::PathBuf;
use image::RgbImage;

mod util;

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