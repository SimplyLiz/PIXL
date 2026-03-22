use crate::types::{Cycle, Palette};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CycleError {
    #[error("cycle '{name}': palette '{palette}' not found")]
    PaletteNotFound { name: String, palette: String },

    #[error("cycle '{name}': symbol '{sym}' not found in palette '{palette}'")]
    SymbolNotFound {
        name: String,
        sym: String,
        palette: String,
    },

    #[error("cycle '{name}': must have at least 2 symbols to cycle")]
    TooFewSymbols { name: String },

    #[error("cycle '{name}': invalid direction '{direction}' (expected: forward, backward, ping-pong)")]
    InvalidDirection { name: String, direction: String },

    #[error("cycle '{name}': fps must be > 0")]
    ZeroFps { name: String },
}

/// Validate a color cycle definition against its palette.
pub fn validate_cycle(
    name: &str,
    cycle: &Cycle,
    palettes: &std::collections::HashMap<String, Palette>,
) -> Vec<CycleError> {
    let mut errors = Vec::new();

    // Palette exists
    let Some(palette) = palettes.get(&cycle.palette) else {
        errors.push(CycleError::PaletteNotFound {
            name: name.to_string(),
            palette: cycle.palette.clone(),
        });
        return errors;
    };

    // At least 2 symbols
    if cycle.symbols.len() < 2 {
        errors.push(CycleError::TooFewSymbols {
            name: name.to_string(),
        });
    }

    // All symbols exist in palette
    for sym_str in &cycle.symbols {
        let ch = sym_str.chars().next();
        match ch {
            Some(c) if sym_str.len() == 1 => {
                if !palette.symbols.contains_key(&c) {
                    errors.push(CycleError::SymbolNotFound {
                        name: name.to_string(),
                        sym: sym_str.clone(),
                        palette: cycle.palette.clone(),
                    });
                }
            }
            _ => {
                errors.push(CycleError::SymbolNotFound {
                    name: name.to_string(),
                    sym: sym_str.clone(),
                    palette: cycle.palette.clone(),
                });
            }
        }
    }

    // Valid direction
    match cycle.direction.as_str() {
        "forward" | "backward" | "ping-pong" => {}
        _ => {
            errors.push(CycleError::InvalidDirection {
                name: name.to_string(),
                direction: cycle.direction.clone(),
            });
        }
    }

    // FPS > 0
    if cycle.fps == 0 {
        errors.push(CycleError::ZeroFps {
            name: name.to_string(),
        });
    }

    errors
}

/// Compute the effective color for a cycling symbol at a given frame tick.
/// Returns None if the symbol is not part of this cycle.
pub fn cycle_color_at_frame(
    sym: char,
    cycle: &Cycle,
    palette: &Palette,
    frame_tick: u64,
) -> Option<crate::types::Rgba> {
    // Find this symbol's position in the cycle
    let sym_str = sym.to_string();
    let pos = cycle.symbols.iter().position(|s| *s == sym_str)?;

    let n = cycle.symbols.len();
    let offset = (frame_tick as usize) % n;

    let effective_idx = match cycle.direction.as_str() {
        "forward" => (pos + offset) % n,
        "backward" => (pos + n - offset) % n,
        "ping-pong" => {
            // ping-pong: 0,1,2,...,n-1,n-2,...,1,0,1,...
            let period = if n > 1 { 2 * (n - 1) } else { 1 };
            let t = offset % period;
            if t < n { t } else { period - t }
        }
        _ => pos, // fallback: no cycling
    };

    let effective_sym_str = &cycle.symbols[effective_idx];
    let effective_ch = effective_sym_str.chars().next()?;
    palette.symbols.get(&effective_ch).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Palette, Rgba};
    use std::collections::HashMap;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert('~', Rgba { r: 26, g: 58, b: 92, a: 255 });
        symbols.insert('h', Rgba { r: 106, g: 90, b: 157, a: 255 });
        symbols.insert('+', Rgba { r: 74, g: 58, b: 109, a: 255 });
        symbols.insert('o', Rgba { r: 200, g: 160, b: 53, a: 255 });
        Palette { symbols }
    }

    fn test_cycle() -> Cycle {
        Cycle {
            palette: "test".to_string(),
            symbols: vec!["~".to_string(), "h".to_string(), "+".to_string()],
            direction: "forward".to_string(),
            fps: 8,
        }
    }

    #[test]
    fn validate_valid_cycle() {
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());
        let errors = validate_cycle("water", &test_cycle(), &palettes);
        assert!(errors.is_empty(), "got errors: {:?}", errors);
    }

    #[test]
    fn validate_missing_symbol() {
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());
        let cycle = Cycle {
            palette: "test".to_string(),
            symbols: vec!["~".to_string(), "X".to_string()],
            direction: "forward".to_string(),
            fps: 8,
        };
        let errors = validate_cycle("bad", &cycle, &palettes);
        assert!(errors.iter().any(|e| matches!(e, CycleError::SymbolNotFound { .. })));
    }

    #[test]
    fn validate_too_few_symbols() {
        let mut palettes = HashMap::new();
        palettes.insert("test".to_string(), test_palette());
        let cycle = Cycle {
            palette: "test".to_string(),
            symbols: vec!["~".to_string()],
            direction: "forward".to_string(),
            fps: 8,
        };
        let errors = validate_cycle("short", &cycle, &palettes);
        assert!(errors.iter().any(|e| matches!(e, CycleError::TooFewSymbols { .. })));
    }

    #[test]
    fn forward_cycle_rotates() {
        let palette = test_palette();
        let cycle = test_cycle(); // ~, h, +

        // Frame 0: ~ stays ~
        let c0 = cycle_color_at_frame('~', &cycle, &palette, 0).unwrap();
        assert_eq!(c0, palette.symbols[&'~']);

        // Frame 1: ~ becomes h
        let c1 = cycle_color_at_frame('~', &cycle, &palette, 1).unwrap();
        assert_eq!(c1, palette.symbols[&'h']);

        // Frame 2: ~ becomes +
        let c2 = cycle_color_at_frame('~', &cycle, &palette, 2).unwrap();
        assert_eq!(c2, palette.symbols[&'+']);

        // Frame 3: wraps back to ~
        let c3 = cycle_color_at_frame('~', &cycle, &palette, 3).unwrap();
        assert_eq!(c3, palette.symbols[&'~']);
    }

    #[test]
    fn non_cycling_symbol_returns_none() {
        let palette = test_palette();
        let cycle = test_cycle();
        assert!(cycle_color_at_frame('o', &cycle, &palette, 0).is_none());
    }
}
