/// Procedural tile variation engine.
/// Generates N variants from a base tile by applying controlled mutations:
/// crack placement, detail shifting, moss/erosion density, and sub-region swaps.
/// All variations stay within the source palette and respect the style latent.

use crate::types::Palette;
use rand::prelude::*;

/// A generated tile variant.
#[derive(Debug, Clone)]
pub struct TileVariant {
    pub name: String,
    pub grid: Vec<Vec<char>>,
    pub mutation: String,
}

/// Generate N variants from a base tile grid.
pub fn generate_variants(
    base_name: &str,
    base_grid: &[Vec<char>],
    palette: &Palette,
    count: usize,
    seed: u64,
    void_sym: char,
) -> Vec<TileVariant> {
    let mut variants = Vec::with_capacity(count);
    let mut rng = StdRng::seed_from_u64(seed);

    let h = base_grid.len();
    let w = if h > 0 { base_grid[0].len() } else { return variants };

    // Collect non-void symbols used in the base tile
    let used_symbols: Vec<char> = {
        let mut syms: Vec<char> = base_grid
            .iter()
            .flat_map(|r| r.iter())
            .filter(|&&c| c != void_sym)
            .copied()
            .collect::<std::collections::HashSet<char>>()
            .into_iter()
            .collect();
        syms.sort();
        syms
    };

    if used_symbols.is_empty() {
        return variants;
    }

    let mutations = [
        "pixel_noise",
        "crack",
        "shift_row",
        "shift_col",
        "swap_symbols",
        "erode_edge",
    ];

    for i in 0..count {
        let mutation_type = mutations[i % mutations.len()];
        let mut grid = base_grid.to_vec();

        match mutation_type {
            "pixel_noise" => {
                apply_pixel_noise(&mut grid, &used_symbols, w, h, &mut rng, void_sym);
            }
            "crack" => {
                apply_crack(&mut grid, &used_symbols, w, h, &mut rng, palette, void_sym);
            }
            "shift_row" => {
                apply_row_shift(&mut grid, w, h, &mut rng);
            }
            "shift_col" => {
                apply_col_shift(&mut grid, w, h, &mut rng);
            }
            "swap_symbols" => {
                apply_symbol_swap(&mut grid, &used_symbols, &mut rng);
            }
            "erode_edge" => {
                apply_edge_erosion(&mut grid, &used_symbols, w, h, &mut rng, void_sym);
            }
            _ => {}
        }

        variants.push(TileVariant {
            name: format!("{}_{}", base_name, i + 1),
            grid,
            mutation: mutation_type.to_string(),
        });
    }

    variants
}

/// Randomly change 5-15% of interior pixels to a different palette symbol.
fn apply_pixel_noise(
    grid: &mut [Vec<char>],
    symbols: &[char],
    w: usize,
    h: usize,
    rng: &mut StdRng,
    void_sym: char,
) {
    let noise_count = (w * h) / 12 + 1;
    for _ in 0..noise_count {
        let x = rng.random_range(1..w.saturating_sub(1).max(1));
        let y = rng.random_range(1..h.saturating_sub(1).max(1));
        if grid[y][x] != void_sym {
            let new_sym = symbols[rng.random_range(0..symbols.len())];
            grid[y][x] = new_sym;
        }
    }
}

/// Draw a vertical or horizontal crack through the interior.
fn apply_crack(
    grid: &mut [Vec<char>],
    symbols: &[char],
    w: usize,
    h: usize,
    rng: &mut StdRng,
    palette: &Palette,
    void_sym: char,
) {
    // Find the darkest non-void symbol for cracks
    let crack_sym = symbols
        .iter()
        .filter(|&&s| s != void_sym)
        .min_by(|&&a, &&b| {
            let la = palette
                .symbols
                .get(&a)
                .map(|c| c.r as u32 + c.g as u32 + c.b as u32)
                .unwrap_or(0);
            let lb = palette
                .symbols
                .get(&b)
                .map(|c| c.r as u32 + c.g as u32 + c.b as u32)
                .unwrap_or(0);
            la.cmp(&lb)
        })
        .copied()
        .unwrap_or(symbols[0]);

    let vertical = rng.random_bool(0.5);
    if vertical {
        let x = rng.random_range(2..w.saturating_sub(2).max(3));
        for y in 1..h - 1 {
            if rng.random_bool(0.7) {
                grid[y][x] = crack_sym;
            }
            // Wobble
            if rng.random_bool(0.3) && x + 1 < w - 1 {
                grid[y][x + 1] = crack_sym;
            }
        }
    } else {
        let y = rng.random_range(2..h.saturating_sub(2).max(3));
        for x in 1..w - 1 {
            if rng.random_bool(0.7) {
                grid[y][x] = crack_sym;
            }
            if rng.random_bool(0.3) && y + 1 < h - 1 {
                grid[y + 1][x] = crack_sym;
            }
        }
    }
}

/// Shift a random interior row left or right by 1 pixel.
fn apply_row_shift(grid: &mut [Vec<char>], w: usize, h: usize, rng: &mut StdRng) {
    if h < 4 || w < 4 {
        return;
    }
    let y = rng.random_range(2..h - 2);
    let shift_right = rng.random_bool(0.5);
    if shift_right {
        let last = grid[y][w - 2];
        for x in (2..w - 1).rev() {
            grid[y][x] = grid[y][x - 1];
        }
        grid[y][1] = last;
    } else {
        let first = grid[y][1];
        for x in 1..w - 2 {
            grid[y][x] = grid[y][x + 1];
        }
        grid[y][w - 2] = first;
    }
}

/// Shift a random interior column up or down by 1 pixel.
fn apply_col_shift(grid: &mut [Vec<char>], w: usize, h: usize, rng: &mut StdRng) {
    if h < 4 || w < 4 {
        return;
    }
    let x = rng.random_range(2..w - 2);
    let shift_down = rng.random_bool(0.5);
    if shift_down {
        let last = grid[h - 2][x];
        for y in (2..h - 1).rev() {
            grid[y][x] = grid[y - 1][x];
        }
        grid[1][x] = last;
    } else {
        let first = grid[1][x];
        for y in 1..h - 2 {
            grid[y][x] = grid[y + 1][x];
        }
        grid[h - 2][x] = first;
    }
}

/// Swap two random symbols in the interior (preserves edges).
fn apply_symbol_swap(grid: &mut [Vec<char>], symbols: &[char], rng: &mut StdRng) {
    if symbols.len() < 2 {
        return;
    }
    let a = symbols[rng.random_range(0..symbols.len())];
    let mut b = symbols[rng.random_range(0..symbols.len())];
    while b == a && symbols.len() > 1 {
        b = symbols[rng.random_range(0..symbols.len())];
    }

    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return };

    // Only swap in interior (preserve edge pixels)
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            if grid[y][x] == a {
                grid[y][x] = b;
            } else if grid[y][x] == b {
                grid[y][x] = a;
            }
        }
    }
}

/// Erode edges slightly — move boundary pixels inward.
fn apply_edge_erosion(
    grid: &mut [Vec<char>],
    symbols: &[char],
    w: usize,
    h: usize,
    rng: &mut StdRng,
    void_sym: char,
) {
    // Find the most common interior symbol
    let interior_sym = symbols
        .iter()
        .filter(|&&s| s != void_sym)
        .max_by_key(|&&s| {
            grid.iter()
                .flat_map(|r| r.iter())
                .filter(|&&c| c == s)
                .count()
        })
        .copied()
        .unwrap_or(symbols[0]);

    // Randomly erode a few pixels on the second row/column from edge
    for _ in 0..3 {
        let edge = rng.random_range(0..4u8);
        match edge {
            0 if h > 2 => {
                // Top edge
                let x = rng.random_range(1..w.saturating_sub(1).max(2));
                grid[1][x] = interior_sym;
            }
            1 if h > 2 => {
                // Bottom edge
                let x = rng.random_range(1..w.saturating_sub(1).max(2));
                grid[h - 2][x] = interior_sym;
            }
            2 if w > 2 => {
                // Left edge
                let y = rng.random_range(1..h.saturating_sub(1).max(2));
                grid[y][1] = interior_sym;
            }
            3 if w > 2 => {
                // Right edge
                let y = rng.random_range(1..h.saturating_sub(1).max(2));
                grid[y][w - 2] = interior_sym;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rgba;
    use std::collections::HashMap;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert('.', Rgba { r: 0, g: 0, b: 0, a: 0 });
        symbols.insert('#', Rgba { r: 42, g: 31, b: 61, a: 255 });
        symbols.insert('+', Rgba { r: 74, g: 58, b: 109, a: 255 });
        symbols.insert('s', Rgba { r: 26, g: 15, b: 46, a: 255 });
        Palette { symbols }
    }

    fn wall_grid() -> Vec<Vec<char>> {
        vec![
            "########".chars().collect(),
            "#++##++#".chars().collect(),
            "#+++++++".chars().collect(),
            "#++#####".chars().collect(),
            "########".chars().collect(),
            "#+++++##".chars().collect(),
            "#+++++++".chars().collect(),
            "########".chars().collect(),
        ]
    }

    #[test]
    fn generates_requested_count() {
        let palette = test_palette();
        let grid = wall_grid();
        let variants = generate_variants("wall", &grid, &palette, 4, 42, '.');
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn variants_differ_from_base() {
        let palette = test_palette();
        let grid = wall_grid();
        let variants = generate_variants("wall", &grid, &palette, 3, 42, '.');

        let mut any_different = false;
        for v in &variants {
            if v.grid != grid {
                any_different = true;
                break;
            }
        }
        assert!(any_different, "at least one variant should differ from base");
    }

    #[test]
    fn preserves_dimensions() {
        let palette = test_palette();
        let grid = wall_grid();
        let variants = generate_variants("wall", &grid, &palette, 6, 42, '.');

        for v in &variants {
            assert_eq!(v.grid.len(), grid.len());
            assert_eq!(v.grid[0].len(), grid[0].len());
        }
    }

    #[test]
    fn edges_mostly_preserved() {
        let palette = test_palette();
        let grid = wall_grid();
        let variants = generate_variants("wall", &grid, &palette, 6, 42, '.');

        // Top and bottom rows should be unchanged in most variants
        // (pixel_noise and crack avoid row 0 and row h-1)
        for v in &variants {
            if v.mutation == "pixel_noise" || v.mutation == "crack" {
                assert_eq!(v.grid[0], grid[0], "top edge should be preserved for {}", v.mutation);
                assert_eq!(
                    v.grid[grid.len() - 1],
                    grid[grid.len() - 1],
                    "bottom edge should be preserved for {}",
                    v.mutation
                );
            }
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let palette = test_palette();
        let grid = wall_grid();
        let v1 = generate_variants("wall", &grid, &palette, 3, 99, '.');
        let v2 = generate_variants("wall", &grid, &palette, 3, 99, '.');
        for (a, b) in v1.iter().zip(v2.iter()) {
            assert_eq!(a.grid, b.grid);
        }
    }

    #[test]
    fn different_seeds_different_output() {
        let palette = test_palette();
        let grid = wall_grid();
        let v1 = generate_variants("wall", &grid, &palette, 3, 42, '.');
        let v2 = generate_variants("wall", &grid, &palette, 3, 99, '.');
        let any_diff = v1.iter().zip(v2.iter()).any(|(a, b)| a.grid != b.grid);
        assert!(any_diff, "different seeds should produce different variants");
    }

    #[test]
    fn names_include_index() {
        let palette = test_palette();
        let grid = wall_grid();
        let variants = generate_variants("wall", &grid, &palette, 3, 42, '.');
        assert_eq!(variants[0].name, "wall_1");
        assert_eq!(variants[1].name, "wall_2");
        assert_eq!(variants[2].name, "wall_3");
    }
}
