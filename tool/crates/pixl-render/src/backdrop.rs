//! Backdrop rendering — multi-layer, parallax, flip flags, blending, animation zones.

use image::{ImageBuffer, Rgba, RgbaImage};
use pixl_core::types::{
    AnimClock, Backdrop, BackdropLayer, BackdropTileRaw, BackdropZone, BlendMode, Cycle,
    FadeTarget, PaletteExt, Palette, PaxFile, Rgba as PaxRgba, TileModifier, TileRef,
    ZoneBehavior, ZoneRect,
};
use std::collections::HashMap;

/// Render the full static backdrop by compositing all layers back-to-front.
pub fn render_backdrop(
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
) -> RgbaImage {
    let mut img = ImageBuffer::from_pixel(backdrop.width, backdrop.height, Rgba([0, 0, 0, 0]));

    for layer in &backdrop.layers {
        let mut layer_img = render_layer(backdrop, layer, tile_grids, palette_ext);
        // Apply GBA BLDY-style fade if configured
        if let Some((target, amount)) = &layer.fade {
            apply_fade(&mut layer_img, *target, *amount);
        }
        blend_onto(&mut img, &layer_img, layer.blend, layer.opacity, layer.offset_x, layer.offset_y);
    }

    img
}

/// Render an animated frame at a given tick.
pub fn render_backdrop_frame(
    base_img: &RgbaImage,
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    cycles: &HashMap<String, Cycle>,
    palettes: &HashMap<String, Palette>,
    animated_tiles: &HashMap<String, Vec<(String, u32)>>,
    tick: u32,
) -> RgbaImage {
    let mut frame = base_img.clone();

    // Apply per-tile frame animation (swap tiles based on tick)
    if !animated_tiles.is_empty() {
        apply_tile_animations(
            &mut frame, backdrop, tile_grids, palette_ext, animated_tiles, tick,
        );
    }

    // Apply zone behaviors
    for zone in &backdrop.zones {
        apply_zone_behavior(
            &mut frame, base_img, backdrop, tile_grids, palette_ext, cycles, zone, tick,
        );
    }

    frame
}

/// Render a single layer with optional camera offset (for parallax).
/// Pixels inside `scroll_lock` stay at their original position regardless
/// of camera offset — used for HUDs, status bars, fixed overlays.
fn render_layer(
    backdrop: &Backdrop,
    layer: &BackdropLayer,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
) -> RgbaImage {
    render_layer_at_scroll(backdrop, layer, tile_grids, palette_ext, 0, 0)
}

/// Render a layer with a camera scroll offset applied.
/// `scroll_lock` regions are exempt from the offset.
pub fn render_layer_at_scroll(
    backdrop: &Backdrop,
    layer: &BackdropLayer,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    camera_x: i32,
    camera_y: i32,
) -> RgbaImage {
    let mut img = ImageBuffer::from_pixel(backdrop.width, backdrop.height, Rgba([0, 0, 0, 0]));

    // Compute this layer's scroll offset from camera position
    let layer_ox = (camera_x as f64 * layer.scroll_factor) as i32;
    let layer_oy = (camera_y as f64 * layer.scroll_factor) as i32;

    for (row_idx, row) in layer.tilemap.iter().enumerate() {
        for (col_idx, tile_ref) in row.iter().enumerate() {
            let grid = match tile_grids.get(&tile_ref.name) {
                Some(g) => g,
                None => continue,
            };

            let tile_x = col_idx as u32 * backdrop.tile_width;
            let tile_y = row_idx as u32 * backdrop.tile_height;

            // Check if this tile is inside the scroll_lock region
            let locked = layer.scroll_lock.as_ref().map_or(false, |lock| {
                tile_x >= lock.x
                    && tile_x < lock.x + lock.w
                    && tile_y >= lock.y
                    && tile_y < lock.y + lock.h
            });

            // Apply scroll offset unless locked
            let (draw_x, draw_y) = if locked {
                (tile_x as i32, tile_y as i32)
            } else {
                (tile_x as i32 - layer_ox, tile_y as i32 - layer_oy)
            };

            if draw_x < 0 || draw_y < 0
                || draw_x >= backdrop.width as i32
                || draw_y >= backdrop.height as i32
            {
                continue;
            }

            render_tile_at(
                &mut img, grid, tile_ref, palette_ext,
                draw_x as u32, draw_y as u32, backdrop.tile_width, backdrop.tile_height,
            );
        }
    }

    img
}

/// Render a single tile (with flip flags) into an image at the given position.
fn render_tile_at(
    img: &mut RgbaImage,
    grid: &[Vec<String>],
    tile_ref: &TileRef,
    palette_ext: &PaletteExt,
    base_x: u32,
    base_y: u32,
    tile_w: u32,
    tile_h: u32,
) {
    let gh = grid.len() as u32;
    let gw = grid.first().map(|r| r.len() as u32).unwrap_or(0);

    for ty in 0..gh.min(tile_h) {
        for tx in 0..gw.min(tile_w) {
            // Apply flip transformations
            let (src_x, src_y) = apply_flips(tx, ty, gw, gh, tile_ref);

            let sym = match grid.get(src_y as usize).and_then(|r| r.get(src_x as usize)) {
                Some(s) => s,
                None => continue,
            };

            let mut color = resolve_symbol_color(sym, palette_ext);
            if color.0[3] == 0 {
                continue; // skip transparent
            }

            // Apply Genesis VDP-style shadow/highlight modifier
            match tile_ref.modifier {
                TileModifier::Shadow => {
                    color.0[0] /= 2;
                    color.0[1] /= 2;
                    color.0[2] /= 2;
                }
                TileModifier::Highlight => {
                    color.0[0] = color.0[0] / 2 + 128;
                    color.0[1] = color.0[1] / 2 + 128;
                    color.0[2] = color.0[2] / 2 + 128;
                }
                TileModifier::None => {}
            }

            let px = base_x + tx;
            let py = base_y + ty;
            if px < img.width() && py < img.height() {
                img.put_pixel(px, py, color);
            }
        }
    }
}

/// Apply flip flags to source coordinates.
fn apply_flips(x: u32, y: u32, w: u32, h: u32, tile_ref: &TileRef) -> (u32, u32) {
    let mut sx = x;
    let mut sy = y;

    // Diagonal flip (transpose) first — enables 90° rotations when combined with h/v
    if tile_ref.flip_d {
        std::mem::swap(&mut sx, &mut sy);
    }

    if tile_ref.flip_h {
        sx = w.saturating_sub(1) - sx;
    }
    if tile_ref.flip_v {
        sy = h.saturating_sub(1) - sy;
    }

    (sx, sy)
}

/// Blend a source image onto a destination with the given blend mode and opacity.
fn blend_onto(
    dst: &mut RgbaImage,
    src: &RgbaImage,
    blend: BlendMode,
    opacity: f64,
    offset_x: i32,
    offset_y: i32,
) {
    let (dw, dh) = dst.dimensions();

    for (sx, sy, src_pixel) in src.enumerate_pixels() {
        let dx = sx as i32 + offset_x;
        let dy = sy as i32 + offset_y;
        if dx < 0 || dy < 0 || dx >= dw as i32 || dy >= dh as i32 {
            continue;
        }
        let (dx, dy) = (dx as u32, dy as u32);

        let src_a = src_pixel.0[3] as f64 / 255.0 * opacity;
        if src_a < 0.004 {
            continue;
        }

        let dst_pixel = dst.get_pixel(dx, dy);

        let blended = match blend {
            BlendMode::Normal => {
                // Standard alpha-over compositing
                blend_normal(dst_pixel, src_pixel, src_a)
            }
            BlendMode::Additive => {
                blend_additive(dst_pixel, src_pixel, src_a)
            }
            BlendMode::Multiply => {
                blend_multiply(dst_pixel, src_pixel, src_a)
            }
            BlendMode::Screen => {
                blend_screen(dst_pixel, src_pixel, src_a)
            }
        };

        dst.put_pixel(dx, dy, blended);
    }
}

fn blend_normal(dst: &Rgba<u8>, src: &Rgba<u8>, src_a: f64) -> Rgba<u8> {
    let inv_a = 1.0 - src_a;
    Rgba([
        (src.0[0] as f64 * src_a + dst.0[0] as f64 * inv_a) as u8,
        (src.0[1] as f64 * src_a + dst.0[1] as f64 * inv_a) as u8,
        (src.0[2] as f64 * src_a + dst.0[2] as f64 * inv_a) as u8,
        ((src_a + dst.0[3] as f64 / 255.0 * inv_a) * 255.0).min(255.0) as u8,
    ])
}

fn blend_additive(dst: &Rgba<u8>, src: &Rgba<u8>, src_a: f64) -> Rgba<u8> {
    Rgba([
        (dst.0[0] as f64 + src.0[0] as f64 * src_a).min(255.0) as u8,
        (dst.0[1] as f64 + src.0[1] as f64 * src_a).min(255.0) as u8,
        (dst.0[2] as f64 + src.0[2] as f64 * src_a).min(255.0) as u8,
        dst.0[3].max((src_a * 255.0) as u8),
    ])
}

fn blend_multiply(dst: &Rgba<u8>, src: &Rgba<u8>, src_a: f64) -> Rgba<u8> {
    let inv_a = 1.0 - src_a;
    Rgba([
        (dst.0[0] as f64 * (src.0[0] as f64 / 255.0 * src_a + inv_a)) as u8,
        (dst.0[1] as f64 * (src.0[1] as f64 / 255.0 * src_a + inv_a)) as u8,
        (dst.0[2] as f64 * (src.0[2] as f64 / 255.0 * src_a + inv_a)) as u8,
        dst.0[3],
    ])
}

fn blend_screen(dst: &Rgba<u8>, src: &Rgba<u8>, src_a: f64) -> Rgba<u8> {
    let inv_a = 1.0 - src_a;
    let screen = |d: u8, s: u8| -> u8 {
        let df = d as f64 / 255.0;
        let sf = s as f64 / 255.0;
        let result = 1.0 - (1.0 - df) * (1.0 - sf * src_a + inv_a * (1.0 - df) / (1.0 - df + 0.001));
        (result.clamp(0.0, 1.0) * 255.0) as u8
    };
    // Simplified screen: dst + src - dst*src, mixed by src_a
    Rgba([
        ((dst.0[0] as f64 + src.0[0] as f64 * src_a - dst.0[0] as f64 * src.0[0] as f64 / 255.0 * src_a).clamp(0.0, 255.0)) as u8,
        ((dst.0[1] as f64 + src.0[1] as f64 * src_a - dst.0[1] as f64 * src.0[1] as f64 / 255.0 * src_a).clamp(0.0, 255.0)) as u8,
        ((dst.0[2] as f64 + src.0[2] as f64 * src_a - dst.0[2] as f64 * src.0[2] as f64 / 255.0 * src_a).clamp(0.0, 255.0)) as u8,
        dst.0[3].max((src_a * 255.0) as u8),
    ])
}

// ── Tile animations ─────────────────────────────────────────────────

/// Apply per-tile frame animation: swap animated tile pixels based on tick.
fn apply_tile_animations(
    frame: &mut RgbaImage,
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    animated_tiles: &HashMap<String, Vec<(String, u32)>>,
    tick: u32,
) {
    for layer in &backdrop.layers {
        for (row_idx, row) in layer.tilemap.iter().enumerate() {
            for (col_idx, tile_ref) in row.iter().enumerate() {
                let anim = match animated_tiles.get(&tile_ref.name) {
                    Some(a) if !a.is_empty() => a,
                    _ => continue,
                };

                // Find which frame to show at this tick
                let total_duration: u32 = anim.iter().map(|(_, d)| d).sum();
                if total_duration == 0 {
                    continue;
                }
                let tick_in_cycle = (tick * 120) % total_duration; // tick * default_ms
                let mut elapsed = 0u32;
                let mut current_tile = &anim[0].0;
                for (tile_name, duration) in anim {
                    elapsed += duration;
                    if tick_in_cycle < elapsed {
                        current_tile = tile_name;
                        break;
                    }
                }

                if let Some(grid) = tile_grids.get(current_tile) {
                    let base_x = col_idx as u32 * backdrop.tile_width;
                    let base_y = row_idx as u32 * backdrop.tile_height;
                    render_tile_at(
                        frame, grid, tile_ref, palette_ext,
                        base_x, base_y, backdrop.tile_width, backdrop.tile_height,
                    );
                }
            }
        }
    }
}

// ── Zone behaviors ──────────────────────────────────────────────────

fn apply_zone_behavior(
    frame: &mut RgbaImage,
    base_img: &RgbaImage,
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    cycles: &HashMap<String, Cycle>,
    zone: &BackdropZone,
    tick: u32,
) {
    let rect = &zone.rect;

    match &zone.behavior {
        ZoneBehavior::Cycle { cycle: name } => {
            if let Some(cycle) = cycles.get(name) {
                apply_cycle(frame, backdrop, tile_grids, palette_ext, cycle, rect, tick);
            }
        }
        ZoneBehavior::Wave { cycle: name, phase_rows, wave_dx } => {
            if let Some(cycle) = cycles.get(name) {
                apply_wave(frame, backdrop, tile_grids, palette_ext, cycle, rect, tick, *phase_rows);
            }
        }
        ZoneBehavior::Flicker { cycle: name, density, seed } => {
            if let Some(cycle) = cycles.get(name) {
                apply_flicker(frame, backdrop, tile_grids, palette_ext, cycle, rect, tick, *density, *seed);
            }
        }
        ZoneBehavior::ScrollDown { speed, wrap } => {
            apply_scroll_down(frame, base_img, rect, tick, *speed, *wrap);
        }
        ZoneBehavior::HScrollSine { amplitude, period, speed } => {
            apply_hscroll_sine(frame, base_img, rect, tick, *amplitude, *period, *speed);
        }
        ZoneBehavior::ColorGradient { from, to, vertical } => {
            apply_color_gradient(frame, rect, from, to, *vertical);
        }
        ZoneBehavior::Mosaic { size_x, size_y } => {
            apply_mosaic(frame, rect, *size_x, *size_y);
        }
        ZoneBehavior::Window { .. } => {
            // Window zones are applied during layer compositing, not post-render
        }
        ZoneBehavior::VScrollSine { amplitude, period, speed } => {
            apply_vscroll_sine(frame, base_img, rect, tick, *amplitude, *period, *speed);
        }
        ZoneBehavior::PaletteRamp { symbol, from, to } => {
            apply_palette_ramp(frame, backdrop, tile_grids, palette_ext, rect, symbol, from, to);
        }
    }
}

fn apply_cycle(
    frame: &mut RgbaImage,
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    cycle: &Cycle,
    rect: &ZoneRect,
    tick: u32,
) {
    let len = cycle.symbols.len();
    if len < 2 { return; }
    let shift = compute_cycle_shift(cycle, tick);

    for py in rect.y..(rect.y + rect.h).min(backdrop.height) {
        for px in rect.x..(rect.x + rect.w).min(backdrop.width) {
            if let Some(sym) = get_symbol_at(backdrop, tile_grids, px, py) {
                if let Some(idx) = cycle.symbols.iter().position(|s| *s == sym) {
                    let new_sym = &cycle.symbols[(idx + shift) % len];
                    frame.put_pixel(px, py, resolve_symbol_color(new_sym, palette_ext));
                }
            }
        }
    }
}

fn apply_wave(
    frame: &mut RgbaImage,
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    cycle: &Cycle,
    rect: &ZoneRect,
    tick: u32,
    phase_rows: u32,
) {
    let len = cycle.symbols.len();
    if len < 2 { return; }
    let phase_rows = phase_rows.max(1);

    for py in rect.y..(rect.y + rect.h).min(backdrop.height) {
        let row_offset = (py - rect.y) / phase_rows;
        let shift = compute_cycle_shift(cycle, tick.wrapping_add(row_offset));

        for px in rect.x..(rect.x + rect.w).min(backdrop.width) {
            if let Some(sym) = get_symbol_at(backdrop, tile_grids, px, py) {
                if let Some(idx) = cycle.symbols.iter().position(|s| *s == sym) {
                    let new_sym = &cycle.symbols[(idx + shift) % len];
                    frame.put_pixel(px, py, resolve_symbol_color(new_sym, palette_ext));
                }
            }
        }
    }
}

fn apply_flicker(
    frame: &mut RgbaImage,
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    cycle: &Cycle,
    rect: &ZoneRect,
    tick: u32,
    density: f64,
    seed: u64,
) {
    let len = cycle.symbols.len();
    if len < 2 { return; }
    let shift = compute_cycle_shift(cycle, tick);

    for py in rect.y..(rect.y + rect.h).min(backdrop.height) {
        for px in rect.x..(rect.x + rect.w).min(backdrop.width) {
            let hash = simple_hash(seed, px, py, tick);
            if (hash as f64 / u32::MAX as f64) > density {
                continue;
            }
            if let Some(sym) = get_symbol_at(backdrop, tile_grids, px, py) {
                if let Some(idx) = cycle.symbols.iter().position(|s| *s == sym) {
                    let new_sym = &cycle.symbols[(idx + shift) % len];
                    frame.put_pixel(px, py, resolve_symbol_color(new_sym, palette_ext));
                }
            }
        }
    }
}

fn apply_scroll_down(
    frame: &mut RgbaImage,
    base_img: &RgbaImage,
    rect: &ZoneRect,
    tick: u32,
    speed: f64,
    wrap: bool,
) {
    let offset = (tick as f64 * speed) as u32;
    let zone_h = rect.h;

    for py in rect.y..rect.y + rect.h {
        for px in rect.x..rect.x + rect.w {
            if px >= base_img.width() || py >= base_img.height() {
                continue;
            }
            let src_y = if wrap {
                rect.y + ((py - rect.y + zone_h - (offset % zone_h)) % zone_h)
            } else {
                let sy = py as i64 - offset as i64;
                if sy < rect.y as i64 { continue; }
                sy as u32
            };
            if src_y < base_img.height() {
                frame.put_pixel(px, py, *base_img.get_pixel(px, src_y));
            }
        }
    }
}

// ── SNES HDMA-style scanline effects ────────────────────────────────

/// Sinusoidal horizontal scroll per scanline — the classic SNES water waviness.
fn apply_hscroll_sine(
    frame: &mut RgbaImage,
    base_img: &RgbaImage,
    rect: &ZoneRect,
    tick: u32,
    amplitude: u32,
    period: u32,
    speed: f64,
) {
    let (w, h) = frame.dimensions();
    let period = period.max(1) as f64;
    let phase = tick as f64 * speed * 2.0 * std::f64::consts::PI / 60.0;

    for py in rect.y..(rect.y + rect.h).min(h) {
        let scanline = (py - rect.y) as f64;
        let offset = (amplitude as f64 * (phase + scanline * 2.0 * std::f64::consts::PI / period).sin()) as i32;

        if offset == 0 { continue; }

        for px in rect.x..(rect.x + rect.w).min(w) {
            let src_x = px as i32 - offset;
            if src_x >= rect.x as i32 && src_x < (rect.x + rect.w) as i32 && src_x >= 0 && (src_x as u32) < w {
                frame.put_pixel(px, py, *base_img.get_pixel(src_x as u32, py));
            }
        }
    }
}

/// Apply a color tint gradient across a zone (vertical or horizontal).
fn apply_color_gradient(
    frame: &mut RgbaImage,
    rect: &ZoneRect,
    from: &PaxRgba,
    to: &PaxRgba,
    vertical: bool,
) {
    let (w, h) = frame.dimensions();
    let span = if vertical { rect.h } else { rect.w }.max(1) as f64;

    for py in rect.y..(rect.y + rect.h).min(h) {
        for px in rect.x..(rect.x + rect.w).min(w) {
            let t = if vertical {
                (py - rect.y) as f64 / span
            } else {
                (px - rect.x) as f64 / span
            };

            let pixel = frame.get_pixel(px, py);
            // Multiply-blend the gradient color onto the existing pixel
            let gr = (from.r as f64 * (1.0 - t) + to.r as f64 * t) / 255.0;
            let gg = (from.g as f64 * (1.0 - t) + to.g as f64 * t) / 255.0;
            let gb = (from.b as f64 * (1.0 - t) + to.b as f64 * t) / 255.0;

            frame.put_pixel(px, py, Rgba([
                (pixel.0[0] as f64 * gr).min(255.0) as u8,
                (pixel.0[1] as f64 * gg).min(255.0) as u8,
                (pixel.0[2] as f64 * gb).min(255.0) as u8,
                pixel.0[3],
            ]));
        }
    }
}

/// GBA-style mosaic: pixelate a region with independent X/Y block sizes.
fn apply_mosaic(
    frame: &mut RgbaImage,
    rect: &ZoneRect,
    size_x: u32,
    size_y: u32,
) {
    if size_x <= 1 && size_y <= 1 { return; }
    let (w, h) = frame.dimensions();
    let sx = size_x.max(1);
    let sy = size_y.max(1);

    // For each block, sample the top-left pixel and fill the block
    let mut by = rect.y;
    while by < (rect.y + rect.h).min(h) {
        let mut bx = rect.x;
        while bx < (rect.x + rect.w).min(w) {
            let sample = *frame.get_pixel(bx.min(w - 1), by.min(h - 1));

            for dy in 0..sy {
                for dx in 0..sx {
                    let px = bx + dx;
                    let py = by + dy;
                    if px < (rect.x + rect.w).min(w) && py < (rect.y + rect.h).min(h) {
                        frame.put_pixel(px, py, sample);
                    }
                }
            }
            bx += sx;
        }
        by += sy;
    }
}

/// Genesis VSRAM-style per-column vertical scroll with sine offset.
fn apply_vscroll_sine(
    frame: &mut RgbaImage,
    base_img: &RgbaImage,
    rect: &ZoneRect,
    tick: u32,
    amplitude: u32,
    period: u32,
    speed: f64,
) {
    let (w, h) = frame.dimensions();
    let period = period.max(1) as f64;
    let phase = tick as f64 * speed * 2.0 * std::f64::consts::PI / 60.0;

    for px in rect.x..(rect.x + rect.w).min(w) {
        let column = (px - rect.x) as f64;
        let offset = (amplitude as f64 * (phase + column * 2.0 * std::f64::consts::PI / period).sin()) as i32;

        if offset == 0 { continue; }

        for py in rect.y..(rect.y + rect.h).min(h) {
            let src_y = py as i32 - offset;
            if src_y >= rect.y as i32 && src_y < (rect.y + rect.h) as i32
                && src_y >= 0 && (src_y as u32) < h
            {
                frame.put_pixel(px, py, *base_img.get_pixel(px, src_y as u32));
            }
        }
    }
}

/// Konami raster-style per-scanline palette entry replacement.
/// Interpolates one palette symbol's color from `from` to `to` across the zone height.
fn apply_palette_ramp(
    frame: &mut RgbaImage,
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    rect: &ZoneRect,
    symbol: &str,
    from: &PaxRgba,
    to: &PaxRgba,
) {
    let (w, h) = frame.dimensions();
    let span = rect.h.max(1) as f64;

    for py in rect.y..(rect.y + rect.h).min(h) {
        let t = (py - rect.y) as f64 / span;

        // Interpolated color for this scanline
        let ramp_color = Rgba([
            (from.r as f64 * (1.0 - t) + to.r as f64 * t) as u8,
            (from.g as f64 * (1.0 - t) + to.g as f64 * t) as u8,
            (from.b as f64 * (1.0 - t) + to.b as f64 * t) as u8,
            (from.a as f64 * (1.0 - t) + to.a as f64 * t) as u8,
        ]);

        for px in rect.x..(rect.x + rect.w).min(w) {
            // Only replace pixels that match the target symbol
            if let Some(sym) = get_symbol_at(backdrop, tile_grids, px, py) {
                if sym == symbol {
                    frame.put_pixel(px, py, ramp_color);
                }
            }
        }
    }
}

/// GBA BLDY-style fade: darken to black or brighten to white.
fn apply_fade(img: &mut RgbaImage, target: FadeTarget, amount: f64) {
    let amount = amount.clamp(0.0, 1.0);
    if amount < 0.004 { return; }

    for pixel in img.pixels_mut() {
        if pixel.0[3] == 0 { continue; }
        match target {
            FadeTarget::Black => {
                // I = I * (1 - amount)
                pixel.0[0] = (pixel.0[0] as f64 * (1.0 - amount)) as u8;
                pixel.0[1] = (pixel.0[1] as f64 * (1.0 - amount)) as u8;
                pixel.0[2] = (pixel.0[2] as f64 * (1.0 - amount)) as u8;
            }
            FadeTarget::White => {
                // I = I + (255 - I) * amount
                pixel.0[0] = (pixel.0[0] as f64 + (255.0 - pixel.0[0] as f64) * amount) as u8;
                pixel.0[1] = (pixel.0[1] as f64 + (255.0 - pixel.0[1] as f64) * amount) as u8;
                pixel.0[2] = (pixel.0[2] as f64 + (255.0 - pixel.0[2] as f64) * amount) as u8;
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Look up which symbol is at a pixel position (checks first layer with a match).
fn get_symbol_at(
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    px: u32,
    py: u32,
) -> Option<String> {
    // Check layers back-to-front, return first non-transparent hit
    for layer in backdrop.layers.iter().rev() {
        let col = px / backdrop.tile_width;
        let row = py / backdrop.tile_height;
        if let Some(tile_ref) = layer.tilemap.get(row as usize).and_then(|r| r.get(col as usize)) {
            if let Some(grid) = tile_grids.get(&tile_ref.name) {
                let local_x = px % backdrop.tile_width;
                let local_y = py % backdrop.tile_height;
                let gw = grid.first().map(|r| r.len() as u32).unwrap_or(0);
                let gh = grid.len() as u32;
                let (sx, sy) = apply_flips(local_x, local_y, gw, gh, tile_ref);
                if let Some(sym) = grid.get(sy as usize).and_then(|r| r.get(sx as usize)) {
                    return Some(sym.clone());
                }
            }
        }
    }
    None
}

fn resolve_symbol_color(sym: &str, palette_ext: &PaletteExt) -> Rgba<u8> {
    if sym.len() == 1 {
        if let Some(ch) = sym.chars().next() {
            if let Some(c) = palette_ext.base.get(&ch) {
                return Rgba([c.r, c.g, c.b, c.a]);
            }
        }
    }
    if let Some(c) = palette_ext.extended.get(sym) {
        return Rgba([c.r, c.g, c.b, c.a]);
    }
    Rgba([255, 0, 255, 255]) // magenta = missing
}

fn compute_cycle_shift(cycle: &Cycle, tick: u32) -> usize {
    let len = cycle.symbols.len();
    if len < 2 { return 0; }
    match cycle.direction.as_str() {
        "forward" => (tick as usize) % len,
        "backward" => (len - (tick as usize % len)) % len,
        "ping-pong" => {
            let period = (len - 1) * 2;
            let pos = (tick as usize) % period;
            if pos < len { pos } else { period - pos }
        }
        _ => (tick as usize) % len,
    }
}

fn simple_hash(seed: u64, x: u32, y: u32, tick: u32) -> u32 {
    let mut h = seed.wrapping_mul(0x517cc1b727220a95);
    h = h.wrapping_add(x as u64 * 0x6c62272e07bb0142);
    h = h.wrapping_add(y as u64 * 0x85157af5d7882837);
    h = h.wrapping_add(tick as u64 * 0x9e3779b97f4a7c15);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h >> 32) as u32
}

/// Resolve global animation clock references into per-tile frame sequences.
pub fn resolve_anim_clock_tiles(
    pax: &PaxFile,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
) -> HashMap<String, Vec<(String, u32)>> {
    let mut animated = HashMap::new();

    for (tile_name, tile_raw) in &pax.backdrop_tile {
        // Check anim_clock reference
        if let Some(clock_name) = &tile_raw.anim_clock {
            if let Some(clock) = pax.anim_clock.get(clock_name) {
                let fps = clock.fps.max(1);
                let duration_ms = 1000 / fps;
                let mut frames = Vec::new();

                for i in 0..clock.frames {
                    let companion = if i == 0 {
                        tile_name.clone()
                    } else {
                        format!("{}_{}", tile_name, i)
                    };
                    // Only add if the companion tile exists
                    if tile_grids.contains_key(&companion) || i == 0 {
                        frames.push((companion, duration_ms));
                    }
                }

                if frames.len() > 1 {
                    animated.insert(tile_name.clone(), frames);
                }
            }
        }

        // Also check explicit animation field
        if !tile_raw.animation.is_empty() {
            let frames: Vec<(String, u32)> = tile_raw.animation.iter()
                .map(|f| (f.tile.clone(), f.duration_ms))
                .collect();
            if !frames.is_empty() {
                animated.insert(tile_name.clone(), frames);
            }
        }
    }

    animated
}

/// Export an animated backdrop as GIF.
pub fn export_backdrop_gif(
    backdrop: &Backdrop,
    tile_grids: &HashMap<String, Vec<Vec<String>>>,
    palette_ext: &PaletteExt,
    cycles: &HashMap<String, Cycle>,
    palettes: &HashMap<String, Palette>,
    pax: Option<&PaxFile>,
    num_frames: u32,
    frame_duration_ms: u32,
    scale: u32,
) -> Result<Vec<u8>, String> {
    let base = render_backdrop(backdrop, tile_grids, palette_ext);
    let animated_tiles = pax
        .map(|p| resolve_anim_clock_tiles(p, tile_grids))
        .unwrap_or_default();

    let mut frames = Vec::with_capacity(num_frames as usize);
    for tick in 0..num_frames {
        let frame = render_backdrop_frame(
            &base, backdrop, tile_grids, palette_ext, cycles, palettes, &animated_tiles, tick,
        );

        if scale > 1 {
            let (w, h) = frame.dimensions();
            let scaled = image::imageops::resize(
                &frame, w * scale, h * scale, image::imageops::Nearest,
            );
            frames.push(scaled);
        } else {
            frames.push(frame);
        }
    }

    crate::gif::encode_rgba_gif(&frames, frame_duration_ms)
}
