//! PAX-L serializer — converts PaxFile to compact PAX-L text.

use super::bpe::{self, AutoStamp};
use super::encoding::{self, TileEncoding};
use super::{PaxlConfig, PaxlError};
use crate::types::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Serialize a PaxFile to PAX-L compact text.
pub fn serialize(file: &PaxFile, config: &PaxlConfig) -> Result<String, PaxlError> {
    let mut buf = String::with_capacity(4096);

    // Detect single-palette optimization
    let palettes_used: HashSet<&str> = file
        .tile
        .values()
        .map(|t| t.palette.as_str())
        .chain(file.stamp.values().map(|s| s.palette.as_str()))
        .collect();
    let single_palette = palettes_used.len() == 1;

    // Resolve palettes and existing stamps for BPE
    let palettes = crate::parser::resolve_all_palettes(file).unwrap_or_default();
    let mut resolved_stamps: HashMap<String, Stamp> = HashMap::new();
    for (name, raw) in &file.stamp {
        if let Ok((sw, sh)) = parse_size(&raw.size) {
            if let Some(pal) = palettes.get(&raw.palette) {
                if let Ok(grid) = crate::grid::parse_grid(&raw.grid, sw, sh, pal) {
                    resolved_stamps.insert(
                        name.clone(),
                        Stamp {
                            palette: raw.palette.clone(),
                            width: sw,
                            height: sh,
                            grid,
                        },
                    );
                }
            }
        }
    }

    // Run BPE auto-stamp extraction
    let auto_stamps = bpe::extract_stamps(file, &palettes, &resolved_stamps, 3);

    // Header
    emit_header(&mut buf, &file.pax);

    // Themes
    for (name, theme) in &file.theme {
        emit_theme(&mut buf, name, theme);
    }

    // Style latent
    for (name, style) in &file.style {
        emit_style(&mut buf, name, style);
    }

    // Palettes
    for (name, palette_raw) in &file.palette {
        emit_palette(&mut buf, name, palette_raw);
    }

    // Extended palettes
    for (name, ext) in &file.palette_ext {
        emit_palette_ext(&mut buf, name, ext);
    }

    // Swaps
    for (name, swap) in &file.palette_swap {
        emit_swap(&mut buf, name, swap);
    }

    // Cycles
    for (name, cycle) in &file.cycle {
        emit_cycle(&mut buf, name, cycle);
    }

    // Animation clocks
    for (name, clock) in &file.anim_clock {
        emit_anim_clock(&mut buf, name, clock);
    }

    // Stamps (user-defined)
    for (name, stamp) in &file.stamp {
        emit_stamp(&mut buf, name, stamp, single_palette);
    }

    // Pre-compute tile encodings and track which auto-stamps are actually used
    let mut tile_encodings: Vec<(String, TileEncoding)> = Vec::new();
    let mut used_stamps: HashSet<String> = HashSet::new();

    for (name, tile) in &file.tile {
        let mut enc = encoding::select_encoding(tile, &file.tile, config);

        // Try BPE compose decomposition for grid-encoded tiles
        if !auto_stamps.is_empty() {
            if let TileEncoding::Grid { ref body, .. } = enc {
                let (w, h) = tile
                    .size
                    .as_deref()
                    .and_then(|s| parse_size(s).ok())
                    .unwrap_or((16, 16));
                let grid: Vec<Vec<char>> = body
                    .lines()
                    .map(|l| l.trim())
                    .filter(|l| !l.is_empty())
                    .map(|l| l.chars().collect())
                    .collect();
                if grid.len() == h as usize
                    && grid.iter().all(|r| r.len() == w as usize)
                {
                    if let Some(layout) = bpe::try_compose_decomposition(
                        &grid,
                        w,
                        h,
                        &auto_stamps,
                        &resolved_stamps,
                    ) {
                        let compose_tokens = encoding::estimate_tokens(&layout);
                        let grid_tokens = encoding::estimate_tokens(body);
                        if compose_tokens < grid_tokens {
                            // Track which stamps this layout uses
                            for token in layout.split_whitespace() {
                                if let Some(sname) = token.strip_prefix('@') {
                                    used_stamps.insert(sname.to_string());
                                }
                            }
                            enc = TileEncoding::Compose(layout);
                        }
                    }
                }
            }
        }

        tile_encodings.push((name.clone(), enc));
    }

    // Emit only auto-stamps that are actually used by compose tiles
    let used_auto: Vec<&AutoStamp> = auto_stamps
        .iter()
        .filter(|s| used_stamps.contains(&s.name))
        .collect();
    if !used_auto.is_empty() {
        buf.push_str("// auto-discovered stamps\n");
        for auto in used_auto {
            emit_auto_stamp(&mut buf, auto);
        }
    }

    // Tiles — use pre-computed encodings
    for (name, pre_enc) in &tile_encodings {
        let tile = &file.tile[name];
        emit_tile_with_encoding(
            &mut buf,
            name,
            tile,
            pre_enc,
            single_palette,
        );
    }

    // Spritesets
    for (name, ss) in &file.spriteset {
        emit_spriteset(&mut buf, name, ss, single_palette);
    }

    // Composites
    for (name, comp) in &file.composite {
        emit_composite(&mut buf, name, comp);
    }

    // Objects
    for (name, obj) in &file.object {
        emit_object(&mut buf, name, obj);
    }

    // Tile runs
    for (name, run) in &file.tile_run {
        emit_tile_run(&mut buf, name, run);
    }

    // Tilemaps
    for (name, tm) in &file.tilemap {
        emit_tilemap(&mut buf, name, tm);
    }

    // Backdrop tiles
    for (name, bt) in &file.backdrop_tile {
        emit_backdrop_tile(&mut buf, name, bt, single_palette);
    }

    // Backdrops
    for (name, bd) in &file.backdrop {
        emit_backdrop(&mut buf, name, bd);
    }

    // WFC rules
    if let Some(ref rules) = file.wfc_rules {
        emit_wfc(&mut buf, rules);
    }

    // Atlas
    if let Some(ref atlas) = file.atlas {
        emit_atlas(&mut buf, atlas);
    }

    Ok(buf)
}

// ── Header ─────────────────────────────────────────────────────────

fn emit_header(buf: &mut String, header: &Header) {
    let _ = write!(buf, "@pax {} {} L1", header.name, header.version);
    if !header.author.is_empty() && header.author != "claude" {
        let _ = write!(buf, " author={}", header.author);
    }
    if let Some(ref profile) = header.color_profile {
        if profile != "srgb" {
            let _ = write!(buf, " profile={}", profile);
        }
    }
    buf.push('\n');
}

// ── Theme ──────────────────────────────────────────────────────────

fn emit_theme(buf: &mut String, name: &str, theme: &Theme) {
    let _ = write!(buf, "@theme {} {}", name, theme.palette);
    if let Some(s) = theme.scale {
        let _ = write!(buf, " s{}", s);
    }
    if let Some(c) = theme.canvas {
        let _ = write!(buf, " c{}", c);
    }
    if let Some(p) = theme.max_palette_size {
        let _ = write!(buf, " p{}", p);
    }
    if let Some(ref ls) = theme.light_source {
        let abbrev = match ls.as_str() {
            "top-left" => "tl",
            "top-right" => "tr",
            "bottom-left" => "bl",
            "bottom-right" => "br",
            "top" => "t",
            "left" => "l",
            other => other,
        };
        let _ = write!(buf, " {}", abbrev);
    }
    if let Some(ref ext) = theme.extends {
        let _ = write!(buf, " :{}", ext);
    }
    buf.push('\n');

    // Roles
    if !theme.roles.is_empty() {
        buf.push_str("@roles");
        for (role, sym) in &theme.roles {
            let _ = write!(buf, " {}{}", sym, role);
        }
        buf.push('\n');
    }

    // Constraints
    if !theme.constraints.is_empty() {
        buf.push_str("@constraints");
        for (key, val) in &theme.constraints {
            match key.as_str() {
                "fg_brighter_than_bg" => buf.push_str(" fg>bg"),
                "shadow_darker_than_bg" => buf.push_str(" shadow<bg"),
                "accent_hue_distinct_from_bg" => buf.push_str(" accent!=bg"),
                "max_colors_per_tile" => {
                    let _ = write!(buf, " max_colors={}", val);
                }
                other => {
                    let _ = write!(buf, " {}={}", other, val);
                }
            }
        }
        buf.push('\n');
    }
}

// ── Style ──────────────────────────────────────────────────────────

fn emit_style(buf: &mut String, name: &str, style: &crate::style::StyleLatent) {
    let _ = writeln!(
        buf,
        "@style {} light={:.2} run={:.1} shadow={:.2} breadth={:.0} density={:.2} entropy={:.1} hue={:.0} lum={:.2}",
        name,
        style.light_direction,
        style.run_length_mean,
        style.shadow_ratio,
        style.palette_breadth,
        style.pixel_density,
        style.palette_entropy,
        style.hue_bias,
        style.luminance_mean,
    );
}

// ── Palette ────────────────────────────────────────────────────────

fn emit_palette(buf: &mut String, name: &str, palette: &PaletteRaw) {
    let _ = writeln!(buf, "@pal {}", name);
    // Emit symbols in pairs per line for density
    let entries: Vec<(&String, &String)> = palette.iter().collect();
    for chunk in entries.chunks(4) {
        buf.push_str("  ");
        for (i, (sym, hex)) in chunk.iter().enumerate() {
            if i > 0 {
                buf.push_str("  ");
            }
            let _ = write!(buf, "{} {}", sym, hex);
        }
        buf.push('\n');
    }
}

fn emit_palette_ext(buf: &mut String, name: &str, ext: &PaletteExtRaw) {
    let _ = writeln!(buf, "@pal_ext {} :{}", name, ext.base);
    let entries: Vec<(&String, &String)> = ext.symbols.iter().collect();
    for chunk in entries.chunks(4) {
        buf.push_str("  ");
        for (i, (sym, hex)) in chunk.iter().enumerate() {
            if i > 0 {
                buf.push_str("  ");
            }
            let _ = write!(buf, "{} {}", sym, hex);
        }
        buf.push('\n');
    }
}

// ── Swap ───────────────────────────────────────────────────────────

fn emit_swap(buf: &mut String, name: &str, swap: &PaletteSwap) {
    let _ = write!(buf, "@swap {} {}", name, swap.base);
    if let Some(ref target) = swap.target {
        let _ = write!(buf, " target={}", target);
    }
    if swap.partial {
        buf.push_str(" partial");
        for (sym, hex) in &swap.map {
            let _ = write!(buf, " {}={}", sym, hex);
        }
    }
    buf.push('\n');
}

// ── Cycle ──────────────────────────────────────────────────────────

fn emit_cycle(buf: &mut String, name: &str, cycle: &Cycle) {
    let syms = cycle.symbols.join(",");
    let _ = writeln!(
        buf,
        "@cycle {} {} {} {} {}fps",
        name, cycle.palette, syms, cycle.direction, cycle.fps
    );
}

// ── Animation Clock ────────────────────────────────────────────────

fn emit_anim_clock(buf: &mut String, name: &str, clock: &AnimClock) {
    let _ = writeln!(
        buf,
        "@clock {} fps={} frames={} {}",
        name, clock.fps, clock.frames, clock.mode
    );
}

// ── Stamp ──────────────────────────────────────────────────────────

fn emit_stamp(buf: &mut String, name: &str, stamp: &StampRaw, single_palette: bool) {
    let _ = write!(buf, "@stamp {} {}", name, stamp.size);
    if !single_palette {
        let _ = write!(buf, " pal={}", stamp.palette);
    }
    buf.push('\n');
    // Compact: rows joined by | for small stamps
    let rows: Vec<&str> = stamp
        .grid
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let total_chars: usize = rows.iter().map(|r| r.len()).sum::<usize>() + rows.len();
    if total_chars <= 40 {
        // Single line with | separators
        let _ = writeln!(buf, "  {}", rows.join("|"));
    } else {
        // Multi-line
        for row in rows {
            let _ = writeln!(buf, "  {}", row);
        }
    }
}

// ── Tile ───────────────────────────────────────────────────────────

/// Emit an auto-discovered stamp (from BPE).
fn emit_auto_stamp(buf: &mut String, stamp: &AutoStamp) {
    let _ = write!(buf, "@stamp {} {}x{}", stamp.name, stamp.width, stamp.height);
    buf.push('\n');
    let rows: Vec<String> = stamp.grid.iter().map(|r| r.iter().collect::<String>()).collect();
    let total_chars: usize = rows.iter().map(|r| r.len()).sum::<usize>() + rows.len();
    if total_chars <= 40 {
        let _ = writeln!(buf, "  {}", rows.join("|"));
    } else {
        for row in &rows {
            let _ = writeln!(buf, "  {}", row);
        }
    }
}

fn emit_tile_with_encoding(
    buf: &mut String,
    name: &str,
    tile: &TileRaw,
    enc: &TileEncoding,
    single_palette: bool,
) {
    // Determine row count for [N] marker
    let row_count = match &enc {
        TileEncoding::Template(_) => 0,
        TileEncoding::Delta { patches, .. } => {
            // Count patch lines: group patches into lines for compact output
            if patches.is_empty() {
                0
            } else {
                // Rough: ~4 patches per line
                (patches.len() + 3) / 4
            }
        }
        TileEncoding::Fill { pattern, .. } => {
            pattern.lines().filter(|l| !l.trim().is_empty()).count()
        }
        TileEncoding::Compose(layout) => {
            layout.lines().filter(|l| !l.trim().is_empty()).count()
        }
        TileEncoding::Grid { row_count, .. } => *row_count,
        TileEncoding::Rle { row_count, .. } => *row_count,
    };

    // Header line
    match &enc {
        TileEncoding::Template(base) => {
            let _ = write!(buf, "@tile {} :{}", name, base);
            if !single_palette {
                let _ = write!(buf, " pal={}", tile.palette);
            }
            emit_tile_metadata(buf, tile);
            buf.push('\n');
            return;
        }
        _ => {
            let size = tile.size.as_deref().unwrap_or("16x16");
            let _ = write!(buf, "@tile {} {}[{}]", name, size, row_count);
        }
    }

    if !single_palette {
        let _ = write!(buf, " pal={}", tile.palette);
    }

    // Edge class
    if let Some(ref ec) = tile.edge_class {
        if ec.n == ec.e && ec.e == ec.s && ec.s == ec.w {
            let _ = write!(buf, " e={}", ec.n);
        } else {
            let _ = write!(buf, " e={}/{}/{}/{}", ec.n, ec.e, ec.s, ec.w);
        }
    }

    // Weight (omit default 1.0)
    if (tile.weight - 1.0).abs() > 0.001 {
        let _ = write!(buf, " w{}", tile.weight);
    }

    // Affordance + collision (from semantic)
    if let Some(ref sem) = tile.semantic {
        if let Some(ref aff) = sem.affordance {
            let _ = write!(buf, " {}", aff);
        }
        if let Some(ref col) = sem.collision {
            // Omit col=full for obstacles, col=none for walkable
            let aff = sem.affordance.as_deref().unwrap_or("");
            let should_omit = (col == "full" && aff == "obstacle")
                || (col == "none" && aff == "walkable");
            if !should_omit {
                let _ = write!(buf, " col={}", col);
            }
        }
    }

    // Symmetry (omit "none")
    if let Some(ref sym) = tile.symmetry {
        if sym != "none" {
            let _ = write!(buf, " sym={}", sym);
        }
    }

    // Auto-rotate (omit "none")
    if let Some(ref rot) = tile.auto_rotate {
        if rot != "none" {
            let _ = write!(buf, " rot={}", rot);
        }
    }

    // Tags
    if !tile.tags.is_empty() {
        let _ = write!(buf, " tags:{}", tile.tags.join(","));
    }

    // Palette swaps
    if !tile.palette_swaps.is_empty() {
        let _ = write!(buf, " swaps:{}", tile.palette_swaps.join(","));
    }

    // Cycles
    if !tile.cycles.is_empty() {
        let _ = write!(buf, " cycles:{}", tile.cycles.join(","));
    }

    // Encoding type hint for compose
    if matches!(enc, TileEncoding::Compose(_)) {
        buf.push_str(" compose");
    }

    buf.push('\n');

    // Semantic tags (separate line if non-trivial)
    if let Some(ref sem) = tile.semantic {
        if !sem.tags.is_empty() {
            buf.push_str("  sem:");
            for (key, val) in &sem.tags {
                match val {
                    toml::Value::Boolean(true) => {
                        let _ = write!(buf, " {}", key);
                    }
                    toml::Value::String(s) => {
                        let _ = write!(buf, " {}={}", key, s);
                    }
                    other => {
                        let _ = write!(buf, " {}={}", key, other);
                    }
                }
            }
            buf.push('\n');
        }
    }

    // Grid body
    match enc {
        TileEncoding::Template(_) => unreachable!(),
        TileEncoding::Delta { base, patches } => {
            let _ = writeln!(buf, "  @delta {}", base);
            // Emit patches, ~4 per line for compactness
            for chunk in patches.chunks(4) {
                buf.push_str("  ");
                for (i, p) in chunk.iter().enumerate() {
                    if i > 0 {
                        buf.push_str("  ");
                    }
                    let _ = write!(buf, "+{},{} {}", p.x, p.y, p.sym);
                }
                buf.push('\n');
            }
        }
        TileEncoding::Fill {
            pattern,
            fill_w,
            fill_h,
        } => {
            let _ = writeln!(buf, "  @fill {}x{}", fill_w, fill_h);
            for line in pattern.lines().filter(|l| !l.trim().is_empty()) {
                let _ = writeln!(buf, "  {}", line.trim());
            }
        }
        TileEncoding::Compose(layout) => {
            for line in layout.lines().filter(|l| !l.trim().is_empty()) {
                let _ = writeln!(buf, "  {}", line.trim());
            }
        }
        TileEncoding::Grid { body, .. } => {
            for line in body.lines().filter(|l| !l.trim().is_empty()) {
                let _ = writeln!(buf, "  {}", line.trim());
            }
        }
        TileEncoding::Rle { body, .. } => {
            for line in body.lines().filter(|l| !l.trim().is_empty()) {
                let _ = writeln!(buf, "  {}", line.trim());
            }
        }
    }
}

fn emit_tile_metadata(buf: &mut String, tile: &TileRaw) {
    if let Some(ref ec) = tile.edge_class {
        if ec.n == ec.e && ec.e == ec.s && ec.s == ec.w {
            let _ = write!(buf, " e={}", ec.n);
        } else {
            let _ = write!(buf, " e={}/{}/{}/{}", ec.n, ec.e, ec.s, ec.w);
        }
    }
    if !tile.palette_swaps.is_empty() {
        let _ = write!(buf, " swaps:{}", tile.palette_swaps.join(","));
    }
}

// ── Spriteset ──────────────────────────────────────────────────────

fn emit_spriteset(buf: &mut String, name: &str, ss: &SpritesetRaw, single_palette: bool) {
    let _ = write!(buf, "@spriteset {} {}", name, ss.size);
    if !single_palette {
        let _ = write!(buf, " pal={}", ss.palette);
    }
    if !ss.palette_swaps.is_empty() {
        let _ = write!(buf, " swaps:{}", ss.palette_swaps.join(","));
    }
    buf.push('\n');

    for sprite in &ss.sprite {
        let _ = write!(buf, "@sprite {}.{} fps={}", name, sprite.name, sprite.fps);
        if sprite.r#loop {
            buf.push_str(" loop");
        }
        if let Some(s) = sprite.scale {
            if (s - 1.0).abs() > 0.001 {
                let _ = write!(buf, " scale={}", s);
            }
        }
        buf.push('\n');

        for frame in &sprite.frames {
            let enc = frame.encoding.as_deref().unwrap_or("grid");
            match enc {
                "grid" => {
                    let _ = write!(buf, "  @frame {}", frame.index);
                    if let Some(ms) = frame.duration_ms {
                        let _ = write!(buf, " ms={}", ms);
                    }
                    if let Some(ref m) = frame.mirror {
                        let _ = write!(buf, " mirror={}", m);
                    }
                    buf.push('\n');
                    if let Some(ref grid) = frame.grid {
                        for line in grid.lines().filter(|l| !l.trim().is_empty()) {
                            let _ = writeln!(buf, "    {}", line.trim());
                        }
                    }
                }
                "delta" => {
                    let _ = write!(buf, "  @frame {} delta={}", frame.index, frame.base.unwrap_or(1));
                    if let Some(ms) = frame.duration_ms {
                        let _ = write!(buf, " ms={}", ms);
                    }
                    buf.push('\n');
                    for c in &frame.changes {
                        let _ = writeln!(buf, "    +{},{} {}", c.x, c.y, c.sym);
                    }
                }
                "linked" => {
                    let _ = writeln!(buf, "  @frame {} link={}", frame.index, frame.link_to.unwrap_or(1));
                }
                _ => {
                    let _ = writeln!(buf, "  @frame {} enc={}", frame.index, enc);
                }
            }
        }

        if !sprite.tags.is_empty() {
            for tag in &sprite.tags {
                let _ = writeln!(buf, "  @tags {} {}-{}", tag.name, tag.from_frame, tag.to_frame);
            }
        }
    }
}

// ── Composite ──────────────────────────────────────────────────────

fn emit_composite(buf: &mut String, name: &str, comp: &CompositeRaw) {
    let _ = writeln!(buf, "@composite {} {} tile={}", name, comp.size, comp.tile_size);
    for line in comp.layout.lines().filter(|l| !l.trim().is_empty()) {
        let _ = writeln!(buf, "  {}", line.trim());
    }

    for (vname, variant) in &comp.variant {
        let _ = write!(buf, "@variant {}.{}", name, vname);
        for (slot, tile) in &variant.slot {
            let _ = write!(buf, " {}={}", slot, tile);
        }
        buf.push('\n');
    }

    if !comp.offset.is_empty() {
        let _ = write!(buf, "@offset {}", name);
        for (slot, off) in &comp.offset {
            let _ = write!(buf, " {}=[{},{}]", slot, off[0], off[1]);
        }
        buf.push('\n');
    }

    for (aname, anim) in &comp.anim {
        let _ = write!(buf, "@anim {}.{} fps={}", name, aname, anim.fps);
        if anim.r#loop {
            buf.push_str(" loop");
        }
        if let Some(ref src) = anim.source {
            let _ = write!(buf, " source={}", src);
        }
        if let Some(ref m) = anim.mirror {
            let _ = write!(buf, " mirror={}", m);
        }
        buf.push('\n');
        for frame in &anim.frame {
            let _ = write!(buf, "  @f {}", frame.index);
            if !frame.swap.is_empty() {
                let swaps: Vec<String> = frame.swap.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                let _ = write!(buf, " swap:{}", swaps.join(","));
            }
            if !frame.offset.is_empty() {
                let offs: Vec<String> = frame.offset.iter().map(|(k, v)| format!("{}=[{},{}]", k, v[0], v[1])).collect();
                let _ = write!(buf, " offset:{}", offs.join(","));
            }
            buf.push('\n');
        }
    }
}

// ── Object ─────────────────────────────────────────────────────────

fn emit_object(buf: &mut String, name: &str, obj: &ObjectRaw) {
    let _ = write!(buf, "@object {} {}", name, obj.size_tiles);
    if let Some(ref base) = obj.base_tile {
        let _ = write!(buf, " base={}", base);
    }
    if !obj.above_player_rows.is_empty() {
        let rows: Vec<String> = obj.above_player_rows.iter().map(|r| r.to_string()).collect();
        let _ = write!(buf, " above={}", rows.join(","));
    }
    if !obj.below_player_rows.is_empty() {
        let rows: Vec<String> = obj.below_player_rows.iter().map(|r| r.to_string()).collect();
        let _ = write!(buf, " below={}", rows.join(","));
    }
    buf.push('\n');
    for line in obj.tiles.lines().filter(|l| !l.trim().is_empty()) {
        let _ = writeln!(buf, "  {}", line.trim());
    }
    if let Some(ref col) = obj.collision {
        let rows: Vec<&str> = col.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        let _ = writeln!(buf, "  @col {}", rows.join("|"));
    }
}

// ── Tile Run ───────────────────────────────────────────────────────

fn emit_tile_run(buf: &mut String, name: &str, run: &TileRun) {
    let _ = write!(buf, "@run {} {}", name, run.orientation);
    let _ = write!(buf, " left={} mid={} right={}", run.left, run.middle, run.right);
    if let Some(ref s) = run.single {
        let _ = write!(buf, " single={}", s);
    }
    buf.push('\n');
}

// ── Tilemap ────────────────────────────────────────────────────────

fn emit_tilemap(buf: &mut String, name: &str, tm: &crate::tilemap::TilemapRaw) {
    let _ = writeln!(
        buf,
        "@tilemap {} {}x{} tile={}x{}",
        name, tm.width, tm.height, tm.tile_width, tm.tile_height
    );
    // Layers are a HashMap<String, TilemapLayerRaw>
    let mut layers: Vec<(&String, &crate::tilemap::TilemapLayerRaw)> =
        tm.layer.iter().collect();
    layers.sort_by_key(|(_, l)| l.z_order);
    for (lname, layer) in layers {
        let _ = write!(buf, "@layer {} z={}", lname, layer.z_order);
        if layer.collision {
            buf.push_str(" collision");
        }
        if layer.blend != "normal" {
            let _ = write!(buf, " blend={}", layer.blend);
        }
        buf.push('\n');
        if let Some(ref grid) = layer.grid {
            for line in grid.lines().filter(|l| !l.trim().is_empty()) {
                let _ = writeln!(buf, "  {}", line.trim());
            }
        }
    }
}

// ── WFC ────────────────────────────────────────────────────────────

fn emit_wfc(buf: &mut String, rules: &WfcRules) {
    buf.push_str("@wfc\n");
    for rule in &rules.forbids {
        // Convert "affordance:obstacle forbids affordance:hazard adjacent"
        // to compact "forbid obstacle~hazard adjacent"
        let _ = writeln!(buf, "  forbid {}", compact_wfc_rule(rule));
    }
    for rule in &rules.requires {
        let _ = write!(buf, "  require {}", compact_wfc_rule(rule));
        if (rules.require_boost - 3.0).abs() > 0.001 {
            let _ = write!(buf, " boost={}", rules.require_boost);
        }
        buf.push('\n');
    }
    for (gname, members) in &rules.variant_groups {
        let _ = writeln!(buf, "  group {}: {}", gname, members.join(" "));
    }
}

/// Compact a WFC rule string.
/// "affordance:obstacle forbids affordance:hazard adjacent" → "obstacle~hazard adjacent"
fn compact_wfc_rule(rule: &str) -> String {
    let parts: Vec<&str> = rule.split_whitespace().collect();
    if parts.len() >= 4 {
        // Extract the value after ':'
        let left = parts[0].split(':').last().unwrap_or(parts[0]);
        let right = parts[2].split(':').last().unwrap_or(parts[2]);
        let qualifier = parts[3];
        format!("{}~{} {}", left, right, qualifier)
    } else {
        rule.to_string()
    }
}

// ── Atlas ──────────────────────────────────────────────────────────

fn emit_atlas(buf: &mut String, atlas: &AtlasConfig) {
    let _ = write!(buf, "@atlas {}", atlas.format);
    if atlas.padding != 1 {
        let _ = write!(buf, " pad={}", atlas.padding);
    }
    if atlas.scale != 1 {
        let _ = write!(buf, " s{}", atlas.scale);
    }
    if atlas.columns != 8 {
        let _ = write!(buf, " cols={}", atlas.columns);
    }
    buf.push('\n');
    if !atlas.include.is_empty() {
        let _ = writeln!(buf, "  include {}", atlas.include.join(" "));
    }
    let _ = write!(buf, "  out {}", atlas.output);
    if let Some(ref map) = atlas.map_output {
        let _ = write!(buf, " map {}", map);
    }
    buf.push('\n');
}

// ── Backdrop Tile ──────────────────────────────────────────────────

fn emit_backdrop_tile(buf: &mut String, name: &str, bt: &BackdropTileRaw, single_palette: bool) {
    let size = bt.size.as_deref().unwrap_or("16x16");
    let _ = write!(buf, "@bgtile {} {}", name, size);
    if !single_palette {
        let _ = write!(buf, " pal={}", bt.palette);
    }
    if let Some(ref ext) = bt.palette_ext {
        let _ = write!(buf, " ext={}", ext);
    }
    if bt.rle.is_some() {
        buf.push_str(" rle");
    }
    buf.push('\n');

    if let Some(ref grid) = bt.grid {
        for line in grid.lines().filter(|l| !l.trim().is_empty()) {
            let _ = writeln!(buf, "  {}", line.trim());
        }
    }
    if let Some(ref rle) = bt.rle {
        for line in rle.lines().filter(|l| !l.trim().is_empty()) {
            let _ = writeln!(buf, "  {}", line.trim());
        }
    }

    if !bt.animation.is_empty() {
        let frames: Vec<String> = bt
            .animation
            .iter()
            .map(|f| format!("{}:{}ms", f.tile, f.duration_ms))
            .collect();
        let _ = writeln!(buf, "  @anim {}", frames.join(" "));
    }
}

// ── Backdrop ───────────────────────────────────────────────────────

fn emit_backdrop(buf: &mut String, name: &str, bd: &BackdropRaw) {
    let _ = write!(buf, "@backdrop {} {}", name, bd.size);
    let _ = write!(buf, " tile={}", bd.tile_size);
    let _ = write!(buf, " pal={}", bd.palette);
    if let Some(ref ext) = bd.palette_ext {
        let _ = write!(buf, " ext={}", ext);
    }
    buf.push('\n');

    // Single-layer tilemap
    if let Some(ref tilemap) = bd.tilemap {
        for line in tilemap.lines().filter(|l| !l.trim().is_empty()) {
            let _ = writeln!(buf, "  {}", line.trim());
        }
    }

    // Multi-layer
    for layer in &bd.layer {
        let _ = write!(buf, "@blayer {}", layer.name);
        if (layer.scroll_factor - 1.0).abs() > 0.001 {
            let _ = write!(buf, " scroll={}", layer.scroll_factor);
        }
        if (layer.opacity - 1.0).abs() > 0.001 {
            let _ = write!(buf, " opacity={}", layer.opacity);
        }
        if layer.blend != "normal" {
            let _ = write!(buf, " blend={}", layer.blend);
        }
        buf.push('\n');
        for line in layer.tilemap.lines().filter(|l| !l.trim().is_empty()) {
            let _ = writeln!(buf, "  {}", line.trim());
        }
    }

    // Zones
    for zone in &bd.zone {
        let _ = write!(buf, "@zone {}", zone.name);
        match zone.behavior.as_str() {
            "cycle" => {
                if let Some(ref c) = zone.cycle {
                    let _ = write!(buf, " cycle={}", c);
                }
            }
            "wave" => {
                if let Some(ref c) = zone.cycle {
                    let _ = write!(buf, " wave={}", c);
                }
                if let Some(pr) = zone.phase_rows {
                    let _ = write!(buf, " phase={}", pr);
                }
            }
            "flicker" => {
                if let Some(ref c) = zone.cycle {
                    let _ = write!(buf, " flicker={}", c);
                }
            }
            "scroll_down" => {
                buf.push_str(" scroll_down");
                if let Some(sp) = zone.speed {
                    let _ = write!(buf, " speed={}", sp);
                }
            }
            other => {
                let _ = write!(buf, " {}", other);
            }
        }
        buf.push('\n');
        let _ = writeln!(
            buf,
            "  rect {},{} {}x{}",
            zone.rect.x, zone.rect.y, zone.rect.w, zone.rect.h
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_pax;

    #[test]
    fn serialize_dungeon_example() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let file = parse_pax(&source).unwrap();
        let config = PaxlConfig::default();
        let paxl = serialize(&file, &config).unwrap();

        // Basic structure checks
        assert!(paxl.starts_with("@pax"));
        assert!(paxl.contains("@theme"));
        assert!(paxl.contains("@pal dungeon"));
        assert!(paxl.contains("@tile wall_solid"));
        assert!(paxl.contains("@tile floor_stone"));
        assert!(paxl.contains("@tile water_surface"));
        assert!(paxl.contains("@wfc"));
        assert!(paxl.contains("@atlas"));

        // Should be shorter than TOML
        assert!(
            paxl.len() < source.len(),
            "PAX-L ({} bytes) should be shorter than TOML ({} bytes)",
            paxl.len(),
            source.len()
        );

        // Write to /tmp for token analysis
        let _ = std::fs::write("/tmp/dungeon.paxl", &paxl);
    }

    #[test]
    fn serialize_pax21_features() {
        let source = std::fs::read_to_string("../../examples/pax21_features.pax")
            .expect("pax21_features.pax should exist");
        let file = parse_pax(&source).unwrap();
        let config = PaxlConfig::default();
        let paxl = serialize(&file, &config).unwrap();

        // Delta tile should use @delta
        assert!(paxl.contains("@delta wall_4x4"));

        // Fill tile should use @fill
        assert!(paxl.contains("@fill 2x2"));

        // Style should be emitted
        assert!(paxl.contains("@style"));

        // Row refs should appear (wall_8x4 has dup rows)
        assert!(paxl.contains("=1") || paxl.contains("=2"));
    }

    #[test]
    fn single_palette_omits_pal() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let file = parse_pax(&source).unwrap();
        let config = PaxlConfig::default();
        let paxl = serialize(&file, &config).unwrap();

        // dungeon.pax uses single palette "dungeon" — tiles shouldn't have pal=
        // (The @tile lines should NOT contain "pal=dungeon")
        for line in paxl.lines() {
            if line.starts_with("@tile") && !line.contains(':') {
                // Non-template tiles
                assert!(
                    !line.contains("pal="),
                    "Single-palette file should omit pal= on tile: {}",
                    line
                );
            }
        }
    }

    #[test]
    fn default_weight_omitted() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let file = parse_pax(&source).unwrap();
        let config = PaxlConfig::default();
        let paxl = serialize(&file, &config).unwrap();

        // floor_stone has weight=1.0 — should NOT appear as w1
        // But wall_solid has weight=0.4 — should appear
        let lines: Vec<&str> = paxl.lines().collect();
        for line in &lines {
            if line.contains("floor_stone") && line.starts_with("@tile") {
                assert!(!line.contains("w1"), "Default weight 1.0 should be omitted");
            }
        }
    }
}
