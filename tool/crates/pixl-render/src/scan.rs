//! Sprite sheet scanning and patch extraction.
//!
//! Scans reference images (sprite sheets, tilesets, individual sprites) and
//! extracts quality-filtered patches suitable for ML training or style analysis.
//!
//! This module generalizes the prototype from `tool/scripts/train_tiles.py` and
//! `training/prepare_eotb_optimal.py` into production Rust code.

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ─── Types ───────────────────────────────────────────────────────────────────

/// Result of scanning a single image source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Source file path.
    pub source: PathBuf,
    /// Source image dimensions.
    pub source_size: (u32, u32),
    /// Background colors detected in this image.
    pub bg_colors: Vec<[u8; 3]>,
    /// All extracted patches (before quality filtering).
    pub total_patches: usize,
    /// Patches that passed quality filtering.
    pub quality_patches: usize,
    /// Per-patch metadata.
    pub patches: Vec<PatchInfo>,
}

/// Metadata for a single extracted patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchInfo {
    /// Unique patch ID within the scan.
    pub id: usize,
    /// Source image this patch came from.
    pub source: PathBuf,
    /// Bounding box in source image: (x, y, w, h).
    pub bbox: (u32, u32, u32, u32),
    /// Quality metrics.
    pub quality: PatchQuality,
    /// Auto-detected category (wall, floor, enemy, item, etc.).
    pub category: String,
    /// Output filename for the extracted patch.
    pub filename: String,
}

/// Quality metrics for a patch, used for filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchQuality {
    /// Fraction of pixels matching a background/transparency color.
    pub bg_ratio: f64,
    /// Number of distinct colors (quantized to reduce noise).
    pub unique_colors: usize,
    /// Fraction of adjacent pixel pairs that differ (texture detail).
    pub edge_density: f64,
    /// Variance of luminance values (information content).
    pub luminance_variance: f64,
}

/// Configuration for the scan pipeline.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    /// Size of extracted patches (default: 16).
    pub patch_size: u32,
    /// Stride for sliding window extraction (default: patch_size for non-overlapping).
    pub stride: u32,
    /// Minimum unique colors to pass quality filter (default: 2).
    pub min_colors: usize,
    /// Maximum background ratio to pass quality filter (default: 0.85).
    pub max_bg_ratio: f64,
    /// Minimum luminance variance to pass quality filter (default: 10.0).
    pub min_lum_variance: f64,
    /// If set, tiles from grid-based sheets are cut at this native size
    /// then resized to patch_size (e.g. 32 for DCSS tiles → 16x16 patches).
    pub native_tile_size: Option<u32>,
    /// Additional background colors to detect (beyond auto-detection).
    pub extra_bg_colors: Vec<[u8; 3]>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            patch_size: 16,
            stride: 16,
            min_colors: 2,
            max_bg_ratio: 0.85,
            min_lum_variance: 10.0,
            native_tile_size: None,
            extra_bg_colors: vec![],
        }
    }
}

/// Complete scan manifest — output of the scan phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanManifest {
    /// Scan configuration used.
    pub patch_size: u32,
    pub stride: u32,
    /// Per-source results.
    pub sources: Vec<ScanResult>,
    /// Aggregate stats.
    pub total_patches_raw: usize,
    pub total_patches_quality: usize,
    pub total_filtered: usize,
    /// Category breakdown.
    pub categories: std::collections::HashMap<String, usize>,
}

// ─── Background color detection ─────────────────────────────────────────────

/// Well-known background/transparency key colors in pixel art sprite sheets.
const KNOWN_BG_COLORS: &[[u8; 3]] = &[
    [0, 255, 255],     // cyan
    [87, 255, 255],    // light cyan (Spriters Resource)
    [255, 0, 255],     // magenta
    [0, 128, 128],     // dark teal
    [255, 0, 128],     // hot pink
    [128, 0, 255],     // purple key
];

/// Detect background colors in an image by analyzing the most common colors.
/// Returns colors that appear in >15% of pixels and match known BG patterns.
pub fn detect_bg_colors(img: &RgbaImage) -> Vec<[u8; 3]> {
    let (w, h) = img.dimensions();
    let total = (w * h) as usize;
    if total == 0 {
        return vec![];
    }

    // Count color frequencies
    let mut counts = std::collections::HashMap::<[u8; 3], usize>::new();
    for pixel in img.pixels() {
        let rgb = [pixel[0], pixel[1], pixel[2]];
        *counts.entry(rgb).or_default() += 1;
    }

    let mut bg = vec![];

    // Check known BG colors
    for &known in KNOWN_BG_COLORS {
        if let Some(&count) = counts.get(&known) {
            if count > total / 20 {
                // > 5% of pixels
                bg.push(known);
            }
        }
    }

    // Check the most common color — if it's very dominant and saturated,
    // it's likely a key color
    if let Some((&color, &count)) = counts.iter().max_by_key(|(_, c)| *c) {
        let ratio = count as f64 / total as f64;
        let (r, g, b) = (color[0] as f64, color[1] as f64, color[2] as f64);
        let max_c = r.max(g).max(b);
        let min_c = r.min(g).min(b);
        let saturation = if max_c > 0.0 {
            (max_c - min_c) / max_c
        } else {
            0.0
        };

        // Highly saturated dominant color = likely BG key
        if ratio > 0.15 && saturation > 0.5 && !bg.contains(&color) {
            bg.push(color);
        }
    }

    // Always include pure black as potential BG (common in sliced tiles)
    if !bg.contains(&[0, 0, 0]) {
        bg.push([0, 0, 0]);
    }

    bg
}

/// Check if a pixel matches any background color.
fn is_bg_pixel(r: u8, g: u8, b: u8, bg_colors: &[[u8; 3]]) -> bool {
    bg_colors.iter().any(|bg| bg[0] == r && bg[1] == g && bg[2] == b)
}

// ─── Gutter detection & sprite sheet slicing ────────────────────────────────

/// Find tile bounding boxes in a sprite sheet by detecting background gutters.
///
/// Scans for rows/columns that are predominantly background color, then
/// treats contiguous non-background bands as tiles.
pub fn find_tile_bboxes(
    img: &RgbaImage,
    bg_colors: &[[u8; 3]],
    min_size: u32,
) -> Vec<(u32, u32, u32, u32)> {
    let (w, h) = img.dimensions();
    let threshold = 0.90; // row/col must be >90% BG to be a gutter

    // Compute BG fraction per row and column
    let row_bg_frac: Vec<f64> = (0..h)
        .map(|y| {
            let bg_count = (0..w)
                .filter(|&x| {
                    let p = img.get_pixel(x, y);
                    is_bg_pixel(p[0], p[1], p[2], bg_colors) || p[3] < 128
                })
                .count();
            bg_count as f64 / w as f64
        })
        .collect();

    let col_bg_frac: Vec<f64> = (0..w)
        .map(|x| {
            let bg_count = (0..h)
                .filter(|&y| {
                    let p = img.get_pixel(x, y);
                    is_bg_pixel(p[0], p[1], p[2], bg_colors) || p[3] < 128
                })
                .count();
            bg_count as f64 / h as f64
        })
        .collect();

    // Find contiguous non-gutter bands
    let row_bands = find_bands(&row_bg_frac, threshold, min_size);
    let col_bands = find_bands(&col_bg_frac, threshold, min_size);

    // Each intersection is a potential tile
    let mut bboxes = vec![];
    for &(y_start, y_end) in &row_bands {
        for &(x_start, x_end) in &col_bands {
            // Trim BG borders within the cell
            let mut min_x = x_end;
            let mut min_y = y_end;
            let mut max_x = x_start;
            let mut max_y = y_start;

            for y in y_start..y_end {
                for x in x_start..x_end {
                    let p = img.get_pixel(x, y);
                    if !is_bg_pixel(p[0], p[1], p[2], bg_colors) && p[3] >= 128 {
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                    }
                }
            }

            if max_x >= min_x && max_y >= min_y {
                let tw = max_x - min_x + 1;
                let th = max_y - min_y + 1;
                if tw >= min_size && th >= min_size {
                    bboxes.push((min_x, min_y, tw, th));
                }
            }
        }
    }

    // Sort top-to-bottom, left-to-right
    bboxes.sort_by_key(|&(x, y, _, _)| (y / 40, x));
    bboxes
}

/// Find contiguous bands where `frac[i] < threshold`.
fn find_bands(frac: &[f64], threshold: f64, min_size: u32) -> Vec<(u32, u32)> {
    let mut bands = vec![];
    let mut start: Option<u32> = None;

    for (i, &f) in frac.iter().enumerate() {
        let i = i as u32;
        if f < threshold {
            if start.is_none() {
                start = Some(i);
            }
        } else if let Some(s) = start {
            if i - s >= min_size {
                bands.push((s, i));
            }
            start = None;
        }
    }
    if let Some(s) = start {
        let len = frac.len() as u32;
        if len - s >= min_size {
            bands.push((s, len));
        }
    }
    bands
}

// ─── Patch extraction ───────────────────────────────────────────────────────

/// Extract patches from an image using a sliding window.
pub fn extract_patches(
    img: &RgbaImage,
    patch_size: u32,
    stride: u32,
) -> Vec<(RgbaImage, u32, u32)> {
    let (w, h) = img.dimensions();
    let mut patches = vec![];

    if w < patch_size || h < patch_size {
        // Too small — resize to patch_size directly
        let resized = image::imageops::resize(
            img,
            patch_size,
            patch_size,
            image::imageops::FilterType::Nearest,
        );
        patches.push((resized, 0, 0));
        return patches;
    }

    for y in (0..=h.saturating_sub(patch_size)).step_by(stride as usize) {
        for x in (0..=w.saturating_sub(patch_size)).step_by(stride as usize) {
            let patch = image::imageops::crop_imm(img, x, y, patch_size, patch_size).to_image();
            patches.push((patch, x, y));
        }
    }

    patches
}

// ─── Quality assessment ─────────────────────────────────────────────────────

/// Compute quality metrics for a patch.
pub fn assess_patch(patch: &RgbaImage, bg_colors: &[[u8; 3]]) -> PatchQuality {
    let (w, h) = patch.dimensions();
    let total = (w * h) as f64;

    // Background ratio
    let mut bg_count = 0usize;
    for p in patch.pixels() {
        if is_bg_pixel(p[0], p[1], p[2], bg_colors) || p[3] < 128 {
            bg_count += 1;
        }
    }
    let bg_ratio = bg_count as f64 / total;

    // Unique colors (quantized to reduce noise)
    let mut colors = std::collections::HashSet::new();
    for p in patch.pixels() {
        if !is_bg_pixel(p[0], p[1], p[2], bg_colors) && p[3] >= 128 {
            colors.insert([p[0] / 8, p[1] / 8, p[2] / 8]);
        }
    }

    // Edge density (transitions between different colors)
    let mut edges = 0usize;
    let mut edge_total = 0usize;
    for y in 0..h {
        for x in 0..w {
            let p = patch.get_pixel(x, y);
            if x + 1 < w {
                edge_total += 1;
                let q = patch.get_pixel(x + 1, y);
                if p[0] != q[0] || p[1] != q[1] || p[2] != q[2] {
                    edges += 1;
                }
            }
            if y + 1 < h {
                edge_total += 1;
                let q = patch.get_pixel(x, y + 1);
                if p[0] != q[0] || p[1] != q[1] || p[2] != q[2] {
                    edges += 1;
                }
            }
        }
    }
    let edge_density = if edge_total > 0 {
        edges as f64 / edge_total as f64
    } else {
        0.0
    };

    // Luminance variance
    let mut lum_sum = 0.0f64;
    let mut lum_sq_sum = 0.0f64;
    let mut lum_count = 0usize;
    for p in patch.pixels() {
        let l = 0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64;
        lum_sum += l;
        lum_sq_sum += l * l;
        lum_count += 1;
    }
    let luminance_variance = if lum_count > 1 {
        let mean = lum_sum / lum_count as f64;
        lum_sq_sum / lum_count as f64 - mean * mean
    } else {
        0.0
    };

    PatchQuality {
        bg_ratio,
        unique_colors: colors.len(),
        edge_density,
        luminance_variance,
    }
}

/// Check if a patch passes quality thresholds.
pub fn passes_quality(q: &PatchQuality, config: &ScanConfig) -> bool {
    q.bg_ratio <= config.max_bg_ratio
        && q.unique_colors >= config.min_colors
        && q.luminance_variance >= config.min_lum_variance
}

// ─── Auto-classification ────────────────────────────────────────────────────

/// Auto-classify a patch based on its source filename and visual features.
pub fn classify_patch(source_name: &str, quality: &PatchQuality) -> String {
    let name = source_name.to_lowercase();

    // Filename-based classification
    if name.contains("wall") {
        return "wall".into();
    }
    if name.contains("floor") || name.contains("cobble") || name.contains("ground") {
        return "floor".into();
    }
    if name.contains("ceiling") || name.contains("roof") {
        return "ceiling".into();
    }
    if name.contains("door") || name.contains("gate") {
        return "door".into();
    }
    if name.contains("chest") || name.contains("container") || name.contains("crate") {
        return "item".into();
    }
    if name.contains("enemy") || name.contains("monster") || name.contains("boss") {
        return "enemy".into();
    }
    if name.contains("water") || name.contains("lava") || name.contains("slime") {
        return "liquid".into();
    }
    if name.contains("tree") || name.contains("grass") || name.contains("plant") {
        return "vegetation".into();
    }
    if name.contains("pillar") || name.contains("column") {
        return "pillar".into();
    }

    // Feature-based fallback
    if quality.bg_ratio > 0.5 {
        "sprite".into() // lots of transparency → likely a sprite
    } else if quality.edge_density < 0.15 {
        "floor".into() // low detail → likely a floor/ceiling
    } else {
        "tile".into() // generic
    }
}

// ─── Main scan pipeline ─────────────────────────────────────────────────────

/// Scan a single image file and extract quality patches.
pub fn scan_image(path: &Path, config: &ScanConfig) -> Result<ScanResult, String> {
    let img = image::open(path)
        .map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    // Detect background colors
    let mut bg_colors = detect_bg_colors(&rgba);
    for extra in &config.extra_bg_colors {
        if !bg_colors.contains(extra) {
            bg_colors.push(*extra);
        }
    }

    let source_name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut patches = vec![];
    let mut total_raw = 0usize;
    let mut patch_id = 0usize;

    // Decide extraction strategy
    let is_spritesheet = w > config.patch_size * 3 && h > config.patch_size * 3;

    if is_spritesheet {
        // Strategy A: Detect tile boundaries in sprite sheet
        let bboxes = find_tile_bboxes(&rgba, &bg_colors, 8);

        if bboxes.len() > 1 {
            // Multiple tiles found — extract each one, then patch-extract from it
            for (bx, by, bw, bh) in &bboxes {
                let tile_img =
                    image::imageops::crop_imm(&rgba, *bx, *by, *bw, *bh).to_image();

                // If tile is larger than patch_size, extract patches from it
                if *bw > config.patch_size || *bh > config.patch_size {
                    let stride = if *bw <= config.patch_size * 3 {
                        config.patch_size // non-overlapping for small tiles
                    } else {
                        config.stride
                    };
                    for (patch_img, px, py) in extract_patches(&tile_img, config.patch_size, stride)
                    {
                        total_raw += 1;
                        let quality = assess_patch(&patch_img, &bg_colors);
                        if passes_quality(&quality, config) {
                            let category = classify_patch(&source_name, &quality);
                            let filename =
                                format!("{}_{:04}.png", source_name.replace(' ', "_"), patch_id);
                            patches.push(PatchInfo {
                                id: patch_id,
                                source: path.to_path_buf(),
                                bbox: (bx + px, by + py, config.patch_size, config.patch_size),
                                quality,
                                category,
                                filename,
                            });
                            patch_id += 1;
                        }
                    }
                } else {
                    // Tile is smaller than or equal to patch_size — resize directly
                    total_raw += 1;
                    let resized = image::imageops::resize(
                        &tile_img,
                        config.patch_size,
                        config.patch_size,
                        image::imageops::FilterType::Nearest,
                    );
                    let quality = assess_patch(&resized, &bg_colors);
                    if passes_quality(&quality, config) {
                        let category = classify_patch(&source_name, &quality);
                        let filename =
                            format!("{}_{:04}.png", source_name.replace(' ', "_"), patch_id);
                        patches.push(PatchInfo {
                            id: patch_id,
                            source: path.to_path_buf(),
                            bbox: (*bx, *by, *bw, *bh),
                            quality,
                            category,
                            filename,
                        });
                        patch_id += 1;
                    }
                }
            }
        } else {
            // No clear tile boundaries — use sliding window
            for (patch_img, px, py) in
                extract_patches(&rgba, config.patch_size, config.stride)
            {
                total_raw += 1;
                let quality = assess_patch(&patch_img, &bg_colors);
                if passes_quality(&quality, config) {
                    let category = classify_patch(&source_name, &quality);
                    let filename =
                        format!("{}_{:04}.png", source_name.replace(' ', "_"), patch_id);
                    patches.push(PatchInfo {
                        id: patch_id,
                        source: path.to_path_buf(),
                        bbox: (px, py, config.patch_size, config.patch_size),
                        quality,
                        category,
                        filename,
                    });
                    patch_id += 1;
                }
            }
        }
    } else if let Some(native) = config.native_tile_size {
        // Strategy B: Grid-based tileset (known tile size)
        for y in (0..h.saturating_sub(native - 1)).step_by(native as usize) {
            for x in (0..w.saturating_sub(native - 1)).step_by(native as usize) {
                let tile = image::imageops::crop_imm(&rgba, x, y, native, native).to_image();
                let resized = image::imageops::resize(
                    &tile,
                    config.patch_size,
                    config.patch_size,
                    image::imageops::FilterType::Nearest,
                );
                total_raw += 1;
                let quality = assess_patch(&resized, &bg_colors);
                if passes_quality(&quality, config) {
                    let category = classify_patch(&source_name, &quality);
                    let filename =
                        format!("{}_{:04}.png", source_name.replace(' ', "_"), patch_id);
                    patches.push(PatchInfo {
                        id: patch_id,
                        source: path.to_path_buf(),
                        bbox: (x, y, native, native),
                        quality,
                        category,
                        filename,
                    });
                    patch_id += 1;
                }
            }
        }
    } else {
        // Strategy C: Single image or small tile — extract patches directly
        for (patch_img, px, py) in extract_patches(&rgba, config.patch_size, config.stride) {
            total_raw += 1;
            let quality = assess_patch(&patch_img, &bg_colors);
            if passes_quality(&quality, config) {
                let category = classify_patch(&source_name, &quality);
                let filename =
                    format!("{}_{:04}.png", source_name.replace(' ', "_"), patch_id);
                patches.push(PatchInfo {
                    id: patch_id,
                    source: path.to_path_buf(),
                    bbox: (px, py, config.patch_size, config.patch_size),
                    quality,
                    category,
                    filename,
                });
                patch_id += 1;
            }
        }
    }

    Ok(ScanResult {
        source: path.to_path_buf(),
        source_size: (w, h),
        bg_colors,
        total_patches: total_raw,
        quality_patches: patches.len(),
        patches,
    })
}

/// Scan a directory of images recursively.
pub fn scan_directory(dir: &Path, config: &ScanConfig) -> Result<ScanManifest, String> {
    let mut sources = vec![];
    let mut total_raw = 0;
    let mut total_quality = 0;
    let mut categories = std::collections::HashMap::new();

    let entries = collect_image_files(dir)?;

    for path in &entries {
        match scan_image(path, config) {
            Ok(result) => {
                total_raw += result.total_patches;
                total_quality += result.quality_patches;
                for patch in &result.patches {
                    *categories.entry(patch.category.clone()).or_insert(0) += 1;
                }
                sources.push(result);
            }
            Err(e) => {
                eprintln!("  SKIP {}: {e}", path.display());
            }
        }
    }

    Ok(ScanManifest {
        patch_size: config.patch_size,
        stride: config.stride,
        sources,
        total_patches_raw: total_raw,
        total_patches_quality: total_quality,
        total_filtered: total_raw - total_quality,
        categories,
    })
}

/// Collect all image files from a directory (recursive).
fn collect_image_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = vec![];

    if dir.is_file() {
        files.push(dir.to_path_buf());
        return Ok(files);
    }

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory {}: {e}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_image_files(&path)?);
        } else if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Save extracted patches to a directory and write the scan manifest.
pub fn save_scan(
    manifest: &ScanManifest,
    source_images: &[(PathBuf, RgbaImage)],
    output_dir: &Path,
) -> Result<(), String> {
    let patches_dir = output_dir.join("patches");
    std::fs::create_dir_all(&patches_dir)
        .map_err(|e| format!("Cannot create {}: {e}", patches_dir.display()))?;

    // Build a lookup from source path to loaded image
    let img_map: std::collections::HashMap<&Path, &RgbaImage> = source_images
        .iter()
        .map(|(p, img)| (p.as_path(), img))
        .collect();

    for source_result in &manifest.sources {
        let Some(src_img) = img_map.get(source_result.source.as_path()) else {
            continue;
        };

        for patch_info in &source_result.patches {
            let (x, y, w, h) = patch_info.bbox;
            let cropped = image::imageops::crop_imm(*src_img, x, y, w, h).to_image();

            // Resize to patch_size if needed
            let final_img = if w != manifest.patch_size || h != manifest.patch_size {
                image::imageops::resize(
                    &cropped,
                    manifest.patch_size,
                    manifest.patch_size,
                    image::imageops::FilterType::Nearest,
                )
            } else {
                cropped
            };

            let out_path = patches_dir.join(&patch_info.filename);
            final_img
                .save(&out_path)
                .map_err(|e| format!("Failed to save {}: {e}", out_path.display()))?;
        }
    }

    // Write manifest
    let manifest_path = output_dir.join("scan_manifest.json");
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
    std::fs::write(&manifest_path, json)
        .map_err(|e| format!("Failed to write manifest: {e}"))?;

    Ok(())
}
