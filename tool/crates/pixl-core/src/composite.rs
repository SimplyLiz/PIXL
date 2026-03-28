use crate::rotate::flip_grid_h;
use crate::types::{
    Composite, CompositeAnim, CompositeAnimRaw, CompositeFrame, CompositeRaw, Tile, TileModifier,
    TileRef, parse_size,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompositeError {
    #[error("invalid size '{0}': {1}")]
    BadSize(String, String),

    #[error("size {0}x{1} is not divisible by tile_size {2}x{3}")]
    SizeNotDivisible(u32, u32, u32, u32),

    #[error("layout has {got} rows, expected {expected}")]
    RowCount { expected: u32, got: u32 },

    #[error("layout row {row} has {got} columns, expected {expected}")]
    ColCount { row: usize, expected: u32, got: u32 },

    #[error("tile '{0}' not found")]
    TileNotFound(String),

    #[error("tile '{name}' is {got_w}x{got_h}, expected {exp_w}x{exp_h}")]
    TileSizeMismatch {
        name: String,
        exp_w: u32,
        exp_h: u32,
        got_w: u32,
        got_h: u32,
    },

    #[error("invalid slot address '{0}': expected 'row_col'")]
    BadSlotAddress(String),

    #[error("slot '{0}' is out of bounds (grid is {1}x{2})")]
    SlotOutOfBounds(String, u32, u32),

    #[error("variant '{variant}': {source}")]
    Variant {
        variant: String,
        source: Box<CompositeError>,
    },

    #[error("anim '{anim}': {source}")]
    Anim {
        anim: String,
        source: Box<CompositeError>,
    },

    #[error("anim '{0}' references unknown source '{1}'")]
    UnknownSource(String, String),

    #[error("offset '{slot}' must have exactly 2 values [dx, dy], got {got}")]
    BadOffset { slot: String, got: usize },
}

/// Parse a "row_col" slot address into (row, col).
pub fn parse_slot(s: &str) -> Result<(u32, u32), CompositeError> {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() != 2 {
        return Err(CompositeError::BadSlotAddress(s.to_string()));
    }
    let row = parts[0]
        .parse::<u32>()
        .map_err(|_| CompositeError::BadSlotAddress(s.to_string()))?;
    let col = parts[1]
        .parse::<u32>()
        .map_err(|_| CompositeError::BadSlotAddress(s.to_string()))?;
    Ok((row, col))
}

/// Parse a HashMap<String, Vec<i32>> of offsets into HashMap<(u32,u32), (i32,i32)>.
fn parse_offsets(
    raw: &HashMap<String, Vec<i32>>,
    rows: u32,
    cols: u32,
) -> Result<HashMap<(u32, u32), (i32, i32)>, CompositeError> {
    let mut out = HashMap::new();
    for (key, val) in raw {
        let (r, c) = parse_slot(key)?;
        if r >= rows || c >= cols {
            return Err(CompositeError::SlotOutOfBounds(
                key.clone(),
                rows,
                cols,
            ));
        }
        if val.len() != 2 {
            return Err(CompositeError::BadOffset {
                slot: key.clone(),
                got: val.len(),
            });
        }
        out.insert((r, c), (val[0], val[1]));
    }
    Ok(out)
}

/// Resolve a CompositeRaw into a Composite.
pub fn resolve_composite(
    raw: &CompositeRaw,
    name: &str,
) -> Result<Composite, CompositeError> {
    let (width, height) =
        parse_size(&raw.size).map_err(|e| CompositeError::BadSize(raw.size.clone(), e))?;
    let (tw, th) = parse_size(&raw.tile_size)
        .map_err(|e| CompositeError::BadSize(raw.tile_size.clone(), e))?;

    if width % tw != 0 || height % th != 0 {
        return Err(CompositeError::SizeNotDivisible(width, height, tw, th));
    }

    let cols = width / tw;
    let rows = height / th;

    // Parse layout grid
    let layout_rows: Vec<&str> = raw
        .layout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if layout_rows.len() as u32 != rows {
        return Err(CompositeError::RowCount {
            expected: rows,
            got: layout_rows.len() as u32,
        });
    }

    let mut slots: Vec<Vec<TileRef>> = Vec::with_capacity(rows as usize);
    for (ri, row_str) in layout_rows.iter().enumerate() {
        let tokens: Vec<&str> = row_str.split_whitespace().collect();
        if tokens.len() as u32 != cols {
            return Err(CompositeError::ColCount {
                row: ri,
                expected: cols,
                got: tokens.len() as u32,
            });
        }
        let row: Vec<TileRef> = tokens.iter().map(|t| TileRef::parse(t)).collect();
        slots.push(row);
    }

    // Parse base offsets
    let offsets = parse_offsets(&raw.offset, rows, cols)?;

    // Resolve variants
    let mut variants: HashMap<String, HashMap<(u32, u32), TileRef>> = HashMap::new();
    for (vname, vraw) in &raw.variant {
        let mut slot_overrides = HashMap::new();
        for (key, tile_str) in &vraw.slot {
            let (r, c) = parse_slot(key).map_err(|e| CompositeError::Variant {
                variant: vname.clone(),
                source: Box::new(e),
            })?;
            if r >= rows || c >= cols {
                return Err(CompositeError::Variant {
                    variant: vname.clone(),
                    source: Box::new(CompositeError::SlotOutOfBounds(
                        key.clone(),
                        rows,
                        cols,
                    )),
                });
            }
            slot_overrides.insert((r, c), TileRef::parse(tile_str));
        }
        variants.insert(vname.clone(), slot_overrides);
    }

    // Resolve animations
    let mut animations: HashMap<String, CompositeAnim> = HashMap::new();
    for (aname, araw) in &raw.anim {
        let anim = resolve_anim(araw, aname, rows, cols)?;
        animations.insert(aname.clone(), anim);
    }

    // Resolve derived animations (source + mirror)
    let source_keys: Vec<(String, String)> = raw
        .anim
        .iter()
        .filter_map(|(aname, araw)| {
            araw.source
                .as_ref()
                .map(|src| (aname.clone(), src.clone()))
        })
        .collect();

    for (aname, src_name) in &source_keys {
        if !animations.contains_key(src_name) {
            return Err(CompositeError::UnknownSource(
                aname.clone(),
                src_name.clone(),
            ));
        }
        let source_anim = animations[src_name].clone();
        let derived = animations.get_mut(aname).unwrap();
        if derived.frames.is_empty() {
            derived.frames = source_anim.frames;
        }
    }

    Ok(Composite {
        name: name.to_string(),
        width,
        height,
        tile_width: tw,
        tile_height: th,
        cols,
        rows,
        slots,
        offsets,
        variants,
        animations,
    })
}

fn resolve_anim(
    raw: &CompositeAnimRaw,
    name: &str,
    rows: u32,
    cols: u32,
) -> Result<CompositeAnim, CompositeError> {
    let mut frames = Vec::new();
    for fraw in &raw.frame {
        let mut swaps = HashMap::new();
        for (key, tile_str) in &fraw.swap {
            let (r, c) = parse_slot(key).map_err(|e| CompositeError::Anim {
                anim: name.to_string(),
                source: Box::new(e),
            })?;
            if r >= rows || c >= cols {
                return Err(CompositeError::Anim {
                    anim: name.to_string(),
                    source: Box::new(CompositeError::SlotOutOfBounds(
                        key.clone(),
                        rows,
                        cols,
                    )),
                });
            }
            swaps.insert((r, c), TileRef::parse(tile_str));
        }
        let offsets = parse_offsets(&fraw.offset, rows, cols).map_err(|e| {
            CompositeError::Anim {
                anim: name.to_string(),
                source: Box::new(e),
            }
        })?;
        frames.push(CompositeFrame {
            index: fraw.index,
            swaps,
            offsets,
        });
    }

    Ok(CompositeAnim {
        fps: raw.fps,
        loop_mode: raw.r#loop,
        mirror: raw.mirror.clone(),
        source: raw.source.clone(),
        frames,
    })
}

/// Flip a grid vertically (mirror top-bottom).
fn flip_grid_v(grid: &[Vec<char>]) -> Vec<Vec<char>> {
    grid.iter().rev().cloned().collect()
}

/// Apply TileRef flip flags to a grid (public for seam checking).
pub fn apply_flips_pub(grid: &[Vec<char>], tile_ref: &TileRef) -> Vec<Vec<char>> {
    apply_flips(grid, tile_ref)
}

/// Apply TileRef flip flags to a grid.
fn apply_flips(grid: &[Vec<char>], tile_ref: &TileRef) -> Vec<Vec<char>> {
    let mut g = grid.to_vec();
    if tile_ref.flip_d {
        // Diagonal flip = transpose
        let h = g.len();
        let w = if h > 0 { g[0].len() } else { 0 };
        let mut transposed = vec![vec!['.'; h]; w];
        for y in 0..h {
            for x in 0..w {
                transposed[x][y] = g[y][x];
            }
        }
        g = transposed;
    }
    if tile_ref.flip_h {
        g = flip_grid_h(&g);
    }
    if tile_ref.flip_v {
        g = flip_grid_v(&g);
    }
    g
}

/// Apply TileModifier to a resolved grid (modifies palette symbols — for now, pass-through).
/// Modifier effects (shadow/highlight) are applied at render time on RGBA values,
/// not at the character grid level. This function is a no-op placeholder.
fn apply_modifier(_grid: &[Vec<char>], _modifier: TileModifier) -> Vec<Vec<char>> {
    // Modifier application happens in the renderer, not here
    _grid.to_vec()
}

/// Compose a composite into a full character grid.
///
/// Resolves the base layout, applies optional variant overrides and animation
/// frame swaps, then blits each tile onto the output canvas with offsets.
pub fn compose_grid(
    composite: &Composite,
    variant: Option<&str>,
    frame_index: Option<u32>,
    tiles: &HashMap<String, Tile>,
    void_sym: char,
) -> Result<Vec<Vec<char>>, CompositeError> {
    let w = composite.width as usize;
    let h = composite.height as usize;
    let mut canvas = vec![vec![void_sym; w]; h];

    // Build effective slot map: base → variant overrides → frame swaps
    let mut effective_slots: Vec<Vec<TileRef>> = composite.slots.clone();

    if let Some(vname) = variant {
        if let Some(overrides) = composite.variants.get(vname) {
            for (&(r, c), tile_ref) in overrides {
                effective_slots[r as usize][c as usize] = tile_ref.clone();
            }
        }
    }

    // Find the active animation frame
    let active_frame = frame_index.and_then(|fi| {
        // Check all animations for matching frame index in the right order
        // When variant is set, prefer animation with matching name, else use first match
        composite
            .animations
            .values()
            .flat_map(|a| a.frames.iter())
            .find(|f| f.index == fi)
    });

    if let Some(frame) = active_frame {
        for (&(r, c), tile_ref) in &frame.swaps {
            effective_slots[r as usize][c as usize] = tile_ref.clone();
        }
    }

    // Blit each slot onto the canvas
    for r in 0..composite.rows {
        for c in 0..composite.cols {
            let tile_ref = &effective_slots[r as usize][c as usize];

            // Skip void slots
            if tile_ref.name == "_" {
                continue;
            }

            let tile = tiles
                .get(&tile_ref.name)
                .ok_or_else(|| CompositeError::TileNotFound(tile_ref.name.clone()))?;

            // Validate tile dimensions
            if tile.width != composite.tile_width || tile.height != composite.tile_height {
                return Err(CompositeError::TileSizeMismatch {
                    name: tile_ref.name.clone(),
                    exp_w: composite.tile_width,
                    exp_h: composite.tile_height,
                    got_w: tile.width,
                    got_h: tile.height,
                });
            }

            // Apply flips
            let grid = apply_flips(&tile.grid, tile_ref);

            // Determine offset
            let (dx, dy) = active_frame
                .and_then(|f| f.offsets.get(&(r, c)))
                .or_else(|| composite.offsets.get(&(r, c)))
                .copied()
                .unwrap_or((0, 0));

            // Blit onto canvas
            let base_x = (c * composite.tile_width) as i32 + dx;
            let base_y = (r * composite.tile_height) as i32 + dy;

            for ty in 0..composite.tile_height {
                for tx in 0..composite.tile_width {
                    let cx = base_x + tx as i32;
                    let cy = base_y + ty as i32;

                    if cx < 0 || cy < 0 || cx >= w as i32 || cy >= h as i32 {
                        continue; // clipped
                    }

                    let sym = grid[ty as usize][tx as usize];
                    if sym != void_sym {
                        canvas[cy as usize][cx as usize] = sym;
                    }
                }
            }
        }
    }

    // Apply mirror for derived animations
    let should_mirror = frame_index.is_some()
        && composite
            .animations
            .values()
            .any(|a| a.mirror.as_deref() == Some("h"));

    if should_mirror {
        canvas = flip_grid_h(&canvas);
    }

    Ok(canvas)
}

/// Compose a specific animation frame, looking up by animation name.
pub fn compose_anim_frame(
    composite: &Composite,
    anim_name: &str,
    frame_index: u32,
    variant: Option<&str>,
    tiles: &HashMap<String, Tile>,
    void_sym: char,
) -> Result<Vec<Vec<char>>, CompositeError> {
    let w = composite.width as usize;
    let h = composite.height as usize;
    let mut canvas = vec![vec![void_sym; w]; h];

    let anim = composite.animations.get(anim_name);

    // Build effective slot map
    let mut effective_slots: Vec<Vec<TileRef>> = composite.slots.clone();

    if let Some(vname) = variant {
        if let Some(overrides) = composite.variants.get(vname) {
            for (&(r, c), tile_ref) in overrides {
                effective_slots[r as usize][c as usize] = tile_ref.clone();
            }
        }
    }

    let active_frame = anim.and_then(|a| a.frames.iter().find(|f| f.index == frame_index));

    if let Some(frame) = active_frame {
        for (&(r, c), tile_ref) in &frame.swaps {
            effective_slots[r as usize][c as usize] = tile_ref.clone();
        }
    }

    // Blit each slot
    for r in 0..composite.rows {
        for c in 0..composite.cols {
            let tile_ref = &effective_slots[r as usize][c as usize];
            if tile_ref.name == "_" {
                continue;
            }

            let tile = tiles
                .get(&tile_ref.name)
                .ok_or_else(|| CompositeError::TileNotFound(tile_ref.name.clone()))?;

            if tile.width != composite.tile_width || tile.height != composite.tile_height {
                return Err(CompositeError::TileSizeMismatch {
                    name: tile_ref.name.clone(),
                    exp_w: composite.tile_width,
                    exp_h: composite.tile_height,
                    got_w: tile.width,
                    got_h: tile.height,
                });
            }

            let grid = apply_flips(&tile.grid, tile_ref);

            let (dx, dy) = active_frame
                .and_then(|f| f.offsets.get(&(r, c)))
                .or_else(|| composite.offsets.get(&(r, c)))
                .copied()
                .unwrap_or((0, 0));

            let base_x = (c * composite.tile_width) as i32 + dx;
            let base_y = (r * composite.tile_height) as i32 + dy;

            for ty in 0..composite.tile_height {
                for tx in 0..composite.tile_width {
                    let cx = base_x + tx as i32;
                    let cy = base_y + ty as i32;
                    if cx < 0 || cy < 0 || cx >= w as i32 || cy >= h as i32 {
                        continue;
                    }
                    let sym = grid[ty as usize][tx as usize];
                    if sym != void_sym {
                        canvas[cy as usize][cx as usize] = sym;
                    }
                }
            }
        }
    }

    // Apply animation-level mirror
    if let Some(a) = anim {
        if a.mirror.as_deref() == Some("h") {
            canvas = flip_grid_h(&canvas);
        } else if a.mirror.as_deref() == Some("v") {
            canvas = flip_grid_v(&canvas);
        }
    }

    Ok(canvas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EdgeClass, Encoding, Symmetry, AutoRotate, Semantic};

    fn make_tile(name: &str, w: u32, h: u32, fill: char) -> Tile {
        Tile {
            name: name.to_string(),
            palette: "test".to_string(),
            width: w,
            height: h,
            encoding: Encoding::Grid,
            symmetry: Symmetry::None,
            auto_rotate: AutoRotate::None,
            edge_class: EdgeClass {
                n: "open".to_string(),
                e: "open".to_string(),
                s: "open".to_string(),
                w: "open".to_string(),
            },
            tags: vec![],
            target_layer: None,
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: None,
            grid: vec![vec![fill; w as usize]; h as usize],
        }
    }

    fn make_tile_with_grid(name: &str, grid: Vec<Vec<char>>) -> Tile {
        let h = grid.len() as u32;
        let w = if h > 0 { grid[0].len() as u32 } else { 0 };
        let mut t = make_tile(name, w, h, '.');
        t.grid = grid;
        t
    }

    #[test]
    fn resolve_basic_2x2() {
        let raw = CompositeRaw {
            size: "4x4".to_string(),
            tile_size: "2x2".to_string(),
            layout: "a b\nc d".to_string(),
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };

        let comp = resolve_composite(&raw, "test").unwrap();
        assert_eq!(comp.rows, 2);
        assert_eq!(comp.cols, 2);
        assert_eq!(comp.slots[0][0].name, "a");
        assert_eq!(comp.slots[0][1].name, "b");
        assert_eq!(comp.slots[1][0].name, "c");
        assert_eq!(comp.slots[1][1].name, "d");
    }

    #[test]
    fn resolve_with_flip_flags() {
        let raw = CompositeRaw {
            size: "4x4".to_string(),
            tile_size: "2x2".to_string(),
            layout: "head_l head_l!h\nbody_l body_r".to_string(),
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };

        let comp = resolve_composite(&raw, "test").unwrap();
        assert_eq!(comp.slots[0][1].name, "head_l");
        assert!(comp.slots[0][1].flip_h);
    }

    #[test]
    fn compose_grid_basic() {
        let raw = CompositeRaw {
            size: "4x4".to_string(),
            tile_size: "2x2".to_string(),
            layout: "tl tr\nbl br".to_string(),
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };
        let comp = resolve_composite(&raw, "test").unwrap();

        let mut tiles = HashMap::new();
        tiles.insert("tl".to_string(), make_tile("tl", 2, 2, 'A'));
        tiles.insert("tr".to_string(), make_tile("tr", 2, 2, 'B'));
        tiles.insert("bl".to_string(), make_tile("bl", 2, 2, 'C'));
        tiles.insert("br".to_string(), make_tile("br", 2, 2, 'D'));

        let grid = compose_grid(&comp, None, None, &tiles, '.').unwrap();
        assert_eq!(grid[0], vec!['A', 'A', 'B', 'B']);
        assert_eq!(grid[1], vec!['A', 'A', 'B', 'B']);
        assert_eq!(grid[2], vec!['C', 'C', 'D', 'D']);
        assert_eq!(grid[3], vec!['C', 'C', 'D', 'D']);
    }

    #[test]
    fn compose_with_flip() {
        let raw = CompositeRaw {
            size: "4x2".to_string(),
            tile_size: "2x2".to_string(),
            layout: "tile tile!h".to_string(),
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };
        let comp = resolve_composite(&raw, "test").unwrap();

        let grid_data = vec![vec!['1', '2'], vec!['3', '4']];
        let mut tiles = HashMap::new();
        tiles.insert("tile".to_string(), make_tile_with_grid("tile", grid_data));

        let grid = compose_grid(&comp, None, None, &tiles, '.').unwrap();
        // Left tile: 12 / 34
        // Right tile (flipped h): 21 / 43
        assert_eq!(grid[0], vec!['1', '2', '2', '1']);
        assert_eq!(grid[1], vec!['3', '4', '4', '3']);
    }

    #[test]
    fn compose_with_void_slot() {
        let raw = CompositeRaw {
            size: "4x4".to_string(),
            tile_size: "2x2".to_string(),
            layout: "a _\n_ a".to_string(),
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };
        let comp = resolve_composite(&raw, "test").unwrap();

        let mut tiles = HashMap::new();
        tiles.insert("a".to_string(), make_tile("a", 2, 2, '#'));

        let grid = compose_grid(&comp, None, None, &tiles, '.').unwrap();
        assert_eq!(grid[0], vec!['#', '#', '.', '.']);
        assert_eq!(grid[1], vec!['#', '#', '.', '.']);
        assert_eq!(grid[2], vec!['.', '.', '#', '#']);
        assert_eq!(grid[3], vec!['.', '.', '#', '#']);
    }

    #[test]
    fn compose_with_variant() {
        let mut variant_map = HashMap::new();
        let mut slot_map = HashMap::new();
        slot_map.insert("0_0".to_string(), "alt".to_string());
        variant_map.insert(
            "v1".to_string(),
            crate::types::CompositeVariantRaw { slot: slot_map },
        );

        let raw = CompositeRaw {
            size: "4x2".to_string(),
            tile_size: "2x2".to_string(),
            layout: "a b".to_string(),
            offset: HashMap::new(),
            variant: variant_map,
            anim: HashMap::new(),
        };
        let comp = resolve_composite(&raw, "test").unwrap();

        let mut tiles = HashMap::new();
        tiles.insert("a".to_string(), make_tile("a", 2, 2, 'A'));
        tiles.insert("b".to_string(), make_tile("b", 2, 2, 'B'));
        tiles.insert("alt".to_string(), make_tile("alt", 2, 2, 'X'));

        let grid = compose_grid(&comp, Some("v1"), None, &tiles, '.').unwrap();
        assert_eq!(grid[0], vec!['X', 'X', 'B', 'B']);
        assert_eq!(grid[1], vec!['X', 'X', 'B', 'B']);
    }

    #[test]
    fn compose_with_offset() {
        let mut offsets = HashMap::new();
        offsets.insert("0_0".to_string(), vec![1, 0]);

        let raw = CompositeRaw {
            size: "4x2".to_string(),
            tile_size: "2x2".to_string(),
            layout: "a b".to_string(),
            offset: offsets,
            variant: HashMap::new(),
            anim: HashMap::new(),
        };
        let comp = resolve_composite(&raw, "test").unwrap();

        let mut tiles = HashMap::new();
        tiles.insert("a".to_string(), make_tile("a", 2, 2, 'A'));
        tiles.insert("b".to_string(), make_tile("b", 2, 2, 'B'));

        let grid = compose_grid(&comp, None, None, &tiles, '.').unwrap();
        // 'a' shifted right by 1px: col 1-2 instead of 0-1
        // 'b' at normal position: col 2-3
        // 'b' overwrites 'a' at col 2
        assert_eq!(grid[0], vec!['.', 'A', 'B', 'B']);
        assert_eq!(grid[1], vec!['.', 'A', 'B', 'B']);
    }

    #[test]
    fn bad_size_not_divisible() {
        let raw = CompositeRaw {
            size: "5x5".to_string(),
            tile_size: "2x2".to_string(),
            layout: "a b\nc d".to_string(),
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };
        let err = resolve_composite(&raw, "test").unwrap_err();
        assert!(matches!(err, CompositeError::SizeNotDivisible(..)));
    }

    #[test]
    fn bad_layout_row_count() {
        let raw = CompositeRaw {
            size: "4x4".to_string(),
            tile_size: "2x2".to_string(),
            layout: "a b".to_string(), // only 1 row, need 2
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };
        let err = resolve_composite(&raw, "test").unwrap_err();
        assert!(matches!(err, CompositeError::RowCount { expected: 2, got: 1 }));
    }

    #[test]
    fn parse_slot_valid() {
        assert_eq!(parse_slot("0_1").unwrap(), (0, 1));
        assert_eq!(parse_slot("3_2").unwrap(), (3, 2));
    }

    #[test]
    fn parse_slot_invalid() {
        assert!(parse_slot("0").is_err());
        assert!(parse_slot("a_b").is_err());
        assert!(parse_slot("0_1_2").is_err());
    }
}
