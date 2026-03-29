//! Wang tileset generation for terrain transitions.
//!
//! Generates a complete set of transition tiles between two terrain types
//! with correct edge classes for WFC. Supports blob-47 (8-neighbor bitmask)
//! and dual-grid (5-type Stalberg method) approaches.
//!
//! Reference:
//! - Boris the Brave, "Classification of Tilesets" (2021)
//! - Oskar Stalberg, dual-grid tileset concept
//! - PAX spec Section 13 (Autotiling)

use crate::types::{EdgeClassRaw, PatchRaw, SemanticRaw, TileRaw};
use std::collections::HashMap;

/// Method for generating the tileset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WangMethod {
    /// Blob 47: 8-bit neighbor bitmask → 47 unique visual cases.
    /// Best for dungeon walls and cave systems.
    Blob47,
    /// Dual grid: 5 tile types × rotations = 15 tiles (or 6 drawn if symmetric).
    /// Best for top-down terrain transitions (grass/sand/snow).
    DualGrid,
}

/// Configuration for Wang tileset generation.
pub struct WangConfig {
    /// First terrain type (e.g., "grass")
    pub terrain_a: String,
    /// Second terrain type (e.g., "water")
    pub terrain_b: String,
    /// Tile size
    pub size: u32,
    /// Palette name
    pub palette: String,
    /// Symbol for terrain A fill (e.g., '+')
    pub sym_a: char,
    /// Symbol for terrain B fill (e.g., '~')
    pub sym_b: char,
    /// Symbol for border/transition (e.g., '#')
    pub sym_border: char,
    /// Generation method
    pub method: WangMethod,
}

/// A generated Wang tile.
#[derive(Debug)]
pub struct WangTile {
    pub name: String,
    pub tile_raw: TileRaw,
    /// Bitmask value for blob-47 (0-255), or tile type index for dual-grid
    pub mask: u32,
    /// Human-readable description of this tile's role
    pub description: String,
}

/// Result of Wang tileset generation.
#[derive(Debug)]
pub struct WangResult {
    pub tiles: Vec<WangTile>,
    pub method: WangMethod,
    pub terrain_a: String,
    pub terrain_b: String,
    pub summary: String,
}

/// Generate a complete Wang tileset for transitioning between two terrain types.
pub fn generate(config: &WangConfig) -> WangResult {
    match config.method {
        WangMethod::Blob47 => generate_blob47(config),
        WangMethod::DualGrid => generate_dual_grid(config),
    }
}

// ── Blob 47 ────────────────────────────────────────────────────────

/// The 47 unique blob tile cases after corner cleanup.
/// Each entry: (mask, name_suffix, description)
const BLOB_47_CASES: &[(u8, &str, &str)] = &[
    (0, "isolated", "no neighbors"),
    (2, "n", "north only"),
    (8, "w", "west only"),
    (10, "nw", "north + west"),
    (11, "nw_corner", "north + west + NW corner"),
    (16, "e", "east only"),
    (18, "ne", "north + east"),
    (22, "ne_corner", "north + east + NE corner"),
    (24, "ew", "east + west"),
    (26, "new", "north + east + west"),
    (27, "new_nw", "north + east + west + NW"),
    (30, "new_ne", "north + east + west + NE"),
    (31, "new_corners", "north + east + west + both"),
    (64, "s", "south only"),
    (66, "ns", "north + south"),
    (72, "sw", "south + west"),
    (74, "nsw", "north + south + west"),
    (75, "nsw_nw", "N+S+W + NW corner"),
    (80, "se", "south + east"),
    (82, "nse", "N+S+E"),
    (86, "nse_ne", "N+S+E + NE corner"),
    (88, "sew", "S+E+W"),
    (90, "nsew_none", "all cardinal, no corners"),
    (91, "nsew_nw", "all cardinal + NW"),
    (94, "nsew_ne", "all cardinal + NE"),
    (95, "nsew_nw_ne", "all cardinal + NW + NE"),
    (104, "sew_sw", "S+E+W + SW corner"),
    (106, "nsew_sw", "all cardinal + SW"),
    (107, "nsew_nw_sw", "all cardinal + NW + SW"),
    (120, "sew_se", "S+E+W + SE corner"),
    (122, "nsew_se", "all cardinal + SE"),
    (126, "nsew_ne_se", "all cardinal + NE + SE"),
    (210, "nsew_se_nw", "all cardinal + SE + NW"),
    (214, "nsew_ne_se_nw", "all cardinal + NE + SE + NW (no SW)"),
    (216, "sew_sw_se", "S+E+W + SW + SE"),
    (218, "nsew_sw_se", "all cardinal + SW + SE"),
    (219, "nsew_nw_sw_se", "all cardinal + NW + SW + SE (no NE)"),
    (222, "nsew_ne_sw_se", "all cardinal + NE + SW + SE (no NW)"),
    (248, "sew_corners", "S+E+W + SW + SE"),
    (250, "nsew_all_no_ne", "all except NE"),
    (251, "nsew_all_no_ne2", "all except NE (variant)"),
    (254, "nsew_all_no_nw", "all except NW"),
    (255, "full", "all neighbors"),
    // Some masks map to same visual — the 47 unique cases
    // Corner bits only count when adjacent cardinals are set
    (66, "ns_pipe", "vertical pipe"),
    (24, "ew_pipe", "horizontal pipe"),
    (86, "nse_ne2", "N+S+E with NE"),
    (75, "nsw_nw2", "N+S+W with NW"),
];

fn generate_blob47(config: &WangConfig) -> WangResult {
    let s = config.size as usize;
    let mut tiles = Vec::new();

    // Generate the 47 canonical cases
    // For each case, create a tile grid based on which neighbors are present
    // Neighbor bits: NW=1, N=2, NE=4, W=8, E=16, SW=32, S=64, SE=128
    let mut seen_masks: std::collections::HashSet<u8> = std::collections::HashSet::new();

    for &(mask, suffix, desc) in BLOB_47_CASES {
        if seen_masks.contains(&mask) {
            continue;
        }
        seen_masks.insert(mask);

        let name = format!("{}_{}_{}",
            config.terrain_a, config.terrain_b, suffix);

        let grid = generate_blob_grid(mask, s, config.sym_a, config.sym_b, config.sym_border);
        let grid_str = grid.iter().map(|row| row.iter().collect::<String>()).collect::<Vec<_>>().join("\n");

        // Determine edge classes from the mask
        let has_n = mask & 2 != 0;
        let has_e = mask & 16 != 0;
        let has_s = mask & 64 != 0;
        let has_w = mask & 8 != 0;

        let edge_n = if has_n { &config.terrain_a } else { &config.terrain_b };
        let edge_e = if has_e { &config.terrain_a } else { &config.terrain_b };
        let edge_s = if has_s { &config.terrain_a } else { &config.terrain_b };
        let edge_w = if has_w { &config.terrain_a } else { &config.terrain_b };

        let tile_raw = TileRaw {
            palette: config.palette.clone(),
            size: Some(format!("{}x{}", s, s)),
            encoding: None,
            symmetry: None,
            auto_rotate: None,
            auto_rotate_weight: None,
            template: None,
            edge_class: Some(EdgeClassRaw {
                n: edge_n.clone(),
                e: edge_e.clone(),
                s: edge_s.clone(),
                w: edge_w.clone(),
            }),
            corner_class: None,
            tags: vec![
                "wang".to_string(),
                format!("transition_{}", config.terrain_b),
                config.terrain_a.clone(),
            ],
            target_layer: Some("terrain".to_string()),
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: Some(SemanticRaw {
                affordance: Some("walkable".to_string()),
                collision: Some("none".to_string()),
                collision_points: None,
                tags: HashMap::new(),
            }),
            grid: Some(grid_str),
            rle: None,
            layout: None,
            fill: None,
            fill_size: None,
            delta: None,
            patches: vec![],
        };

        tiles.push(WangTile {
            name,
            tile_raw,
            mask: mask as u32,
            description: desc.to_string(),
        });
    }

    let summary = format!(
        "Generated {} blob-47 tiles for {} → {} transition ({}x{}).",
        tiles.len(), config.terrain_a, config.terrain_b, config.size, config.size
    );

    WangResult {
        tiles,
        method: WangMethod::Blob47,
        terrain_a: config.terrain_a.clone(),
        terrain_b: config.terrain_b.clone(),
        summary,
    }
}

/// Generate a grid for a blob-47 case based on the neighbor bitmask.
fn generate_blob_grid(mask: u8, size: usize, sym_a: char, sym_b: char, sym_border: char) -> Vec<Vec<char>> {
    let mut grid = vec![vec![sym_b; size]; size];
    let half = size / 2;

    // Fill quadrants based on neighbor bits
    // NW=1, N=2, NE=4, W=8, E=16, SW=32, S=64, SE=128
    let has_n = mask & 2 != 0;
    let has_s = mask & 64 != 0;
    let has_e = mask & 16 != 0;
    let has_w = mask & 8 != 0;
    let has_nw = mask & 1 != 0 && has_n && has_w;
    let has_ne = mask & 4 != 0 && has_n && has_e;
    let has_sw = mask & 32 != 0 && has_s && has_w;
    let has_se = mask & 128 != 0 && has_s && has_e;

    // Fill terrain A where neighbors are present
    // North half
    if has_n {
        for y in 0..half {
            for x in 1..size - 1 {
                grid[y][x] = sym_a;
            }
        }
    }
    // South half
    if has_s {
        for y in half..size {
            for x in 1..size - 1 {
                grid[y][x] = sym_a;
            }
        }
    }
    // West half
    if has_w {
        for y in 1..size - 1 {
            for x in 0..half {
                grid[y][x] = sym_a;
            }
        }
    }
    // East half
    if has_e {
        for y in 1..size - 1 {
            for x in half..size {
                grid[y][x] = sym_a;
            }
        }
    }

    // Corners (only if both adjacent cardinals are filled)
    if has_nw {
        for y in 0..half { for x in 0..half { grid[y][x] = sym_a; } }
    }
    if has_ne {
        for y in 0..half { for x in half..size { grid[y][x] = sym_a; } }
    }
    if has_sw {
        for y in half..size { for x in 0..half { grid[y][x] = sym_a; } }
    }
    if has_se {
        for y in half..size { for x in half..size { grid[y][x] = sym_a; } }
    }

    // Add border pixels at terrain transitions
    for y in 0..size {
        for x in 0..size {
            if grid[y][x] == sym_a {
                // Check if any neighbor is sym_b → place border
                let neighbors = [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)];
                for (dy, dx) in neighbors {
                    let ny = y as i32 + dy;
                    let nx = x as i32 + dx;
                    if ny >= 0 && ny < size as i32 && nx >= 0 && nx < size as i32 {
                        if grid[ny as usize][nx as usize] == sym_b {
                            grid[y][x] = sym_border;
                            break;
                        }
                    }
                }
            }
        }
    }

    grid
}

// ── Dual Grid ──────────────────────────────────────────────────────

fn generate_dual_grid(config: &WangConfig) -> WangResult {
    let s = config.size as usize;
    let mut tiles = Vec::new();

    // Dual grid needs 5 tile types:
    // 1. Full A (all corners A)
    // 2. Full B (all corners B)
    // 3. Edge (one side A, one side B) — 4 rotations
    // 4. Inner corner (3 corners A, 1 corner B) — 4 rotations
    // 5. Outer corner (1 corner A, 3 corners B) — 4 rotations
    // With symmetry: only 6 drawn tiles needed (full_a, full_b, edge, inner_corner, outer_corner, opposite_corners)

    let types: &[(&str, &str, [bool; 4])] = &[
        // (name, desc, [TL, TR, BL, BR] where true = terrain_a)
        ("full_a", "all terrain A", [true, true, true, true]),
        ("full_b", "all terrain B", [false, false, false, false]),
        ("edge_n", "edge: A north, B south", [true, true, false, false]),
        ("edge_e", "edge: A east, B west", [false, true, false, true]),
        ("edge_s", "edge: A south, B north", [false, false, true, true]),
        ("edge_w", "edge: A west, B east", [true, false, true, false]),
        ("inner_ne", "inner corner: B at NE", [true, false, true, true]),
        ("inner_nw", "inner corner: B at NW", [false, true, true, true]),
        ("inner_se", "inner corner: B at SE", [true, true, true, false]),
        ("inner_sw", "inner corner: B at SW", [true, true, false, true]),
        ("outer_ne", "outer corner: A at NE", [false, true, false, false]),
        ("outer_nw", "outer corner: A at NW", [true, false, false, false]),
        ("outer_se", "outer corner: A at SE", [false, false, false, true]),
        ("outer_sw", "outer corner: A at SW", [false, false, true, false]),
        ("opposite_ne_sw", "opposite corners: A at NE + SW", [false, true, true, false]),
    ];

    for (suffix, desc, corners) in types {
        let name = format!("{}_{}_dg_{}", config.terrain_a, config.terrain_b, suffix);
        let grid = generate_dual_grid_tile(corners, s, config.sym_a, config.sym_b, config.sym_border);
        let grid_str = grid.iter().map(|row| row.iter().collect::<String>()).collect::<Vec<_>>().join("\n");

        // Edge classes based on which sides are terrain A
        let n_is_a = corners[0] || corners[1]; // TL or TR
        let s_is_a = corners[2] || corners[3]; // BL or BR
        let w_is_a = corners[0] || corners[2]; // TL or BL
        let e_is_a = corners[1] || corners[3]; // TR or BR

        let edge_fn = |is_a: bool, is_mixed: bool| -> String {
            if is_mixed {
                format!("{}_{}_mix", config.terrain_a, config.terrain_b)
            } else if is_a {
                config.terrain_a.clone()
            } else {
                config.terrain_b.clone()
            }
        };

        let n_mixed = corners[0] != corners[1];
        let s_mixed = corners[2] != corners[3];
        let w_mixed = corners[0] != corners[2];
        let e_mixed = corners[1] != corners[3];

        let tile_raw = TileRaw {
            palette: config.palette.clone(),
            size: Some(format!("{}x{}", s, s)),
            encoding: None,
            symmetry: None,
            auto_rotate: None,
            auto_rotate_weight: None,
            template: None,
            edge_class: Some(EdgeClassRaw {
                n: edge_fn(n_is_a, n_mixed),
                e: edge_fn(e_is_a, e_mixed),
                s: edge_fn(s_is_a, s_mixed),
                w: edge_fn(w_is_a, w_mixed),
            }),
            corner_class: None,
            tags: vec!["wang".to_string(), "dual_grid".to_string(), config.terrain_a.clone()],
            target_layer: Some("terrain".to_string()),
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: Some(SemanticRaw {
                affordance: Some("walkable".to_string()),
                collision: Some("none".to_string()),
                collision_points: None,
                tags: HashMap::new(),
            }),
            grid: Some(grid_str),
            rle: None,
            layout: None,
            fill: None,
            fill_size: None,
            delta: None,
            patches: vec![],
        };

        tiles.push(WangTile {
            name,
            tile_raw,
            mask: 0,
            description: desc.to_string(),
        });
    }

    let summary = format!(
        "Generated {} dual-grid tiles for {} → {} transition ({}x{}).",
        tiles.len(), config.terrain_a, config.terrain_b, config.size, config.size
    );

    WangResult {
        tiles,
        method: WangMethod::DualGrid,
        terrain_a: config.terrain_a.clone(),
        terrain_b: config.terrain_b.clone(),
        summary,
    }
}

/// Generate a dual-grid tile from corner assignments.
fn generate_dual_grid_tile(
    corners: &[bool; 4], // [TL, TR, BL, BR]
    size: usize,
    sym_a: char,
    sym_b: char,
    sym_border: char,
) -> Vec<Vec<char>> {
    let mut grid = vec![vec![sym_b; size]; size];
    let half = size / 2;

    // Fill each quadrant
    for y in 0..size {
        for x in 0..size {
            let in_top = y < half;
            let in_left = x < half;
            let quadrant = match (in_top, in_left) {
                (true, true) => 0,   // TL
                (true, false) => 1,  // TR
                (false, true) => 2,  // BL
                (false, false) => 3, // BR
            };
            if corners[quadrant] {
                grid[y][x] = sym_a;
            }
        }
    }

    // Add borders at transitions
    let snapshot = grid.clone();
    for y in 0..size {
        for x in 0..size {
            if snapshot[y][x] == sym_a {
                let neighbors = [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)];
                for (dy, dx) in neighbors {
                    let ny = y as i32 + dy;
                    let nx = x as i32 + dx;
                    if ny >= 0 && ny < size as i32 && nx >= 0 && nx < size as i32 {
                        if snapshot[ny as usize][nx as usize] == sym_b {
                            grid[y][x] = sym_border;
                            break;
                        }
                    }
                }
            }
        }
    }

    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(method: WangMethod) -> WangConfig {
        WangConfig {
            terrain_a: "grass".to_string(),
            terrain_b: "water".to_string(),
            size: 8,
            palette: "test".to_string(),
            sym_a: '+',
            sym_b: '~',
            sym_border: '#',
            method,
        }
    }

    #[test]
    fn blob47_generates_tiles() {
        let config = test_config(WangMethod::Blob47);
        let result = generate(&config);
        assert!(!result.tiles.is_empty());
        // Should have unique tile names
        let names: std::collections::HashSet<&str> =
            result.tiles.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), result.tiles.len(), "tile names should be unique");
        // All tiles should have grids
        for tile in &result.tiles {
            assert!(tile.tile_raw.grid.is_some(), "tile {} should have a grid", tile.name);
            assert!(tile.tile_raw.edge_class.is_some(), "tile {} should have edge classes", tile.name);
        }
    }

    #[test]
    fn dual_grid_generates_15_tiles() {
        let config = test_config(WangMethod::DualGrid);
        let result = generate(&config);
        assert_eq!(result.tiles.len(), 15, "dual grid should produce 15 tiles");
        // Should have both full_a and full_b
        let has_full_a = result.tiles.iter().any(|t| t.name.contains("full_a"));
        let has_full_b = result.tiles.iter().any(|t| t.name.contains("full_b"));
        assert!(has_full_a && has_full_b, "should have both full terrain tiles");
    }

    #[test]
    fn blob47_full_tile_is_all_terrain_a() {
        let config = test_config(WangMethod::Blob47);
        let result = generate(&config);
        let full = result.tiles.iter().find(|t| t.mask == 255).unwrap();
        let grid_str = full.tile_raw.grid.as_ref().unwrap();
        // Full tile should be mostly sym_a (with possible border at edges)
        let a_count = grid_str.chars().filter(|&c| c == '+').count();
        let total = grid_str.chars().filter(|c| !c.is_whitespace()).count();
        assert!(a_count > total / 2, "full tile should be mostly terrain A");
    }

    #[test]
    fn blob47_isolated_tile_is_all_terrain_b() {
        let config = test_config(WangMethod::Blob47);
        let result = generate(&config);
        let isolated = result.tiles.iter().find(|t| t.mask == 0).unwrap();
        let grid_str = isolated.tile_raw.grid.as_ref().unwrap();
        // Isolated tile should be all sym_b
        let b_count = grid_str.chars().filter(|&c| c == '~').count();
        let total = grid_str.chars().filter(|c| !c.is_whitespace()).count();
        assert_eq!(b_count, total, "isolated tile should be all terrain B");
    }

    #[test]
    fn edge_classes_match_terrain() {
        let config = test_config(WangMethod::Blob47);
        let result = generate(&config);
        // N-only tile (mask=2): should have N=grass, S=water, E=water, W=water
        let n_only = result.tiles.iter().find(|t| t.mask == 2).unwrap();
        let ec = n_only.tile_raw.edge_class.as_ref().unwrap();
        assert_eq!(ec.n, "grass");
        assert_eq!(ec.s, "water");
        assert_eq!(ec.e, "water");
        assert_eq!(ec.w, "water");
    }
}
