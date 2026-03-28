use crate::types::{Palette, Rgba, Theme};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("theme '{name}': palette '{palette}' not found")]
    PaletteNotFound { name: String, palette: String },

    #[error("theme '{name}': extends '{parent}' not found")]
    ParentNotFound { name: String, parent: String },

    #[error("theme '{name}': circular extends chain detected (visited: {chain})")]
    CircularExtends { name: String, chain: String },

    #[error(
        "theme '{name}': role '{role}' maps to symbol '{sym}' which is not in palette '{palette}'"
    )]
    RoleSymbolNotInPalette {
        name: String,
        role: String,
        sym: String,
        palette: String,
    },

    #[error("theme '{name}': constraint '{constraint}' violated — {reason}")]
    ConstraintViolation {
        name: String,
        constraint: String,
        reason: String,
    },
}

/// Resolved theme with inherited values applied.
#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub name: String,
    pub palette: String,
    pub scale: u32,
    pub canvas: u32,
    pub max_palette_size: Option<u32>,
    pub light_source: Option<String>,
    pub roles: HashMap<String, char>,
}

/// Resolve a theme, applying inheritance from parent themes.
/// Detects circular extends chains.
pub fn resolve_theme(
    name: &str,
    themes: &HashMap<String, Theme>,
    palettes: &HashMap<String, Palette>,
) -> Result<ResolvedTheme, ThemeError> {
    // Collect the inheritance chain
    let mut chain = vec![name.to_string()];
    let mut current = name;

    loop {
        let theme = themes
            .get(current)
            .ok_or_else(|| ThemeError::ParentNotFound {
                name: name.to_string(),
                parent: current.to_string(),
            })?;

        match &theme.extends {
            Some(parent) => {
                if chain.contains(parent) {
                    return Err(ThemeError::CircularExtends {
                        name: name.to_string(),
                        chain: chain.join(" -> "),
                    });
                }
                chain.push(parent.clone());
                current = parent;
            }
            None => break,
        }
    }

    // Resolve bottom-up: start from root ancestor, overlay child values
    let mut resolved = ResolvedTheme {
        name: name.to_string(),
        palette: String::new(),
        scale: 1,
        canvas: 16,
        max_palette_size: None,
        light_source: None,
        roles: HashMap::new(),
    };

    // Walk chain from root to leaf
    for theme_name in chain.iter().rev() {
        let theme = &themes[theme_name];
        resolved.palette = theme.palette.clone();
        if let Some(s) = theme.scale {
            resolved.scale = s;
        }
        if let Some(c) = theme.canvas {
            resolved.canvas = c;
        }
        if theme.max_palette_size.is_some() {
            resolved.max_palette_size = theme.max_palette_size;
        }
        if theme.light_source.is_some() {
            resolved.light_source = theme.light_source.clone();
        }
        // Roles: child overrides parent
        for (role, sym_str) in &theme.roles {
            let chars: Vec<char> = sym_str.chars().collect();
            if chars.len() == 1 {
                resolved.roles.insert(role.clone(), chars[0]);
            } else if !sym_str.is_empty() {
                return Err(ThemeError::ConstraintViolation {
                    name: name.to_string(),
                    constraint: format!("role '{}'", role),
                    reason: format!("role symbol must be exactly 1 char, got '{}'", sym_str),
                });
            }
            // Empty string: role silently dropped (no symbol assigned)
        }
    }

    // Validate palette exists
    if !palettes.contains_key(&resolved.palette) {
        return Err(ThemeError::PaletteNotFound {
            name: name.to_string(),
            palette: resolved.palette.clone(),
        });
    }

    // Validate role symbols exist in palette
    let palette = &palettes[&resolved.palette];
    for (role, &sym) in &resolved.roles {
        if !palette.symbols.contains_key(&sym) {
            return Err(ThemeError::RoleSymbolNotInPalette {
                name: name.to_string(),
                role: role.clone(),
                sym: sym.to_string(),
                palette: resolved.palette.clone(),
            });
        }
    }

    Ok(resolved)
}

/// Evaluate declarative theme constraints.
/// Returns warnings (not errors) for V1.
pub fn evaluate_constraints(
    theme: &Theme,
    resolved: &ResolvedTheme,
    palette: &Palette,
) -> Vec<ThemeError> {
    let mut warnings = Vec::new();

    for constraint_name in theme.constraints.keys() {
        match constraint_name.as_str() {
            "fg_brighter_than_bg" => {
                if let (Some(&fg), Some(&bg)) = (resolved.roles.get("fg"), resolved.roles.get("bg"))
                    && let (Some(fg_c), Some(bg_c)) =
                        (palette.symbols.get(&fg), palette.symbols.get(&bg))
                    && luminance(fg_c) <= luminance(bg_c)
                {
                    warnings.push(ThemeError::ConstraintViolation {
                        name: resolved.name.clone(),
                        constraint: constraint_name.clone(),
                        reason: format!(
                            "fg luminance ({:.3}) <= bg luminance ({:.3})",
                            luminance(fg_c),
                            luminance(bg_c)
                        ),
                    });
                }
            }
            "shadow_darker_than_bg" => {
                if let (Some(&shadow), Some(&bg)) =
                    (resolved.roles.get("shadow"), resolved.roles.get("bg"))
                    && let (Some(s_c), Some(bg_c)) =
                        (palette.symbols.get(&shadow), palette.symbols.get(&bg))
                    && luminance(s_c) >= luminance(bg_c)
                {
                    warnings.push(ThemeError::ConstraintViolation {
                        name: resolved.name.clone(),
                        constraint: constraint_name.clone(),
                        reason: format!(
                            "shadow luminance ({:.3}) >= bg luminance ({:.3})",
                            luminance(s_c),
                            luminance(bg_c)
                        ),
                    });
                }
            }
            "accent_hue_distinct_from_bg" => {
                if let (Some(&accent), Some(&bg)) =
                    (resolved.roles.get("accent"), resolved.roles.get("bg"))
                    && let (Some(a_c), Some(bg_c)) =
                        (palette.symbols.get(&accent), palette.symbols.get(&bg))
                {
                    let dist = hue_distance(a_c, bg_c);
                    if dist < 40.0 {
                        warnings.push(ThemeError::ConstraintViolation {
                            name: resolved.name.clone(),
                            constraint: constraint_name.clone(),
                            reason: format!("hue distance {:.1}° < 40°", dist),
                        });
                    }
                }
            }
            "palette_granularity" => {
                // NES attribute table constraint: palette can only change at NxN tile boundaries.
                // Value is the block size in tiles (NES = 2 for 2x2 tile blocks).
                // This is a generation/validation hint — stored for external tools to check.
                // No runtime evaluation needed here; the warning is informational.
                if let Some(val) = theme.constraints.get(constraint_name) {
                    if let Some(n) = val.as_integer() {
                        if n < 1 || n > 8 {
                            warnings.push(ThemeError::ConstraintViolation {
                                name: resolved.name.clone(),
                                constraint: constraint_name.clone(),
                                reason: format!("palette_granularity must be 1-8, got {}", n),
                            });
                        }
                    }
                }
            }
            _ => {} // Unknown constraints are silently ignored in V1
        }
    }

    warnings
}

/// Relative luminance (sRGB to linear, ITU-R BT.709 weights).
fn luminance(c: &Rgba) -> f32 {
    fn linearize(v: u8) -> f32 {
        let s = v as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linearize(c.r) + 0.7152 * linearize(c.g) + 0.0722 * linearize(c.b)
}

/// Hue in degrees (0-360).
fn hue_degrees(c: &Rgba) -> f32 {
    let r = c.r as f32 / 255.0;
    let g = c.g as f32 / 255.0;
    let b = c.b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    if delta < 1e-6 {
        return 0.0;
    }
    let hue = if (max - r).abs() < 1e-6 {
        (g - b) / delta
    } else if (max - g).abs() < 1e-6 {
        2.0 + (b - r) / delta
    } else {
        4.0 + (r - g) / delta
    };
    (hue * 60.0).rem_euclid(360.0)
}

/// Angular distance between two hues (0-180).
fn hue_distance(a: &Rgba, b: &Rgba) -> f32 {
    let ha = hue_degrees(a);
    let hb = hue_degrees(b);
    let diff = (ha - hb).abs();
    diff.min(360.0 - diff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Rgba;

    fn dungeon_palette() -> Palette {
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
        ); // dark purple
        symbols.insert(
            '+',
            Rgba {
                r: 74,
                g: 58,
                b: 109,
                a: 255,
            },
        ); // lighter purple
        symbols.insert(
            's',
            Rgba {
                r: 26,
                g: 15,
                b: 46,
                a: 255,
            },
        ); // very dark
        symbols.insert(
            'o',
            Rgba {
                r: 200,
                g: 160,
                b: 53,
                a: 255,
            },
        ); // gold
        symbols.insert(
            'r',
            Rgba {
                r: 139,
                g: 26,
                b: 26,
                a: 255,
            },
        ); // red
        symbols.insert(
            'g',
            Rgba {
                r: 45,
                g: 90,
                b: 39,
                a: 255,
            },
        ); // green
        Palette { symbols }
    }

    fn dark_fantasy_theme() -> Theme {
        let mut roles = HashMap::new();
        roles.insert("void".to_string(), ".".to_string());
        roles.insert("bg".to_string(), "#".to_string());
        roles.insert("fg".to_string(), "+".to_string());
        roles.insert("shadow".to_string(), "s".to_string());
        roles.insert("accent".to_string(), "o".to_string());

        let mut constraints = HashMap::new();
        constraints.insert(
            "fg_brighter_than_bg".to_string(),
            toml::Value::Boolean(true),
        );
        constraints.insert(
            "shadow_darker_than_bg".to_string(),
            toml::Value::Boolean(true),
        );
        constraints.insert(
            "accent_hue_distinct_from_bg".to_string(),
            toml::Value::Boolean(true),
        );

        Theme {
            palette: "dungeon".to_string(),
            scale: Some(2),
            canvas: Some(16),
            max_palette_size: Some(16),
            light_source: Some("top-left".to_string()),
            extends: None,
            roles,
            constraints,
        }
    }

    #[test]
    fn resolve_simple_theme() {
        let mut themes = HashMap::new();
        themes.insert("dark_fantasy".to_string(), dark_fantasy_theme());
        let mut palettes = HashMap::new();
        palettes.insert("dungeon".to_string(), dungeon_palette());

        let resolved = resolve_theme("dark_fantasy", &themes, &palettes).unwrap();
        assert_eq!(resolved.palette, "dungeon");
        assert_eq!(resolved.scale, 2);
        assert_eq!(resolved.roles["fg"], '+');
        assert_eq!(resolved.roles["bg"], '#');
    }

    #[test]
    fn theme_inheritance() {
        let mut themes = HashMap::new();
        themes.insert("dark_fantasy".to_string(), dark_fantasy_theme());

        let mut child_roles = HashMap::new();
        child_roles.insert("accent".to_string(), "r".to_string()); // override accent

        themes.insert(
            "blood_theme".to_string(),
            Theme {
                palette: "dungeon".to_string(),
                scale: None,  // inherit from parent
                canvas: None, // inherit from parent
                max_palette_size: None,
                light_source: None,
                extends: Some("dark_fantasy".to_string()),
                roles: child_roles,
                constraints: HashMap::new(),
            },
        );

        let mut palettes = HashMap::new();
        palettes.insert("dungeon".to_string(), dungeon_palette());

        let resolved = resolve_theme("blood_theme", &themes, &palettes).unwrap();
        assert_eq!(resolved.roles["accent"], 'r'); // overridden
        assert_eq!(resolved.roles["fg"], '+'); // inherited
    }

    #[test]
    fn circular_extends_detected() {
        let mut themes = HashMap::new();
        themes.insert(
            "a".to_string(),
            Theme {
                palette: "dungeon".to_string(),
                scale: None,
                canvas: None,
                max_palette_size: None,
                light_source: None,
                extends: Some("b".to_string()),
                roles: HashMap::new(),
                constraints: HashMap::new(),
            },
        );
        themes.insert(
            "b".to_string(),
            Theme {
                palette: "dungeon".to_string(),
                scale: None,
                canvas: None,
                max_palette_size: None,
                light_source: None,
                extends: Some("a".to_string()),
                roles: HashMap::new(),
                constraints: HashMap::new(),
            },
        );
        let mut palettes = HashMap::new();
        palettes.insert("dungeon".to_string(), dungeon_palette());

        let err = resolve_theme("a", &themes, &palettes).unwrap_err();
        assert!(matches!(err, ThemeError::CircularExtends { .. }));
    }

    #[test]
    fn constraints_pass_on_dungeon() {
        let theme = dark_fantasy_theme();
        let palette = dungeon_palette();

        let mut themes = HashMap::new();
        themes.insert("dark_fantasy".to_string(), theme.clone());
        let mut palettes = HashMap::new();
        palettes.insert("dungeon".to_string(), palette.clone());

        let resolved = resolve_theme("dark_fantasy", &themes, &palettes).unwrap();
        let warnings = evaluate_constraints(&theme, &resolved, &palette);
        assert!(
            warnings.is_empty(),
            "expected no constraint warnings for dungeon palette, got: {:?}",
            warnings
        );
    }

    #[test]
    fn luminance_ordering() {
        let dark = Rgba {
            r: 42,
            g: 31,
            b: 61,
            a: 255,
        };
        let light = Rgba {
            r: 74,
            g: 58,
            b: 109,
            a: 255,
        };
        let shadow = Rgba {
            r: 26,
            g: 15,
            b: 46,
            a: 255,
        };
        assert!(luminance(&light) > luminance(&dark));
        assert!(luminance(&shadow) < luminance(&dark));
    }
}
