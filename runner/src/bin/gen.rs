use image::{Rgb, RgbImage};
use std::f32::consts::PI;

fn save(img: RgbImage, name: &str) {
    std::fs::create_dir_all("images/math").unwrap();
    let path = format!("images/math/{name}");
    img.save(&path).unwrap_or_else(|_| panic!("Failed to save {path}"));
    println!("Generated: {name}");
}

fn px(f: impl Fn(u32, u32) -> Rgb<u8>) -> impl Fn(u32, u32) -> Rgb<u8> { f }

// Deterministic pixel hash — avoids DefaultHasher non-determinism
fn hash(x: u32, y: u32, seed: u32) -> u32 {
    let mut v = x.wrapping_mul(2654435761)
        .wrapping_add(y.wrapping_mul(2246822519))
        .wrapping_add(seed);
    v ^= v >> 16;
    v = v.wrapping_mul(0x45d9f3b);
    v ^= v >> 16;
    v
}

fn main() {
    let (w, h) = (512u32, 512u32);

    // ── RLE STRENGTHS (long identical pixel runs per row) ────────────────────

    // Perfect case: single run encodes entire image
    save(RgbImage::from_fn(w, h, px(|_, _| Rgb([245, 245, 245]))),
        "01_rle_solid_white.png");

    // Each row is a single solid color — 1 run per row
    let band_colors: [[u8; 3]; 8] = [
        [220,60,60], [220,140,60], [200,200,60], [60,180,60],
        [60,160,220], [100,60,200], [200,60,180], [120,120,120],
    ];
    save(RgbImage::from_fn(w, h, px(|_, y| Rgb(band_colors[(y / 64) as usize]))),
        "02_rle_horizontal_bands.png");

    // Horizontal stepped gradient — 32 identical pixels per step
    save(RgbImage::from_fn(w, h, px(|x, _| {
        let s = (x / 32) as u8;
        Rgb([s * 16, 80, 220 - s * 8])
    })), "03_rle_stepped_gradient.png");

    // Large checkerboard — 64px runs per row
    save(RgbImage::from_fn(w, h, px(|x, y| {
        if (x / 64 + y / 64) % 2 == 0 { Rgb([240, 240, 240]) } else { Rgb([30, 30, 30]) }
    })), "04_rle_checkerboard_64px.png");

    // Filled circle on white — long white runs plus solid arc runs
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let (dx, dy) = (x as f32 - w as f32 / 2.0, y as f32 - h as f32 / 2.0);
        if dx*dx + dy*dy < 200.0f32.powi(2) { Rgb([200, 60, 60]) } else { Rgb([245, 245, 245]) }
    })), "05_rle_circle_on_white.png");

    // ── RLE WEAKNESSES ───────────────────────────────────────────────────────

    // 1px checkerboard — run length = 1 always, worst case expansion
    save(RgbImage::from_fn(w, h, px(|x, y| {
        if (x + y) % 2 == 0 { Rgb([255, 255, 255]) } else { Rgb([0, 0, 0]) }
    })), "06_rle_weak_checkerboard_1px.png");

    // 1px vertical stripes — every pixel alternates per row
    save(RgbImage::from_fn(w, h, px(|x, _| {
        if x % 2 == 0 { Rgb([240, 80, 80]) } else { Rgb([80, 80, 240]) }
    })), "07_rle_weak_vertical_stripes.png");

    // Diagonal gradient — each row shifts by 1, no long runs
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let v = ((x + y) * 255 / (w + h)) as u8;
        Rgb([v, 100, 255 - v])
    })), "08_rle_weak_diagonal_gradient.png");

    // Random noise — no runs at all
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let v = hash(x, y, 1);
        Rgb([(v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8])
    })), "09_rle_weak_noise.png");

    // ── RLCE STRENGTHS (individual channels have long runs, pixels don't) ───

    // Red→Magenta: R=255 always, G=0 always, only B varies
    // RLE: 512 different pixels/row. RLCE: R=1 run, G=1 run, B varies
    save(RgbImage::from_fn(w, h, px(|x, _| Rgb([255, 0, (x * 255 / w) as u8]))),
        "10_rlce_red_to_magenta.png");

    // Cyan→Green: G=255 always, R=0 always, only B varies
    save(RgbImage::from_fn(w, h, px(|x, _| Rgb([0, 255, 255 - (x * 255 / w) as u8]))),
        "11_rlce_cyan_to_green.png");

    // R varies with x, G varies with y, B=128 constant
    // RLCE: B channel encodes as 1 run per row
    save(RgbImage::from_fn(w, h, px(|x, y| {
        Rgb([(x * 255 / w) as u8, (y * 255 / h) as u8, 128])
    })), "12_rlce_two_channel_gradient.png");

    // Grayscale vertical gradient — R=G=B so each channel is identical
    save(RgbImage::from_fn(w, h, px(|_, y| { let v = (y * 255 / h) as u8; Rgb([v, v, v]) })),
        "13_rlce_grayscale_gradient.png");

    // RLCE WEAKNESS: solid image — RLE = 4 bytes total, RLCE = 6 bytes (3 channels × 2)
    save(RgbImage::from_fn(w, h, px(|_, _| Rgb([100, 149, 237]))),
        "14_rlce_weak_solid_color.png");

    // ── DCT STRENGTHS (smooth gradients, natural textures) ──────────────────

    // Smooth radial gradient — many similar-valued pixels, DCT handles well
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let (dx, dy) = (x as f32 - w as f32 / 2.0, y as f32 - h as f32 / 2.0);
        let dist = (dx*dx + dy*dy).sqrt() / (w as f32 / 2.0);
        let v = ((1.0 - dist.min(1.0)) * 255.0) as u8;
        Rgb([v, (v as f32 * 0.6) as u8, 255 - v])
    })), "15_dct_radial_gradient.png");

    // Sine wave texture — low-frequency signal, DCT compresses near-losslessly
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let fx = (x as f32 * 4.0 * PI / w as f32).sin();
        let fy = (y as f32 * 4.0 * PI / h as f32).sin();
        let v = ((fx * fy + 1.0) * 127.0) as u8;
        Rgb([v, (v as f32 * 0.8) as u8, 200])
    })), "16_dct_sine_texture.png");

    // Concentric rings — smooth radial pattern
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let (dx, dy) = (x as f32 - w as f32 / 2.0, y as f32 - h as f32 / 2.0);
        let dist = (dx*dx + dy*dy).sqrt();
        if (dist as u32 / 32) % 2 == 0 { Rgb([60, 120, 220]) } else { Rgb([220, 220, 220]) }
    })), "17_dct_concentric_rings.png");

    // ── DCT WEAKNESSES (sharp edges, high-frequency detail) ─────────────────

    // Hard edge grid — sharp transitions cause DCT ringing artifacts
    save(RgbImage::from_fn(w, h, px(|x, y| {
        if (x / 32) % 2 == (y / 32) % 2 { Rgb([0, 0, 0]) } else { Rgb([255, 255, 255]) }
    })), "18_dct_weak_hard_grid.png");

    // High-frequency noise blocks — DCT struggles with random high-freq content
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let v = hash(x / 4, y / 4, 7);
        Rgb([(v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8])
    })), "19_dct_weak_block_noise.png");

    // ── HUFFMAN STRENGTHS (skewed symbol distribution) ───────────────────────

    // Sparse dots on white — mostly one value (white), Huffman gives short code to white
    save(RgbImage::from_fn(w, h, px(|x, y| {
        if hash(x, y, 3) % 40 == 0 { Rgb([30, 30, 200]) } else { Rgb([245, 245, 245]) }
    })), "20_huffman_sparse_dots.png");

    // Binary text-like image — two symbols, very skewed (90% white)
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let in_line = y % 20 < 14;
        if in_line && hash(x / 3, y / 14, 5) % 5 == 0 { Rgb([20, 20, 20]) } else { Rgb([245, 245, 245]) }
    })), "21_huffman_text_like.png");

    // ── HUFFMAN WEAKNESSES (uniform symbol distribution) ─────────────────────

    // Full random noise — every byte equally likely, Huffman gives no benefit
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let v = hash(x, y, 9);
        Rgb([(v & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, ((v >> 16) & 0xFF) as u8])
    })), "22_huffman_weak_uniform_noise.png");

    // ── GENERAL ──────────────────────────────────────────────────────────────

    // Sierpinski triangle — fractal self-similarity
    save(RgbImage::from_fn(w, h, px(|x, y| {
        if (x & y) != 0 { Rgb([40, 80, 180]) } else { Rgb([245, 245, 245]) }
    })), "23_general_sierpinski.png");

    // Diagonal color stripes
    save(RgbImage::from_fn(w, h, px(|x, y| {
        let palette: [[u8; 3]; 4] = [[220,80,80],[80,150,220],[80,200,100],[240,200,80]];
        Rgb(palette[((x + y) / 64) as usize % 4])
    })), "24_general_diagonal_stripes.png");

    // QR-code-like blocks — random binary in 16px blocks
    save(RgbImage::from_fn(w, h, px(|x, y| {
        if hash(x / 16, y / 16, 11) % 2 == 0 { Rgb([0, 0, 0]) } else { Rgb([255, 255, 255]) }
    })), "25_general_qr_like.png");

    println!("\nDone.");
}
