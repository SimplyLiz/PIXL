use crate::types::Stamp;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComposeError {
    #[error("row {row}: unknown stamp '@{name}'")]
    UnknownStamp { row: usize, name: String },

    #[error("row {row}: mixed stamp heights — '{stamp}' has height {got}, expected {expected}")]
    MixedHeights {
        row: usize,
        stamp: String,
        expected: u32,
        got: u32,
    },

    #[error("row {row}: width mismatch — composed {got}px, expected {expected}px")]
    RowWidthMismatch { row: usize, expected: u32, got: u32 },

    #[error("height mismatch — composed {got}px, expected {expected}px")]
    HeightMismatch { expected: u32, got: u32 },

    #[error("row {row}: could not determine row height (no stamps in row)")]
    NoStampsInRow { row: usize },

    #[error("layout is empty")]
    Empty,
}

/// Resolve a compose layout string into a full pixel grid.
///
/// Grammar:
///   layout_row ::= (stamp_ref | void_block) (' ' (stamp_ref | void_block))*
///   stamp_ref  ::= '@' identifier
///   void_block ::= '_'
///
/// `_` fills a stamp-sized area with the void symbol ('.').
/// No inline pixel strings in V1.
pub fn resolve_compose(
    layout: &str,
    stamps: &HashMap<String, Stamp>,
    tile_width: u32,
    tile_height: u32,
    void_sym: char,
) -> Result<Vec<Vec<char>>, ComposeError> {
    let rows: Vec<&str> = layout
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if rows.is_empty() {
        return Err(ComposeError::Empty);
    }

    let mut canvas = vec![vec![void_sym; tile_width as usize]; tile_height as usize];
    let mut cursor_y: u32 = 0;

    for (row_idx, row) in rows.iter().enumerate() {
        let tokens: Vec<&str> = row.split_whitespace().collect();

        // Determine row height from stamps in this row
        let row_height = infer_row_height(&tokens, stamps, row_idx)?;

        let mut cursor_x: u32 = 0;

        for token in &tokens {
            if *token == "_" {
                // Void block — fill with void symbol, same size as row height
                // Width = row_height (square assumption for void blocks)
                // Actually, void blocks need to match the stamp width context.
                // For V1: void block width = row_height (square)
                let block_size = row_height;
                cursor_x += block_size;
            } else if let Some(name) = token.strip_prefix('@') {
                let stamp = stamps.get(name).ok_or_else(|| ComposeError::UnknownStamp {
                    row: row_idx,
                    name: name.to_string(),
                })?;

                if stamp.height != row_height {
                    return Err(ComposeError::MixedHeights {
                        row: row_idx,
                        stamp: name.to_string(),
                        expected: row_height,
                        got: stamp.height,
                    });
                }

                // Blit stamp onto canvas
                for sy in 0..stamp.height {
                    for sx in 0..stamp.width {
                        let cx = cursor_x + sx;
                        let cy = cursor_y + sy;
                        if (cy as usize) < canvas.len() && (cx as usize) < canvas[0].len() {
                            canvas[cy as usize][cx as usize] = stamp.grid[sy as usize][sx as usize];
                        }
                    }
                }
                cursor_x += stamp.width;
            }
        }

        if cursor_x != tile_width {
            return Err(ComposeError::RowWidthMismatch {
                row: row_idx,
                expected: tile_width,
                got: cursor_x,
            });
        }

        cursor_y += row_height;
    }

    if cursor_y != tile_height {
        return Err(ComposeError::HeightMismatch {
            expected: tile_height,
            got: cursor_y,
        });
    }

    Ok(canvas)
}

fn infer_row_height(
    tokens: &[&str],
    stamps: &HashMap<String, Stamp>,
    row_idx: usize,
) -> Result<u32, ComposeError> {
    for token in tokens {
        if let Some(name) = token.strip_prefix('@')
            && let Some(stamp) = stamps.get(name) {
                return Ok(stamp.height);
            }
    }
    Err(ComposeError::NoStampsInRow { row: row_idx })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stamps() -> HashMap<String, Stamp> {
        let mut stamps = HashMap::new();
        stamps.insert(
            "a".to_string(),
            Stamp {
                palette: "test".to_string(),
                width: 2,
                height: 2,
                grid: vec![vec!['#', '+'], vec!['+', '#']],
            },
        );
        stamps.insert(
            "b".to_string(),
            Stamp {
                palette: "test".to_string(),
                width: 2,
                height: 2,
                grid: vec![vec!['.', '.'], vec!['.', '.']],
            },
        );
        stamps
    }

    #[test]
    fn compose_2x2_stamps_into_4x4() {
        let stamps = make_stamps();
        let layout = "@a @b\n@b @a";
        let grid = resolve_compose(layout, &stamps, 4, 4, '.').unwrap();
        assert_eq!(grid[0], vec!['#', '+', '.', '.']);
        assert_eq!(grid[1], vec!['+', '#', '.', '.']);
        assert_eq!(grid[2], vec!['.', '.', '#', '+']);
        assert_eq!(grid[3], vec!['.', '.', '+', '#']);
    }

    #[test]
    fn void_block() {
        let stamps = make_stamps();
        let layout = "@a _\n_ @a";
        let grid = resolve_compose(layout, &stamps, 4, 4, '.').unwrap();
        assert_eq!(grid[0], vec!['#', '+', '.', '.']);
        assert_eq!(grid[1], vec!['+', '#', '.', '.']);
        assert_eq!(grid[2], vec!['.', '.', '#', '+']);
        assert_eq!(grid[3], vec!['.', '.', '+', '#']);
    }

    #[test]
    fn unknown_stamp_error() {
        let stamps = make_stamps();
        let layout = "@a @unknown\n@a @a";
        let err = resolve_compose(layout, &stamps, 4, 4, '.').unwrap_err();
        assert!(matches!(err, ComposeError::UnknownStamp { row: 0, .. }));
    }

    #[test]
    fn width_mismatch_error() {
        let stamps = make_stamps();
        let layout = "@a\n@a @a"; // first row = 2px, expected 4px
        let err = resolve_compose(layout, &stamps, 4, 4, '.').unwrap_err();
        assert!(matches!(err, ComposeError::RowWidthMismatch { row: 0, .. }));
    }
}
