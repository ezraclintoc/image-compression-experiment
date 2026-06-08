# Image Compression Experiment

A Rust workspace for implementing and benchmarking image compression algorithms from scratch. Each algorithm lives in its own crate and is compared against the others using a shared runner.

## Workspace Structure

```
compression-core/           — shared types, traits, and utilities
compressors/
  compressor-template/      — blank template for new compressors
  run-length-compression/   — RLE: encodes runs of identical pixels
  run-length-channel-compression/ — RLCE: RLE applied per color channel
  huffman-compressor/       — Huffman coding on RGB pixel triplets
runner/                     — CLI for running and comparing compressors
images/
  png/                      — source images (committed)
    real-life/              — Kodak benchmark set (12 images)
    cartoon/                — flat-color cartoon images
    math/                   — 25 generated test images (see below)
  ppm/                      — converted PPM files (gitignored, regenerate with script)
generate_ppm.sh             — converts images/png/ → images/ppm/ recursively
```

## Running

```bash
# Convert PNGs to PPM first
./generate_ppm.sh

# Stats for a single image
cargo run -p runner --release -- image images/ppm/real-life/kodim05.ppm

# Average stats across a directory
cargo run -p runner --release -- average images/ppm/math/

# Compare all compressors side by side
cargo run -p runner --release -- compare images/ppm/math/

# Regenerate the 25 math test images
cargo run -p runner --bin gen
```

## Compressors

### RLE (Run-Length Encoding)
Scans pixels left-to-right. Encodes runs of identical RGB pixels as `(count, R, G, B)`.

- **Best on**: solid fills, horizontal bands, large uniform regions
- **Worst on**: noise, vertical stripes, diagonal gradients

### RLCE (Run-Length Channel Encoding)
Same as RLE but applied independently to each color channel (R, G, B separately).

- **Best on**: images where one or two channels are constant while others vary (e.g. red→magenta gradient: R and G channels each encode as a single run)
- **Worse than RLE on**: solid-color images (encodes 3 separate runs instead of 1)

### Huffman
Builds a Huffman tree over RGB pixel triplets. More frequent pixels get shorter codes.

- **Best on**: images with a skewed color distribution (few dominant colors)
- **Worst on**: images with many unique colors (noise, natural photos)
- **Note**: operates on full RGB triplets, not individual bytes — a pixel `(255, 0, 0)` is a single symbol

## Test Images

The 25 generated images in `images/png/math/` are designed to expose strengths and weaknesses of specific algorithms:

| Prefix | Targets | Description |
|---|---|---|
| `rle_` | RLE strengths | Solid fills, horizontal bands, large checkerboards |
| `rle_weak_` | RLE weaknesses | 1px checkerboard, vertical stripes, diagonal gradients, noise |
| `rlce_` | RLCE strengths | Single/two-channel gradients, grayscale, constant-channel images |
| `dct_` | DCT (future) | Smooth gradients, sine textures, radial patterns |
| `dct_weak_` | DCT weaknesses | Hard edges, high-frequency block noise |
| `huffman_` | Huffman strengths | Sparse dots on white, text-like binary images |
| `huffman_weak_` | Huffman weaknesses | Uniform random noise |
| `general_` | Mixed | Sierpinski triangle, diagonal stripes, QR-like patterns |

## Authorship

| Code | Author |
|---|---|
| `compressors/run-length-compression/` | Hand-written |
| `compressors/run-length-channel-compression/` | Hand-written |
| `compressors/huffman-compressor/` | Hand-written |
| `compressors/compressor-template/` | Coded with AI assistance |
| `compression-core/` | Coded with AI assistance |
| `runner/` | Coded with AI assistance |
| `images/png/math/` | Coded with AI assistance |
| `generate_ppm.sh` | Coded with AI assistance |

## Adding a New Compressor

1. Copy `compressors/compressor-template/` to `compressors/your-name/`
2. Rename the crate in `Cargo.toml` (`name = "your-name"`)
3. Implement `ImageCompressor` in `src/lib.rs`:
   - `compress`: raw `&[u8]` pixels in, `CompressedImage` out
   - `decompress`: `CompressedImage` in, `RgbImage` out
4. If your compressor needs state to decompress, add a variant to `CompressionMetadata` in `compression-core/src/lib.rs` (see comments there)
5. Add it to `all_compressors()` in `runner/src/main.rs`:
   ```rust
   ("MyAlgo", Box::new(MyCompressor::new())),
   ```
6. Add it as a dependency in `runner/Cargo.toml`
