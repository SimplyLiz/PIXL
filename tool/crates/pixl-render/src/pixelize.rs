//! Convert high-resolution "fake pixel art" images to true 1:1 pixel art.
//!
//! AI image generators produce images that *look* like pixel art but are
//! rendered at high resolution with anti-aliasing and sub-pixel blending.
//! This module downsamples to a target resolution and quantizes the palette
//! to produce clean, grid-aligned output.

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use std::path::{Path, PathBuf};

/// Preset resolution/color configurations.
#[derive(Debug, Clone, Copy)]
pub struct ConvertPreset {
    pub name: &'static str,
    pub max_width: u32,
    pub num_colors: u32,
}

pub const PRESET_SMALL: ConvertPreset = ConvertPreset {
    name: "small",
    max_width: 128,
    num_colors: 16,
};

pub const PRESET_MEDIUM: ConvertPreset = ConvertPreset {
    name: "medium",
    max_width: 160,
    num_colors: 32,
};

pub const PRESET_LARGE: ConvertPreset = ConvertPreset {
    name: "large",
    max_width: 256,
    num_colors: 48,
};

pub const ALL_PRESETS: [ConvertPreset; 3] = [PRESET_SMALL, PRESET_MEDIUM, PRESET_LARGE];

/// Result of a single conversion.
pub struct ConvertResult {
    pub image: RgbaImage,
    pub width: u32,
    pub height: u32,
    pub num_colors: u32,
    pub preset_name: String,
}

/// Convert batch result — all presets for one input image.
pub struct ConvertBatchResult {
    pub original_path: PathBuf,
    pub original_size: (u32, u32),
    pub results: Vec<ConvertResult>,
}

/// Pixelize a single image at a given resolution and color count.
pub fn pixelize(
    img: &DynamicImage,
    max_width: u32,
    num_colors: u32,
) -> ConvertResult {
    let (src_w, src_h) = img.dimensions();
    let aspect = src_h as f64 / src_w as f64;

    let tw = max_width.min(src_w);
    let th = ((tw as f64 * aspect).round() as u32).max(1);

    // Step 1: Downsample with Lanczos3 for high-quality color averaging
    let small = img.resize_exact(tw, th, image::imageops::Lanczos3);

    // Step 2: Palette quantization via median-cut on the RGB image
    let rgba_img = small.to_rgba8();
    let quantized = if num_colors > 0 && num_colors < 256 {
        quantize_median_cut(&rgba_img, num_colors)
    } else {
        rgba_img
    };

    ConvertResult {
        image: quantized,
        width: tw,
        height: th,
        num_colors,
        preset_name: String::new(),
    }
}

/// Convert an image using all three presets, writing results to an output directory.
///
/// Creates:
/// ```text
/// out_dir/
///   originals/   — copy of input
///   small/       — 128px wide, 16 colors
///   medium/      — 160px wide, 32 colors
///   large/       — 256px wide, 48 colors
/// ```
pub fn convert_batch(
    input_path: &Path,
    out_dir: &Path,
) -> Result<ConvertBatchResult, String> {
    let img = image::open(input_path)
        .map_err(|e| format!("cannot open image {}: {}", input_path.display(), e))?;

    let (src_w, src_h) = img.dimensions();
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let ext = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");

    // Create output directories
    let originals_dir = out_dir.join("originals");
    std::fs::create_dir_all(&originals_dir)
        .map_err(|e| format!("cannot create originals dir: {e}"))?;

    // Copy original
    let orig_dest = originals_dir.join(format!("{stem}.{ext}"));
    std::fs::copy(input_path, &orig_dest)
        .map_err(|e| format!("cannot copy original: {e}"))?;

    let mut results = Vec::new();

    for preset in &ALL_PRESETS {
        let preset_dir = out_dir.join(preset.name);
        std::fs::create_dir_all(&preset_dir)
            .map_err(|e| format!("cannot create {} dir: {e}", preset.name))?;

        let mut result = pixelize(&img, preset.max_width, preset.num_colors);
        result.preset_name = preset.name.to_string();

        // Save the 1:1 pixel art file
        let out_path = preset_dir.join(format!("{stem}.png"));
        result.image.save(&out_path)
            .map_err(|e| format!("cannot save {}: {e}", out_path.display()))?;

        results.push(result);
    }

    Ok(ConvertBatchResult {
        original_path: input_path.to_path_buf(),
        original_size: (src_w, src_h),
        results,
    })
}

/// Convert a single image and return the result as PNG bytes (for HTTP/MCP use).
pub fn pixelize_to_png_bytes(
    img: &DynamicImage,
    max_width: u32,
    num_colors: u32,
) -> Result<Vec<u8>, String> {
    let result = pixelize(img, max_width, num_colors);
    let mut buf = std::io::Cursor::new(Vec::new());
    result.image
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| format!("PNG encode error: {e}"))?;
    Ok(buf.into_inner())
}

// ── Palette quantization ────────────────────────────────────────

/// Simple median-cut color quantization.
fn quantize_median_cut(img: &RgbaImage, max_colors: u32) -> RgbaImage {
    let (w, h) = img.dimensions();

    // Collect all opaque pixels
    let mut pixels: Vec<[u8; 3]> = Vec::with_capacity((w * h) as usize);
    for pixel in img.pixels() {
        if pixel.0[3] > 0 {
            pixels.push([pixel.0[0], pixel.0[1], pixel.0[2]]);
        }
    }

    if pixels.is_empty() {
        return img.clone();
    }

    // Build palette via median cut
    let palette = median_cut(&mut pixels, max_colors as usize);

    // Map each pixel to nearest palette entry
    let mut out = ImageBuffer::new(w, h);
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel.0[3] == 0 {
            out.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            continue;
        }
        let rgb = [pixel.0[0], pixel.0[1], pixel.0[2]];
        let nearest = find_nearest(&rgb, &palette);
        out.put_pixel(x, y, Rgba([nearest[0], nearest[1], nearest[2], pixel.0[3]]));
    }

    out
}

fn median_cut(pixels: &mut [[u8; 3]], max_colors: usize) -> Vec<[u8; 3]> {
    if pixels.is_empty() {
        return vec![[0, 0, 0]];
    }

    let mut buckets: Vec<Vec<[u8; 3]>> = vec![pixels.to_vec()];

    while buckets.len() < max_colors {
        // Find bucket with largest range
        let (idx, _) = buckets
            .iter()
            .enumerate()
            .max_by_key(|(_, b)| {
                if b.len() <= 1 {
                    return 0u32;
                }
                let range = channel_range(b);
                range[0].max(range[1]).max(range[2]) as u32 * b.len() as u32
            })
            .unwrap();

        let bucket = buckets.remove(idx);
        if bucket.len() <= 1 {
            buckets.push(bucket);
            break;
        }

        let range = channel_range(&bucket);
        let split_channel = if range[0] >= range[1] && range[0] >= range[2] {
            0
        } else if range[1] >= range[2] {
            1
        } else {
            2
        };

        let mut sorted = bucket;
        sorted.sort_unstable_by_key(|p| p[split_channel]);
        let mid = sorted.len() / 2;
        let (left, right) = sorted.split_at(mid);
        buckets.push(left.to_vec());
        buckets.push(right.to_vec());
    }

    // Average each bucket
    buckets
        .iter()
        .map(|b| {
            let n = b.len() as u32;
            let (mut sr, mut sg, mut sb) = (0u32, 0u32, 0u32);
            for p in b {
                sr += p[0] as u32;
                sg += p[1] as u32;
                sb += p[2] as u32;
            }
            [(sr / n) as u8, (sg / n) as u8, (sb / n) as u8]
        })
        .collect()
}

fn channel_range(pixels: &[[u8; 3]]) -> [u8; 3] {
    let mut min = [255u8; 3];
    let mut max = [0u8; 3];
    for p in pixels {
        for c in 0..3 {
            min[c] = min[c].min(p[c]);
            max[c] = max[c].max(p[c]);
        }
    }
    [max[0] - min[0], max[1] - min[1], max[2] - min[2]]
}

fn find_nearest(pixel: &[u8; 3], palette: &[[u8; 3]]) -> [u8; 3] {
    palette
        .iter()
        .min_by_key(|p| {
            let dr = pixel[0] as i32 - p[0] as i32;
            let dg = pixel[1] as i32 - p[1] as i32;
            let db = pixel[2] as i32 - p[2] as i32;
            // Weighted perceptual distance (human eye is most sensitive to green)
            2 * dr * dr + 4 * dg * dg + 3 * db * db
        })
        .copied()
        .unwrap_or([0, 0, 0])
}

// ── Backdrop import pipeline ─────────────────────────────────────────

/// Result of slicing an image into a backdrop.
pub struct BackdropImportResult {
    pub pax_source: String,
    pub tile_count: usize,
    pub unique_tiles: usize,
    pub cols: u32,
    pub rows: u32,
}

/// Generate PAX TOML source for a backdrop from a pixelized image.
pub fn import_backdrop(
    img: &DynamicImage,
    name: &str,
    max_colors: u32,
    tile_size: u32,
) -> Result<BackdropImportResult, String> {
    let rgba = img.to_rgba8();
    let (img_w, img_h) = rgba.dimensions();

    // Ensure dimensions are divisible by tile size
    let cols = img_w / tile_size;
    let rows = img_h / tile_size;
    if cols == 0 || rows == 0 {
        return Err(format!(
            "image {}x{} too small for {}x{} tiles",
            img_w, img_h, tile_size, tile_size
        ));
    }

    let effective_w = cols * tile_size;
    let effective_h = rows * tile_size;

    // Step 1: Extract all unique colors and build palette
    let mut color_counts: std::collections::HashMap<[u8; 3], u32> = std::collections::HashMap::new();
    for y in 0..effective_h {
        for x in 0..effective_w {
            let px = rgba.get_pixel(x, y);
            if px.0[3] > 0 {
                let rgb = [px.0[0], px.0[1], px.0[2]];
                *color_counts.entry(rgb).or_insert(0) += 1;
            }
        }
    }

    // Quantize to max_colors if needed
    let mut all_pixels: Vec<[u8; 3]> = Vec::new();
    for y in 0..effective_h {
        for x in 0..effective_w {
            let px = rgba.get_pixel(x, y);
            if px.0[3] > 0 {
                all_pixels.push([px.0[0], px.0[1], px.0[2]]);
            }
        }
    }
    let palette_colors = median_cut(&mut all_pixels, max_colors as usize);

    // Step 2: Assign symbols — top 16 most-used get single chars, rest get multi-char
    let base_chars: Vec<char> = ".#+-~bBwWghsmrcdDfoOpPeElLkKtTnNqQxXyYzZ0123456789"
        .chars()
        .collect();

    // Sort palette by frequency (approximate — count pixels that map to each color)
    let mut color_freq: Vec<([u8; 3], u32)> = palette_colors
        .iter()
        .map(|c| {
            let count = color_counts
                .iter()
                .filter(|(rgb, _)| find_nearest(rgb, &palette_colors) == *c)
                .map(|(_, cnt)| cnt)
                .sum();
            (*c, count)
        })
        .collect();
    color_freq.sort_by(|a, b| b.1.cmp(&a.1));

    let base_count = 16.min(color_freq.len()).min(base_chars.len());
    let mut base_syms: Vec<(char, [u8; 3])> = Vec::new();
    let mut ext_syms: Vec<(String, [u8; 3])> = Vec::new();

    for (i, (color, _)) in color_freq.iter().enumerate() {
        if i < base_count {
            base_syms.push((base_chars[i], *color));
        } else {
            let digit = 2 + (i - base_count) / 26;
            let letter = (b'a' + ((i - base_count) % 26) as u8) as char;
            ext_syms.push((format!("{}{}", digit, letter), *color));
        }
    }

    // Build full color→symbol lookup
    let mut color_to_sym: std::collections::HashMap<[u8; 3], String> = std::collections::HashMap::new();
    for (ch, color) in &base_syms {
        color_to_sym.insert(*color, ch.to_string());
    }
    for (sym, color) in &ext_syms {
        color_to_sym.insert(*color, sym.clone());
    }

    // Step 3: Slice into tiles and build symbol grids
    let mut tile_data: Vec<Vec<Vec<String>>> = Vec::new(); // per-slot tile grid
    let mut tile_hashes: Vec<u64> = Vec::new();
    let mut unique_tiles: Vec<(String, Vec<Vec<String>>)> = Vec::new(); // (name, grid)
    let mut hash_to_name: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    let mut tilemap: Vec<Vec<String>> = Vec::new();

    for row in 0..rows {
        let mut tilemap_row = Vec::new();
        for col in 0..cols {
            let mut grid: Vec<Vec<String>> = Vec::new();
            let mut hasher_data: Vec<u8> = Vec::new();

            for ty in 0..tile_size {
                let mut grid_row = Vec::new();
                for tx in 0..tile_size {
                    let px = rgba.get_pixel(col * tile_size + tx, row * tile_size + ty);
                    let rgb = [px.0[0], px.0[1], px.0[2]];
                    let nearest = find_nearest(&rgb, &palette_colors);
                    let sym = color_to_sym.get(&nearest)
                        .cloned()
                        .unwrap_or_else(|| ".".to_string());
                    hasher_data.extend_from_slice(&nearest);
                    grid_row.push(sym);
                }
                grid.push(grid_row);
            }

            // Simple hash for dedup
            let hash = {
                let mut h: u64 = 0xcbf29ce484222325;
                for b in &hasher_data {
                    h ^= *b as u64;
                    h = h.wrapping_mul(0x100000001b3);
                }
                h
            };

            let tile_name = if let Some(existing) = hash_to_name.get(&hash) {
                existing.clone()
            } else {
                let name = format!("bt_{:03}", unique_tiles.len());
                hash_to_name.insert(hash, name.clone());
                unique_tiles.push((name.clone(), grid.clone()));
                name
            };

            tilemap_row.push(tile_name);
        }
        tilemap.push(tilemap_row);
    }

    // Step 4: Generate PAX TOML
    let has_ext = !ext_syms.is_empty();
    let mut pax = String::new();

    // Header
    pax.push_str(&format!(
        "[pax]\nversion = \"2.0\"\nname = \"{name}\"\n\n"
    ));

    // Base palette
    pax.push_str(&format!("[palette.{name}]\n"));
    for (ch, color) in &base_syms {
        pax.push_str(&format!(
            "\"{}\" = \"#{:02x}{:02x}{:02x}ff\"\n",
            ch, color[0], color[1], color[2]
        ));
    }
    pax.push('\n');

    // Extended palette
    if has_ext {
        pax.push_str(&format!("[palette_ext.{name}]\nbase = \"{name}\"\n"));
        for (sym, color) in &ext_syms {
            pax.push_str(&format!(
                "\"{}\" = \"#{:02x}{:02x}{:02x}ff\"\n",
                sym, color[0], color[1], color[2]
            ));
        }
        pax.push('\n');
    }

    // Backdrop tiles (RLE-encoded)
    for (tile_name, grid) in &unique_tiles {
        pax.push_str(&format!("[backdrop_tile.{tile_name}]\n"));
        pax.push_str(&format!("palette = \"{name}\"\n"));
        if has_ext {
            pax.push_str(&format!("palette_ext = \"{name}\"\n"));
        }
        pax.push_str(&format!("size = \"{}x{}\"\n", tile_size, tile_size));

        // Use RLE encoding
        let rle = pixl_core::rle::encode_rle_ext(grid);
        pax.push_str(&format!("rle = '''\n{rle}\n'''\n\n"));
    }

    // Backdrop definition
    pax.push_str(&format!("[backdrop.{name}]\n"));
    pax.push_str(&format!("palette = \"{name}\"\n"));
    if has_ext {
        pax.push_str(&format!("palette_ext = \"{name}\"\n"));
    }
    pax.push_str(&format!("size = \"{}x{}\"\n", effective_w, effective_h));
    pax.push_str(&format!("tile_size = \"{}x{}\"\n", tile_size, tile_size));
    pax.push_str("tilemap = '''\n");
    for row in &tilemap {
        pax.push_str(&row.join("  "));
        pax.push('\n');
    }
    pax.push_str("'''\n\n");

    // Suggested animation zones (commented out)
    pax.push_str("# ── Suggested animation zones (uncomment and tune) ──\n");
    pax.push_str(&format!("# [[backdrop.{name}.zone]]\n"));
    pax.push_str("# name = \"water\"\n");
    pax.push_str(&format!("# rect = {{ x = 0, y = {}, w = {}, h = {} }}\n",
        effective_h / 2, effective_w, effective_h / 2));
    pax.push_str("# behavior = \"cycle\"\n");
    pax.push_str("# cycle = \"water_shimmer\"\n");

    let total_tiles = (cols * rows) as usize;
    Ok(BackdropImportResult {
        pax_source: pax,
        tile_count: total_tiles,
        unique_tiles: unique_tiles.len(),
        cols,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn pixelize_solid_color() {
        let img = DynamicImage::ImageRgba8(
            ImageBuffer::from_pixel(256, 256, Rgba([100, 150, 200, 255])),
        );
        let result = pixelize(&img, 32, 8);
        assert_eq!(result.width, 32);
        assert_eq!(result.height, 32);
        // All pixels should be the same color (solid input)
        let first = result.image.get_pixel(0, 0);
        for pixel in result.image.pixels() {
            assert_eq!(pixel, first);
        }
    }

    #[test]
    fn pixelize_preserves_aspect_ratio() {
        let img = DynamicImage::ImageRgba8(
            ImageBuffer::from_pixel(400, 600, Rgba([0, 0, 0, 255])),
        );
        let result = pixelize(&img, 100, 16);
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 150); // 600/400 * 100
    }

    #[test]
    fn median_cut_produces_correct_palette_size() {
        let mut pixels = vec![
            [255, 0, 0], [0, 255, 0], [0, 0, 255],
            [128, 0, 0], [0, 128, 0], [0, 0, 128],
            [255, 255, 0], [0, 255, 255],
        ];
        let palette = median_cut(&mut pixels, 4);
        assert!(palette.len() <= 4);
        assert!(!palette.is_empty());
    }
}
