//! Sprite animation frame resolution.
//!
//! Resolves raw spriteset frames (grid, delta, linked, mirror) into
//! fully materialized pixel grids ready for rendering.

use crate::cycle;
use crate::grid;
use crate::types::{Cycle, FrameRaw, Palette, SpriteRaw, SpritesetRaw};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnimError {
    #[error("frame {index}: {reason}")]
    FrameError { index: u32, reason: String },

    #[error("sprite '{name}' has no frames")]
    NoFrames { name: String },

    #[error("spriteset '{name}' not found")]
    SpritesetNotFound { name: String },

    #[error("sprite '{name}' not found in spriteset '{spriteset}'")]
    SpriteNotFound { name: String, spriteset: String },

    #[error("palette '{name}' not found")]
    PaletteNotFound { name: String },
}

/// A fully resolved animation frame.
#[derive(Debug, Clone)]
pub struct ResolvedFrame {
    pub index: u32,
    pub duration_ms: u32,
    pub grid: Vec<Vec<char>>,
}

/// Resolve all frames of a sprite into materialized grids.
pub fn resolve_sprite_frames(
    sprite: &SpriteRaw,
    width: u32,
    height: u32,
    palette: &Palette,
    default_fps: u32,
) -> Result<Vec<ResolvedFrame>, AnimError> {
    if sprite.frames.is_empty() {
        return Err(AnimError::NoFrames {
            name: sprite.name.clone(),
        });
    }

    let frame_duration_ms = 1000 / default_fps.max(1);
    let mut resolved: Vec<ResolvedFrame> = Vec::new();
    let mut base_grids: HashMap<u32, Vec<Vec<char>>> = HashMap::new();

    for frame in &sprite.frames {
        let encoding = frame.encoding.as_deref().unwrap_or("grid");
        let grid = match encoding {
            "grid" => resolve_grid_frame(frame, width, height, palette)?,
            "delta" => resolve_delta_frame(frame, &base_grids)?,
            "linked" => resolve_linked_frame(frame, &base_grids)?,
            other => {
                return Err(AnimError::FrameError {
                    index: frame.index,
                    reason: format!("unknown encoding '{other}'"),
                });
            }
        };

        // Apply mirror if specified
        let grid = apply_mirror(&grid, frame.mirror.as_deref());

        base_grids.insert(frame.index, grid.clone());

        let duration = frame.duration_ms.unwrap_or(frame_duration_ms);
        resolved.push(ResolvedFrame {
            index: frame.index,
            duration_ms: duration,
            grid,
        });
    }

    Ok(resolved)
}

/// Resolve frames with color cycling applied at a given tick.
pub fn resolve_frames_with_cycles(
    frames: &[ResolvedFrame],
    cycles: &[&Cycle],
    palette: &Palette,
    tick: u64,
) -> Vec<ResolvedFrame> {
    if cycles.is_empty() {
        return frames.to_vec();
    }

    frames
        .iter()
        .map(|frame| {
            let grid = frame
                .grid
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|&ch| {
                            // Check each cycle for this symbol
                            for c in cycles {
                                if let Some(rgba) =
                                    cycle::cycle_color_at_frame(ch, c, palette, tick)
                                {
                                    // Find which symbol has this color
                                    for (&sym, &col) in &palette.symbols {
                                        if col == rgba {
                                            return sym;
                                        }
                                    }
                                }
                            }
                            ch
                        })
                        .collect()
                })
                .collect();
            ResolvedFrame {
                index: frame.index,
                duration_ms: frame.duration_ms,
                grid,
            }
        })
        .collect()
}

fn resolve_grid_frame(
    frame: &FrameRaw,
    width: u32,
    height: u32,
    palette: &Palette,
) -> Result<Vec<Vec<char>>, AnimError> {
    let grid_str = frame.grid.as_deref().ok_or_else(|| AnimError::FrameError {
        index: frame.index,
        reason: "grid frame missing grid data".to_string(),
    })?;

    grid::parse_grid(grid_str, width, height, palette).map_err(|e| AnimError::FrameError {
        index: frame.index,
        reason: format!("grid parse error: {e}"),
    })
}

fn resolve_delta_frame(
    frame: &FrameRaw,
    base_grids: &HashMap<u32, Vec<Vec<char>>>,
) -> Result<Vec<Vec<char>>, AnimError> {
    let base_idx = frame.base.unwrap_or(1);
    let base_grid = base_grids
        .get(&base_idx)
        .ok_or_else(|| AnimError::FrameError {
            index: frame.index,
            reason: format!("delta base frame {base_idx} not found"),
        })?;

    let mut grid = base_grid.clone();
    for change in &frame.changes {
        let ch = change.sym.chars().next().unwrap_or('.');
        let y = change.y as usize;
        if y < grid.len() && (change.x as usize) < grid[y].len() {
            grid[y][change.x as usize] = ch;
        }
    }

    Ok(grid)
}

fn resolve_linked_frame(
    frame: &FrameRaw,
    base_grids: &HashMap<u32, Vec<Vec<char>>>,
) -> Result<Vec<Vec<char>>, AnimError> {
    let link_idx = frame.link_to.unwrap_or(1);
    base_grids
        .get(&link_idx)
        .cloned()
        .ok_or_else(|| AnimError::FrameError {
            index: frame.index,
            reason: format!("linked frame {link_idx} not found"),
        })
}

fn apply_mirror(grid: &[Vec<char>], mirror: Option<&str>) -> Vec<Vec<char>> {
    match mirror {
        Some("h") => grid
            .iter()
            .map(|row| row.iter().rev().copied().collect())
            .collect(),
        Some("v") => grid.iter().rev().cloned().collect(),
        Some("hv") | Some("vh") => grid
            .iter()
            .rev()
            .map(|row| row.iter().rev().copied().collect())
            .collect(),
        _ => grid.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DeltaChange, Rgba};

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

    #[test]
    fn resolve_grid_frame_works() {
        let frame = FrameRaw {
            index: 1,
            encoding: Some("grid".to_string()),
            grid: Some("##\n++".to_string()),
            base: None,
            changes: vec![],
            link_to: None,
            duration_ms: Some(100),
            mirror: None,
        };

        let sprite = SpriteRaw {
            name: "test".to_string(),
            fps: 8,
            r#loop: true,
            tags: vec![],
            frames: vec![frame],
            scale: None,
        };

        let frames = resolve_sprite_frames(&sprite, 2, 2, &test_palette(), 8).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].grid, vec![vec!['#', '#'], vec!['+', '+']]);
        assert_eq!(frames[0].duration_ms, 100);
    }

    #[test]
    fn resolve_delta_frame_applies_changes() {
        let frame1 = FrameRaw {
            index: 1,
            encoding: Some("grid".to_string()),
            grid: Some("##\n##".to_string()),
            base: None,
            changes: vec![],
            link_to: None,
            duration_ms: None,
            mirror: None,
        };
        let frame2 = FrameRaw {
            index: 2,
            encoding: Some("delta".to_string()),
            grid: None,
            base: Some(1),
            changes: vec![DeltaChange {
                x: 0,
                y: 0,
                sym: "+".to_string(),
            }],
            link_to: None,
            duration_ms: None,
            mirror: None,
        };

        let sprite = SpriteRaw {
            name: "test".to_string(),
            fps: 8,
            r#loop: true,
            tags: vec![],
            frames: vec![frame1, frame2],
            scale: None,
        };

        let frames = resolve_sprite_frames(&sprite, 2, 2, &test_palette(), 8).unwrap();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].grid[0][0], '+');
        assert_eq!(frames[1].grid[0][1], '#');
    }

    #[test]
    fn mirror_horizontal() {
        let grid = vec![vec!['#', '+'], vec!['.', '#']];
        let mirrored = apply_mirror(&grid, Some("h"));
        assert_eq!(mirrored, vec![vec!['+', '#'], vec!['#', '.']]);
    }

    #[test]
    fn mirror_vertical() {
        let grid = vec![vec!['#', '+'], vec!['.', '#']];
        let mirrored = apply_mirror(&grid, Some("v"));
        assert_eq!(mirrored, vec![vec!['.', '#'], vec!['#', '+']]);
    }

    #[test]
    fn no_frames_error() {
        let sprite = SpriteRaw {
            name: "empty".to_string(),
            fps: 8,
            r#loop: true,
            tags: vec![],
            frames: vec![],
            scale: None,
        };
        assert!(resolve_sprite_frames(&sprite, 2, 2, &test_palette(), 8).is_err());
    }
}
