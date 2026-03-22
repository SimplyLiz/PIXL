use crate::grid::{parse_grid, GridError};
use crate::parser::{resolve_all_palettes, ParseError};
use crate::rle::{parse_rle, RleError};
use crate::template::{validate_templates, TemplateError};
use crate::theme::{resolve_theme, ThemeError};
use crate::types::{PaxFile, parse_size};
use thiserror::Error;

#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub stats: ValidationStats,
}

#[derive(Debug, Default)]
pub struct ValidationStats {
    pub palettes: usize,
    pub themes: usize,
    pub stamps: usize,
    pub tiles: usize,
    pub sprites: usize,
    pub objects: usize,
    pub tile_runs: usize,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("{0}")]
    Parse(#[from] ParseError),

    #[error("tile '{tile}': {source}")]
    Grid { tile: String, source: GridError },

    #[error("tile '{tile}': {source}")]
    Rle { tile: String, source: RleError },

    #[error("{0}")]
    Template(#[from] TemplateError),

    #[error("{0}")]
    Theme(#[from] ThemeError),

    #[error("tile '{tile}': no size declared and no template to inherit from")]
    NoSize { tile: String },

    #[error("tile '{tile}': no pixel data (grid/rle/layout) and no template")]
    NoPixelData { tile: String },

    #[error("tile '{tile}': uses {count} symbols but theme max_palette_size = {max}. Excess: {excess}")]
    PaletteSizeExceeded {
        tile: String,
        count: usize,
        max: u32,
        excess: String,
    },

    #[error("stamp '{stamp}': {source}")]
    StampGrid { stamp: String, source: GridError },

    #[error("stamp '{stamp}': invalid size '{size}'")]
    StampSize { stamp: String, size: String },

    #[error("tile '{tile}': invalid size '{size}'")]
    TileSize { tile: String, size: String },

    #[error("tile '{tile}': auto_rotate requires square tiles (width == height), got {w}x{h}")]
    RotateNonSquare { tile: String, w: u32, h: u32 },

    #[error("tile_run '{name}': {side}.edge_class.{dir} = '{got}', expected '{expected}' (must match middle)")]
    TileRunEdgeMismatch {
        name: String,
        side: String,
        dir: String,
        got: String,
        expected: String,
    },

    #[error("sprite '{spriteset}/{sprite}': frame {index}: delta base {base} must be < {index} and Grid-encoded")]
    InvalidDeltaBase {
        spriteset: String,
        sprite: String,
        index: u32,
        base: u32,
    },

    #[error("sprite '{spriteset}/{sprite}': frame indices not contiguous starting at 1")]
    NonContiguousFrames {
        spriteset: String,
        sprite: String,
    },
}

/// Validate an entire PaxFile. Returns all errors and warnings.
pub fn validate(file: &PaxFile, _check_edges: bool) -> ValidationResult {
    let mut errors: Vec<ValidationError> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    // 1. Resolve palettes
    let palettes = match resolve_all_palettes(file) {
        Ok(p) => p,
        Err(e) => {
            errors.push(e.into());
            return ValidationResult {
                errors,
                warnings,
                stats: ValidationStats::default(),
            };
        }
    };

    // 2. Validate templates
    for err in validate_templates(&file.tile) {
        errors.push(err.into());
    }

    // 3. Validate themes
    for name in file.theme.keys() {
        match resolve_theme(name, &file.theme, &palettes) {
            Ok(resolved) => {
                // Evaluate constraints as warnings
                let theme = &file.theme[name];
                for w in crate::theme::evaluate_constraints(theme, &resolved, &palettes[&resolved.palette]) {
                    warnings.push(format!("{}", w));
                }
            }
            Err(e) => errors.push(e.into()),
        }
    }

    // 4. Validate stamps
    for (name, stamp_raw) in &file.stamp {
        let (w, h) = match parse_size(&stamp_raw.size) {
            Ok(s) => s,
            Err(_) => {
                errors.push(ValidationError::StampSize {
                    stamp: name.clone(),
                    size: stamp_raw.size.clone(),
                });
                continue;
            }
        };

        if let Some(palette) = palettes.get(&stamp_raw.palette) {
            if let Err(e) = parse_grid(&stamp_raw.grid, w, h, palette) {
                errors.push(ValidationError::StampGrid {
                    stamp: name.clone(),
                    source: e,
                });
            }
        }
    }

    // 5. Validate tiles
    let active_theme_name = file.pax.theme.as_deref();
    let max_palette_size = active_theme_name
        .and_then(|t| file.theme.get(t))
        .and_then(|t| t.max_palette_size);

    for (name, tile_raw) in &file.tile {
        // Skip template tiles — they inherit from base
        if tile_raw.template.is_some() {
            continue;
        }

        // Must have size
        let Some(ref size_str) = tile_raw.size else {
            errors.push(ValidationError::NoSize { tile: name.clone() });
            continue;
        };

        let (w, h) = match parse_size(size_str) {
            Ok(s) => s,
            Err(_) => {
                errors.push(ValidationError::TileSize {
                    tile: name.clone(),
                    size: size_str.clone(),
                });
                continue;
            }
        };

        // Must have pixel data
        if tile_raw.grid.is_none() && tile_raw.rle.is_none() && tile_raw.layout.is_none() {
            errors.push(ValidationError::NoPixelData { tile: name.clone() });
            continue;
        }

        // Validate grid encoding
        if let Some(ref grid_str) = tile_raw.grid {
            if let Some(palette) = palettes.get(&tile_raw.palette) {
                // Account for symmetry — grid may be half/quarter size
                let (grid_w, grid_h) = match tile_raw.symmetry.as_deref() {
                    Some("horizontal") => (w / 2, h),
                    Some("vertical") => (w, h / 2),
                    Some("quad") => (w / 2, h / 2),
                    _ => (w, h),
                };

                match parse_grid(grid_str, grid_w, grid_h, palette) {
                    Ok(grid) => {
                        // Check max_palette_size
                        if let Some(max) = max_palette_size {
                            let unique: std::collections::HashSet<char> =
                                grid.iter().flat_map(|r| r.iter()).copied().collect();
                            if unique.len() > max as usize {
                                let excess: Vec<String> =
                                    unique.iter().map(|c| c.to_string()).collect();
                                errors.push(ValidationError::PaletteSizeExceeded {
                                    tile: name.clone(),
                                    count: unique.len(),
                                    max,
                                    excess: excess.join(", "),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(ValidationError::Grid {
                            tile: name.clone(),
                            source: e,
                        });
                    }
                }
            }
        }

        // Validate RLE encoding
        if let Some(ref rle_str) = tile_raw.rle {
            if let Some(palette) = palettes.get(&tile_raw.palette) {
                if let Err(e) = parse_rle(rle_str, w, h, palette) {
                    errors.push(ValidationError::Rle {
                        tile: name.clone(),
                        source: e,
                    });
                }
            }
        }

        // Validate auto_rotate on non-square
        if tile_raw.auto_rotate.is_some()
            && tile_raw.auto_rotate.as_deref() != Some("none")
            && w != h
        {
            errors.push(ValidationError::RotateNonSquare {
                tile: name.clone(),
                w,
                h,
            });
        }
    }

    // 6. Validate tile runs — edge compatibility
    for (name, run) in &file.tile_run {
        if let (Some(left), Some(mid), Some(right)) = (
            file.tile.get(&run.left),
            file.tile.get(&run.middle),
            file.tile.get(&run.right),
        ) {
            // left.e must match middle.w
            if let (Some(le), Some(mw)) = (&left.edge_class, &mid.edge_class) {
                if le.e != mw.w {
                    errors.push(ValidationError::TileRunEdgeMismatch {
                        name: name.clone(),
                        side: "left".to_string(),
                        dir: "e/w".to_string(),
                        got: le.e.clone(),
                        expected: mw.w.clone(),
                    });
                }
            }
            // middle.e must match middle.w (self-repeating)
            if let Some(me) = &mid.edge_class {
                if me.e != me.w {
                    errors.push(ValidationError::TileRunEdgeMismatch {
                        name: name.clone(),
                        side: "middle".to_string(),
                        dir: "e/w self-repeat".to_string(),
                        got: me.e.clone(),
                        expected: me.w.clone(),
                    });
                }
            }
            // middle.e must match right.w
            if let (Some(me), Some(rw)) = (&mid.edge_class, &right.edge_class) {
                if me.e != rw.w {
                    errors.push(ValidationError::TileRunEdgeMismatch {
                        name: name.clone(),
                        side: "right".to_string(),
                        dir: "e/w".to_string(),
                        got: me.e.clone(),
                        expected: rw.w.clone(),
                    });
                }
            }
        }
    }

    // 7. Validate spritesets — frame contiguity and delta bases
    for (ss_name, spriteset) in &file.spriteset {
        for sprite in &spriteset.sprite {
            // Check frame indices are contiguous from 1
            let indices: Vec<u32> = sprite.frames.iter().map(|f| f.index).collect();
            let expected: Vec<u32> = (1..=sprite.frames.len() as u32).collect();
            if indices != expected {
                errors.push(ValidationError::NonContiguousFrames {
                    spriteset: ss_name.clone(),
                    sprite: sprite.name.clone(),
                });
            }

            // Check delta bases
            for frame in &sprite.frames {
                if frame.encoding.as_deref() == Some("delta") {
                    if let Some(base) = frame.base {
                        if base >= frame.index {
                            errors.push(ValidationError::InvalidDeltaBase {
                                spriteset: ss_name.clone(),
                                sprite: sprite.name.clone(),
                                index: frame.index,
                                base,
                            });
                        }
                        // Check base is grid-encoded
                        if let Some(base_frame) = sprite.frames.iter().find(|f| f.index == base) {
                            if base_frame.encoding.as_deref() != Some("grid")
                                && base_frame.encoding.is_some()
                            {
                                errors.push(ValidationError::InvalidDeltaBase {
                                    spriteset: ss_name.clone(),
                                    sprite: sprite.name.clone(),
                                    index: frame.index,
                                    base,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 8. Validate cycles
    for (name, cycle) in &file.cycle {
        for err in crate::cycle::validate_cycle(name, cycle, &palettes) {
            warnings.push(format!("{}", err));
        }
    }

    let stats = ValidationStats {
        palettes: file.palette.len(),
        themes: file.theme.len(),
        stamps: file.stamp.len(),
        tiles: file.tile.len(),
        sprites: file.spriteset.values().map(|ss| ss.sprite.len()).sum(),
        objects: file.object.len(),
        tile_runs: file.tile_run.len(),
    };

    ValidationResult {
        errors,
        warnings,
        stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_pax;

    #[test]
    fn validate_dungeon_example() {
        let source =
            std::fs::read_to_string("../../examples/dungeon.pax").expect("dungeon.pax should exist");
        let file = parse_pax(&source).unwrap();
        let result = validate(&file, false);

        if !result.errors.is_empty() {
            for e in &result.errors {
                eprintln!("ERROR: {}", e);
            }
        }
        assert!(
            result.errors.is_empty(),
            "dungeon.pax should validate without errors"
        );

        assert!(result.stats.palettes >= 1);
        assert!(result.stats.themes >= 1);
        assert!(result.stats.tiles >= 5);
        assert!(result.stats.stamps >= 1);
    }

    #[test]
    fn validates_palette_size_limit() {
        // Theme with max_palette_size = 3, tile using 4 symbols
        let source = concat!(
            "[pax]\nversion = \"2.0\"\nname = \"test\"\ntheme = \"t\"\n",
            "[theme.t]\npalette = \"p\"\nmax_palette_size = 3\n",
            "[palette.p]\n\".\" = \"#00000000\"\n",
            "\"#\" = \"#2a1f3d\"\n\"+\" = \"#4a3a6d\"\n\"~\" = \"#1a3a5c\"\n",
            "[tile.bad]\npalette = \"p\"\nsize = \"4x1\"\ngrid = \".#+~\"\n",
        );
        let file = parse_pax(source).unwrap();
        let result = validate(&file, false);
        assert!(
            result.errors.iter().any(|e| matches!(e, ValidationError::PaletteSizeExceeded { .. })),
            "should catch palette size exceeded"
        );
    }
}
