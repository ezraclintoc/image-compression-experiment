use compression_core::{
    CompressedImage, CompressionStats, ImageCompressor, compression_stats, load_image,
};
use huffman_compressor::HuffmanCompressor;
use image::{ImageFormat, RgbImage};
use run_length_channel_compression::RunLengthChannelCompressor;
use run_length_compression::RunLengthCompressor;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Instant;

struct PngCompressor;

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
            metadata: compression_core::CompressionMetadata::None,
        })
    }

    fn decompress(&self, compressed: &CompressedImage) -> Result<RgbImage, String> {
        image::load_from_memory(&compressed.data)
            .map_err(|e| e.to_string())
            .map(|img| img.into_rgb8())
    }
}

fn all_compressors() -> Vec<(&'static str, Box<dyn ImageCompressor>)> {
    vec![
        ("PNG", Box::new(PngCompressor)),
        ("RLE", Box::new(RunLengthCompressor::new())),
        ("RLCE", Box::new(RunLengthChannelCompressor::new())),
        ("Huffman", Box::new(HuffmanCompressor::new())),
    ]
}

fn run_single(
    path: &PathBuf,
    compressor: &dyn ImageCompressor,
) -> Result<CompressionStats, String> {
    let image = load_image(path)?;

    let t0 = Instant::now();
    let compressed = compressor.compress(&image)?;
    let compress_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let decompressed = compressor.decompress(&compressed)?;
    let decompress_ms = t1.elapsed().as_secs_f64() * 1000.0;

    Ok(compression_stats(
        &image,
        &compressed,
        &decompressed,
        compress_ms,
        decompress_ms,
    ))
}

fn cmd_image(path: PathBuf, compressor: &dyn ImageCompressor) {
    match run_single(&path, compressor) {
        Ok(stats) => {
            println!("=== {} ===", path.display());
            stats.print();
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}

fn cmd_average(dir: PathBuf, compressor: &dyn ImageCompressor) {
    let entries = std::fs::read_dir(&dir).expect("Failed to read directory");
    let mut all_stats: Vec<CompressionStats> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        match run_single(&path, compressor) {
            Ok(stats) => {
                println!(
                    "  {} — {}b → {}b ({:.1}%) | compress {:.2}ms | decompress {:.2}ms",
                    path.file_name().unwrap().to_string_lossy(),
                    stats.raw_size,
                    stats.compressed_size,
                    stats.compressed_size as f64 / stats.raw_size as f64 * 100.0,
                    stats.compress_ms,
                    stats.decompress_ms,
                );
                all_stats.push(stats);
            }
            Err(e) => eprintln!("  Skipped {}: {e}", path.display()),
        }
    }

    if all_stats.is_empty() {
        println!("No images found.");
        return;
    }

    println!("\n=== Average ({}) ===", dir.display());
    CompressionStats::print_average(&all_stats);
}

fn cmd_compare(dir: PathBuf) {
    let compressors = all_compressors();
    let col = 14usize;

    let mut image_paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("Failed to read directory")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    image_paths.sort();

    if image_paths.is_empty() {
        println!("No images found.");
        return;
    }

    // Header
    print!("{:<30}", "Image");
    for (name, _) in &compressors {
        print!("{:>col$}", name);
    }
    println!();
    println!("{}", "-".repeat(30 + compressors.len() * col));

    // Per-image rows — show compression ratio
    let mut all_stats: Vec<Vec<CompressionStats>> = Vec::new();
    for path in &image_paths {
        let filename = path.file_name().unwrap().to_string_lossy();
        print!("{:<30}", filename);
        let mut row = Vec::new();
        for (_, compressor) in &compressors {
            match run_single(path, compressor.as_ref()) {
                Ok(stats) => {
                    let ratio = stats.compressed_size as f64 / stats.raw_size as f64 * 100.0;
                    print!("{:>col$}", format!("{:.1}%", ratio));
                    row.push(stats);
                }
                Err(_) => {
                    print!("{:>col$}", "error");
                }
            }
        }
        println!();
        all_stats.push(row);
    }

    // Averages
    println!("{}", "-".repeat(30 + compressors.len() * col));
    print!("{:<30}", "Avg ratio");
    for i in 0..compressors.len() {
        let ratios: Vec<f64> = all_stats
            .iter()
            .filter_map(|row| row.get(i))
            .map(|s| s.compressed_size as f64 / s.raw_size as f64 * 100.0)
            .collect();
        let avg = ratios.iter().sum::<f64>() / ratios.len() as f64;
        print!("{:>col$}", format!("{:.1}%", avg));
    }
    println!();

    print!("{:<30}", "Avg compress (ms)");
    for i in 0..compressors.len() {
        let times: Vec<f64> = all_stats
            .iter()
            .filter_map(|row| row.get(i))
            .map(|s| s.compress_ms)
            .collect();
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        print!("{:>col$}", format!("{:.2}ms", avg));
    }
    println!();

    print!("{:<30}", "Avg decompress (ms)");
    for i in 0..compressors.len() {
        let times: Vec<f64> = all_stats
            .iter()
            .filter_map(|row| row.get(i))
            .map(|s| s.decompress_ms)
            .collect();
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        print!("{:>col$}", format!("{:.2}ms", avg));
    }
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let compressor = RunLengthCompressor::new();

    match args.get(1).map(String::as_str) {
        Some("image") => {
            let path = args.get(2).expect("Usage: runner image <path>");
            cmd_image(PathBuf::from(path), &compressor);
        }
        Some("average") => {
            let dir = args.get(2).expect("Usage: runner average <dir>");
            cmd_average(PathBuf::from(dir), &compressor);
        }
        Some("compare") => {
            let dir = args.get(2).expect("Usage: runner compare <dir>");
            cmd_compare(PathBuf::from(dir));
        }
        _ => {
            eprintln!("Usage:");
            eprintln!("  runner image <path>     — stats for a single image");
            eprintln!("  runner average <dir>    — average stats across a directory");
            eprintln!("  runner compare <dir>    — compare all compressors side by side");
        }
    }
}
