use wasm_bindgen::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Mutex;

use pixl_core::parser::{parse_pax, resolve_all_palettes};
use pixl_core::types::{Palette, PaxFile};

// ── State ───────────────────────────────────────────────

static STATE: Mutex<Option<WasmState>> = Mutex::new(None);

struct WasmState {
    file: PaxFile,
    palettes: HashMap<String, Palette>,
}

fn with_state<F, R>(f: F) -> Result<R, JsValue>
where
    F: FnOnce(&WasmState) -> R,
{
    let guard = STATE.lock().map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    match guard.as_ref() {
        Some(state) => Ok(f(state)),
        None => Err(JsValue::from_str("no file loaded — call load_pax() first")),
    }
}

fn with_state_mut<F, R>(f: F) -> Result<R, JsValue>
where
    F: FnOnce(&mut WasmState) -> R,
{
    let mut guard = STATE.lock().map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    match guard.as_mut() {
        Some(state) => Ok(f(state)),
        None => Err(JsValue::from_str("no file loaded — call load_pax() first")),
    }
}

// ── Public API ──────────────────────────────────────────

/// Load a .pax source string. Must be called before any other function.
#[wasm_bindgen]
pub fn load_pax(source: &str) -> Result<JsValue, JsValue> {
    let file = parse_pax(source).map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    let palettes =
        resolve_all_palettes(&file).map_err(|e| JsValue::from_str(&format!("{}", e)))?;

    let tile_count = file.tile.len();
    let theme_count = file.theme.len();

    let mut guard = STATE.lock().map_err(|e| JsValue::from_str(&format!("{}", e)))?;
    *guard = Some(WasmState { file, palettes });

    Ok(serde_wasm_bindgen::to_value(&json!({
        "ok": true,
        "tiles": tile_count,
        "themes": theme_count,
    }))?)
}

/// Validate the loaded .pax file. Returns JSON with errors and warnings.
#[wasm_bindgen]
pub fn validate(check_edges: bool) -> Result<JsValue, JsValue> {
    with_state(|state| {
        let result = pixl_core::validate::validate(&state.file, check_edges);
        let errors: Vec<String> = result.errors.iter().map(|e| format!("{}", e)).collect();
        serde_wasm_bindgen::to_value(&json!({
            "errors": errors,
            "warnings": result.warnings,
            "stats": {
                "palettes": result.stats.palettes,
                "themes": result.stats.themes,
                "tiles": result.stats.tiles,
                "stamps": result.stats.stamps,
            }
        })).unwrap()
    })
}

/// List all tile names.
#[wasm_bindgen]
pub fn list_tiles() -> Result<JsValue, JsValue> {
    with_state(|state| {
        let names: Vec<&str> = state.file.tile.keys().map(|s| s.as_str()).collect();
        serde_wasm_bindgen::to_value(&names).unwrap()
    })
}

/// Render a tile to PNG bytes (as base64 string).
#[wasm_bindgen]
pub fn render_tile(name: &str, scale: u32) -> Result<String, JsValue> {
    with_state(|state| {
        let (grid, _w, _h) = pixl_core::resolve::resolve_tile_grid(
            name,
            &state.file.tile,
            &state.palettes,
            &HashMap::new(),
        )
        .map_err(|e| format!("{}", e))?;

        let palette_name = state
            .file
            .tile
            .get(name)
            .map(|t| t.palette.as_str())
            .unwrap_or("");
        let palette = state
            .palettes
            .get(palette_name)
            .ok_or("palette not found")?;

        let img = pixl_render::renderer::render_grid(&grid, palette, scale);
        let png_bytes = pixl_render::renderer::encode_png(&img);

        use base64::Engine;
        Ok(base64::engine::general_purpose::STANDARD.encode(&png_bytes))
    })?
}

/// Get the palette symbols as JSON.
#[wasm_bindgen]
pub fn get_palette(name: &str) -> Result<JsValue, JsValue> {
    with_state(|state| {
        let palette = state
            .palettes
            .get(name)
            .ok_or(JsValue::from_str("palette not found"))?;
        let symbols: HashMap<String, String> = palette
            .symbols
            .iter()
            .map(|(ch, rgba)| {
                (
                    ch.to_string(),
                    format!("#{:02x}{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b, rgba.a),
                )
            })
            .collect();
        Ok(serde_wasm_bindgen::to_value(&symbols)?)
    })?
}

/// Auto-classify edge classes for a grid string.
#[wasm_bindgen]
pub fn classify_edges(grid_str: &str, width: u32, height: u32, palette_name: &str) -> Result<JsValue, JsValue> {
    with_state(|state| {
        let palette = state.palettes.get(palette_name)
            .ok_or(JsValue::from_str("palette not found"))?;
        let grid = pixl_core::grid::parse_grid(grid_str, width, height, palette)
            .map_err(|e| JsValue::from_str(&format!("{}", e)))?;
        let ec = pixl_core::edges::auto_classify_edges(&grid);
        Ok(serde_wasm_bindgen::to_value(&json!({
            "n": ec.n, "e": ec.e, "s": ec.s, "w": ec.w,
        }))?)
    })?
}

/// Get anatomy blueprint as text.
#[wasm_bindgen]
pub fn blueprint(model: &str, width: u32, height: u32) -> Result<String, JsValue> {
    pixl_core::blueprint::render_guide(model, width, height)
        .ok_or_else(|| JsValue::from_str("unknown model"))
}

/// Extract style latent from all tiles. Returns JSON description.
#[wasm_bindgen]
pub fn extract_style() -> Result<JsValue, JsValue> {
    with_state(|state| {
        let palette_name = state.file.tile.values()
            .next().map(|t| t.palette.as_str()).unwrap_or("");
        let palette = state.palettes.get(palette_name)
            .ok_or(JsValue::from_str("no palette"))?;

        let mut grids: Vec<Vec<Vec<char>>> = Vec::new();
        for (name, tile_raw) in &state.file.tile {
            if tile_raw.template.is_some() { continue; }
            match pixl_core::resolve::resolve_tile_grid(
                name, &state.file.tile, &state.palettes, &HashMap::new(),
            ) {
                Ok((grid, _, _)) => grids.push(grid),
                Err(_) => continue,
            }
        }

        let refs: Vec<&Vec<Vec<char>>> = grids.iter().collect();
        let latent = pixl_core::style::StyleLatent::extract(&refs, palette, '.');

        Ok(serde_wasm_bindgen::to_value(&json!({
            "description": latent.describe(),
            "sample_count": latent.sample_count,
            "hue_bias": latent.hue_bias,
            "luminance_mean": latent.luminance_mean,
            "pixel_density": latent.pixel_density,
        }))?)
    })?
}
