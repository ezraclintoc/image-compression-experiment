use image::RgbImage;

pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let r = r as f64 / 255.0;
    let g = g as f64 / 255.0;
    let b = b as f64 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        let h = 60.0 * ((g - b) / delta);
        if h < 0.0 { h + 360.0 } else { h }
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (h, s, v)
}

pub fn hsv_histograms(img: &RgbImage) -> ([f64; 36], [f64; 16], [f64; 16]) {
    let mut h_hist = [0u32; 36];
    let mut s_hist = [0u32; 16];
    let mut v_hist = [0u32; 16];
    for pixel in img.pixels() {
        let (h, s, v) = rgb_to_hsv(pixel[0], pixel[1], pixel[2]);
        h_hist[((h / 10.0) as usize).min(35)] += 1;
        s_hist[((s * 15.0) as usize).min(15)] += 1;
        v_hist[((v * 15.0) as usize).min(15)] += 1;
    }
    let total = (img.width() * img.height()) as f64;
    (h_hist.map(|x| x as f64 / total), s_hist.map(|x| x as f64 / total), v_hist.map(|x| x as f64 / total))
}

pub fn sobel_edge_histogram(img: &RgbImage) -> [f64; 16] {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let gray: Vec<f64> = img.pixels()
        .map(|p| 0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64)
        .collect();
    let get = |x: i32, y: i32| gray[(y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) as usize];
    let mut max_mag = 1.0f64;
    let magnitudes: Vec<f64> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| {
        let gx = -get(x-1,y-1) + get(x+1,y-1) - 2.0*get(x-1,y) + 2.0*get(x+1,y) - get(x-1,y+1) + get(x+1,y+1);
        let gy =  get(x-1,y-1) + 2.0*get(x,y-1) + get(x+1,y-1) - get(x-1,y+1) - 2.0*get(x,y+1) - get(x+1,y+1);
        (gx*gx + gy*gy).sqrt()
    }).inspect(|&m| max_mag = max_mag.max(m)).collect();
    let mut hist = [0u32; 16];
    for m in &magnitudes {
        hist[((m / max_mag * 15.0) as usize).min(15)] += 1;
    }
    let total = magnitudes.len() as f64;
    hist.map(|x| x as f64 / total)
}

pub fn histogram_intersection<const N: usize>(a: &[f64; N], b: &[f64; N]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| x.min(*y)).sum()
}

/// Pixel index into a flat row-major buffer.
pub fn pixel_index(x: u32, y: u32, width: u32) -> usize {
    (y * width + x) as usize
}

/// Byte index into a flat RGB buffer (3 bytes per pixel).
pub fn rgb_index(x: u32, y: u32, width: u32) -> usize {
    pixel_index(x, y, width) * 3
}