use crate::types::{Palette, PaletteExt};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RleError {
    #[error("row {row}: expected {expected} pixels, got {got}")]
    WidthMismatch {
        row: usize,
        expected: u32,
        got: usize,
    },

    #[error("expected {expected} rows, got {got}")]
    HeightMismatch { expected: u32, got: usize },

    #[error("row {row}: invalid RLE token '{token}'")]
    InvalidToken { row: usize, token: String },

    #[error("row {row}: unknown symbol '{sym}' in token '{token}'")]
    UnknownSymbol {
        row: usize,
        sym: char,
        token: String,
    },

    #[error("RLE data is empty")]
    Empty,
}

/// Parse a multi-line RLE string into a 2D char array.
/// Format: space-separated `<count><symbol>` tokens, one line per row.
/// Count defaults to 1 if absent (bare symbol).
/// Every row must be explicit — no silent repetition.
pub fn parse_rle(
    raw: &str,
    width: u32,
    height: u32,
    palette: &Palette,
) -> Result<Vec<Vec<char>>, RleError> {
    let lines: Vec<&str> = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(RleError::Empty);
    }

    if lines.len() != height as usize {
        return Err(RleError::HeightMismatch {
            expected: height,
            got: lines.len(),
        });
    }

    let mut grid = Vec::with_capacity(height as usize);

    for (row_idx, line) in lines.iter().enumerate() {
        let mut row = Vec::with_capacity(width as usize);

        for token in line.split_whitespace() {
            let (count, sym) = parse_rle_token(token).map_err(|_| RleError::InvalidToken {
                row: row_idx,
                token: token.to_string(),
            })?;

            if !palette.symbols.contains_key(&sym) {
                return Err(RleError::UnknownSymbol {
                    row: row_idx,
                    sym,
                    token: token.to_string(),
                });
            }

            for _ in 0..count {
                row.push(sym);
            }
        }

        if row.len() != width as usize {
            return Err(RleError::WidthMismatch {
                row: row_idx,
                expected: width,
                got: row.len(),
            });
        }

        grid.push(row);
    }

    Ok(grid)
}

/// Parse a single RLE token: `<digits><char>` or bare `<char>`.
/// Returns (count, symbol).
fn parse_rle_token(token: &str) -> Result<(usize, char), ()> {
    let chars: Vec<char> = token.chars().collect();
    if chars.is_empty() {
        return Err(());
    }

    // Find where digits end and the symbol begins
    let digit_end = chars.iter().position(|c| !c.is_ascii_digit()).ok_or(())?;

    let count = if digit_end == 0 {
        1 // no leading digits → count of 1
    } else {
        token[..digit_end].parse::<usize>().map_err(|_| ())?
    };

    // Everything after digits must be exactly one character
    let remaining: Vec<char> = chars[digit_end..].to_vec();
    if remaining.len() != 1 {
        return Err(());
    }

    Ok((count, remaining[0]))
}

/// Encode a 2D grid as RLE string.
pub fn encode_rle(grid: &[Vec<char>]) -> String {
    let mut lines = Vec::new();

    for row in grid {
        let mut tokens = Vec::new();
        let mut i = 0;

        while i < row.len() {
            let sym = row[i];
            let mut count = 1;
            while i + count < row.len() && row[i + count] == sym {
                count += 1;
            }
            if count == 1 {
                tokens.push(format!("{}", sym));
            } else {
                tokens.push(format!("{}{}", count, sym));
            }
            i += count;
        }

        lines.push(tokens.join(" "));
    }

    lines.join("\n")
}

// ── Extended RLE (multi-char symbols for backdrop tiles) ────────────

/// Parse RLE with multi-char symbol support for extended palettes.
/// Tokens are whitespace-separated: `<count><symbol>` where symbol can be
/// a single char (base palette) or a multi-char key like `2a` (extended palette).
pub fn parse_rle_ext(
    raw: &str,
    width: u32,
    height: u32,
    palette_ext: &PaletteExt,
) -> Result<Vec<Vec<String>>, RleError> {
    let lines: Vec<&str> = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(RleError::Empty);
    }

    if lines.len() != height as usize {
        return Err(RleError::HeightMismatch {
            expected: height,
            got: lines.len(),
        });
    }

    let mut grid = Vec::with_capacity(height as usize);

    for (row_idx, line) in lines.iter().enumerate() {
        let mut row = Vec::with_capacity(width as usize);

        for token in line.split_whitespace() {
            let (count, sym) = parse_rle_token_ext(token).map_err(|_| RleError::InvalidToken {
                row: row_idx,
                token: token.to_string(),
            })?;

            // Validate symbol exists in base or extended palette
            let valid = if sym.len() == 1 {
                let ch = sym.chars().next().unwrap();
                palette_ext.base.contains_key(&ch)
            } else {
                palette_ext.extended.contains_key(&sym)
            };

            if !valid {
                return Err(RleError::InvalidToken {
                    row: row_idx,
                    token: token.to_string(),
                });
            }

            for _ in 0..count {
                row.push(sym.clone());
            }
        }

        if row.len() != width as usize {
            return Err(RleError::WidthMismatch {
                row: row_idx,
                expected: width,
                got: row.len(),
            });
        }

        grid.push(row);
    }

    Ok(grid)
}

/// Parse a single extended RLE token.
///
/// Formats:
/// - `<count>:<multi-char-sym>` — e.g. `5:2f` = 5 copies of symbol "2f"
/// - `<multi-char-sym>` — e.g. `2f` = 1 copy of symbol "2f" (no colon, no leading count)
/// - `<count><single-char>` — e.g. `5f` = 5 copies of "f" (classic RLE)
/// - `<single-char>` — e.g. `f` = 1 copy of "f"
fn parse_rle_token_ext(token: &str) -> Result<(usize, String), ()> {
    if token.is_empty() {
        return Err(());
    }

    // If token contains ':', split on it: count:symbol
    if let Some(colon_pos) = token.find(':') {
        let count_str = &token[..colon_pos];
        let sym = &token[colon_pos + 1..];
        if sym.is_empty() {
            return Err(());
        }
        let count = if count_str.is_empty() {
            1
        } else {
            count_str.parse::<usize>().map_err(|_| ())?
        };
        return Ok((count, sym.to_string()));
    }

    // No colon — classic single-char RLE: leading digits = count, last char = symbol
    let chars: Vec<char> = token.chars().collect();

    // If it's a single char, count = 1
    if chars.len() == 1 {
        return Ok((1, chars[0].to_string()));
    }

    // If the last char is NOT a digit, treat leading digits as count
    if !chars.last().unwrap().is_ascii_digit() {
        let digit_end = chars.iter().position(|c| !c.is_ascii_digit()).ok_or(())?;
        let count = if digit_end == 0 {
            1
        } else {
            token[..digit_end].parse::<usize>().map_err(|_| ())?
        };
        let sym = &token[digit_end..];
        if sym.len() == 1 {
            return Ok((count, sym.to_string()));
        }
        // Multi-char without colon — treat entire token as symbol with count 1
        return Ok((1, token.to_string()));
    }

    // All digits — invalid (no symbol)
    Err(())
}

/// Encode a 2D grid of string symbols as RLE.
/// Multi-char symbols use colon separator: `5:2f` (5 copies of "2f").
/// Single-char symbols use classic format: `5f` (5 copies of "f").
pub fn encode_rle_ext(grid: &[Vec<String>]) -> String {
    let mut lines = Vec::new();

    for row in grid {
        let mut tokens = Vec::new();
        let mut i = 0;

        while i < row.len() {
            let sym = &row[i];
            let mut count = 1;
            while i + count < row.len() && row[i + count] == *sym {
                count += 1;
            }

            let is_multi = sym.len() > 1;

            if count == 1 && !is_multi {
                tokens.push(sym.clone());
            } else if count == 1 && is_multi {
                // Bare multi-char: use 1:sym to avoid ambiguity with "2a" meaning "2 copies of a"
                tokens.push(format!("1:{}", sym));
            } else if is_multi {
                // Multi-char: use colon separator to avoid ambiguity
                tokens.push(format!("{}:{}", count, sym));
            } else {
                // Single-char: classic RLE
                tokens.push(format!("{}{}", count, sym));
            }
            i += count;
        }

        lines.push(tokens.join(" "));
    }

    lines.join("\n")
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
    fn parse_valid_rle() {
        let palette = test_palette();
        let raw = "4#\n2# 2+\n4+\n2+ 2#";
        let grid = parse_rle(raw, 4, 4, &palette).unwrap();
        assert_eq!(grid[0], vec!['#', '#', '#', '#']);
        assert_eq!(grid[1], vec!['#', '#', '+', '+']);
        assert_eq!(grid[2], vec!['+', '+', '+', '+']);
        assert_eq!(grid[3], vec!['+', '+', '#', '#']);
    }

    #[test]
    fn bare_symbols_default_to_count_1() {
        let palette = test_palette();
        let raw = "# + # +\n+ # + #\n# + # +\n+ # + #";
        let grid = parse_rle(raw, 4, 4, &palette).unwrap();
        assert_eq!(grid[0], vec!['#', '+', '#', '+']);
    }

    #[test]
    fn wrong_row_count() {
        let palette = test_palette();
        let raw = "4#\n4+";
        let err = parse_rle(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(
            err,
            RleError::HeightMismatch {
                expected: 4,
                got: 2
            }
        ));
    }

    #[test]
    fn wrong_row_width() {
        let palette = test_palette();
        let raw = "3#\n4+\n4+\n4+";
        let err = parse_rle(raw, 4, 4, &palette).unwrap_err();
        assert!(matches!(
            err,
            RleError::WidthMismatch {
                row: 0,
                expected: 4,
                got: 3
            }
        ));
    }

    #[test]
    fn roundtrip_encode_decode() {
        let palette = test_palette();
        let original = vec![
            vec!['#', '#', '#', '#'],
            vec!['#', '+', '+', '#'],
            vec!['#', '+', '+', '#'],
            vec!['#', '#', '#', '#'],
        ];
        let encoded = encode_rle(&original);
        let decoded = parse_rle(&encoded, 4, 4, &palette).unwrap();
        assert_eq!(original, decoded);
    }
}
