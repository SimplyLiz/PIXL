use image::{DynamicImage, GenericImageView, Rgba};
use pixl_core::types::{Palette, Rgba as PaxRgba};

/// Quantize a reference image into a PAX character grid using the given palette.
/// Downscales to target_size, then maps each pixel to the nearest palette symbol.
pub fn import_reference(
    reference: &DynamicImage,
    target_width: u32,
    target_height: u32,
    palette: &Palette,
    dither: bool,
) -> ImportResult {
    // Step 1: Downscale with Lanczos3 (preserves edges better than bilinear)
    let small = reference.resize_exact(target_width, target_height, image::imageops::Lanczos3);

    // Step 2: Quantize each pixel to nearest palette symbol
    let mut grid = Vec::with_capacity(target_height as usize);
    let mut total_distance: f64 = 0.0;
    let mut clipped = 0u32;

    // Build palette lookup vec for efficiency
    let palette_entries: Vec<(char, PaxRgba)> = palette.symbols.iter().map(|(&c, &rgba)| (c, rgba)).collect();

    for y in 0..target_height {
        let mut row = Vec::with_capacity(target_width as usize);
        for x in 0..target_width {
            let pixel = small.get_pixel(x, y);
            let (sym, dist) = nearest_palette_symbol(&pixel, &palette_entries);
            row.push(sym);
            total_distance += dist;
            if dist > 50.0 {
                clipped += 1;
            }
        }
        grid.push(row);
    }

    // Optional Bayer dithering
    if dither {
        apply_bayer_dither(&mut grid, &small, &palette_entries);
    }

    let total_pixels = (target_width * target_height) as f64;
    let color_accuracy = 1.0 - (total_distance / total_pixels / 255.0).min(1.0);

    // Convert grid to PAX string
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
}

/// Find the nearest palette symbol for a pixel using perceptual weighted distance.
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

/// Perceptual color distance via OKLab — perceptually uniform ΔE.
fn perceptual_distance(a: &Rgba<u8>, b: &PaxRgba) -> f64 {
    let lab_a = pixl_core::oklab::rgb_to_oklab(a.0[0], a.0[1], a.0[2]);
    let lab_b = pixl_core::oklab::rgb_to_oklab(b.r, b.g, b.b);
    pixl_core::oklab::delta_e(&lab_a, &lab_b) as f64
}

/// Apply 4x4 Bayer ordered dithering.
fn apply_bayer_dither(
    grid: &mut [Vec<char>],
    source: &DynamicImage,
    palette: &[(char, PaxRgba)],
) {
    const BAYER_4X4: [[f64; 4]; 4] = [
        [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
        [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
        [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
        [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
    ];

    let spread = 32.0; // dither strength

    for y in 0..grid.len() {
        for x in 0..grid[0].len() {
            let threshold = BAYER_4X4[y % 4][x % 4];
            let pixel = source.get_pixel(x as u32, y as u32);

            // Apply dither offset to pixel before quantization
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
        symbols.insert('.', PaxRgba { r: 0, g: 0, b: 0, a: 255 });
        symbols.insert('#', PaxRgba { r: 128, g: 128, b: 128, a: 255 });
        symbols.insert('+', PaxRgba { r: 255, g: 255, b: 255, a: 255 });
        Palette { symbols }
    }

    #[test]
    fn import_solid_black_image() {
        let img = DynamicImage::ImageRgba8(
            ImageBuffer::from_pixel(32, 32, Rgba([0, 0, 0, 255])),
        );
        let result = import_reference(&img, 4, 4, &test_palette(), false);
        assert_eq!(result.width, 4);
        assert_eq!(result.height, 4);
        // All pixels should map to '.' (black)
        for row in &result.grid {
            for &ch in row {
                assert_eq!(ch, '.');
            }
        }
        assert!(result.color_accuracy > 0.99);
    }

    #[test]
    fn import_solid_white_image() {
        let img = DynamicImage::ImageRgba8(
            ImageBuffer::from_pixel(32, 32, Rgba([255, 255, 255, 255])),
        );
        let result = import_reference(&img, 4, 4, &test_palette(), false);
        for row in &result.grid {
            for &ch in row {
                assert_eq!(ch, '+');
            }
        }
    }

    #[test]
    fn import_downscales() {
        let img = DynamicImage::ImageRgba8(
            ImageBuffer::from_pixel(256, 256, Rgba([128, 128, 128, 255])),
        );
        let result = import_reference(&img, 16, 16, &test_palette(), false);
        assert_eq!(result.grid.len(), 16);
        assert_eq!(result.grid[0].len(), 16);
    }

    #[test]
    fn import_with_dither_produces_grid() {
        let img = DynamicImage::ImageRgba8(
            ImageBuffer::from_pixel(32, 32, Rgba([64, 64, 64, 255])),
        );
        let result = import_reference(&img, 8, 8, &test_palette(), true);
        assert_eq!(result.grid.len(), 8);
        // Dithered result should have a mix of symbols
    }

    #[test]
    fn grid_string_format() {
        let img = DynamicImage::ImageRgba8(
            ImageBuffer::from_pixel(4, 4, Rgba([0, 0, 0, 255])),
        );
        let result = import_reference(&img, 2, 2, &test_palette(), false);
        assert_eq!(result.grid_string, "..\n..");
    }
}
