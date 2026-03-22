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
}

/// Parse a multi-line grid string into a 2D char array.
/// Validates dimensions against declared width/height and symbols against palette.
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

    let mut grid = Vec::with_capacity(height as usize);

    for (row_idx, line) in lines.iter().enumerate() {
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
}
