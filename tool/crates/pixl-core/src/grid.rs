use crate::types::Palette;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GridError {
    #[error("row {row}: expected {expected} columns, got {got}")]
    WidthMismatch {
        row: usize,
        expected: u32,
        got: usize,
    },

    #[error("expected {expected} rows, got {got}")]
    HeightMismatch { expected: u32, got: usize },

    #[error("row {row}, col {col}: unknown symbol '{sym}' (not in palette)")]
    UnknownSymbol { row: usize, col: usize, sym: char },

    #[error("grid is empty")]
    Empty,

    #[error("row {row}: reference ={target} targets another reference (chains forbidden)")]
    ChainedReference { row: usize, target: usize },

    #[error("row {row}: reference ={target} out of range (must be 1..{row})")]
    ReferenceOutOfRange { row: usize, target: usize },
}

/// Parse a row reference like `=3` → Some(3). Returns None if not a reference.
fn parse_row_ref(line: &str) -> Option<usize> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix('=') {
        rest.parse::<usize>().ok()
    } else {
        None
    }
}

/// Parse a multi-line grid string into a 2D char array.
/// Validates dimensions against declared width/height and symbols against palette.
/// Supports `=N` row references (PAX 2.1): `=3` means "same as row 3" (1-indexed).
pub fn parse_grid(
    raw: &str,
    width: u32,
    height: u32,
    palette: &Palette,
) -> Result<Vec<Vec<char>>, GridError> {
    let lines: Vec<&str> = raw
        .lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(GridError::Empty);
    }

    if lines.len() != height as usize {
        return Err(GridError::HeightMismatch {
            expected: height,
            got: lines.len(),
        });
    }

    // First pass: identify literal rows vs references
    let mut is_ref: Vec<bool> = vec![false; lines.len()];
    for (row_idx, line) in lines.iter().enumerate() {
        if parse_row_ref(line).is_some() {
            is_ref[row_idx] = true;
        }
    }

    // Second pass: validate references and expand
    let mut grid: Vec<Vec<char>> = Vec::with_capacity(height as usize);

    for (row_idx, line) in lines.iter().enumerate() {
        if let Some(target) = parse_row_ref(line) {
            // Validate: target must be 1..row_idx (1-indexed, no forward refs)
            if target == 0 || target > row_idx {
                return Err(GridError::ReferenceOutOfRange {
                    row: row_idx,
                    target,
                });
            }
            // Validate: target must be a literal row, not another reference
            if is_ref[target - 1] {
                return Err(GridError::ChainedReference {
                    row: row_idx,
                    target,
                });
            }
            // Copy the referenced row (target is 1-indexed)
            grid.push(grid[target - 1].clone());
        } else {
            let chars: Vec<char> = line.chars().collect();

            if chars.len() != width as usize {
                return Err(GridError::WidthMismatch {
                    row: row_idx,
                    expected: width,
                    got: chars.len(),
                });
            }

            for (col_idx, &ch) in chars.iter().enumerate() {
                if !palette.symbols.contains_key(&ch) {
                    return Err(GridError::UnknownSymbol {
                        row: row_idx,
                        col: col_idx,
                        sym: ch,
                    });
                }
            }

            grid.push(chars);
        }
    }

    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rgba;
    use std::collections::HashMap;

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
    fn parse_valid_grid() {
        let palette = test_palette();
        let raw = "##..\n#+.+\n..##\n++++";
        let grid = parse_grid(raw, 4, 4, &palette).unwrap();
        assert_eq!(grid.len(), 4);
        assert_eq!(grid[0], vec!['#', '#', '.', '.']);
        assert_eq!(grid[1], vec!['#', '+', '.', '+']);
    }

    #[test]
    fn wrong_width() {
        let palette = test_palette();
        let raw = "##.\n#+.+\n..##\n++++";
        let err = parse_grid(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(
            err,
            GridError::WidthMismatch {
                row: 0,
                expected: 4,
                got: 3
            }
        ));
    }

    #[test]
    fn wrong_height() {
        let palette = test_palette();
        let raw = "####\n++++";
        let err = parse_grid(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(
            err,
            GridError::HeightMismatch {
                expected: 4,
                got: 2
            }
        ));
    }

    #[test]
    fn unknown_symbol() {
        let palette = test_palette();
        let raw = "##X#\n++++\n++++\n++++";
        let err = parse_grid(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(
            err,
            GridError::UnknownSymbol {
                row: 0,
                col: 2,
                sym: 'X'
            }
        ));
    }

    #[test]
    fn row_reference_expands() {
        let palette = test_palette();
        let raw = "####\n++++\n=1\n=2";
        let grid = parse_grid(raw, 4, 4, &palette).unwrap();
        assert_eq!(grid[0], vec!['#', '#', '#', '#']);
        assert_eq!(grid[1], vec!['+', '+', '+', '+']);
        assert_eq!(grid[2], vec!['#', '#', '#', '#']); // =1 → row 1
        assert_eq!(grid[3], vec!['+', '+', '+', '+']); // =2 → row 2
    }

    #[test]
    fn row_reference_chain_rejected() {
        let palette = test_palette();
        // Row 3 (=1) is a ref, row 4 (=3) tries to ref a ref → error
        let raw = "####\n++++\n=1\n=3";
        let err = parse_grid(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(err, GridError::ChainedReference { row: 3, target: 3 }));
    }

    #[test]
    fn row_reference_forward_rejected() {
        let palette = test_palette();
        // =4 on row 1 (0-indexed) → forward reference, out of range
        let raw = "####\n=4\n++++\n++++";
        let err = parse_grid(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(
            err,
            GridError::ReferenceOutOfRange { row: 1, target: 4 }
        ));
    }

    #[test]
    fn row_reference_zero_rejected() {
        let palette = test_palette();
        let raw = "####\n=0\n++++\n++++";
        let err = parse_grid(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(
            err,
            GridError::ReferenceOutOfRange { row: 1, target: 0 }
        ));
    }
}
