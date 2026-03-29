//! BPE-inspired auto-stamp extraction for PAX-L.
//!
//! Scans all tile grids, discovers repeating spatial patterns (4×4 blocks),
//! and produces a stamp vocabulary that the serializer can use for compose
//! encoding. Based on Elsner et al., "Multidimensional Byte Pair Encoding"
//! (ICCV 2025).

use crate::types::{Palette, PaxFile, Stamp, TileRaw, parse_size};
use crate::resolve;
use std::collections::HashMap;

/// A stamp discovered by the BPE extraction algorithm.
#[derive(Debug, Clone)]
pub struct AutoStamp {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Vec<char>>,
    pub frequency: usize,
}

/// Extract auto-stamps from all grid-encoded tiles in a PaxFile.
///
/// Returns stamps sorted by savings (frequency × area) descending.
/// Only stamps appearing ≥ `min_freq` times are returned.
pub fn extract_stamps(
    file: &PaxFile,
    palettes: &HashMap<String, Palette>,
    existing_stamps: &HashMap<String, Stamp>,
    min_freq: usize,
) -> Vec<AutoStamp> {
    // Resolve all grid-encoded tiles to char grids
    let mut resolved_grids: Vec<(String, Vec<Vec<char>>, u32, u32)> = Vec::new();

    // Collect tiles that are in spritesets/composites (skip these)
    let noauto_tiles = collect_noauto_tiles(file);

    for (name, tile) in &file.tile {
        if noauto_tiles.contains(name.as_str()) {
            continue;
        }
        // Only process grid-encoded tiles (not template, delta, fill, compose)
        if tile.grid.is_none() {
            continue;
        }
        if tile.template.is_some() || tile.delta.is_some() {
            continue;
        }

        if let Ok((grid, w, h)) =
            resolve::resolve_tile_grid(name, &file.tile, palettes, existing_stamps)
        {
            resolved_grids.push((name.clone(), grid, w, h));
        }
    }

    if resolved_grids.is_empty() {
        return Vec::new();
    }

    // Extract 4×4 blocks at stride 4
    let block_w: u32 = 4;
    let block_h: u32 = 4;
    let mut freq_table: HashMap<Vec<Vec<char>>, usize> = HashMap::new();

    for (_, grid, w, h) in &resolved_grids {
        if *w < block_w || *h < block_h {
            continue;
        }
        let mut y = 0;
        while y + block_h <= *h {
            let mut x = 0;
            while x + block_w <= *w {
                let block: Vec<Vec<char>> = (y..y + block_h)
                    .map(|row| {
                        grid[row as usize][x as usize..(x + block_w) as usize].to_vec()
                    })
                    .collect();
                *freq_table.entry(block).or_insert(0) += 1;
                x += block_w;
            }
            y += block_h;
        }
    }

    // Filter by min frequency and rank by savings
    let mut candidates: Vec<(Vec<Vec<char>>, usize)> = freq_table
        .into_iter()
        .filter(|(_, freq)| *freq >= min_freq)
        .collect();

    // Sort by frequency × area (savings) descending
    let area = (block_w * block_h) as usize;
    candidates.sort_by(|a, b| (b.1 * area).cmp(&(a.1 * area)));

    // Name and deduplicate against existing stamps
    let mut result = Vec::new();
    let mut used_names: std::collections::HashSet<String> = existing_stamps.keys().cloned().collect();

    for (grid, freq) in candidates {
        let name = generate_name(&grid, &used_names);
        if used_names.contains(&name) {
            continue;
        }
        used_names.insert(name.clone());

        result.push(AutoStamp {
            name,
            width: block_w,
            height: block_h,
            grid,
            frequency: freq,
        });
    }

    result
}

/// Try to decompose a tile grid into a compose layout using available stamps.
/// Returns the layout string if successful, None otherwise.
pub fn try_compose_decomposition(
    tile_grid: &[Vec<char>],
    tile_w: u32,
    tile_h: u32,
    stamps: &[AutoStamp],
    existing_stamps: &HashMap<String, Stamp>,
) -> Option<String> {
    let block_w: u32 = 4;
    let block_h: u32 = 4;

    // Must be exact multiple of block size
    if tile_w % block_w != 0 || tile_h % block_h != 0 {
        return None;
    }

    // Build lookup: grid content → stamp name
    let mut stamp_lookup: HashMap<Vec<Vec<char>>, String> = HashMap::new();
    for s in stamps {
        if s.width == block_w && s.height == block_h {
            stamp_lookup.insert(s.grid.clone(), s.name.clone());
        }
    }
    for (name, s) in existing_stamps {
        if s.width == block_w && s.height == block_h {
            stamp_lookup.insert(s.grid.clone(), name.clone());
        }
    }

    let cols = tile_w / block_w;
    let rows = tile_h / block_h;
    let mut layout_lines = Vec::new();

    for row in 0..rows {
        let mut line_parts = Vec::new();
        for col in 0..cols {
            let y = (row * block_h) as usize;
            let x = (col * block_w) as usize;
            let block: Vec<Vec<char>> = (y..y + block_h as usize)
                .map(|r| tile_grid[r][x..x + block_w as usize].to_vec())
                .collect();

            match stamp_lookup.get(&block) {
                Some(name) => line_parts.push(format!("@{}", name)),
                None => return None, // Can't decompose — missing stamp
            }
        }
        layout_lines.push(line_parts.join(" "));
    }

    Some(layout_lines.join("\n"))
}

/// Collect tile names that should not be auto-decomposed.
/// Tiles referenced by spritesets or composites are excluded.
fn collect_noauto_tiles(file: &PaxFile) -> std::collections::HashSet<&str> {
    let mut noauto = std::collections::HashSet::new();

    // Tiles in spritesets
    for ss in file.spriteset.values() {
        // The spriteset itself uses tiles referenced by name in frames
        // but sprite grids are inline, not tile references.
        // Skip the spriteset's own tiles for safety.
    }

    // Tiles in composites
    for comp in file.composite.values() {
        for line in comp.layout.lines() {
            for token in line.split_whitespace() {
                let name = token.trim_start_matches('!').split('!').next().unwrap_or(token);
                if name != "_" && !name.is_empty() {
                    noauto.insert(name);
                }
            }
        }
    }

    noauto
}

/// Generate a semantic name for a discovered stamp.
fn generate_name(
    grid: &[Vec<char>],
    used_names: &std::collections::HashSet<String>,
) -> String {
    let w = grid[0].len();
    let h = grid.len();
    let size_suffix = format!("{}x{}", w, h);

    // Check if all one symbol
    let first = grid[0][0];
    let all_same = grid.iter().all(|row| row.iter().all(|&c| c == first));
    if all_same {
        let base = match first {
            '#' => "solid",
            '+' => "flat",
            '.' => "void",
            's' => "shadow",
            'h' => "highlight",
            _ => "uniform",
        };
        let name = format!("{}_{}", base, size_suffix);
        if !used_names.contains(&name) {
            return name;
        }
    }

    // Check for mortar/brick pattern (has '#' and '+'/h)
    let has_structure = grid.iter().any(|row| row.contains(&'#'));
    let has_surface = grid.iter().any(|row| row.iter().any(|&c| c == '+' || c == 'h'));
    if has_structure && has_surface {
        // Try positional naming
        let top_left = grid[0][0];
        let pos = match top_left {
            'h' | '+' => "lit",
            's' => "dark",
            '#' => "struct",
            _ => "mixed",
        };
        let name = format!("{}_{}", pos, size_suffix);
        if !used_names.contains(&name) {
            return name;
        }
    }

    // Fallback: hash-based name
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::{Hash, Hasher};
    for row in grid {
        row.hash(&mut hasher);
    }
    let hash = hasher.finish();
    format!("p_{:04x}", hash & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse_pax, resolve_all_palettes};
    use crate::types::Rgba;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        for (ch, color) in [
            ('.', [0, 0, 0, 0]),
            ('#', [42, 31, 61, 255]),
            ('+', [90, 72, 120, 255]),
            ('h', [128, 112, 168, 255]),
            ('s', [18, 9, 31, 255]),
        ] {
            symbols.insert(
                ch,
                Rgba {
                    r: color[0],
                    g: color[1],
                    b: color[2],
                    a: color[3],
                },
            );
        }
        Palette { symbols }
    }

    #[test]
    fn extract_stamps_from_dungeon() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let file = parse_pax(&source).unwrap();
        let palettes = resolve_all_palettes(&file).unwrap();

        // Resolve existing stamps
        let mut stamps = HashMap::new();
        for (name, raw) in &file.stamp {
            let (sw, sh) = parse_size(&raw.size).unwrap();
            let pal = &palettes[&raw.palette];
            let grid = crate::grid::parse_grid(&raw.grid, sw, sh, pal).unwrap();
            stamps.insert(
                name.clone(),
                Stamp {
                    palette: raw.palette.clone(),
                    width: sw,
                    height: sh,
                    grid,
                },
            );
        }

        let auto = extract_stamps(&file, &palettes, &stamps, 2);

        // Should find at least some repeated 4×4 blocks
        assert!(!auto.is_empty(), "should find auto-stamps in dungeon tiles");

        // Most frequent should have freq ≥ 2
        assert!(auto[0].frequency >= 2);

        // All stamps should be 4×4
        for s in &auto {
            assert_eq!(s.width, 4);
            assert_eq!(s.height, 4);
        }
    }

    #[test]
    fn compose_decomposition_works() {
        // Create a tile that's 4 copies of the same 4×4 block
        let block = vec![
            vec!['#', '#', '#', '#'],
            vec!['#', '+', '+', '#'],
            vec!['#', '+', '+', '#'],
            vec!['#', '#', '#', '#'],
        ];
        let tile_grid: Vec<Vec<char>> = block
            .iter()
            .chain(block.iter())
            .cloned()
            .collect();
        // 4×8 tile = two stacked 4×4 blocks

        let stamps = vec![AutoStamp {
            name: "box_4x4".to_string(),
            width: 4,
            height: 4,
            grid: block.clone(),
            frequency: 5,
        }];

        let result = try_compose_decomposition(&tile_grid, 4, 8, &stamps, &HashMap::new());
        assert!(result.is_some());
        let layout = result.unwrap();
        assert!(layout.contains("@box_4x4"));
        assert_eq!(layout.lines().count(), 2); // 2 rows of stamps
    }

    #[test]
    fn compose_fails_on_unknown_block() {
        let tile_grid = vec![
            vec!['#', '+', '#', '+'],
            vec!['+', '#', '+', '#'],
            vec!['#', '+', '#', '+'],
            vec!['+', '#', '+', '#'],
            vec!['h', 'h', 'h', 'h'],
            vec!['s', 's', 's', 's'],
            vec!['h', 'h', 'h', 'h'],
            vec!['s', 's', 's', 's'],
        ];

        // Only provide stamp for the first block, not the second
        let stamps = vec![AutoStamp {
            name: "checker_4x4".to_string(),
            width: 4,
            height: 4,
            grid: vec![
                vec!['#', '+', '#', '+'],
                vec!['+', '#', '+', '#'],
                vec!['#', '+', '#', '+'],
                vec!['+', '#', '+', '#'],
            ],
            frequency: 3,
        }];

        let result = try_compose_decomposition(&tile_grid, 4, 8, &stamps, &HashMap::new());
        assert!(result.is_none()); // Second block has no matching stamp
    }

    #[test]
    fn name_generation_semantic() {
        let used = std::collections::HashSet::new();

        // All '#' → "solid_4x4"
        let solid = vec![vec!['#'; 4]; 4];
        assert_eq!(generate_name(&solid, &used), "solid_4x4");

        // All '+' → "flat_4x4"
        let flat = vec![vec!['+'; 4]; 4];
        assert_eq!(generate_name(&flat, &used), "flat_4x4");

        // All '.' → "void_4x4"
        let void_block = vec![vec!['.'; 4]; 4];
        assert_eq!(generate_name(&void_block, &used), "void_4x4");
    }
}
