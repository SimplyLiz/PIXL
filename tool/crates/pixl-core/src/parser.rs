use crate::types::{
    Backdrop, BackdropLayer, BackdropZone, BlendMode, FadeTarget, Palette, PaletteExt,
    PaletteExtRaw, PaletteRaw, PaxFile, Rgba, TileRef, ZoneBehavior,
};
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

    #[error("{0}")]
    Resolve(String),
}

/// Parse a .pax file from a TOML string.
pub fn parse_pax(source: &str) -> Result<PaxFile, ParseError> {
    let file: PaxFile = toml::from_str(source)?;
    Ok(file)
}

/// Resolve a raw palette (HashMap<String, String>) into a typed Palette.
pub fn resolve_palette(name: &str, raw: &PaletteRaw) -> Result<Palette, ParseError> {
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
pub fn resolve_all_palettes(file: &PaxFile) -> Result<HashMap<String, Palette>, ParseError> {
    let mut palettes = HashMap::new();
    for (name, raw) in &file.palette {
        let palette = resolve_palette(name, raw)?;
        palettes.insert(name.clone(), palette);
    }
    Ok(palettes)
}

/// Resolve an extended palette: merge base palette + multi-char extensions.
pub fn resolve_palette_ext(
    name: &str,
    raw: &PaletteExtRaw,
    palettes: &HashMap<String, Palette>,
) -> Result<PaletteExt, ParseError> {
    let base_palette = palettes.get(&raw.base).ok_or_else(|| ParseError::Resolve(format!(
            "palette_ext '{}': base palette '{}' not found",
            name, raw.base
        ))
    )?;

    let mut extended = HashMap::new();
    for (key, hex) in &raw.symbols {
        // Skip the "base" key (it's metadata, not a color)
        if key == "base" {
            continue;
        }
        let rgba = Rgba::from_hex(hex).map_err(|reason| ParseError::PaletteInvalidColor {
            palette: name.to_string(),
            sym: key.clone(),
            hex: hex.clone(),
            reason,
        })?;
        extended.insert(key.clone(), rgba);
    }

    Ok(PaletteExt {
        base: base_palette.symbols.clone(),
        extended,
    })
}

/// Parse a tilemap string into a grid of TileRef entries.
fn parse_tilemap_grid(tilemap_str: &str) -> Vec<Vec<TileRef>> {
    tilemap_str
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|line| {
            line.split_whitespace()
                .map(|s| TileRef::parse(s))
                .collect()
        })
        .collect()
}

/// Resolve a backdrop into a Backdrop struct.
/// Supports both single-tilemap (backward compat) and multi-layer formats.
pub fn resolve_backdrop(
    name: &str,
    file: &PaxFile,
) -> Result<Backdrop, ParseError> {
    let raw = file.backdrop.get(name).ok_or_else(|| ParseError::Resolve(format!("backdrop '{}' not found", name))
    )?;

    let (total_w, total_h) = crate::types::parse_size(&raw.size)
        .map_err(ParseError::Resolve)?;
    let (tile_w, tile_h) = crate::types::parse_size(&raw.tile_size)
        .map_err(ParseError::Resolve)?;

    let cols = total_w / tile_w;
    let rows = total_h / tile_h;

    // Build layers: either from explicit `layer` array or from single `tilemap`
    let layers = if !raw.layer.is_empty() {
        raw.layer
            .iter()
            .map(|lr| {
                let fade = lr.fade.as_ref().map(|f| {
                    let target = if f.target == "white" { FadeTarget::White } else { FadeTarget::Black };
                    (target, f.amount)
                });
                BackdropLayer {
                    name: lr.name.clone(),
                    tilemap: parse_tilemap_grid(&lr.tilemap),
                    scroll_factor: lr.scroll_factor,
                    opacity: lr.opacity,
                    blend: BlendMode::from_str(&lr.blend),
                    offset_x: lr.offset_x,
                    offset_y: lr.offset_y,
                    fade,
                    scroll_lock: lr.scroll_lock.clone(),
                }
            })
            .collect()
    } else if let Some(tilemap_str) = &raw.tilemap {
        vec![BackdropLayer {
            name: "main".to_string(),
            tilemap: parse_tilemap_grid(tilemap_str),
            scroll_factor: 1.0,
            opacity: 1.0,
            blend: BlendMode::Normal,
            offset_x: 0,
            offset_y: 0,
            fade: None,
            scroll_lock: None,
        }]
    } else {
        vec![]
    };

    // Resolve zones
    let zones: Vec<BackdropZone> = raw
        .zone
        .iter()
        .map(|z| {
            let behavior = match z.behavior.as_str() {
                "cycle" => ZoneBehavior::Cycle {
                    cycle: z.cycle.clone().unwrap_or_default(),
                },
                "wave" => ZoneBehavior::Wave {
                    cycle: z.cycle.clone().unwrap_or_default(),
                    phase_rows: z.phase_rows.unwrap_or(4),
                    wave_dx: z.wave_dx.unwrap_or(1),
                },
                "flicker" => ZoneBehavior::Flicker {
                    cycle: z.cycle.clone().unwrap_or_default(),
                    density: z.density.unwrap_or(0.3),
                    seed: z.seed.unwrap_or(42),
                },
                "scroll_down" => ZoneBehavior::ScrollDown {
                    speed: z.speed.unwrap_or(1.0),
                    wrap: z.wrap.unwrap_or(true),
                },
                "hscroll_sine" => ZoneBehavior::HScrollSine {
                    amplitude: z.amplitude.unwrap_or(3),
                    period: z.period.unwrap_or(32),
                    speed: z.speed.unwrap_or(2.0),
                },
                "color_gradient" => {
                    let from = z.from.as_deref().and_then(|h| Rgba::from_hex(h).ok())
                        .unwrap_or(Rgba { r: 0, g: 0, b: 0, a: 255 });
                    let to = z.to.as_deref().and_then(|h| Rgba::from_hex(h).ok())
                        .unwrap_or(Rgba { r: 255, g: 255, b: 255, a: 255 });
                    ZoneBehavior::ColorGradient {
                        from, to,
                        vertical: z.direction.as_deref() != Some("horizontal"),
                    }
                },
                "mosaic" => ZoneBehavior::Mosaic {
                    size_x: z.size_x.unwrap_or(2),
                    size_y: z.size_y.unwrap_or(2),
                },
                "window" => ZoneBehavior::Window {
                    layers_visible: z.layers_visible.clone().unwrap_or_default(),
                    blend_override: z.blend_override.as_deref().map(BlendMode::from_str),
                    opacity_override: z.opacity_override,
                },
                "vscroll_sine" => ZoneBehavior::VScrollSine {
                    amplitude: z.amplitude.unwrap_or(2),
                    period: z.period.unwrap_or(16),
                    speed: z.speed.unwrap_or(1.5),
                },
                "palette_ramp" => {
                    let from = z.from.as_deref().and_then(|h| Rgba::from_hex(h).ok())
                        .unwrap_or(Rgba { r: 0, g: 0, b: 0, a: 255 });
                    let to = z.to.as_deref().and_then(|h| Rgba::from_hex(h).ok())
                        .unwrap_or(Rgba { r: 255, g: 255, b: 255, a: 255 });
                    ZoneBehavior::PaletteRamp {
                        symbol: z.symbol.clone().unwrap_or_default(),
                        from, to,
                    }
                },
                _ => ZoneBehavior::Cycle {
                    cycle: z.cycle.clone().unwrap_or_default(),
                },
            };
            BackdropZone {
                name: z.name.clone(),
                rect: z.rect.clone(),
                behavior,
                layer: z.layer.clone(),
            }
        })
        .collect();

    Ok(Backdrop {
        name: name.to_string(),
        width: total_w,
        height: total_h,
        tile_width: tile_w,
        tile_height: tile_h,
        cols,
        rows,
        layers,
        zones,
    })
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
