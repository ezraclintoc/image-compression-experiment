use compression_core::{compression_stats, load_image, CompressionStats, ImageCompressor};
use run_length_compression::RunLengthCompressor;
use std::path::PathBuf;
use std::time::Instant;

fn run_single(path: &PathBuf, compressor: &dyn ImageCompressor) -> Result<CompressionStats, String> {
    let image = load_image(path)?;

    let t0 = Instant::now();
    let compressed = compressor.compress(&image)?;
    let compress_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let t1 = Instant::now();
    let decompressed = compressor.decompress(&compressed)?;
    let decompress_ms = t1.elapsed().as_secs_f64() * 1000.0;

    Ok(compression_stats(&image, &compressed, &decompressed, compress_ms, decompress_ms))
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
        if !path.is_file() { continue; }
        match run_single(&path, compressor) {
            Ok(stats) => {
                println!("  {} — {}b → {}b ({:.1}%) | compress {:.2}ms | decompress {:.2}ms",
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
        _ => {
            eprintln!("Usage:");
            eprintln!("  runner image <path>     — stats for a single image");
            eprintln!("  runner average <dir>    — average stats across a directory");
        }
    }
}
