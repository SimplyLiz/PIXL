use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use pixl_core::types::{Palette, Rgba as PaxRgba};

/// Quantize a reference image into a PAX character grid using the given palette.
///
/// Pipeline for AI-generated pixel art:
/// 1. Detect the native pixel grid (how big each "art pixel" is in the source)
/// 2. Center-sample each pixel block (avoids anti-aliasing at block edges)
/// 3. If native > target, nearest-neighbor downscale
/// 4. Quantize each pixel to nearest palette symbol
/// 5. Enforce outlines: boundary pixels → darkest palette color
pub fn import_reference(
    reference: &DynamicImage,
    target_width: u32,
    target_height: u32,
    palette: &Palette,
    dither: bool,
) -> ImportResult {
    let rgba = reference.to_rgba8();
    let (src_w, src_h) = rgba.dimensions();

    // Step 1: Detect native pixel grid
    let detected = detect_pixel_size(reference);
    let native_w = (src_w / detected).max(1);
    let native_h = (src_h / detected).max(1);

    // Step 2: Center-sample each pixel block.
    // Instead of Nearest/Lanczos resize, sample the CENTER pixel of each NxN block.
    // This avoids anti-aliasing artifacts at block boundaries.
    let sampled = if detected > 1 {
        let mut img = RgbaImage::new(native_w, native_h);
        let half = detected / 2;
        for y in 0..native_h {
            for x in 0..native_w {
                let src_x = (x * detected + half).min(src_w - 1);
                let src_y = (y * detected + half).min(src_h - 1);
                img.put_pixel(x, y, *rgba.get_pixel(src_x, src_y));
            }
        }
        DynamicImage::ImageRgba8(img)
    } else {
        reference.clone()
    };

    // Step 3: Resize to target if native != target
    let final_img = if native_w != target_width || native_h != target_height {
        sampled.resize_exact(target_width, target_height, image::imageops::Nearest)
    } else {
        sampled
    };

    // Step 4: Quantize to palette
    let palette_entries: Vec<(char, PaxRgba)> = palette
        .symbols
        .iter()
        .map(|(&c, &rgba)| (c, rgba))
        .collect();

    let void_sym = palette_entries
        .iter()
        .find(|(_, rgba)| rgba.a < 128)
        .map(|&(c, _)| c)
        .unwrap_or('.');

    const ALPHA_THRESHOLD: u8 = 128;

    let mut grid = Vec::with_capacity(target_height as usize);
    let mut total_distance: f64 = 0.0;
    let mut clipped = 0u32;

    for y in 0..target_height {
        let mut row = Vec::with_capacity(target_width as usize);
        for x in 0..target_width {
            let pixel = final_img.get_pixel(x, y);

            if pixel.0[3] < ALPHA_THRESHOLD {
                row.push(void_sym);
                continue;
            }

            let (sym, dist) = nearest_palette_symbol(&pixel, &palette_entries);
            row.push(sym);
            total_distance += dist;
            if dist > 50.0 {
                clipped += 1;
            }
        }
        grid.push(row);
    }

    if dither {
        apply_bayer_dither(&mut grid, &final_img, &palette_entries);
    }

    // Step 5: Enforce outlines — boundary pixels (non-void touching void)
    // get replaced with the darkest non-void palette color.
    enforce_outlines(&mut grid, &palette_entries, void_sym);

    let total_pixels = (target_width * target_height) as f64;
    let non_void = grid.iter().flat_map(|r| r.iter()).filter(|&&c| c != void_sym).count() as f64;
    let color_accuracy = if non_void > 0.0 {
        1.0 - (total_distance / non_void / 255.0).min(1.0)
    } else {
        1.0
    };

    let grid_string: String = grid
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");

    ImportResult {
        grid,
        grid_string,
        width: target_width,
        height: target_height,
        color_accuracy,
        clipped_colors: clipped,
        detected_pixel_size: detected,
        native_resolution: (native_w, native_h),
    }
}

/// Result of an import operation.
pub struct ImportResult {
    pub grid: Vec<Vec<char>>,
    pub grid_string: String,
    pub width: u32,
    pub height: u32,
    pub color_accuracy: f64,
    pub clipped_colors: u32,
    /// Detected pixel block size in the source image (e.g., 32 means each art pixel was 32x32 screen pixels).
    pub detected_pixel_size: u32,
    /// Native resolution of the pixel art (source_size / detected_pixel_size).
    pub native_resolution: (u32, u32),
}

// ── Pixel grid detection ────────────────────────────────────────────

/// Public wrapper for pixel size detection (used by diffusion bridge).
pub fn detect_pixel_size_pub(img: &DynamicImage) -> u32 {
    detect_pixel_size(img)
}

/// Detect the size of each "art pixel" in a high-resolution pixel art image.
///
/// Scans horizontal and vertical edges to find the most common block size.
/// Returns the estimated pixel size (e.g., 32 means each art pixel is 32x32).
fn detect_pixel_size(img: &DynamicImage) -> u32 {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    if w <= 64 || h <= 64 {
        return 1; // Already low-res, no detection needed
    }

    // Sample the middle rows/cols to avoid edge artifacts
    let sample_y = h / 2;
    let sample_x = w / 2;

    // Scan horizontally: count consecutive identical pixels
    let mut h_runs = Vec::new();
    let mut run_len = 1u32;
    for x in 1..w {
        let prev = rgba.get_pixel(x - 1, sample_y);
        let curr = rgba.get_pixel(x, sample_y);
        if pixels_similar(prev, curr) {
            run_len += 1;
        } else {
            if run_len >= 2 {
                h_runs.push(run_len);
            }
            run_len = 1;
        }
    }
    if run_len >= 2 {
        h_runs.push(run_len);
    }

    // Scan vertically
    let mut v_runs = Vec::new();
    run_len = 1;
    for y in 1..h {
        let prev = rgba.get_pixel(sample_x, y - 1);
        let curr = rgba.get_pixel(sample_x, y);
        if pixels_similar(prev, curr) {
            run_len += 1;
        } else {
            if run_len >= 2 {
                v_runs.push(run_len);
            }
            run_len = 1;
        }
    }
    if run_len >= 2 {
        v_runs.push(run_len);
    }

    // Find the most common run length (the pixel block size)
    let all_runs: Vec<u32> = h_runs.iter().chain(v_runs.iter()).copied().collect();
    if all_runs.is_empty() {
        return 1;
    }

    // Find mode of run lengths, biased toward common pixel art sizes
    let mut freq: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for &r in &all_runs {
        // Snap to common sizes: runs of 14-18 → 16, 28-36 → 32, etc.
        let snapped = snap_to_common_size(r);
        *freq.entry(snapped).or_insert(0) += 1;
    }

    let pixel_size = freq
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(size, _)| size)
        .unwrap_or(1);

    pixel_size.clamp(1, w / 4) // Don't return anything larger than 1/4 of image
}

/// Snap a run length to a common pixel art block size.
fn snap_to_common_size(run: u32) -> u32 {
    // Common pixel art rendering sizes
    const COMMON: &[u32] = &[8, 12, 16, 20, 24, 32, 40, 48, 64];
    COMMON
        .iter()
        .min_by_key(|&&s| (s as i32 - run as i32).unsigned_abs())
        .copied()
        .unwrap_or(run)
}

/// Check if two pixels are similar enough to be "the same art pixel."
fn pixels_similar(a: &Rgba<u8>, b: &Rgba<u8>) -> bool {
    let dr = (a.0[0] as i16 - b.0[0] as i16).unsigned_abs();
    let dg = (a.0[1] as i16 - b.0[1] as i16).unsigned_abs();
    let db = (a.0[2] as i16 - b.0[2] as i16).unsigned_abs();
    let da = (a.0[3] as i16 - b.0[3] as i16).unsigned_abs();
    // Tolerance for JPEG artifacts and subtle anti-aliasing
    (dr + dg + db) < 30 && da < 30
}

/// Find the next power-of-two between min and max (inclusive).
/// If no POT fits, returns min.
fn next_pot_between(min: u32, max: u32) -> u32 {
    let mut pot = min.next_power_of_two();
    if pot > max {
        // Try min itself
        pot = min;
    }
    pot.min(max).max(min)
}

// ── Palette quantization helpers ────────────────────────────────────

/// Enforce dark outlines: boundary pixels that are too light get darkened.
/// Only replaces pixels significantly lighter than the darkest palette color —
/// preserves mid-tone boundary pixels (like purple robe edges).
fn enforce_outlines(grid: &mut [Vec<char>], palette: &[(char, PaxRgba)], void_sym: char) {
    let h = grid.len();
    if h == 0 {
        return;
    }
    let w = grid[0].len();

    // Find darkest non-void palette symbol and its lightness
    let darkest_entry = palette
        .iter()
        .filter(|&(c, rgba)| *c != void_sym && rgba.a >= 128)
        .min_by(|(_, a), (_, b)| {
            let la = pixl_core::oklab::lightness(a.r, a.g, a.b);
            let lb = pixl_core::oklab::lightness(b.r, b.g, b.b);
            la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
        });

    let (darkest_sym, darkest_lightness) = match darkest_entry {
        Some(&(c, ref rgba)) => (c, pixl_core::oklab::lightness(rgba.r, rgba.g, rgba.b)),
        None => return,
    };

    // Only darken boundary pixels that are much lighter than the darkest color.
    // Threshold: if pixel lightness > darkest + 0.35, it's too light for an outline.
    let lightness_threshold = darkest_lightness + 0.35;

    // Build lightness lookup
    let lightness_map: std::collections::HashMap<char, f32> = palette
        .iter()
        .map(|&(c, ref rgba)| (c, pixl_core::oklab::lightness(rgba.r, rgba.g, rgba.b)))
        .collect();

    let mut to_darken: Vec<(usize, usize)> = Vec::new();
    for y in 0..h {
        for x in 0..w {
            if grid[y][x] == void_sym {
                continue;
            }
            let touches_void = (x > 0 && grid[y][x - 1] == void_sym)
                || (x + 1 < w && grid[y][x + 1] == void_sym)
                || (y > 0 && grid[y - 1][x] == void_sym)
                || (y + 1 < h && grid[y + 1][x] == void_sym);

            if touches_void {
                let pixel_l = lightness_map.get(&grid[y][x]).copied().unwrap_or(0.5);
                if pixel_l > lightness_threshold {
                    to_darken.push((x, y));
                }
            }
        }
    }

    for (x, y) in to_darken {
        grid[y][x] = darkest_sym;
    }
}

fn nearest_palette_symbol(pixel: &Rgba<u8>, palette: &[(char, PaxRgba)]) -> (char, f64) {
    palette
        .iter()
        .map(|(sym, rgba)| {
            let dist = perceptual_distance(pixel, rgba);
            (*sym, dist)
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(('.', 999.0))
}

fn perceptual_distance(a: &Rgba<u8>, b: &PaxRgba) -> f64 {
    let lab_a = pixl_core::oklab::rgb_to_oklab(a.0[0], a.0[1], a.0[2]);
    let lab_b = pixl_core::oklab::rgb_to_oklab(b.r, b.g, b.b);
    pixl_core::oklab::delta_e(&lab_a, &lab_b) as f64
}

fn apply_bayer_dither(grid: &mut [Vec<char>], source: &DynamicImage, palette: &[(char, PaxRgba)]) {
    const BAYER_4X4: [[f64; 4]; 4] = [
        [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
        [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
        [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
        [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
    ];

    let spread = 32.0;

    for y in 0..grid.len() {
        for x in 0..grid[0].len() {
            let threshold = BAYER_4X4[y % 4][x % 4];
            let pixel = source.get_pixel(x as u32, y as u32);

            let dithered = Rgba([
                (pixel.0[0] as f64 + (threshold - 0.5) * spread).clamp(0.0, 255.0) as u8,
                (pixel.0[1] as f64 + (threshold - 0.5) * spread).clamp(0.0, 255.0) as u8,
                (pixel.0[2] as f64 + (threshold - 0.5) * spread).clamp(0.0, 255.0) as u8,
                pixel.0[3],
            ]);

            let (sym, _) = nearest_palette_symbol(&dithered, palette);
            grid[y][x] = sym;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use std::collections::HashMap;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert('.', PaxRgba { r: 0, g: 0, b: 0, a: 0 });
        symbols.insert('#', PaxRgba { r: 128, g: 128, b: 128, a: 255 });
        symbols.insert('+', PaxRgba { r: 255, g: 255, b: 255, a: 255 });
        Palette { symbols }
    }

    #[test]
    fn import_solid_black_image() {
        let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(32, 32, Rgba([0, 0, 0, 0])));
        let result = import_reference(&img, 4, 4, &test_palette(), false);
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        // All transparent pixels → void
        for row in &result.grid {
            for &ch in row {
                assert_eq!(ch, '.');
            }
        }
    }

    #[test]
    fn import_solid_white_image() {
        let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(32, 32, Rgba([255, 255, 255, 255])));
        let result = import_reference(&img, 4, 4, &test_palette(), false);
        for row in &result.grid {
            for &ch in row {
                assert_eq!(ch, '+');
            }
        }
    }

    #[test]
    fn import_downscales() {
        let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(256, 256, Rgba([128, 128, 128, 255])));
        let result = import_reference(&img, 16, 16, &test_palette(), false);
        assert_eq!(result.grid.len(), 16);
        assert_eq!(result.grid[0].len(), 16);
    }

    #[test]
    fn detect_pixel_size_on_lowres() {
        let img = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(16, 16, Rgba([0, 0, 0, 255])));
        assert_eq!(detect_pixel_size(&img), 1);
    }

    #[test]
    fn detect_pixel_size_on_scaled_up() {
        // Create a 4x4 pixel art image scaled up 8x to 32x32
        let mut img = RgbaImage::new(32, 32);
        let colors = [
            Rgba([255, 0, 0, 255]),
            Rgba([0, 255, 0, 255]),
            Rgba([0, 0, 255, 255]),
            Rgba([255, 255, 0, 255]),
        ];
        for y in 0..32u32 {
            for x in 0..32u32 {
                let art_x = x / 8;
                let art_y = y / 8;
                let idx = ((art_y * 4 + art_x) % 4) as usize;
                img.put_pixel(x, y, colors[idx]);
            }
        }
        // Should be too small for detection (32x32 < 64 threshold)
        let detected = detect_pixel_size(&DynamicImage::ImageRgba8(img));
        assert_eq!(detected, 1);
    }

    #[test]
    fn detect_pixel_size_on_large_scaled() {
        // Create a 16x16 art scaled up 16x to 256x256
        let mut img = RgbaImage::new(256, 256);
        for y in 0..256u32 {
            for x in 0..256u32 {
                let art_x = x / 16;
                let art_y = y / 16;
                // Alternating colors per art pixel
                let c = if (art_x + art_y) % 2 == 0 { 200u8 } else { 50u8 };
                img.put_pixel(x, y, Rgba([c, c, c, 255]));
            }
        }
        let detected = detect_pixel_size(&DynamicImage::ImageRgba8(img));
        assert_eq!(detected, 16);
    }

    #[test]
    fn transparent_pixels_become_void() {
        let mut img = RgbaImage::new(4, 4);
        // Left half opaque white, right half transparent
        for y in 0..4u32 {
            for x in 0..4u32 {
                if x < 2 {
                    img.put_pixel(x, y, Rgba([255, 255, 255, 255]));
                } else {
                    img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }
        let result = import_reference(&DynamicImage::ImageRgba8(img), 4, 4, &test_palette(), false);
        for row in &result.grid {
            assert_eq!(row[0], '+'); // white (interior)
            // row[1] is boundary — outline enforcement makes it darkest ('#')
            assert_eq!(row[1], '#'); // dark outline (was white, but touches void)
            assert_eq!(row[2], '.'); // void
            assert_eq!(row[3], '.'); // void
        }
    }

    #[test]
    fn next_pot_between_values() {
        assert_eq!(next_pot_between(16, 64), 16);
        assert_eq!(next_pot_between(16, 48), 16);
        assert_eq!(next_pot_between(8, 32), 8);
    }

    #[test]
    fn snap_common_sizes() {
        assert_eq!(snap_to_common_size(15), 16);
        assert_eq!(snap_to_common_size(17), 16);
        assert_eq!(snap_to_common_size(30), 32);
        assert_eq!(snap_to_common_size(33), 32);
    }
}
