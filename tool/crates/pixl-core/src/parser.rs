use crate::types::{PaxFile, Palette, PaletteRaw, Rgba};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("palette '{palette}': key '{key}' must be exactly one character")]
    PaletteKeyNotChar { palette: String, key: String },

    #[error("palette '{palette}': invalid hex color '{hex}' for symbol '{sym}': {reason}")]
    PaletteInvalidColor {
        palette: String,
        sym: String,
        hex: String,
        reason: String,
    },
}

/// Parse a .pax file from a TOML string.
pub fn parse_pax(source: &str) -> Result<PaxFile, ParseError> {
    let file: PaxFile = toml::from_str(source)?;
    Ok(file)
}

/// Resolve a raw palette (HashMap<String, String>) into a typed Palette.
pub fn resolve_palette(
    name: &str,
    raw: &PaletteRaw,
) -> Result<Palette, ParseError> {
    let mut symbols = HashMap::with_capacity(raw.len());

    for (key, hex) in raw {
        // Validate single-char key
        let mut chars = key.chars();
        let ch = chars.next().ok_or_else(|| ParseError::PaletteKeyNotChar {
            palette: name.to_string(),
            key: key.clone(),
        })?;
        if chars.next().is_some() {
            return Err(ParseError::PaletteKeyNotChar {
                palette: name.to_string(),
                key: key.clone(),
            });
        }

        // Parse hex color
        let rgba = Rgba::from_hex(hex).map_err(|reason| ParseError::PaletteInvalidColor {
            palette: name.to_string(),
            sym: key.clone(),
            hex: hex.clone(),
            reason,
        })?;

        symbols.insert(ch, rgba);
    }

    Ok(Palette { symbols })
}

/// Resolve all palettes in a PaxFile.
pub fn resolve_all_palettes(
    file: &PaxFile,
) -> Result<HashMap<String, Palette>, ParseError> {
    let mut palettes = HashMap::new();
    for (name, raw) in &file.palette {
        let palette = resolve_palette(name, raw)?;
        palettes.insert(name.clone(), palette);
    }
    Ok(palettes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_pax() {
        let source = concat!(
            "[pax]\n",
            "version = \"2.0\"\n",
            "name = \"test\"\n",
            "\n",
            "[palette.test]\n",
            "\".\" = \"#00000000\"\n",
            "\"#\" = \"#2a1f3d\"\n",
            "\"+\" = \"#4a3a6d\"\n",
        );
        let file = parse_pax(source).unwrap();
        assert_eq!(file.pax.version, "2.0");
        assert_eq!(file.pax.name, "test");
        assert!(file.palette.contains_key("test"));
    }

    #[test]
    fn resolve_palette_valid() {
        let mut raw = HashMap::new();
        raw.insert(".".to_string(), "#00000000".to_string());
        raw.insert("#".to_string(), "#2a1f3d".to_string());

        let palette = resolve_palette("test", &raw).unwrap();
        assert_eq!(palette.symbols.len(), 2);
        assert_eq!(palette.symbols[&'.'].a, 0);
        assert_eq!(palette.symbols[&'#'].r, 42);
    }

    #[test]
    fn reject_multi_char_key() {
        let mut raw = HashMap::new();
        raw.insert("ab".to_string(), "#000000".to_string());

        let err = resolve_palette("test", &raw).unwrap_err();
        assert!(matches!(err, ParseError::PaletteKeyNotChar { .. }));
    }

    #[test]
    fn reject_invalid_hex() {
        let mut raw = HashMap::new();
        raw.insert(".".to_string(), "#GGHHII".to_string());

        let err = resolve_palette("test", &raw).unwrap_err();
        assert!(matches!(err, ParseError::PaletteInvalidColor { .. }));
    }

    #[test]
    fn parse_dungeon_example() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let file = parse_pax(&source).unwrap();
        assert_eq!(file.pax.version, "2.0");
        assert!(file.palette.contains_key("dungeon"));
        assert!(file.tile.contains_key("wall_solid"));
        assert!(file.tile.contains_key("floor_stone"));

        // Resolve palette
        let palettes = resolve_all_palettes(&file).unwrap();
        let dungeon = &palettes["dungeon"];
        assert!(dungeon.symbols.contains_key(&'#'));
        assert!(dungeon.symbols.contains_key(&'+'));
        assert!(dungeon.symbols.contains_key(&'~'));
    }
}
