//! PAX-L deserializer — parses compact PAX-L text into a PaxFile.
//!
//! Supports strict mode (reject errors) and lenient mode (warn + auto-fix).

use super::PaxlError;
use crate::types::*;
use std::collections::HashMap;

/// Parse PAX-L text into a PaxFile.
///
/// In strict mode, any structural error is fatal.
/// In lenient mode, common LLM mistakes (off-by-one rows, extra blanks) are
/// accepted with warnings.
pub fn from_paxl(source: &str, strict: bool) -> Result<(PaxFile, Vec<String>), PaxlError> {
    let mut parser = PaxlParser::new(source, strict);
    parser.parse()
}

struct PaxlParser<'a> {
    lines: Vec<(usize, &'a str)>, // (1-indexed line number, text)
    cursor: usize,
    strict: bool,
    warnings: Vec<String>,
    default_palette: Option<String>,
}

impl<'a> PaxlParser<'a> {
    fn new(source: &'a str, strict: bool) -> Self {
        let lines: Vec<(usize, &str)> = source
            .lines()
            .enumerate()
            .map(|(i, l)| (i + 1, l))
            .collect();
        Self {
            lines,
            cursor: 0,
            strict,
            warnings: Vec::new(),
            default_palette: None,
        }
    }

    fn parse(&mut self) -> Result<(PaxFile, Vec<String>), PaxlError> {
        let mut file = PaxFile {
            pax: Header {
                version: "2.1".to_string(),
                name: String::new(),
                author: String::new(),
                created: None,
                theme: None,
                color_profile: None,
            },
            theme: HashMap::new(),
            palette: HashMap::new(),
            palette_swap: HashMap::new(),
            cycle: HashMap::new(),
            stamp: HashMap::new(),
            tile: HashMap::new(),
            spriteset: HashMap::new(),
            object: HashMap::new(),
            tile_run: HashMap::new(),
            wfc_rules: None,
            atlas: None,
            anim_clock: HashMap::new(),
            tilemap: HashMap::new(),
            palette_ext: HashMap::new(),
            backdrop_tile: HashMap::new(),
            backdrop: HashMap::new(),
            composite: HashMap::new(),
            style: HashMap::new(),
        };

        while self.cursor < self.lines.len() {
            let (line_no, line) = self.lines[self.cursor];
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") {
                self.cursor += 1;
                continue;
            }

            // Indented lines outside a block context → skip with warning
            if !trimmed.starts_with('@') && line.starts_with(' ') {
                self.cursor += 1;
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("@pax ") {
                self.parse_header(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@theme ") {
                self.parse_theme(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@roles") {
                // Roles are attached to the last theme — store as metadata
                self.parse_roles(&mut file, rest.trim())?;
            } else if let Some(rest) = trimmed.strip_prefix("@constraints") {
                self.parse_constraints(&mut file, rest.trim())?;
            } else if let Some(rest) = trimmed.strip_prefix("@style ") {
                self.parse_style(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@pal_ext ") {
                self.parse_palette_ext(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@pal ") {
                self.parse_palette(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@swap ") {
                self.parse_swap(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@cycle ") {
                self.parse_cycle(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@clock ") {
                self.parse_clock(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@stamp ") {
                self.parse_stamp(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@tile ") {
                self.parse_tile(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@wfc") {
                self.parse_wfc(&mut file, rest.trim())?;
            } else if let Some(rest) = trimmed.strip_prefix("@atlas ") {
                self.parse_atlas(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@run ") {
                self.parse_run(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@spriteset ") {
                self.parse_spriteset(&mut file, rest)?;
            } else if trimmed.starts_with("@sprite ")
                || trimmed.starts_with("@frame ")
                || trimmed.starts_with("@tags ")
            {
                // Nested sprite directives handled inside parse_spriteset
                self.skip_body_lines();
            } else if let Some(rest) = trimmed.strip_prefix("@composite ") {
                self.parse_composite(&mut file, rest)?;
            } else if trimmed.starts_with("@variant ")
                || trimmed.starts_with("@offset ")
                || trimmed.starts_with("@anim ")
                || trimmed.starts_with("@f ")
            {
                // Nested composite directives handled inside parse_composite
                self.skip_body_lines();
            } else if let Some(rest) = trimmed.strip_prefix("@object ") {
                self.parse_object(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@tilemap ") {
                self.parse_tilemap(&mut file, rest)?;
            } else if trimmed.starts_with("@layer ") {
                // Nested layer directives handled inside parse_tilemap
                self.skip_body_lines();
            } else if let Some(rest) = trimmed.strip_prefix("@bgtile ") {
                self.parse_backdrop_tile(&mut file, rest)?;
            } else if let Some(rest) = trimmed.strip_prefix("@backdrop ") {
                self.parse_backdrop(&mut file, rest)?;
            } else if trimmed.starts_with("@blayer ")
                || trimmed.starts_with("@zone ")
            {
                // Nested backdrop directives handled inside parse_backdrop
                self.skip_body_lines();
            } else {
                // Unknown directive — warn in lenient, error in strict
                if self.strict {
                    return Err(PaxlError::Parse {
                        line: line_no,
                        message: format!("unknown directive: {}", trimmed),
                    });
                }
                self.warnings
                    .push(format!("line {}: skipping unknown: {}", line_no, trimmed));
            }
            self.cursor += 1;
        }

        Ok((file, self.warnings.clone()))
    }

    /// Skip indented body lines following a directive (for unimplemented parsers).
    fn skip_body_lines(&mut self) {
        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            if !next_line.starts_with(' ') || next_line.trim().starts_with('@') {
                break;
            }
            self.cursor += 1;
        }
    }

    // ── Header ─────────────────────────────────────────────

    fn parse_header(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() >= 2 {
            file.pax.name = parts[0].to_string();
            file.pax.version = parts[1].to_string();
        }
        for part in parts.iter().skip(2) {
            if *part == "L1" {
                continue; // PAX-L version marker, acknowledged
            }
            if let Some(val) = part.strip_prefix("author=") {
                file.pax.author = val.to_string();
            }
            if let Some(val) = part.strip_prefix("profile=") {
                file.pax.color_profile = Some(val.to_string());
            }
        }
        Ok(())
    }

    // ── Theme ──────────────────────────────────────────────

    fn parse_theme(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }
        let name = parts[0].to_string();
        let palette = parts.get(1).unwrap_or(&"").to_string();
        self.default_palette = Some(palette.clone());

        let mut theme = Theme {
            palette,
            scale: None,
            canvas: None,
            max_palette_size: None,
            light_source: None,
            extends: None,
            roles: HashMap::new(),
            constraints: HashMap::new(),
        };

        for part in parts.iter().skip(2) {
            if let Some(val) = part.strip_prefix('s') {
                if let Ok(n) = val.parse::<u32>() {
                    theme.scale = Some(n);
                }
            } else if let Some(val) = part.strip_prefix('c') {
                if let Ok(n) = val.parse::<u32>() {
                    theme.canvas = Some(n);
                }
            } else if let Some(val) = part.strip_prefix('p') {
                if let Ok(n) = val.parse::<u32>() {
                    theme.max_palette_size = Some(n);
                }
            } else if let Some(ext) = part.strip_prefix(':') {
                theme.extends = Some(ext.to_string());
            } else {
                // Light source abbreviation
                let ls = match *part {
                    "tl" => "top-left",
                    "tr" => "top-right",
                    "bl" => "bottom-left",
                    "br" => "bottom-right",
                    "t" => "top",
                    "l" => "left",
                    other => other,
                };
                theme.light_source = Some(ls.to_string());
            }
        }

        file.pax.theme = Some(name.clone());
        file.theme.insert(name, theme);
        Ok(())
    }

    // ── Roles ──────────────────────────────────────────────

    fn parse_roles(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        // Find the last theme and attach roles to it
        let theme_name = file.pax.theme.clone().unwrap_or_default();
        if let Some(theme) = file.theme.get_mut(&theme_name) {
            for token in rest.split_whitespace() {
                if token.len() >= 2 {
                    let sym = &token[..1];
                    let role = &token[1..];
                    theme.roles.insert(role.to_string(), sym.to_string());
                }
            }
        }
        Ok(())
    }

    // ── Constraints ────────────────────────────────────────

    fn parse_constraints(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let theme_name = file.pax.theme.clone().unwrap_or_default();
        if let Some(theme) = file.theme.get_mut(&theme_name) {
            for token in rest.split_whitespace() {
                if token.contains("fg>bg") {
                    theme.constraints.insert(
                        "fg_brighter_than_bg".to_string(),
                        toml::Value::Boolean(true),
                    );
                } else if token.contains("shadow<bg") {
                    theme.constraints.insert(
                        "shadow_darker_than_bg".to_string(),
                        toml::Value::Boolean(true),
                    );
                } else if token.contains("accent!=bg") {
                    theme.constraints.insert(
                        "accent_hue_distinct_from_bg".to_string(),
                        toml::Value::Boolean(true),
                    );
                } else if let Some(val) = token.strip_prefix("max_colors=") {
                    if let Ok(n) = val.parse::<i64>() {
                        theme.constraints.insert(
                            "max_colors_per_tile".to_string(),
                            toml::Value::Integer(n),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    // ── Style ──────────────────────────────────────────────

    fn parse_style(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"default").to_string();

        let mut style = crate::style::StyleLatent::default();
        for part in parts.iter().skip(1) {
            if let Some((key, val)) = part.split_once('=') {
                match key {
                    "light" => style.light_direction = val.parse().unwrap_or(0.15),
                    "run" => style.run_length_mean = val.parse().unwrap_or(3.0),
                    "shadow" => style.shadow_ratio = val.parse().unwrap_or(0.3),
                    "breadth" => style.palette_breadth = val.parse().unwrap_or(4.0),
                    "density" => style.pixel_density = val.parse().unwrap_or(0.8),
                    "entropy" => style.palette_entropy = val.parse().unwrap_or(2.0),
                    "hue" => style.hue_bias = val.parse().unwrap_or(0.0),
                    "lum" => style.luminance_mean = val.parse().unwrap_or(0.4),
                    _ => {}
                }
            }
        }
        file.style.insert(name, style);
        Ok(())
    }

    // ── Palette ────────────────────────────────────────────

    fn parse_palette(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let name = rest.trim().to_string();
        let mut palette = PaletteRaw::new();

        // Collect indented continuation lines
        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            if !next_line.starts_with(' ') || next_line.trim().starts_with('@') {
                break;
            }
            self.cursor += 1;

            // Parse pairs: "sym hex sym hex ..."
            let tokens: Vec<&str> = next_line.split_whitespace().collect();
            let mut i = 0;
            while i + 1 < tokens.len() {
                palette.insert(tokens[i].to_string(), tokens[i + 1].to_string());
                i += 2;
            }
        }

        if self.default_palette.is_none() {
            self.default_palette = Some(name.clone());
        }
        file.palette.insert(name, palette);
        Ok(())
    }

    // ── Palette ext ────────────────────────────────────────

    fn parse_palette_ext(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let base = parts
            .get(1)
            .unwrap_or(&"")
            .strip_prefix(':')
            .unwrap_or("")
            .to_string();

        let mut symbols = HashMap::new();
        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            if !next_line.starts_with(' ') || next_line.trim().starts_with('@') {
                break;
            }
            self.cursor += 1;
            let tokens: Vec<&str> = next_line.split_whitespace().collect();
            let mut i = 0;
            while i + 1 < tokens.len() {
                symbols.insert(tokens[i].to_string(), tokens[i + 1].to_string());
                i += 2;
            }
        }

        file.palette_ext
            .insert(name, PaletteExtRaw { base, symbols });
        Ok(())
    }

    // ── Swap ───────────────────────────────────────────────

    fn parse_swap(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let base = parts.get(1).unwrap_or(&"").to_string();

        let mut swap = PaletteSwap {
            base,
            target: None,
            partial: false,
            map: HashMap::new(),
        };

        for part in parts.iter().skip(2) {
            if *part == "partial" {
                swap.partial = true;
            } else if let Some(val) = part.strip_prefix("target=") {
                swap.target = Some(val.to_string());
            } else if let Some((sym, hex)) = part.split_once('=') {
                swap.map.insert(sym.to_string(), hex.to_string());
            }
        }

        file.palette_swap.insert(name, swap);
        Ok(())
    }

    // ── Cycle ──────────────────────────────────────────────

    fn parse_cycle(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() < 4 {
            return Ok(());
        }
        let name = parts[0].to_string();
        let palette = parts[1].to_string();
        let symbols: Vec<String> = parts[2].split(',').map(|s| s.to_string()).collect();
        let direction = parts[3].to_string();
        let fps = parts
            .get(4)
            .and_then(|p| p.strip_suffix("fps"))
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(8);

        file.cycle.insert(
            name,
            Cycle {
                palette,
                symbols,
                direction,
                fps,
            },
        );
        Ok(())
    }

    // ── Clock ──────────────────────────────────────────────

    fn parse_clock(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let mut clock = AnimClock {
            fps: 6,
            frames: 4,
            mode: "loop".to_string(),
        };
        for part in parts.iter().skip(1) {
            if let Some(val) = part.strip_prefix("fps=") {
                clock.fps = val.parse().unwrap_or(6);
            } else if let Some(val) = part.strip_prefix("frames=") {
                clock.frames = val.parse().unwrap_or(4);
            } else if *part == "loop" || *part == "ping-pong" {
                clock.mode = part.to_string();
            }
        }
        file.anim_clock.insert(name, clock);
        Ok(())
    }

    // ── Stamp ──────────────────────────────────────────────

    fn parse_stamp(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let size = parts.get(1).unwrap_or(&"4x4").to_string();
        let palette = parts
            .iter()
            .find_map(|p| p.strip_prefix("pal="))
            .map(|s| s.to_string())
            .or_else(|| self.default_palette.clone())
            .unwrap_or_default();

        let mut grid_lines = Vec::new();
        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            if !next_line.starts_with(' ') || next_line.trim().starts_with('@') {
                break;
            }
            self.cursor += 1;
            let trimmed = next_line.trim();
            // Stamps can use | separator on one line
            if trimmed.contains('|') {
                for part in trimmed.split('|') {
                    grid_lines.push(part.to_string());
                }
            } else {
                grid_lines.push(trimmed.to_string());
            }
        }

        let grid = grid_lines.join("\n");
        file.stamp.insert(
            name,
            StampRaw {
                palette,
                size,
                grid,
            },
        );
        Ok(())
    }

    // ── Tile ───────────────────────────────────────────────

    fn parse_tile(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        let name = parts[0].to_string();

        // Check for template syntax: @tile name :base_tile
        if let Some(second) = parts.get(1) {
            if let Some(base) = second.strip_prefix(':') {
                return self.parse_template_tile(file, &name, base, &parts[2..]);
            }
        }

        // Parse size[row_count]
        let size_spec = parts.get(1).unwrap_or(&"16x16");
        let (size_str, expected_rows) = parse_size_marker(size_spec);

        let mut tile = TileRaw {
            palette: self
                .default_palette
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            size: Some(size_str.to_string()),
            encoding: None,
            symmetry: None,
            auto_rotate: None,
            auto_rotate_weight: None,
            template: None,
            edge_class: None,
            corner_class: None,
            tags: vec![],
            target_layer: None,
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: None,
            grid: None,
            rle: None,
            layout: None,
            fill: None,
            fill_size: None,
            delta: None,
            patches: vec![],
        };

        let mut is_compose = false;

        // Parse inline metadata
        for part in parts.iter().skip(2) {
            if let Some(val) = part.strip_prefix("pal=") {
                tile.palette = val.to_string();
            } else if let Some(val) = part.strip_prefix("e=") {
                tile.edge_class = Some(parse_edge_spec(val));
            } else if let Some(val) = part.strip_prefix('w') {
                if let Ok(w) = val.parse::<f64>() {
                    tile.weight = w;
                }
            } else if let Some(val) = part.strip_prefix("sym=") {
                tile.symmetry = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("rot=") {
                tile.auto_rotate = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("col=") {
                let sem = tile.semantic.get_or_insert_with(|| SemanticRaw {
                    affordance: None,
                    collision: Some(val.to_string()),
                    collision_points: None,
                    tags: HashMap::new(),
                });
                sem.collision = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("tags:") {
                tile.tags = val.split(',').map(|s| s.to_string()).collect();
            } else if let Some(val) = part.strip_prefix("swaps:") {
                tile.palette_swaps = val.split(',').map(|s| s.to_string()).collect();
            } else if let Some(val) = part.strip_prefix("cycles:") {
                tile.cycles = val.split(',').map(|s| s.to_string()).collect();
            } else if *part == "compose" {
                is_compose = true;
            } else if *part == "obstacle" || *part == "walkable" || *part == "hazard"
                || *part == "portal" || *part == "interactive"
            {
                let sem = tile.semantic.get_or_insert_with(|| SemanticRaw {
                    affordance: Some(part.to_string()),
                    collision: None,
                    collision_points: None,
                    tags: HashMap::new(),
                });
                sem.affordance = Some(part.to_string());
                // Infer default collision
                if sem.collision.is_none() {
                    sem.collision = Some(
                        match *part {
                            "obstacle" => "full",
                            "walkable" | "portal" | "interactive" => "none",
                            "hazard" => "full",
                            _ => "none",
                        }
                        .to_string(),
                    );
                }
            }
        }

        // Collect body lines
        let mut body_lines = Vec::new();
        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            let trimmed = next_line.trim();
            // Body lines can contain @ for compose (@stamp refs), @delta, @fill
            // Only break on @ if it's a top-level directive (not indented)
            if trimmed.starts_with('@') && !next_line.starts_with(' ') {
                break;
            }
            // Indented @ lines are part of the body (compose stamp refs)
            if trimmed.starts_with('@') && next_line.starts_with(' ') {
                // OK — compose stamp reference like "  @brick_2x2 @brick_2x2"
            } else if !next_line.starts_with(' ') && !trimmed.is_empty() {
                break;
            }
            if !next_line.starts_with(' ') && !trimmed.is_empty() {
                break;
            }
            if trimmed.is_empty() {
                self.cursor += 1;
                continue;
            }
            self.cursor += 1;
            body_lines.push(trimmed.to_string());
        }

        // Parse semantic tags line
        let sem_lines: Vec<String> = body_lines
            .iter()
            .filter(|l| l.starts_with("sem:"))
            .cloned()
            .collect();
        let grid_lines: Vec<String> = body_lines
            .into_iter()
            .filter(|l| !l.starts_with("sem:"))
            .collect();

        for sem_line in &sem_lines {
            let rest = sem_line.strip_prefix("sem:").unwrap_or("").trim();
            let sem = tile.semantic.get_or_insert_with(|| SemanticRaw {
                affordance: None,
                collision: None,
                collision_points: None,
                tags: HashMap::new(),
            });
            for token in rest.split_whitespace() {
                if let Some((key, val)) = token.split_once('=') {
                    sem.tags
                        .insert(key.to_string(), toml::Value::String(val.to_string()));
                } else {
                    sem.tags
                        .insert(token.to_string(), toml::Value::Boolean(true));
                }
            }
        }

        // Determine encoding from body content
        if let Some(first) = grid_lines.first() {
            if first.starts_with("@delta ") {
                // Delta encoding
                let base = first
                    .strip_prefix("@delta ")
                    .unwrap_or("")
                    .trim()
                    .to_string();
                tile.delta = Some(base);
                for line in grid_lines.iter().skip(1) {
                    // Parse "+x,y sym" patches — split on '+' boundaries
                    let parts: Vec<&str> = line.split('+').collect();
                    for part in parts.iter().skip(1) {
                        let tokens: Vec<&str> = part.split_whitespace().collect();
                        if tokens.len() >= 2 {
                            let coord = tokens[0].trim_end_matches(',');
                            if let Some((x, y)) = coord.split_once(',') {
                                tile.patches.push(PatchRaw {
                                    x: x.parse().unwrap_or(0),
                                    y: y.parse().unwrap_or(0),
                                    sym: tokens[1].to_string(),
                                });
                            }
                        }
                    }
                }
            } else if first.starts_with("@fill ") {
                // Fill encoding
                let fill_spec = first.strip_prefix("@fill ").unwrap_or("4x4").trim();
                tile.fill_size = Some(fill_spec.to_string());
                tile.encoding = Some("fill".to_string());
                let pattern: Vec<&str> = grid_lines[1..]
                    .iter()
                    .map(|l| l.as_str())
                    .collect();
                tile.fill = Some(pattern.join("\n"));
            } else if is_compose || first.contains('@') {
                // Compose encoding
                tile.encoding = Some("compose".to_string());
                tile.layout = Some(grid_lines.join("\n"));
            } else {
                // Grid encoding (with possible =N row refs)
                tile.grid = Some(grid_lines.join("\n"));
            }
        }

        // Validate row count against [N] marker
        // For fill/delta, the @fill/@delta directive line doesn't count
        if let Some(expected) = expected_rows {
            let actual = if grid_lines.first().map_or(false, |l| {
                l.starts_with("@fill ") || l.starts_with("@delta ")
            }) {
                (grid_lines.len().saturating_sub(1)) as u32
            } else {
                grid_lines.len() as u32
            };
            if actual != expected {
                if self.strict {
                    return Err(PaxlError::RowCountMismatch {
                        tile: name.clone(),
                        declared: expected as usize,
                        actual: actual as usize,
                    });
                } else {
                    self.warnings.push(format!(
                        "tile '{}': expected {} rows (from [{}]), got {}",
                        name, expected, expected, actual
                    ));
                }
            }
        }

        file.tile.insert(name, tile);
        Ok(())
    }

    fn parse_template_tile(
        &mut self,
        file: &mut PaxFile,
        name: &str,
        base: &str,
        extra_parts: &[&str],
    ) -> Result<(), PaxlError> {
        let mut tile = TileRaw {
            palette: self
                .default_palette
                .clone()
                .unwrap_or_else(|| "default".to_string()),
            size: None,
            encoding: None,
            symmetry: None,
            auto_rotate: None,
            auto_rotate_weight: None,
            template: Some(base.to_string()),
            edge_class: None,
            corner_class: None,
            tags: vec![],
            target_layer: None,
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: None,
            grid: None,
            rle: None,
            layout: None,
            fill: None,
            fill_size: None,
            delta: None,
            patches: vec![],
        };

        for part in extra_parts {
            if let Some(val) = part.strip_prefix("pal=") {
                tile.palette = val.to_string();
            } else if let Some(val) = part.strip_prefix("e=") {
                tile.edge_class = Some(parse_edge_spec(val));
            } else if let Some(val) = part.strip_prefix("swaps:") {
                tile.palette_swaps = val.split(',').map(|s| s.to_string()).collect();
            }
        }

        file.tile.insert(name.to_string(), tile);
        Ok(())
    }

    // ── WFC ────────────────────────────────────────────────

    fn parse_wfc(&mut self, file: &mut PaxFile, _rest: &str) -> Result<(), PaxlError> {
        let mut rules = WfcRules {
            forbids: vec![],
            requires: vec![],
            require_boost: 3.0,
            variant_groups: HashMap::new(),
            subcomplete: false,
        };

        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            if !next_line.starts_with(' ') || next_line.trim().starts_with('@') {
                break;
            }
            self.cursor += 1;
            let trimmed = next_line.trim();

            if let Some(rest) = trimmed.strip_prefix("forbid ") {
                rules.forbids.push(expand_wfc_rule(rest));
            } else if let Some(rest) = trimmed.strip_prefix("require ") {
                // Check for boost= parameter
                let (rule, boost) = extract_boost(rest);
                if let Some(b) = boost {
                    rules.require_boost = b;
                }
                rules.requires.push(expand_wfc_rule(&rule));
            } else if let Some(rest) = trimmed.strip_prefix("group ") {
                if let Some((gname, members)) = rest.split_once(':') {
                    let member_list: Vec<String> = members
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                    rules
                        .variant_groups
                        .insert(gname.trim().to_string(), member_list);
                }
            }
        }

        file.wfc_rules = Some(rules);
        Ok(())
    }

    // ── Atlas ──────────────────────────────────────────────

    fn parse_atlas(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let format = parts.first().unwrap_or(&"texturepacker").to_string();

        let mut atlas = AtlasConfig {
            format,
            padding: 1,
            scale: 1,
            columns: 8,
            include: vec![],
            output: String::new(),
            map_output: None,
        };

        for part in parts.iter().skip(1) {
            if let Some(val) = part.strip_prefix("pad=") {
                atlas.padding = val.parse().unwrap_or(1);
            } else if let Some(val) = part.strip_prefix('s') {
                if let Ok(n) = val.parse::<u32>() {
                    atlas.scale = n;
                }
            } else if let Some(val) = part.strip_prefix("cols=") {
                atlas.columns = val.parse().unwrap_or(8);
            }
        }

        // Collect body lines
        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            if !next_line.starts_with(' ') || next_line.trim().starts_with('@') {
                break;
            }
            self.cursor += 1;
            let trimmed = next_line.trim();
            if let Some(rest) = trimmed.strip_prefix("include ") {
                atlas.include = rest.split_whitespace().map(|s| s.to_string()).collect();
            } else if let Some(rest) = trimmed.strip_prefix("out ") {
                let out_parts: Vec<&str> = rest.split_whitespace().collect();
                atlas.output = out_parts.first().unwrap_or(&"").to_string();
                if let Some(map_rest) = rest.split_once("map ") {
                    atlas.map_output = Some(map_rest.1.trim().to_string());
                }
            }
        }

        file.atlas = Some(atlas);
        Ok(())
    }

    // ── Tile Run ───────────────────────────────────────────

    // ── Spriteset ──────────────────────────────────────────

    fn parse_spriteset(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let size = parts.get(1).unwrap_or(&"16x16").to_string();
        let palette = parts
            .iter()
            .find_map(|p| p.strip_prefix("pal="))
            .map(|s| s.to_string())
            .or_else(|| self.default_palette.clone())
            .unwrap_or_default();
        let swaps: Vec<String> = parts
            .iter()
            .find_map(|p| p.strip_prefix("swaps:"))
            .map(|s| s.split(',').map(|x| x.to_string()).collect())
            .unwrap_or_default();

        let mut ss = SpritesetRaw {
            palette,
            size,
            palette_swaps: swaps,
            cycles: vec![],
            sprite: vec![],
        };

        // Parse nested @sprite directives
        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let trimmed = next.trim();
            if let Some(sprite_rest) = trimmed.strip_prefix("@sprite ") {
                self.cursor += 1;
                let sprite = self.parse_sprite(&name, sprite_rest)?;
                ss.sprite.push(sprite);
            } else if trimmed.starts_with('@') && !trimmed.starts_with("@sprite") {
                break; // Next top-level directive
            } else if next.starts_with(' ') || trimmed.is_empty() {
                self.cursor += 1; // Skip body/blank
            } else {
                break;
            }
        }

        file.spriteset.insert(name, ss);
        Ok(())
    }

    fn parse_sprite(&mut self, _ss_name: &str, rest: &str) -> Result<SpriteRaw, PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        // name is "ssname.spritename" — extract sprite name after dot
        let full_name = parts.first().unwrap_or(&"");
        let sprite_name = full_name
            .split('.')
            .nth(1)
            .unwrap_or(full_name)
            .to_string();
        let fps = parts
            .iter()
            .find_map(|p| p.strip_prefix("fps="))
            .and_then(|v| v.parse().ok())
            .unwrap_or(8);
        let is_loop = parts.contains(&"loop");
        let scale = parts
            .iter()
            .find_map(|p| p.strip_prefix("scale="))
            .and_then(|v| v.parse().ok());

        let mut frames = Vec::new();
        let mut tags = Vec::new();

        // Parse nested @frame and @tags
        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let trimmed = next.trim();
            if let Some(frame_rest) = trimmed.strip_prefix("@frame ") {
                self.cursor += 1;
                let frame = self.parse_frame(frame_rest)?;
                frames.push(frame);
            } else if let Some(tags_rest) = trimmed.strip_prefix("@tags ") {
                self.cursor += 1;
                if let Some(tag) = parse_anim_tag(tags_rest) {
                    tags.push(tag);
                }
            } else if trimmed.starts_with("@sprite ") || (trimmed.starts_with('@') && !next.starts_with(' ')) {
                break;
            } else if next.starts_with(' ') || trimmed.is_empty() {
                self.cursor += 1;
            } else {
                break;
            }
        }

        Ok(SpriteRaw {
            name: sprite_name,
            fps,
            r#loop: is_loop,
            scale,
            frames,
            tags,
        })
    }

    fn parse_frame(&mut self, rest: &str) -> Result<FrameRaw, PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let index = parts
            .first()
            .and_then(|p| p.parse().ok())
            .unwrap_or(1);

        let duration_ms = parts
            .iter()
            .find_map(|p| p.strip_prefix("ms="))
            .and_then(|v| v.parse().ok());
        let mirror = parts
            .iter()
            .find_map(|p| p.strip_prefix("mirror="))
            .map(|s| s.to_string());

        // Detect encoding
        if let Some(link_str) = parts.iter().find_map(|p| p.strip_prefix("link=")) {
            return Ok(FrameRaw {
                index,
                encoding: Some("linked".to_string()),
                grid: None,
                base: None,
                changes: vec![],
                link_to: link_str.parse().ok(),
                duration_ms,
                mirror,
            });
        }

        if let Some(delta_str) = parts.iter().find_map(|p| p.strip_prefix("delta=")) {
            let base: Option<u32> = delta_str.parse().ok();
            let mut changes = Vec::new();
            // Collect delta changes from body
            while self.cursor + 1 < self.lines.len() {
                let (_, next) = self.lines[self.cursor + 1];
                let t = next.trim();
                if !next.starts_with(' ') || t.starts_with('@') {
                    break;
                }
                if t.is_empty() {
                    self.cursor += 1;
                    continue;
                }
                self.cursor += 1;
                // Parse "+x,y sym" tokens
                for part in t.split('+').skip(1) {
                    let tokens: Vec<&str> = part.split_whitespace().collect();
                    if tokens.len() >= 2 {
                        let coord = tokens[0].trim_end_matches(',');
                        if let Some((x, y)) = coord.split_once(',') {
                            changes.push(DeltaChange {
                                x: x.parse().unwrap_or(0),
                                y: y.parse().unwrap_or(0),
                                sym: tokens[1].to_string(),
                            });
                        }
                    }
                }
            }
            return Ok(FrameRaw {
                index,
                encoding: Some("delta".to_string()),
                grid: None,
                base,
                changes,
                link_to: None,
                duration_ms,
                mirror,
            });
        }

        // Grid encoding — collect body lines
        let mut grid_lines = Vec::new();
        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let t = next.trim();
            if !next.starts_with(' ') || t.starts_with('@') {
                break;
            }
            if t.is_empty() {
                self.cursor += 1;
                continue;
            }
            self.cursor += 1;
            grid_lines.push(t.to_string());
        }

        Ok(FrameRaw {
            index,
            encoding: Some("grid".to_string()),
            grid: if grid_lines.is_empty() {
                None
            } else {
                Some(grid_lines.join("\n"))
            },
            base: None,
            changes: vec![],
            link_to: None,
            duration_ms,
            mirror,
        })
    }

    // ── Composite ─────────────────────────────────────────

    fn parse_composite(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let size = parts.get(1).unwrap_or(&"32x32").to_string();
        let tile_size = parts
            .iter()
            .find_map(|p| p.strip_prefix("tile="))
            .unwrap_or("16x16")
            .to_string();

        // Collect layout body
        let mut layout_lines = Vec::new();
        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let t = next.trim();
            if t.starts_with('@') && !next.starts_with("    ") {
                break;
            }
            if !next.starts_with(' ') && !t.is_empty() {
                break;
            }
            if t.is_empty() {
                self.cursor += 1;
                continue;
            }
            self.cursor += 1;
            layout_lines.push(t.to_string());
        }

        let mut comp = CompositeRaw {
            size,
            tile_size,
            layout: layout_lines.join("\n"),
            offset: HashMap::new(),
            variant: HashMap::new(),
            anim: HashMap::new(),
        };

        // Parse nested @variant, @offset, @anim directives
        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let t = next.trim();
            if let Some(vrest) = t.strip_prefix("@variant ") {
                self.cursor += 1;
                let vparts: Vec<&str> = vrest.split_whitespace().collect();
                let vname = vparts
                    .first()
                    .unwrap_or(&"")
                    .split('.')
                    .nth(1)
                    .unwrap_or("")
                    .to_string();
                let mut slot = HashMap::new();
                for p in vparts.iter().skip(1) {
                    if let Some((k, v)) = p.split_once('=') {
                        slot.insert(k.to_string(), v.to_string());
                    }
                }
                comp.variant
                    .insert(vname, CompositeVariantRaw { slot });
            } else if let Some(orest) = t.strip_prefix("@offset ") {
                self.cursor += 1;
                let oparts: Vec<&str> = orest.split_whitespace().collect();
                for p in oparts.iter().skip(1) {
                    if let Some((k, v)) = p.split_once("=[") {
                        let v = v.trim_end_matches(']');
                        let coords: Vec<i32> = v
                            .split(',')
                            .filter_map(|n| n.trim().parse().ok())
                            .collect();
                        if coords.len() == 2 {
                            comp.offset.insert(k.to_string(), coords);
                        }
                    }
                }
            } else if let Some(arest) = t.strip_prefix("@anim ") {
                self.cursor += 1;
                let aparts: Vec<&str> = arest.split_whitespace().collect();
                let aname = aparts
                    .first()
                    .unwrap_or(&"")
                    .split('.')
                    .nth(1)
                    .unwrap_or("")
                    .to_string();
                let fps = aparts
                    .iter()
                    .find_map(|p| p.strip_prefix("fps="))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8);
                let is_loop = aparts.contains(&"loop");
                let source = aparts
                    .iter()
                    .find_map(|p| p.strip_prefix("source="))
                    .map(|s| s.to_string());
                let mirror = aparts
                    .iter()
                    .find_map(|p| p.strip_prefix("mirror="))
                    .map(|s| s.to_string());

                let mut frames = Vec::new();
                // Parse @f directives
                while self.cursor + 1 < self.lines.len() {
                    let (_, fnext) = self.lines[self.cursor + 1];
                    let ft = fnext.trim();
                    if let Some(frest) = ft.strip_prefix("@f ") {
                        self.cursor += 1;
                        let fparts: Vec<&str> = frest.split_whitespace().collect();
                        let fidx: u32 = fparts
                            .first()
                            .and_then(|p| p.parse().ok())
                            .unwrap_or(1);
                        let mut swap = HashMap::new();
                        let mut offset = HashMap::new();
                        for fp in &fparts[1..] {
                            if let Some(srest) = fp.strip_prefix("swap:") {
                                for pair in srest.split(',') {
                                    if let Some((k, v)) = pair.split_once('=') {
                                        swap.insert(k.to_string(), v.to_string());
                                    }
                                }
                            }
                            if let Some(orest) = fp.strip_prefix("offset:") {
                                for pair in orest.split(',') {
                                    if let Some((k, v)) = pair.split_once("=[") {
                                        let v = v.trim_end_matches(']');
                                        let coords: Vec<i32> = v
                                            .split(',')
                                            .filter_map(|n| n.trim().parse().ok())
                                            .collect();
                                        if coords.len() == 2 {
                                            offset.insert(k.to_string(), coords);
                                        }
                                    }
                                }
                            }
                        }
                        frames.push(CompositeFrameRaw {
                            index: fidx,
                            swap,
                            offset,
                        });
                    } else if ft.starts_with('@') || (!fnext.starts_with(' ') && !ft.is_empty()) {
                        break;
                    } else {
                        self.cursor += 1;
                    }
                }

                comp.anim.insert(
                    aname,
                    CompositeAnimRaw {
                        fps,
                        r#loop: is_loop,
                        source,
                        mirror,
                        frame: frames,
                    },
                );
            } else if t.starts_with('@') || (!next.starts_with(' ') && !t.is_empty()) {
                break;
            } else {
                self.cursor += 1;
            }
        }

        file.composite.insert(name, comp);
        Ok(())
    }

    // ── Object ────────────────────────────────────────────

    fn parse_object(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let size_tiles = parts.get(1).unwrap_or(&"1x1").to_string();
        let base_tile = parts
            .iter()
            .find_map(|p| p.strip_prefix("base="))
            .map(|s| s.to_string());
        let above: Vec<u32> = parts
            .iter()
            .find_map(|p| p.strip_prefix("above="))
            .map(|s| s.split(',').filter_map(|n| n.parse().ok()).collect())
            .unwrap_or_default();
        let below: Vec<u32> = parts
            .iter()
            .find_map(|p| p.strip_prefix("below="))
            .map(|s| s.split(',').filter_map(|n| n.parse().ok()).collect())
            .unwrap_or_default();

        let mut tile_lines = Vec::new();
        let mut collision = None;

        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let t = next.trim();
            if !next.starts_with(' ') || t.starts_with('@') && !t.starts_with("@col ") {
                break;
            }
            if t.is_empty() {
                self.cursor += 1;
                continue;
            }
            self.cursor += 1;
            if let Some(col_rest) = t.strip_prefix("@col ") {
                collision = Some(col_rest.replace('|', "\n"));
            } else {
                tile_lines.push(t.to_string());
            }
        }

        file.object.insert(
            name,
            ObjectRaw {
                size_tiles,
                base_tile,
                above_player_rows: above,
                below_player_rows: below,
                tiles: tile_lines.join("\n"),
                collision,
            },
        );
        Ok(())
    }

    // ── Tilemap ───────────────────────────────────────────

    fn parse_tilemap(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();

        // Parse "WxH" for tilemap dimensions
        let dims = parts.get(1).unwrap_or(&"10x10");
        let (width, height) = if let Some((w, h)) = dims.split_once('x') {
            (
                w.parse::<u32>().unwrap_or(10),
                h.parse::<u32>().unwrap_or(10),
            )
        } else {
            (10, 10)
        };

        // Parse "tile=WxH"
        let tile_spec = parts
            .iter()
            .find_map(|p| p.strip_prefix("tile="))
            .unwrap_or("16x16");
        let (tile_width, tile_height) = if let Some((w, h)) = tile_spec.split_once('x') {
            (
                w.parse::<u32>().unwrap_or(16),
                h.parse::<u32>().unwrap_or(16),
            )
        } else {
            (16, 16)
        };

        let mut layers = HashMap::new();

        // Parse @layer directives
        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let t = next.trim();
            if let Some(lrest) = t.strip_prefix("@layer ") {
                self.cursor += 1;
                let lparts: Vec<&str> = lrest.split_whitespace().collect();
                let lname = lparts.first().unwrap_or(&"").to_string();
                let z_order = lparts
                    .iter()
                    .find_map(|p| p.strip_prefix("z="))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0);
                let collision = lparts.contains(&"collision");
                let blend = lparts
                    .iter()
                    .find_map(|p| p.strip_prefix("blend="))
                    .unwrap_or("normal")
                    .to_string();

                let mut grid_lines = Vec::new();
                while self.cursor + 1 < self.lines.len() {
                    let (_, gnext) = self.lines[self.cursor + 1];
                    let gt = gnext.trim();
                    if !gnext.starts_with(' ') || gt.starts_with('@') {
                        break;
                    }
                    if gt.is_empty() {
                        self.cursor += 1;
                        continue;
                    }
                    self.cursor += 1;
                    grid_lines.push(gt.to_string());
                }

                layers.insert(
                    lname,
                    crate::tilemap::TilemapLayerRaw {
                        z_order,
                        blend,
                        collision,
                        collision_mode: None,
                        layer_role: None,
                        cycles: vec![],
                        scroll_factor: None,
                        grid: if grid_lines.is_empty() {
                            None
                        } else {
                            Some(grid_lines.join("\n"))
                        },
                    },
                );
            } else if t.starts_with('@') || (!next.starts_with(' ') && !t.is_empty()) {
                break;
            } else {
                self.cursor += 1;
            }
        }

        file.tilemap.insert(
            name,
            crate::tilemap::TilemapRaw {
                width,
                height,
                tile_width,
                tile_height,
                layer: layers,
                constraints: None,
                objects: vec![],
            },
        );
        Ok(())
    }

    // ── Backdrop Tile ─────────────────────────────────────

    fn parse_backdrop_tile(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let size = parts
            .get(1)
            .filter(|s| s.contains('x'))
            .map(|s| s.to_string());
        let palette = parts
            .iter()
            .find_map(|p| p.strip_prefix("pal="))
            .map(|s| s.to_string())
            .or_else(|| self.default_palette.clone())
            .unwrap_or_default();
        let palette_ext = parts
            .iter()
            .find_map(|p| p.strip_prefix("ext="))
            .map(|s| s.to_string());
        let is_rle = parts.contains(&"rle");

        let mut grid_lines = Vec::new();
        let mut animation = Vec::new();

        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let t = next.trim();
            if !next.starts_with(' ') || (t.starts_with('@') && !t.starts_with("@anim ")) {
                break;
            }
            if t.is_empty() {
                self.cursor += 1;
                continue;
            }
            self.cursor += 1;
            if let Some(anim_rest) = t.strip_prefix("@anim ") {
                for frame_spec in anim_rest.split_whitespace() {
                    if let Some((tile, ms_str)) = frame_spec.split_once(':') {
                        let ms = ms_str
                            .trim_end_matches("ms")
                            .parse::<u32>()
                            .unwrap_or(120);
                        animation.push(BackdropTileFrameRaw {
                            tile: tile.to_string(),
                            duration_ms: ms,
                        });
                    }
                }
            } else {
                grid_lines.push(t.to_string());
            }
        }

        let grid_str = if grid_lines.is_empty() {
            None
        } else {
            Some(grid_lines.join("\n"))
        };

        file.backdrop_tile.insert(
            name,
            BackdropTileRaw {
                palette,
                palette_ext,
                size,
                template: None,
                grid: if is_rle { None } else { grid_str.clone() },
                rle: if is_rle { grid_str } else { None },
                animation,
                anim_clock: None,
            },
        );
        Ok(())
    }

    // ── Backdrop ──────────────────────────────────────────

    fn parse_backdrop(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let size = parts.get(1).unwrap_or(&"160x160").to_string();
        let tile_size = parts
            .iter()
            .find_map(|p| p.strip_prefix("tile="))
            .unwrap_or("16x16")
            .to_string();
        let palette = parts
            .iter()
            .find_map(|p| p.strip_prefix("pal="))
            .map(|s| s.to_string())
            .or_else(|| self.default_palette.clone())
            .unwrap_or_default();
        let palette_ext = parts
            .iter()
            .find_map(|p| p.strip_prefix("ext="))
            .map(|s| s.to_string());

        let mut tilemap_lines = Vec::new();
        let mut layers = Vec::new();
        let mut zones = Vec::new();

        // Collect body: either single tilemap or @blayer/@zone
        while self.cursor + 1 < self.lines.len() {
            let (_, next) = self.lines[self.cursor + 1];
            let t = next.trim();

            if let Some(lrest) = t.strip_prefix("@blayer ") {
                self.cursor += 1;
                let lparts: Vec<&str> = lrest.split_whitespace().collect();
                let lname = lparts.first().unwrap_or(&"").to_string();
                let scroll_factor = lparts
                    .iter()
                    .find_map(|p| p.strip_prefix("scroll="))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0);
                let opacity = lparts
                    .iter()
                    .find_map(|p| p.strip_prefix("opacity="))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.0);
                let blend = lparts
                    .iter()
                    .find_map(|p| p.strip_prefix("blend="))
                    .unwrap_or("normal")
                    .to_string();

                let mut layer_grid = Vec::new();
                while self.cursor + 1 < self.lines.len() {
                    let (_, gnext) = self.lines[self.cursor + 1];
                    let gt = gnext.trim();
                    if !gnext.starts_with(' ') || gt.starts_with('@') {
                        break;
                    }
                    if gt.is_empty() {
                        self.cursor += 1;
                        continue;
                    }
                    self.cursor += 1;
                    layer_grid.push(gt.to_string());
                }

                layers.push(BackdropLayerRaw {
                    name: lname,
                    tilemap: layer_grid.join("\n"),
                    scroll_factor,
                    opacity,
                    blend,
                    offset_x: 0,
                    offset_y: 0,
                    fade: None,
                    scroll_lock: None,
                });
            } else if let Some(zrest) = t.strip_prefix("@zone ") {
                self.cursor += 1;
                let zparts: Vec<&str> = zrest.split_whitespace().collect();
                let zname = zparts.first().unwrap_or(&"").to_string();

                let mut zone = BackdropZoneRaw {
                    name: zname,
                    rect: ZoneRect { x: 0, y: 0, w: 0, h: 0 },
                    behavior: String::new(),
                    cycle: None,
                    speed: None,
                    wrap: None,
                    density: None,
                    seed: None,
                    phase_rows: None,
                    wave_dx: None,
                    layer: None,
                    amplitude: None,
                    period: None,
                    from: None,
                    to: None,
                    direction: None,
                    size_x: None,
                    size_y: None,
                    layers_visible: None,
                    blend_override: None,
                    opacity_override: None,
                    symbol: None,
                };

                // Parse behavior from remaining parts
                for p in &zparts[1..] {
                    if let Some(val) = p.strip_prefix("cycle=") {
                        zone.behavior = "cycle".to_string();
                        zone.cycle = Some(val.to_string());
                    } else if let Some(val) = p.strip_prefix("wave=") {
                        zone.behavior = "wave".to_string();
                        zone.cycle = Some(val.to_string());
                    } else if let Some(val) = p.strip_prefix("flicker=") {
                        zone.behavior = "flicker".to_string();
                        zone.cycle = Some(val.to_string());
                    } else if *p == "scroll_down" {
                        zone.behavior = "scroll_down".to_string();
                    } else if let Some(val) = p.strip_prefix("speed=") {
                        zone.speed = val.parse().ok();
                    } else if let Some(val) = p.strip_prefix("phase=") {
                        zone.phase_rows = val.parse().ok();
                    }
                }

                // Parse rect line
                while self.cursor + 1 < self.lines.len() {
                    let (_, rnext) = self.lines[self.cursor + 1];
                    let rt = rnext.trim();
                    if !rnext.starts_with(' ') || rt.starts_with('@') {
                        break;
                    }
                    if rt.is_empty() {
                        self.cursor += 1;
                        continue;
                    }
                    self.cursor += 1;
                    if let Some(rect_rest) = rt.strip_prefix("rect ") {
                        // "x,y WxH"
                        let rparts: Vec<&str> = rect_rest.split_whitespace().collect();
                        if let Some((xy, wh)) = rparts.first().zip(rparts.get(1)) {
                            if let (Some((x, y)), Some((w, h))) =
                                (xy.split_once(','), wh.split_once('x'))
                            {
                                zone.rect = ZoneRect {
                                    x: x.parse().unwrap_or(0),
                                    y: y.parse().unwrap_or(0),
                                    w: w.parse().unwrap_or(0),
                                    h: h.parse().unwrap_or(0),
                                };
                            }
                        }
                    }
                }

                zones.push(zone);
            } else if t.starts_with('@') || (!next.starts_with(' ') && !t.is_empty()) {
                break;
            } else if next.starts_with(' ') && !t.is_empty() {
                // Single-layer tilemap body
                self.cursor += 1;
                tilemap_lines.push(t.to_string());
            } else {
                self.cursor += 1;
            }
        }

        file.backdrop.insert(
            name,
            BackdropRaw {
                palette,
                palette_ext,
                size,
                tile_size,
                tilemap: if tilemap_lines.is_empty() {
                    None
                } else {
                    Some(tilemap_lines.join("\n"))
                },
                tags: vec![],
                zone: zones,
                layer: layers,
            },
        );
        Ok(())
    }

    // ── Tile Run ──────────────────────────────────────────

    fn parse_run(&mut self, file: &mut PaxFile, rest: &str) -> Result<(), PaxlError> {
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let name = parts.first().unwrap_or(&"").to_string();
        let orientation = parts.get(1).unwrap_or(&"horizontal").to_string();

        let mut run = TileRun {
            orientation,
            left: String::new(),
            middle: String::new(),
            right: String::new(),
            single: None,
        };

        // Collect body or inline params
        let search_parts = if parts.len() > 2 { &parts[2..] } else { &[] };
        // Also check next line
        let mut all_parts: Vec<String> = search_parts.iter().map(|s| s.to_string()).collect();
        while self.cursor + 1 < self.lines.len() {
            let (_, next_line) = self.lines[self.cursor + 1];
            if !next_line.starts_with(' ') || next_line.trim().starts_with('@') {
                break;
            }
            self.cursor += 1;
            all_parts.extend(next_line.split_whitespace().map(|s| s.to_string()));
        }

        for part in &all_parts {
            if let Some(val) = part.strip_prefix("left=") {
                run.left = val.to_string();
            } else if let Some(val) = part.strip_prefix("mid=") {
                run.middle = val.to_string();
            } else if let Some(val) = part.strip_prefix("right=") {
                run.right = val.to_string();
            } else if let Some(val) = part.strip_prefix("single=") {
                run.single = Some(val.to_string());
            }
        }

        file.tile_run.insert(name, run);
        Ok(())
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Parse "@tags blink 3-4" → AnimTagRaw { name: "blink", from: 3, to: 4 }
fn parse_anim_tag(rest: &str) -> Option<AnimTagRaw> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    if parts.len() >= 2 {
        let name = parts[0].to_string();
        if let Some((from, to)) = parts[1].split_once('-') {
            return Some(AnimTagRaw {
                name,
                from_frame: from.parse().ok()?,
                to_frame: to.parse().ok()?,
            });
        }
    }
    None
}

/// Parse "16x16[16]" → ("16x16", Some(16))
fn parse_size_marker(spec: &str) -> (&str, Option<u32>) {
    if let Some(bracket_pos) = spec.find('[') {
        let size = &spec[..bracket_pos];
        let count = spec[bracket_pos + 1..]
            .trim_end_matches(']')
            .parse::<u32>()
            .ok();
        (size, count)
    } else {
        (spec, None)
    }
}

/// Parse edge spec: "solid" → all same, "solid/floor/solid/floor" → N/E/S/W
fn parse_edge_spec(spec: &str) -> EdgeClassRaw {
    if spec.contains('/') {
        let parts: Vec<&str> = spec.split('/').collect();
        EdgeClassRaw {
            n: parts.first().unwrap_or(&"").to_string(),
            e: parts.get(1).unwrap_or(&"").to_string(),
            s: parts.get(2).unwrap_or(&"").to_string(),
            w: parts.get(3).unwrap_or(&"").to_string(),
        }
    } else {
        EdgeClassRaw {
            n: spec.to_string(),
            e: spec.to_string(),
            s: spec.to_string(),
            w: spec.to_string(),
        }
    }
}

/// Expand compact WFC rule: "obstacle~hazard adjacent" →
/// "affordance:obstacle forbids affordance:hazard adjacent"
fn expand_wfc_rule(compact: &str) -> String {
    let parts: Vec<&str> = compact.split_whitespace().collect();
    if parts.len() >= 2 {
        if let Some((left, right)) = parts[0].split_once('~') {
            let qualifier = parts.get(1).unwrap_or(&"adjacent");
            return format!(
                "affordance:{} forbids affordance:{} {}",
                left, right, qualifier
            );
        }
    }
    compact.to_string()
}

/// Extract boost= parameter from a require rule.
fn extract_boost(rule: &str) -> (String, Option<f64>) {
    if let Some(pos) = rule.find("boost=") {
        let before = rule[..pos].trim().to_string();
        let boost_val = rule[pos + 6..]
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<f64>().ok());
        (before, boost_val)
    } else {
        (rule.to_string(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paxl::serialize;
    use crate::parser::parse_pax;

    #[test]
    fn roundtrip_dungeon() {
        // Parse TOML → PaxFile → serialize to PAX-L → deserialize back
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let original = parse_pax(&source).unwrap();
        let config = crate::paxl::PaxlConfig::default();
        let paxl_text = serialize::serialize(&original, &config).unwrap();

        let (roundtripped, warnings) = from_paxl(&paxl_text, false).unwrap();

        // Verify key structural elements survived the roundtrip
        assert_eq!(roundtripped.pax.name, original.pax.name);
        assert_eq!(roundtripped.palette.len(), original.palette.len());
        assert!(roundtripped.palette.contains_key("dungeon"));

        // All tiles should be present
        for name in original.tile.keys() {
            assert!(
                roundtripped.tile.contains_key(name),
                "missing tile: {}",
                name
            );
        }

        // WFC rules should be present
        assert!(roundtripped.wfc_rules.is_some());

        // Atlas should be present
        assert!(roundtripped.atlas.is_some());
    }

    #[test]
    fn roundtrip_pax21_features() {
        let source = std::fs::read_to_string("../../examples/pax21_features.pax")
            .expect("pax21_features.pax should exist");
        let original = parse_pax(&source).unwrap();
        let config = crate::paxl::PaxlConfig::default();
        let paxl_text = serialize::serialize(&original, &config).unwrap();

        let (roundtripped, _) = from_paxl(&paxl_text, false).unwrap();

        // Style latent should survive
        assert!(roundtripped.style.contains_key("test"));

        // Delta tile should be present
        assert!(roundtripped.tile.contains_key("wall_cracked"));
        assert!(roundtripped.tile["wall_cracked"].delta.is_some());

        // Fill tile should be present
        assert!(roundtripped.tile.contains_key("water_4x4"));
        assert!(roundtripped.tile["water_4x4"].fill.is_some());
    }

    #[test]
    fn parse_simple_paxl() {
        let paxl = r#"@pax test 2.1 L1
@theme test_theme test_pal s2 c16 p16 tl
@pal test_pal
  . #00000000  # #2a1f3d  + #5a4878
@tile wall 4x4[4] e=solid obstacle
  ####
  #++#
  =2
  =1
"#;
        let (file, warnings) = from_paxl(paxl, true).unwrap();
        assert_eq!(file.pax.name, "test");
        assert_eq!(file.pax.version, "2.1");
        assert!(file.tile.contains_key("wall"));
        let wall = &file.tile["wall"];
        assert!(wall.grid.is_some());
        // Grid should contain =N references (they're preserved in the grid string)
        let grid = wall.grid.as_ref().unwrap();
        assert!(grid.contains("=2") || grid.contains("=1"));
    }

    #[test]
    fn parse_template_tile() {
        let paxl = r#"@pax test 2.1 L1
@pal p
  . #000000  # #ffffff
@tile base 4x4[2] e=solid obstacle
  ####
  ####
@tile child :base swaps:frozen e=ice/ice/ice/ice
"#;
        let (file, _) = from_paxl(paxl, false).unwrap();
        assert!(file.tile.contains_key("child"));
        assert_eq!(
            file.tile["child"].template.as_deref(),
            Some("base")
        );
    }

    #[test]
    fn lenient_mode_accepts_extras() {
        let paxl = r#"@pax test 2.1 L1
@pal p
  . #000000
@unknown_directive something
@tile wall 4x4[2] e=solid obstacle
  ####
  ####
"#;
        let (file, warnings) = from_paxl(paxl, false).unwrap();
        // Should succeed in lenient mode with warning
        assert!(file.tile.contains_key("wall"));
        assert!(!warnings.is_empty());
    }

    #[test]
    fn strict_mode_rejects_unknown() {
        let paxl = "@pax test 2.1 L1\n@unknown_thing blah\n";
        let result = from_paxl(paxl, true);
        assert!(result.is_err());
    }

    #[test]
    fn parse_size_marker_works() {
        assert_eq!(parse_size_marker("16x16[16]"), ("16x16", Some(16)));
        assert_eq!(parse_size_marker("32x32[4]"), ("32x32", Some(4)));
        assert_eq!(parse_size_marker("16x16"), ("16x16", None));
    }

    #[test]
    fn parse_edge_spec_works() {
        let all = parse_edge_spec("solid");
        assert_eq!(all.n, "solid");
        assert_eq!(all.e, "solid");

        let split = parse_edge_spec("solid/floor/solid/floor");
        assert_eq!(split.n, "solid");
        assert_eq!(split.e, "floor");
        assert_eq!(split.s, "solid");
        assert_eq!(split.w, "floor");
    }
}
