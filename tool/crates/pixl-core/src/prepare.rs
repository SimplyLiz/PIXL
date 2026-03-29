//! Training data preparation from scanned patches.
//!
//! Converts scanned image patches into LoRA-ready JSONL training data:
//! 1. Palette extraction per category
//! 2. Quantize patches to PAX character grids
//! 3. Feature computation (density, symmetry, edge complexity)
//! 4. Structured label generation
//! 5. Geometric + color augmentation
//! 6. Stratified sampling for uniform feature coverage

use crate::types::Palette;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Types ───────────────────────────────────────────────────────────────────

/// Configuration for data preparation.
#[derive(Debug, Clone)]
pub struct PrepareConfig {
    /// Style tag injected into labels (e.g. "my-game", "eotb").
    pub style_tag: String,
    /// Augmentation level: 4 = rotations, 8 = + flips.
    pub aug_level: u8,
    /// Enable warm/cool/dark color shifts (3× more data).
    pub color_aug: bool,
    /// Max samples per stratification bin.
    pub max_per_bin: usize,
    /// Max palette colors per category.
    pub max_colors: usize,
    /// Symbols assigned to extracted palette colors (ordered by brightness).
    pub symbol_pool: String,
}

impl Default for PrepareConfig {
    fn default() -> Self {
        Self {
            style_tag: "custom".into(),
            aug_level: 4,
            color_aug: true,
            max_per_bin: 150,
            max_colors: 10,
            symbol_pool: ".#+=~gorhwsABCDE".into(),
        }
    }
}

/// A single training sample in chat format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Result of the prepare pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareResult {
    pub total_patches: usize,
    pub total_augmented: usize,
    pub total_stratified: usize,
    pub train_count: usize,
    pub valid_count: usize,
    pub test_count: usize,
    pub categories: HashMap<String, usize>,
    pub bins_filled: usize,
}

// ─── Constants ──────────────────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = "You are a pixel art tile generator. Given a description, output a PAX-format character grid.\nRules:\n- Use only the symbols from the palette provided\n- Each row must be exactly the specified width\n- Total rows must equal the specified height\n- '.' means transparent/void\n- Output ONLY the grid, no explanation";

// ─── Palette extraction ─────────────────────────────────────────────────────

/// Extract a palette from a set of RGBA pixel arrays.
/// Returns (symbol, (r, g, b)) pairs sorted by brightness.
pub fn extract_palette_from_pixels(
    pixels: &[&[u8]],  // flat RGBA pixel data
    max_colors: usize,
    symbol_pool: &str,
) -> Vec<(char, [u8; 3])> {
    let mut color_counts: HashMap<[u8; 3], usize> = HashMap::new();

    for chunk in pixels.iter().flat_map(|px| px.chunks_exact(4)) {
        let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        if a < 128 {
            continue;
        }
        // Quantize to reduce noise
        let qr = (r / 8) * 8;
        let qg = (g / 8) * 8;
        let qb = (b / 8) * 8;
        *color_counts.entry([qr, qg, qb]).or_default() += 1;
    }

    if color_counts.is_empty() {
        return vec![];
    }

    // Top N by frequency, sorted by brightness
    let mut top: Vec<_> = color_counts.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    top.truncate(max_colors);

    let mut colors: Vec<[u8; 3]> = top.into_iter().map(|(c, _)| c).collect();
    colors.sort_by_key(|c| (c[0] as u32 + c[1] as u32 + c[2] as u32));

    let symbols: Vec<char> = symbol_pool.chars().collect();
    let mut palette = vec![];
    for (i, rgb) in colors.iter().enumerate() {
        if i + 1 < symbols.len() {
            palette.push((symbols[i + 1], *rgb)); // skip '.' which is void
        }
    }
    palette
}

// ─── Quantization ───────────────────────────────────────────────────────────

/// Quantize RGBA pixel data to a PAX character grid.
pub fn quantize_to_grid(
    pixels: &[u8],  // flat RGBA
    width: usize,
    height: usize,
    palette: &[(char, [u8; 3])],
) -> Vec<Vec<char>> {
    let mut grid = vec![vec!['.'; width]; height];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            if idx + 3 >= pixels.len() {
                continue;
            }
            let (r, g, b, a) = (pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3]);

            if a < 128 {
                continue;
            }

            let mut best_sym = '.';
            let mut best_dist = f64::MAX;
            for &(sym, [pr, pg, pb]) in palette {
                let dr = r as f64 - pr as f64;
                let dg = g as f64 - pg as f64;
                let db = b as f64 - pb as f64;
                let d = dr * dr * 0.30 + dg * dg * 0.59 + db * db * 0.11;
                if d < best_dist {
                    best_dist = d;
                    best_sym = sym;
                }
            }
            grid[y][x] = best_sym;
        }
    }
    grid
}

// ─── Features ───────────────────────────────────────────────────────────────

/// Visual features computed from a character grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridFeatures {
    pub density: f64,
    pub symmetry: f64,
    pub edge_complexity: f64,
    pub unique_symbols: usize,
}

/// Compute visual features from a character grid.
pub fn compute_features(grid: &[Vec<char>]) -> GridFeatures {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { 0 };
    let total = h * w;

    if total == 0 {
        return GridFeatures {
            density: 0.0,
            symmetry: 0.0,
            edge_complexity: 0.0,
            unique_symbols: 0,
        };
    }

    let non_void = grid.iter().flat_map(|r| r.iter()).filter(|&&c| c != '.').count();
    let density = non_void as f64 / total as f64;

    // Symmetry
    let mut h_matches = 0usize;
    let mut v_matches = 0usize;
    for y in 0..h {
        for x in 0..w / 2 {
            if grid[y][x] == grid[y][w - 1 - x] {
                h_matches += 1;
            }
        }
    }
    for y in 0..h / 2 {
        for x in 0..w {
            if grid[y][x] == grid[h - 1 - y][x] {
                v_matches += 1;
            }
        }
    }
    let sym_h = h_matches as f64 / (h * (w / 2)).max(1) as f64;
    let sym_v = v_matches as f64 / ((h / 2) * w).max(1) as f64;
    let symmetry = sym_h.max(sym_v);

    // Edge complexity
    let mut edges = 0usize;
    let mut edge_total = 0usize;
    for y in 0..h {
        for x in 0..w {
            if x + 1 < w {
                edge_total += 1;
                if grid[y][x] != grid[y][x + 1] {
                    edges += 1;
                }
            }
            if y + 1 < h {
                edge_total += 1;
                if grid[y][x] != grid[y + 1][x] {
                    edges += 1;
                }
            }
        }
    }
    let edge_complexity = if edge_total > 0 {
        edges as f64 / edge_total as f64
    } else {
        0.0
    };

    // Unique symbols
    let mut syms = std::collections::HashSet::new();
    for row in grid {
        for &c in row {
            if c != '.' {
                syms.insert(c);
            }
        }
    }

    GridFeatures {
        density,
        symmetry,
        edge_complexity,
        unique_symbols: syms.len(),
    }
}

// ─── Labels ─────────────────────────────────────────────────────────────────

/// Generate a structured label from features.
pub fn make_label(
    features: &GridFeatures,
    style_tag: &str,
    category: &str,
    aug_tag: &str,
    color_tag: &str,
) -> String {
    let mut parts = vec![
        format!("style:{}", style_tag),
        format!("type:{}", category),
    ];

    let density_label = if features.density < 0.2 {
        "sparse"
    } else if features.density < 0.5 {
        "moderate"
    } else if features.density < 0.8 {
        "dense"
    } else {
        "solid"
    };
    parts.push(format!("density:{}", density_label));

    let sym_label = if features.symmetry > 0.85 {
        "high"
    } else if features.symmetry > 0.6 {
        "medium"
    } else {
        "low"
    };
    parts.push(format!("symmetry:{}", sym_label));

    let detail_label = if features.edge_complexity < 0.15 {
        "flat"
    } else if features.edge_complexity < 0.35 {
        "simple"
    } else if features.edge_complexity < 0.55 {
        "moderate"
    } else {
        "complex"
    };
    parts.push(format!("detail:{}", detail_label));

    let color_label = if features.unique_symbols <= 2 {
        "minimal"
    } else if features.unique_symbols <= 4 {
        "few"
    } else {
        "rich"
    };
    parts.push(format!("colors:{}", color_label));

    if !aug_tag.is_empty() && aug_tag != "orig" {
        parts.push(format!("aug:{}", aug_tag));
    }
    if !color_tag.is_empty() {
        parts.push(format!("palette:{}", color_tag));
    }

    parts.join(", ")
}

// ─── Augmentation ───────────────────────────────────────────────────────────

/// Rotate a grid 90° clockwise.
pub fn rotate_90(grid: &[Vec<char>]) -> Vec<Vec<char>> {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { 0 };
    (0..w)
        .map(|y| (0..h).rev().map(|x| grid[x][y]).collect())
        .collect()
}

/// Flip a grid horizontally.
pub fn flip_h(grid: &[Vec<char>]) -> Vec<Vec<char>> {
    grid.iter().map(|row| row.iter().rev().copied().collect()).collect()
}

/// Generate geometric augmentations.
pub fn augment_grid(grid: &[Vec<char>], level: u8) -> Vec<(Vec<Vec<char>>, &'static str)> {
    let r90 = rotate_90(grid);
    let r180 = rotate_90(&r90);
    let r270 = rotate_90(&r180);

    let mut variants = vec![
        (grid.to_vec(), "orig"),
        (r90, "r90"),
        (r180, "r180"),
        (r270, "r270"),
    ];

    if level >= 8 {
        let flipped = flip_h(grid);
        let fr90 = rotate_90(&flipped);
        let fr180 = rotate_90(&fr90);
        let fr270 = rotate_90(&fr180);
        variants.extend([
            (flipped, "flip"),
            (fr90, "flip_r90"),
            (fr180, "flip_r180"),
            (fr270, "flip_r270"),
        ]);
    }

    variants
}

/// Apply a color shift to a palette.
pub fn shift_palette(palette: &[(char, [u8; 3])], shift: &str) -> Vec<(char, [u8; 3])> {
    palette
        .iter()
        .map(|&(sym, [r, g, b])| {
            let (nr, ng, nb) = match shift {
                "warm" => (
                    ((r as f32 * 1.1 + 8.0).min(255.0)) as u8,
                    (g as f32 * 0.95) as u8,
                    ((b as f32 * 0.85 - 5.0).max(0.0)) as u8,
                ),
                "cool" => (
                    ((r as f32 * 0.85 - 5.0).max(0.0)) as u8,
                    (g as f32 * 0.95) as u8,
                    ((b as f32 * 1.1 + 8.0).min(255.0)) as u8,
                ),
                "dark" => (
                    (r as f32 * 0.75) as u8,
                    (g as f32 * 0.75) as u8,
                    (b as f32 * 0.75) as u8,
                ),
                _ => (r, g, b),
            };
            (sym, [nr, ng, nb])
        })
        .collect()
}

// ─── Stratification ─────────────────────────────────────────────────────────

/// Deterministic Fisher-Yates shuffle using a simple splitmix64 PRNG.
pub fn fisher_yates_shuffle_pub<T>(items: &mut [T], seed: u64) {
    let n = items.len();
    if n <= 1 {
        return;
    }
    let mut state = seed;
    for i in (1..n).rev() {
        // splitmix64 step
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        let j = (z % (i as u64 + 1)) as usize;
        items.swap(i, j);
    }
}

/// Stratified sampling: bin by density × edge_complexity, cap per bin.
/// Tries 5×5 bins first; falls back to 3×3 if fewer than 30% of bins fill.
pub fn stratified_sample(
    samples: Vec<(TrainingSample, GridFeatures)>,
    max_per_bin: usize,
    seed: u64,
) -> (Vec<TrainingSample>, usize) {
    // Probe 5×5 bin coverage without allocating
    let mut probe_bins = std::collections::HashSet::new();
    for (_, features) in &samples {
        let d = (features.density * 5.0).min(4.0) as u8;
        let e = (features.edge_complexity * 5.0).min(4.0) as u8;
        probe_bins.insert((d, e));
    }
    let bin_count = if probe_bins.len() * 100 / 25 >= 30 { 5.0 } else { 3.0 };

    stratify_with_bins(samples, bin_count, max_per_bin, seed)
}

fn stratify_with_bins(
    samples: Vec<(TrainingSample, GridFeatures)>,
    bin_count: f64,
    max_per_bin: usize,
    seed: u64,
) -> (Vec<TrainingSample>, usize) {
    let bin_max = (bin_count - 1.0) as u8;

    let mut bins: HashMap<(u8, u8), Vec<TrainingSample>> = HashMap::new();

    for (sample, features) in samples {
        let d_bin = (features.density * bin_count).min(bin_max as f64) as u8;
        let e_bin = (features.edge_complexity * bin_count).min(bin_max as f64) as u8;
        bins.entry((d_bin, e_bin)).or_default().push(sample);
    }

    let filled = bins.values().filter(|v| !v.is_empty()).count();

    // Fisher-Yates shuffle each bin, then take up to max_per_bin
    let mut result = vec![];
    let mut bin_keys: Vec<_> = bins.keys().copied().collect();
    bin_keys.sort();
    for key in bin_keys {
        let items = bins.get_mut(&key).unwrap();
        let bin_seed = seed
            .wrapping_add(key.0 as u64 * 31)
            .wrapping_add(key.1 as u64 * 37);
        fisher_yates_shuffle_pub(items, bin_seed);
        result.extend(items.drain(..max_per_bin.min(items.len())));
    }

    fisher_yates_shuffle_pub(&mut result, seed.wrapping_add(0xdeadbeef));

    (result, filled)
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Convert a grid to a string.
pub fn grid_to_string(grid: &[Vec<char>]) -> String {
    grid.iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a palette as a description string.
pub fn palette_to_desc(palette: &[(char, [u8; 3])]) -> String {
    palette
        .iter()
        .map(|(sym, [r, g, b])| format!("'{}'=({},{},{})", sym, r, g, b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Create a training sample in chat format.
pub fn make_sample(palette_desc: &str, label: &str, grid_str: &str) -> TrainingSample {
    TrainingSample {
        messages: vec![
            ChatMessage {
                role: "system".into(),
                content: SYSTEM_PROMPT.into(),
            },
            ChatMessage {
                role: "user".into(),
                content: format!("Palette: {}\n{}", palette_desc, label),
            },
            ChatMessage {
                role: "assistant".into(),
                content: grid_str.into(),
            },
        ],
    }
}

/// Write training samples to a JSONL file.
pub fn write_jsonl(samples: &[TrainingSample], path: &Path) -> Result<(), String> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)
        .map_err(|e| format!("Cannot create {}: {e}", path.display()))?;
    for sample in samples {
        let json = serde_json::to_string(sample)
            .map_err(|e| format!("JSON serialize error: {e}"))?;
        writeln!(f, "{}", json)
            .map_err(|e| format!("Write error: {e}"))?;
    }
    Ok(())
}
