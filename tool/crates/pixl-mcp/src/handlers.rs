use crate::state::McpState;
use pixl_core::{blueprint, edges, grid, style::StyleLatent, types::parse_size, validate};
use pixl_render::renderer;
use serde_json::{Value, json};
use std::sync::Mutex;

/// Handle an MCP tool call. Returns the JSON result.
pub fn handle_tool(state: &Mutex<McpState>, tool_name: &str, args: &Value) -> Value {
    match tool_name {
        "pixl_session_start" => handle_session_start(state),
        "pixl_get_palette" => handle_get_palette(state, args),
        "pixl_create_tile" => handle_create_tile(state, args),
        "pixl_validate" => handle_validate(state, args),
        "pixl_render_tile" => handle_render_tile(state, args),
        "pixl_check_edge_pair" => handle_check_edge_pair(state, args),
        "pixl_list_tiles" => handle_list_tiles(state),
        "pixl_get_file" => handle_get_file(state, args),
        "pixl_delete_tile" => handle_delete_tile(state, args),
        "pixl_get_blueprint" => handle_get_blueprint(args),
        "pixl_learn_style" => handle_learn_style(state, args),
        "pixl_check_style" => handle_check_style(state, args),
        _ => json!({"error": format!("unknown tool: {}", tool_name)}),
    }
}

fn handle_session_start(state: &Mutex<McpState>) -> Value {
    let st = state.lock().unwrap();

    let palette_symbols: Value = st
        .palettes
        .iter()
        .map(|(name, pal)| {
            let syms: Value = pal
                .symbols
                .iter()
                .map(|(ch, rgba)| {
                    (
                        ch.to_string(),
                        json!({
                            "hex": format!("#{:02x}{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b, rgba.a),
                        }),
                    )
                })
                .collect::<serde_json::Map<String, Value>>()
                .into();
            (name.clone(), syms)
        })
        .collect::<serde_json::Map<String, Value>>()
        .into();

    json!({
        "active_theme": st.active_theme(),
        "palettes": palette_symbols,
        "canvas_size": st.file.pax.theme.as_deref()
            .and_then(|t| st.file.theme.get(t))
            .and_then(|t| t.canvas)
            .unwrap_or(16),
        "max_palette_size": st.max_palette_size(),
        "light_source": st.light_source(),
        "available_stamps": st.stamp_names(),
        "available_tiles": st.tile_names(),
        "suggested_workflow": "1. Examine palette symbols. 2. Create tiles with pixl_create_tile. 3. Check edges with pixl_check_edge_pair. 4. Validate with pixl_validate. 5. Export with pixl_get_file."
    })
}

fn handle_get_palette(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();
    let theme_name = args["theme"].as_str().unwrap_or("");

    let theme = match st.file.theme.get(theme_name) {
        Some(t) => t,
        None => return json!({"error": format!("theme '{}' not found", theme_name)}),
    };

    let palette = match st.palettes.get(&theme.palette) {
        Some(p) => p,
        None => return json!({"error": format!("palette '{}' not found", theme.palette)}),
    };

    let symbols: Value = palette
        .symbols
        .iter()
        .map(|(ch, rgba)| {
            let role = theme
                .roles
                .iter()
                .find(|(_, v)| v.starts_with(*ch))
                .map(|(k, _)| k.as_str());

            (
                ch.to_string(),
                json!({
                    "hex": format!("#{:02x}{:02x}{:02x}{:02x}", rgba.r, rgba.g, rgba.b, rgba.a),
                    "role": role,
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>()
        .into();

    json!({
        "theme": theme_name,
        "palette": theme.palette,
        "symbols": symbols,
        "max_palette_size": theme.max_palette_size,
        "light_source": theme.light_source,
    })
}

fn handle_create_tile(state: &Mutex<McpState>, args: &Value) -> Value {
    let mut st = state.lock().unwrap();

    let name = args["name"].as_str().unwrap_or("").to_string();
    let palette_name = args["palette"].as_str().unwrap_or("").to_string();
    let size_str = args["size"].as_str().unwrap_or("16x16");
    let grid_str = args["grid"].as_str().unwrap_or("");

    let (w, h) = match parse_size(size_str) {
        Ok(s) => s,
        Err(e) => return json!({"ok": false, "error": e}),
    };

    let palette = match st.palettes.get(&palette_name) {
        Some(p) => p.clone(),
        None => {
            return json!({"ok": false, "error": format!("palette '{}' not found", palette_name)});
        }
    };

    // Parse grid
    let parsed_grid = match grid::parse_grid(grid_str, w, h, &palette) {
        Ok(g) => g,
        Err(e) => return json!({"ok": false, "error": format!("{}", e)}),
    };

    // Auto-classify edges
    let auto_edges = edges::auto_classify_edges(&parsed_grid);

    // Render preview at 16x
    let preview_img = renderer::render_grid(&parsed_grid, &palette, 16);
    let preview_b64 = renderer::png_to_base64(&renderer::encode_png(&preview_img));

    // Store tile in state
    let edge_class = if let Some(ec) = args.get("edge_class") {
        Some(pixl_core::types::EdgeClassRaw {
            n: ec["n"].as_str().unwrap_or(&auto_edges.n).to_string(),
            e: ec["e"].as_str().unwrap_or(&auto_edges.e).to_string(),
            s: ec["s"].as_str().unwrap_or(&auto_edges.s).to_string(),
            w: ec["w"].as_str().unwrap_or(&auto_edges.w).to_string(),
        })
    } else {
        Some(pixl_core::types::EdgeClassRaw {
            n: auto_edges.n.clone(),
            e: auto_edges.e.clone(),
            s: auto_edges.s.clone(),
            w: auto_edges.w.clone(),
        })
    };

    let tags: Vec<String> = args
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    st.file.tile.insert(
        name.clone(),
        pixl_core::types::TileRaw {
            palette: palette_name,
            size: Some(size_str.to_string()),
            encoding: None,
            symmetry: args
                .get("symmetry")
                .and_then(|v| v.as_str())
                .map(String::from),
            auto_rotate: None,
            auto_rotate_weight: None,
            template: None,
            edge_class,
            tags,
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: None,
            grid: Some(grid_str.to_string()),
            rle: None,
            layout: None,
        },
    );

    json!({
        "ok": true,
        "name": name,
        "size": size_str,
        "edge_class_auto": {
            "n": auto_edges.n,
            "e": auto_edges.e,
            "s": auto_edges.s,
            "w": auto_edges.w,
        },
        "preview_b64": preview_b64,
        "refinement_count": 0,
    })
}

fn handle_validate(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();
    let check_edges = args
        .get("check_edges")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let result = validate::validate(&st.file, check_edges);

    let errors: Vec<String> = result.errors.iter().map(|e| format!("{}", e)).collect();
    let warnings: Vec<String> = result.warnings.clone();

    json!({
        "errors": errors,
        "warnings": warnings,
        "stats": {
            "palettes": result.stats.palettes,
            "themes": result.stats.themes,
            "stamps": result.stats.stamps,
            "tiles": result.stats.tiles,
            "sprites": result.stats.sprites,
        }
    })
}

fn handle_render_tile(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();
    let name = args["name"].as_str().unwrap_or("");
    let scale = args.get("scale").and_then(|v| v.as_u64()).unwrap_or(16) as u32;

    let tile_raw = match st.file.tile.get(name) {
        Some(t) => t,
        None => return json!({"error": format!("tile '{}' not found", name)}),
    };

    let palette = match st.palettes.get(&tile_raw.palette) {
        Some(p) => p,
        None => return json!({"error": format!("palette '{}' not found", tile_raw.palette)}),
    };

    let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
    let (w, h) = match parse_size(size_str) {
        Ok(s) => s,
        Err(e) => return json!({"error": e}),
    };

    let grid_str = match &tile_raw.grid {
        Some(g) => g,
        None => return json!({"error": "tile has no grid data"}),
    };

    let parsed = match grid::parse_grid(grid_str, w, h, palette) {
        Ok(g) => g,
        Err(e) => return json!({"error": format!("{}", e)}),
    };

    let img = renderer::render_grid(&parsed, palette, scale);
    let b64 = renderer::png_to_base64(&renderer::encode_png(&img));

    json!({
        "name": name,
        "size": size_str,
        "scale": scale,
        "preview_b64": b64,
    })
}

fn handle_check_edge_pair(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();
    let tile_a = args["tile_a"].as_str().unwrap_or("");
    let tile_b = args["tile_b"].as_str().unwrap_or("");
    let direction = args["direction"].as_str().unwrap_or("");

    let a = match st.file.tile.get(tile_a) {
        Some(t) => t,
        None => return json!({"error": format!("tile '{}' not found", tile_a)}),
    };
    let b = match st.file.tile.get(tile_b) {
        Some(t) => t,
        None => return json!({"error": format!("tile '{}' not found", tile_b)}),
    };

    let (a_edge, b_edge) = match direction {
        "north" => (
            a.edge_class.as_ref().map(|ec| &ec.n),
            b.edge_class.as_ref().map(|ec| &ec.s),
        ),
        "south" => (
            a.edge_class.as_ref().map(|ec| &ec.s),
            b.edge_class.as_ref().map(|ec| &ec.n),
        ),
        "east" => (
            a.edge_class.as_ref().map(|ec| &ec.e),
            b.edge_class.as_ref().map(|ec| &ec.w),
        ),
        "west" => (
            a.edge_class.as_ref().map(|ec| &ec.w),
            b.edge_class.as_ref().map(|ec| &ec.e),
        ),
        _ => return json!({"error": format!("invalid direction: {}", direction)}),
    };

    let compatible = match (a_edge, b_edge) {
        (Some(ae), Some(be)) => ae == be,
        _ => false,
    };

    json!({
        "compatible": compatible,
        "tile_a": tile_a,
        "tile_b": tile_b,
        "direction": direction,
        "edge_a": a_edge,
        "edge_b": b_edge,
        "reason": if compatible { "edge classes match" } else { "edge classes differ" },
    })
}

fn handle_list_tiles(state: &Mutex<McpState>) -> Value {
    let st = state.lock().unwrap();

    let tiles: Vec<Value> = st
        .file
        .tile
        .iter()
        .map(|(name, raw)| {
            json!({
                "name": name,
                "size": raw.size,
                "edge_class": raw.edge_class.as_ref().map(|ec| json!({"n": ec.n, "e": ec.e, "s": ec.s, "w": ec.w})),
                "tags": raw.tags,
                "template": raw.template,
            })
        })
        .collect();

    json!({"tiles": tiles})
}

fn handle_get_file(state: &Mutex<McpState>, _args: &Value) -> Value {
    let st = state.lock().unwrap();
    match st.to_pax_source() {
        Ok(source) => json!({"pax_source": source}),
        Err(e) => json!({"error": e}),
    }
}

fn handle_delete_tile(state: &Mutex<McpState>, args: &Value) -> Value {
    let mut st = state.lock().unwrap();
    let name = args["name"].as_str().unwrap_or("");
    let deleted = st.delete_tile(name);
    json!({
        "ok": deleted,
        "name": name,
        "message": if deleted { "tile deleted" } else { "tile not found" },
    })
}

fn handle_learn_style(state: &Mutex<McpState>, args: &Value) -> Value {
    let mut st = state.lock().unwrap();

    // Collect reference tile names (or use all)
    let tile_filter: Option<Vec<String>> = args
        .get("tiles")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    // Get first palette
    let palette_name = st
        .file
        .tile
        .values()
        .next()
        .map(|t| t.palette.clone())
        .unwrap_or_default();
    let palette = match st.palettes.get(&palette_name) {
        Some(p) => p.clone(),
        None => return json!({"error": "no palette found"}),
    };

    // Collect grids
    let mut grids: Vec<Vec<Vec<char>>> = Vec::new();
    let mut used_names: Vec<String> = Vec::new();

    for (name, tile_raw) in &st.file.tile {
        if tile_raw.template.is_some() || tile_raw.grid.is_none() {
            continue;
        }
        if let Some(ref filter) = tile_filter {
            if !filter.contains(name) {
                continue;
            }
        }
        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = match parse_size(size_str) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if let Some(ref grid_str) = tile_raw.grid {
            if let Ok(g) = grid::parse_grid(grid_str, w, h, &palette) {
                grids.push(g);
                used_names.push(name.clone());
            }
        }
    }

    if grids.is_empty() {
        return json!({"error": "no valid tiles found for style extraction"});
    }

    let grid_refs: Vec<&Vec<Vec<char>>> = grids.iter().collect();
    let latent = StyleLatent::extract(&grid_refs, &palette, '.');

    // Store in session state
    let description = latent.describe();
    st.style_latent = Some(latent);

    json!({
        "ok": true,
        "description": description,
        "reference_tiles": used_names,
        "sample_count": grids.len(),
    })
}

fn handle_check_style(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();

    let name = args["name"].as_str().unwrap_or("");

    let latent = match &st.style_latent {
        Some(l) => l,
        None => return json!({"error": "no style latent — call pixl_learn_style first"}),
    };

    let tile_raw = match st.file.tile.get(name) {
        Some(t) => t,
        None => return json!({"error": format!("tile '{}' not found", name)}),
    };

    let palette = match st.palettes.get(&tile_raw.palette) {
        Some(p) => p,
        None => return json!({"error": "palette not found"}),
    };

    let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
    let (w, h) = match parse_size(size_str) {
        Ok(s) => s,
        Err(e) => return json!({"error": e}),
    };

    let grid_str = match &tile_raw.grid {
        Some(g) => g,
        None => return json!({"error": "tile has no grid data"}),
    };

    let parsed = match grid::parse_grid(grid_str, w, h, palette) {
        Ok(g) => g,
        Err(e) => return json!({"error": format!("{}", e)}),
    };

    let score = latent.score_tile(&parsed, palette, '.');
    let assessment = if score > 0.85 {
        "excellent match"
    } else if score > 0.7 {
        "good match"
    } else if score > 0.5 {
        "moderate match — may need refinement"
    } else {
        "poor match — consider adjusting to match reference style"
    };

    json!({
        "name": name,
        "score": (score * 100.0).round() / 100.0,
        "assessment": assessment,
        "style_description": latent.describe(),
    })
}

fn handle_get_blueprint(args: &Value) -> Value {
    let model = args
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("humanoid_chibi");
    let width = args["width"].as_u64().unwrap_or(32) as u32;
    let height = args["height"].as_u64().unwrap_or(48) as u32;

    match blueprint::render_guide(model, width, height) {
        Some(guide) => {
            let resolved = blueprint::resolve(model, width, height).unwrap();
            json!({
                "model": model,
                "width": width,
                "height": height,
                "guide_text": guide,
                "eye_size": resolved.eye_size,
                "omitted": resolved.omitted,
                "landmarks": resolved.landmarks.iter().map(|l| json!({
                    "name": l.name,
                    "x": l.x,
                    "y": l.y,
                })).collect::<Vec<_>>(),
            })
        }
        None => json!({"error": format!("unknown model: {}", model)}),
    }
}
