//! Smart encoding selection for PAX-L tile output.
//!
//! Compares multiple representations (grid+refs, fill, delta) and picks
//! the one with fewest estimated tokens.

use crate::types::{PatchRaw, TileRaw};
use std::collections::HashMap;

/// The chosen encoding for a tile's grid data in PAX-L.
pub enum TileEncoding {
    /// Template: `:base_tile` (no grid data)
    Template(String),
    /// Delta: `@delta base` + patch lines
    Delta {
        base: String,
        patches: Vec<PatchRaw>,
    },
    /// Fill: `@fill WxH` + pattern rows
    Fill {
        pattern: String,
        fill_w: u32,
        fill_h: u32,
    },
    /// Compose: stamp reference layout
    Compose(String),
    /// Raw grid (possibly with =N row references)
    Grid {
        body: String,
        row_count: usize,
    },
    /// RLE encoding
    Rle {
        body: String,
        row_count: usize,
    },
}

/// Select the best encoding for a tile.
pub fn select_encoding(
    tile: &TileRaw,
    all_tiles: &HashMap<String, TileRaw>,
    config: &super::PaxlConfig,
) -> TileEncoding {
    // 1. Template — no grid data at all
    if let Some(ref tmpl) = tile.template {
        return TileEncoding::Template(tmpl.clone());
    }

    // 2. Delta — already declared in the .pax file
    if let Some(ref delta_base) = tile.delta {
        return TileEncoding::Delta {
            base: delta_base.clone(),
            patches: tile.patches.clone(),
        };
    }

    // 3. Fill — already declared in the .pax file
    if let Some(ref fill_str) = tile.fill {
        let (fw, fh) = tile
            .fill_size
            .as_deref()
            .and_then(|s| crate::types::parse_size(s).ok())
            .unwrap_or((4, 4));
        return TileEncoding::Fill {
            pattern: fill_str.clone(),
            fill_w: fw,
            fill_h: fh,
        };
    }

    // 4. Compose — already declared
    if let Some(ref layout) = tile.layout {
        return TileEncoding::Compose(layout.clone());
    }

    // 5. RLE — already declared
    if let Some(ref rle_str) = tile.rle {
        let row_count = rle_str.lines().filter(|l| !l.trim().is_empty()).count();
        return TileEncoding::Rle {
            body: rle_str.clone(),
            row_count,
        };
    }

    // 6. Grid — apply optimizations
    if let Some(ref grid_str) = tile.grid {
        let (w, h) = tile
            .size
            .as_deref()
            .and_then(|s| crate::types::parse_size(s).ok())
            .unwrap_or((16, 16));

        let mut best_body = grid_str.clone();
        let mut best_tokens = estimate_tokens(grid_str);

        // Try fill pattern detection
        if config.fill_detect {
            if let Some((pattern, fw, fh)) = detect_fill_pattern(grid_str, w, h) {
                let fill_tokens = estimate_tokens(&pattern) + 10; // overhead for @fill line
                if fill_tokens < best_tokens {
                    return TileEncoding::Fill {
                        pattern,
                        fill_w: fw,
                        fill_h: fh,
                    };
                }
            }
        }

        // Try row references
        if config.row_refs {
            let (with_refs, _) = apply_row_refs(grid_str);
            let ref_tokens = estimate_tokens(&with_refs);
            if ref_tokens < best_tokens {
                best_body = with_refs;
                best_tokens = ref_tokens;
            }
        }

        // Try delta against existing tiles
        if config.delta_detect {
            if let Some((base_name, patches)) =
                find_best_delta(tile, all_tiles, w, h, config.delta_threshold)
            {
                let delta_tokens = estimate_delta_tokens(&base_name, &patches);
                if delta_tokens < best_tokens {
                    return TileEncoding::Delta {
                        base: base_name,
                        patches,
                    };
                }
            }
        }

        let row_count = best_body.lines().filter(|l| !l.trim().is_empty()).count();
        return TileEncoding::Grid {
            body: best_body,
            row_count,
        };
    }

    // Fallback — empty grid (shouldn't happen for valid tiles)
    TileEncoding::Grid {
        body: String::new(),
        row_count: 0,
    }
}

/// Replace duplicate rows with =N references.
/// Returns (modified grid string, row count).
pub fn apply_row_refs(grid_str: &str) -> (String, usize) {
    let rows: Vec<&str> = grid_str
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect();

    if rows.is_empty() {
        return (String::new(), 0);
    }

    let mut output_lines = Vec::with_capacity(rows.len());
    // Map row content → first occurrence index (1-indexed)
    let mut seen: HashMap<&str, usize> = HashMap::new();

    for (i, row) in rows.iter().enumerate() {
        if let Some(&first_idx) = seen.get(row) {
            output_lines.push(format!("={}", first_idx));
        } else {
            seen.insert(row, i + 1); // 1-indexed
            output_lines.push(row.to_string());
        }
    }

    let count = output_lines.len();
    (output_lines.join("\n"), count)
}

/// Detect if a grid is a tiled repetition of a smaller pattern.
/// Tries sizes: 2×2, 4×2, 2×4, 4×4, 8×4, 4×8, 8×8.
pub fn detect_fill_pattern(grid_str: &str, tile_w: u32, tile_h: u32) -> Option<(String, u32, u32)> {
    let rows: Vec<&str> = grid_str
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect();

    if rows.is_empty() || rows.len() != tile_h as usize {
        return None;
    }

    let pattern_sizes: &[(u32, u32)] = &[
        (2, 2),
        (4, 2),
        (2, 4),
        (4, 4),
        (8, 4),
        (4, 8),
        (8, 8),
    ];

    for &(pw, ph) in pattern_sizes {
        if tile_w % pw != 0 || tile_h % ph != 0 {
            continue;
        }
        if pw >= tile_w && ph >= tile_h {
            continue; // Pattern same size as tile — no savings
        }

        // Extract the top-left pattern
        let pattern: Vec<String> = rows[..ph as usize]
            .iter()
            .map(|r| {
                let chars: Vec<char> = r.chars().collect();
                chars[..pw as usize].iter().collect::<String>()
            })
            .collect();

        // Verify tiling
        let mut matches = true;
        'outer: for y in 0..tile_h as usize {
            let row_chars: Vec<char> = rows[y].chars().collect();
            let pat_y = y % ph as usize;
            let pat_row: Vec<char> = pattern[pat_y].chars().collect();
            for x in 0..tile_w as usize {
                let pat_x = x % pw as usize;
                if x < row_chars.len() && pat_x < pat_row.len() {
                    if row_chars[x] != pat_row[pat_x] {
                        matches = false;
                        break 'outer;
                    }
                }
            }
        }

        if matches {
            return Some((pattern.join("\n"), pw, ph));
        }
    }

    None
}

/// Find the best delta base tile: same palette, same size, fewest diffs.
fn find_best_delta(
    tile: &TileRaw,
    all_tiles: &HashMap<String, TileRaw>,
    w: u32,
    h: u32,
    threshold: usize,
) -> Option<(String, Vec<PatchRaw>)> {
    let grid_str = tile.grid.as_ref()?;
    let tile_rows: Vec<Vec<char>> = grid_str
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .map(|l| l.chars().collect())
        .collect();

    if tile_rows.len() != h as usize {
        return None;
    }

    let mut best: Option<(String, Vec<PatchRaw>)> = None;
    let mut best_count = threshold + 1;

    for (name, candidate) in all_tiles {
        // Skip self, deltas, templates, different palette/size
        if candidate.delta.is_some() || candidate.template.is_some() {
            continue;
        }
        if candidate.palette != tile.palette {
            continue;
        }
        let cand_size = candidate.size.as_deref().unwrap_or("");
        let tile_size = tile.size.as_deref().unwrap_or("");
        if cand_size != tile_size {
            continue;
        }
        let cand_grid = match &candidate.grid {
            Some(g) => g,
            None => continue,
        };

        let cand_rows: Vec<Vec<char>> = cand_grid
            .lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .map(|l| l.chars().collect())
            .collect();

        if cand_rows.len() != h as usize {
            continue;
        }

        // Compute diff
        let mut patches = Vec::new();
        for y in 0..h as usize {
            if cand_rows[y].len() != w as usize || tile_rows[y].len() != w as usize {
                patches.clear();
                break;
            }
            for x in 0..w as usize {
                if tile_rows[y][x] != cand_rows[y][x] {
                    patches.push(PatchRaw {
                        x: x as u32,
                        y: y as u32,
                        sym: tile_rows[y][x].to_string(),
                    });
                }
            }
        }

        if !patches.is_empty() && patches.len() < best_count {
            best_count = patches.len();
            best = Some((name.clone(), patches));
        }
    }

    best
}

/// Rough token estimation. Simple heuristic: chars / 3.5.
/// Good enough for comparing encoding options relative to each other.
pub fn estimate_tokens(s: &str) -> usize {
    (s.len() as f64 / 3.5).ceil() as usize
}

/// Estimate tokens for a delta encoding.
fn estimate_delta_tokens(base_name: &str, patches: &[PatchRaw]) -> usize {
    // @delta base_name\n  +x,y sym  +x,y sym ...
    let header = format!("@delta {}\n", base_name);
    let patch_text: String = patches
        .iter()
        .map(|p| format!("  +{},{} {}", p.x, p.y, p.sym))
        .collect::<Vec<_>>()
        .join("\n");
    estimate_tokens(&format!("{}{}", header, patch_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_refs_detects_duplicates() {
        let grid = "####\n++++\n####\n++++";
        let (result, count) = apply_row_refs(grid);
        assert_eq!(count, 4);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], "####");
        assert_eq!(lines[1], "++++");
        assert_eq!(lines[2], "=1"); // dup of row 1
        assert_eq!(lines[3], "=2"); // dup of row 2
    }

    #[test]
    fn row_refs_no_dups() {
        let grid = "####\n#++#\n++++\n++##";
        let (result, count) = apply_row_refs(grid);
        assert_eq!(count, 4);
        // All unique — no =N in output
        assert!(!result.contains('='));
    }

    #[test]
    fn fill_detects_2x2_checkerboard() {
        let grid = "#+#+" .to_string() + "\n"
            + "+#+" + "#" + "\n"
            + "#+#+" + "\n"
            + "+#+" + "#";
        let result = detect_fill_pattern(&grid, 4, 4);
        assert!(result.is_some());
        let (pattern, pw, ph) = result.unwrap();
        assert_eq!(pw, 2);
        assert_eq!(ph, 2);
        assert_eq!(pattern, "#+" .to_string() + "\n" + "+#");
    }

    #[test]
    fn fill_rejects_irregular() {
        let grid = "####\n#++#\n++++\n++##";
        let result = detect_fill_pattern(grid, 4, 4);
        assert!(result.is_none());
    }

    #[test]
    fn token_estimation_reasonable() {
        // ~100 chars should be ~28 tokens
        let s = "#".repeat(100);
        let tokens = estimate_tokens(&s);
        assert!(tokens > 20 && tokens < 40);
    }
}
