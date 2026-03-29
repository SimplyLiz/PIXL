/// Unified tile grid resolver — resolves any TileRaw to a pixel grid
/// regardless of encoding (grid, RLE, compose, template, symmetry).
use crate::compose;
use crate::grid;
use crate::rle;
use crate::rotate;
use crate::symmetry;
use crate::types::{Palette, Stamp, Symmetry, TileRaw, parse_size};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("tile has no size and no template")]
    NoSize,
    #[error("tile has no pixel data (grid/rle/layout/fill/delta) and no template")]
    NoPixelData,
    #[error("template '{0}' not found")]
    TemplateNotFound(String),
    #[error("palette '{0}' not found")]
    PaletteNotFound(String),
    #[error("grid error: {0}")]
    Grid(#[from] grid::GridError),
    #[error("RLE error: {0}")]
    Rle(#[from] rle::RleError),
    #[error("compose error: {0}")]
    Compose(#[from] compose::ComposeError),
    #[error("symmetry error: {0}")]
    Symmetry(#[from] symmetry::SymmetryError),
    #[error("size parse error: {0}")]
    Size(String),
    #[error("delta base tile '{0}' not found")]
    DeltaBaseNotFound(String),
    #[error("delta chain: '{0}' is itself a delta tile")]
    DeltaChain(String),
    #[error("fill: pattern {fw}x{fh} doesn't evenly divide tile {tw}x{th}")]
    FillDimensionMismatch { fw: u32, fh: u32, tw: u32, th: u32 },
    #[error("patch at ({x},{y}) out of bounds for {w}x{h} tile")]
    PatchOutOfBounds { x: u32, y: u32, w: u32, h: u32 },
    #[error("patch symbol is empty")]
    PatchEmptySymbol,
}

/// Tile a small pattern grid to fill a larger area.
pub fn expand_fill(
    pattern: &[Vec<char>],
    pattern_w: u32,
    pattern_h: u32,
    tile_w: u32,
    tile_h: u32,
) -> Result<Vec<Vec<char>>, ResolveError> {
    if tile_w % pattern_w != 0 || tile_h % pattern_h != 0 {
        return Err(ResolveError::FillDimensionMismatch {
            fw: pattern_w,
            fh: pattern_h,
            tw: tile_w,
            th: tile_h,
        });
    }
    let mut grid = Vec::with_capacity(tile_h as usize);
    for y in 0..tile_h as usize {
        let src_y = y % pattern_h as usize;
        let mut row = Vec::with_capacity(tile_w as usize);
        for x in 0..tile_w as usize {
            let src_x = x % pattern_w as usize;
            row.push(pattern[src_y][src_x]);
        }
        grid.push(row);
    }
    Ok(grid)
}

/// Rotation suffixes and the number of 90° CW rotations they represent.
const ROTATION_SUFFIXES: &[(&str, u32)] = &[
    ("_270f", 0), // flip + 270 (checked first since longer)
    ("_180f", 0), // flip + 180
    ("_90f", 0),  // flip + 90
    ("_flip", 0), // flip only
    ("_270", 3),
    ("_180", 2),
    ("_90", 1),
];

/// Try to strip a rotation suffix and return (base_name, rotations, flip).
fn parse_rotation_suffix(name: &str) -> Option<(&str, u32, bool)> {
    for &(suffix, _) in ROTATION_SUFFIXES {
        if let Some(base) = name.strip_suffix(suffix) {
            let flip = suffix.contains('f') || suffix == "_flip";
            let rotations = match suffix {
                "_90" | "_90f" => 1,
                "_180" | "_180f" => 2,
                "_270" | "_270f" => 3,
                "_flip" => 0,
                _ => 0,
            };
            return Some((base, rotations, flip));
        }
    }
    None
}

/// Extract the base tile name from a rotation-suffixed name.
/// Returns None if the name has no rotation suffix.
pub fn base_tile_name(name: &str) -> Option<&str> {
    parse_rotation_suffix(name).map(|(base, _, _)| base)
}

/// Resolve a tile to its full pixel grid.
pub fn resolve_tile_grid(
    name: &str,
    tiles: &HashMap<String, TileRaw>,
    palettes: &HashMap<String, Palette>,
    stamps: &HashMap<String, Stamp>,
) -> Result<(Vec<Vec<char>>, u32, u32), ResolveError> {
    // Try direct lookup first
    let tile_raw = match tiles.get(name) {
        Some(t) => t,
        None => {
            // Try as a rotated variant of a base tile
            if let Some((base_name, rotations, flip)) = parse_rotation_suffix(name) {
                if let Some(base_tile) = tiles.get(base_name) {
                    if base_tile.auto_rotate.is_some() {
                        let (mut grid, w, h) =
                            resolve_tile_grid(base_name, tiles, palettes, stamps)?;
                        if flip {
                            grid = rotate::flip_grid_h(&grid);
                        }
                        for _ in 0..rotations {
                            grid = rotate::rotate_grid_cw(&grid);
                        }
                        // For square tiles, dimensions stay the same after rotation
                        return Ok((grid, w, h));
                    }
                }
            }
            return Err(ResolveError::NoPixelData);
        }
    };

    // PAX 2.1: handle delta tiles — resolve base grid, apply patches
    if let Some(ref delta_base) = tile_raw.delta {
        let base_tile = tiles
            .get(delta_base.as_str())
            .ok_or_else(|| ResolveError::DeltaBaseNotFound(delta_base.clone()))?;
        // Reject delta chains
        if base_tile.delta.is_some() {
            return Err(ResolveError::DeltaChain(delta_base.clone()));
        }
        let (mut base_grid, w, h) = resolve_tile_grid(delta_base, tiles, palettes, stamps)?;
        // Apply patches on the fully expanded grid
        for patch in &tile_raw.patches {
            if patch.x >= w || patch.y >= h {
                return Err(ResolveError::PatchOutOfBounds {
                    x: patch.x,
                    y: patch.y,
                    w,
                    h,
                });
            }
            let sym = patch
                .sym
                .chars()
                .next()
                .ok_or(ResolveError::PatchEmptySymbol)?;
            base_grid[patch.y as usize][patch.x as usize] = sym;
        }
        return Ok((base_grid, w, h));
    }

    // Handle template tiles
    let effective = if let Some(ref template_name) = tile_raw.template {
        tiles
            .get(template_name.as_str())
            .ok_or_else(|| ResolveError::TemplateNotFound(template_name.clone()))?
    } else {
        tile_raw
    };

    let size_str = effective
        .size
        .as_deref()
        .or(tile_raw.size.as_deref())
        .ok_or(ResolveError::NoSize)?;
    let (w, h) = parse_size(size_str).map_err(ResolveError::Size)?;

    let palette = palettes
        .get(&tile_raw.palette)
        .ok_or_else(|| ResolveError::PaletteNotFound(tile_raw.palette.clone()))?;

    // Determine symmetry
    let sym = match effective.symmetry.as_deref() {
        Some("horizontal") => Symmetry::Horizontal,
        Some("vertical") => Symmetry::Vertical,
        Some("quad") => Symmetry::Quad,
        _ => Symmetry::None,
    };

    // Grid dimensions accounting for symmetry
    let (grid_w, grid_h) = match sym {
        Symmetry::None => (w, h),
        Symmetry::Horizontal => (w / 2, h),
        Symmetry::Vertical => (w, h / 2),
        Symmetry::Quad => (w / 2, h / 2),
    };

    // Resolve the grid based on encoding
    let partial_grid = if let Some(ref grid_str) = effective.grid {
        grid::parse_grid(grid_str, grid_w, grid_h, palette)?
    } else if let Some(ref rle_str) = effective.rle {
        rle::parse_rle(rle_str, grid_w, grid_h, palette)?
    } else if let Some(ref layout_str) = effective.layout {
        compose::resolve_compose(layout_str, stamps, w, h, '.')?
    } else if let Some(ref fill_str) = effective.fill {
        // PAX 2.1: pattern fill encoding
        let fill_size_str = effective
            .fill_size
            .as_deref()
            .ok_or_else(|| ResolveError::Size("fill requires fill_size".into()))?;
        let (fw, fh) = parse_size(fill_size_str).map_err(ResolveError::Size)?;
        let pattern = grid::parse_grid(fill_str, fw, fh, palette)?;
        expand_fill(&pattern, fw, fh, w, h)?
    } else {
        return Err(ResolveError::NoPixelData);
    };

    // Expand symmetry
    let full_grid = symmetry::expand_symmetry(&partial_grid, w, h, sym)?;

    Ok((full_grid, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EdgeClassRaw, Rgba};

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert(
            '.',
            Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        );
        symbols.insert(
            '#',
            Rgba {
                r: 42,
                g: 31,
                b: 61,
                a: 255,
            },
        );
        symbols.insert(
            '+',
            Rgba {
                r: 74,
                g: 58,
                b: 109,
                a: 255,
            },
        );
        Palette { symbols }
    }

    fn make_tile(size: &str, grid: &str) -> TileRaw {
        TileRaw {
            palette: "test".to_string(),
            size: Some(size.to_string()),
            encoding: None,
            symmetry: None,
            auto_rotate: None,
            auto_rotate_weight: None,
            template: None,
            edge_class: None,
            corner_class: None,
            tags: vec![],
            target_layer: None,
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: None,
            grid: Some(grid.to_string()),
            rle: None,
            layout: None,
            fill: None,
            fill_size: None,
            delta: None,
            patches: vec![],
        }
    }

    #[test]
    fn resolve_grid_tile() {
        let mut tiles = HashMap::new();
        tiles.insert("wall".to_string(), make_tile("4x2", "####\n++++"));
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let (grid, w, h) = resolve_tile_grid("wall", &tiles, &palettes, &HashMap::new()).unwrap();
        assert_eq!(w, 4);
        assert_eq!(h, 2);
        assert_eq!(grid[0], vec!['#', '#', '#', '#']);
        assert_eq!(grid[1], vec!['+', '+', '+', '+']);
    }

    #[test]
    fn resolve_rle_tile() {
        let mut tile = make_tile("4x2", "");
        tile.grid = None;
        tile.rle = Some("4#\n4+".to_string());

        let mut tiles = HashMap::new();
        tiles.insert("wall".to_string(), tile);
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let (grid, _, _) = resolve_tile_grid("wall", &tiles, &palettes, &HashMap::new()).unwrap();
        assert_eq!(grid[0], vec!['#', '#', '#', '#']);
        assert_eq!(grid[1], vec!['+', '+', '+', '+']);
    }

    #[test]
    fn resolve_template_tile() {
        let mut tiles = HashMap::new();
        tiles.insert("base".to_string(), make_tile("4x2", "####\n++++"));

        let mut child = TileRaw {
            palette: "test".to_string(),
            size: None,
            template: Some("base".to_string()),
            ..make_tile("", "")
        };
        child.grid = None;
        tiles.insert("child".to_string(), child);

        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let (grid, w, h) = resolve_tile_grid("child", &tiles, &palettes, &HashMap::new()).unwrap();
        assert_eq!(w, 4);
        assert_eq!(h, 2);
        assert_eq!(grid[0], vec!['#', '#', '#', '#']);
    }

    #[test]
    fn resolve_quad_symmetry() {
        let mut tile = make_tile("4x4", "##\n#+");
        tile.symmetry = Some("quad".to_string());

        let mut tiles = HashMap::new();
        tiles.insert("gem".to_string(), tile);
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let (grid, w, h) = resolve_tile_grid("gem", &tiles, &palettes, &HashMap::new()).unwrap();
        assert_eq!(w, 4);
        assert_eq!(h, 4);
        assert_eq!(grid[0], vec!['#', '#', '#', '#']);
        assert_eq!(grid[1], vec!['#', '+', '+', '#']);
        assert_eq!(grid[2], vec!['#', '+', '+', '#']);
        assert_eq!(grid[3], vec!['#', '#', '#', '#']);
    }

    #[test]
    fn resolve_fill_tile() {
        // 2x2 pattern tiled into 4x4
        let mut tile = make_tile("4x4", "");
        tile.grid = None;
        tile.fill = Some("#+\n+#".to_string());
        tile.fill_size = Some("2x2".to_string());

        let mut tiles = HashMap::new();
        tiles.insert("checker".to_string(), tile);
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let (grid, w, h) =
            resolve_tile_grid("checker", &tiles, &palettes, &HashMap::new()).unwrap();
        assert_eq!(w, 4);
        assert_eq!(h, 4);
        assert_eq!(grid[0], vec!['#', '+', '#', '+']);
        assert_eq!(grid[1], vec!['+', '#', '+', '#']);
        assert_eq!(grid[2], vec!['#', '+', '#', '+']);
        assert_eq!(grid[3], vec!['+', '#', '+', '#']);
    }

    #[test]
    fn resolve_fill_bad_dimensions() {
        // 3x3 pattern into 4x4 → doesn't divide evenly
        let mut tile = make_tile("4x4", "");
        tile.grid = None;
        tile.fill = Some("#+#\n+#.\n#+#".to_string());
        tile.fill_size = Some("3x3".to_string());

        let mut tiles = HashMap::new();
        tiles.insert("bad".to_string(), tile);
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let err = resolve_tile_grid("bad", &tiles, &palettes, &HashMap::new()).unwrap_err();
        assert!(matches!(err, ResolveError::FillDimensionMismatch { .. }));
    }

    #[test]
    fn resolve_delta_tile() {
        use crate::types::PatchRaw;

        let mut tiles = HashMap::new();
        tiles.insert("base".to_string(), make_tile("4x2", "####\n++++"));

        let mut delta_tile = make_tile("4x2", "");
        delta_tile.grid = None;
        delta_tile.delta = Some("base".to_string());
        delta_tile.patches = vec![
            PatchRaw {
                x: 1,
                y: 0,
                sym: "+".to_string(),
            },
            PatchRaw {
                x: 2,
                y: 1,
                sym: "#".to_string(),
            },
        ];
        tiles.insert("variant".to_string(), delta_tile);

        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let (grid, _, _) =
            resolve_tile_grid("variant", &tiles, &palettes, &HashMap::new()).unwrap();
        assert_eq!(grid[0], vec!['#', '+', '#', '#']); // patch at (1,0)
        assert_eq!(grid[1], vec!['+', '+', '#', '+']); // patch at (2,1)
    }

    #[test]
    fn resolve_delta_chain_rejected() {
        use crate::types::PatchRaw;

        let mut tiles = HashMap::new();
        tiles.insert("base".to_string(), make_tile("4x2", "####\n++++"));

        let mut delta1 = make_tile("4x2", "");
        delta1.grid = None;
        delta1.delta = Some("base".to_string());
        delta1.patches = vec![PatchRaw {
            x: 0,
            y: 0,
            sym: "+".to_string(),
        }];
        tiles.insert("d1".to_string(), delta1);

        let mut delta2 = make_tile("4x2", "");
        delta2.grid = None;
        delta2.delta = Some("d1".to_string()); // chain!
        delta2.patches = vec![];
        tiles.insert("d2".to_string(), delta2);

        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let err = resolve_tile_grid("d2", &tiles, &palettes, &HashMap::new()).unwrap_err();
        assert!(matches!(err, ResolveError::DeltaChain(_)));
    }

    #[test]
    fn resolve_delta_patch_out_of_bounds() {
        use crate::types::PatchRaw;

        let mut tiles = HashMap::new();
        tiles.insert("base".to_string(), make_tile("4x2", "####\n++++"));

        let mut delta_tile = make_tile("4x2", "");
        delta_tile.grid = None;
        delta_tile.delta = Some("base".to_string());
        delta_tile.patches = vec![PatchRaw {
            x: 10,
            y: 0,
            sym: "+".to_string(),
        }];
        tiles.insert("bad".to_string(), delta_tile);

        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());

        let err = resolve_tile_grid("bad", &tiles, &palettes, &HashMap::new()).unwrap_err();
        assert!(matches!(err, ResolveError::PatchOutOfBounds { .. }));
    }
}
