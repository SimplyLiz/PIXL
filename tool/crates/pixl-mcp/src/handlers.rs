use crate::inference::InferenceServer;
use crate::state::McpState;
use pixl_core::feedback::{FeedbackAction, FeedbackEvent, RejectReason};
use pixl_core::{blueprint, edges, grid, style::StyleLatent, types::parse_size, validate};
use pixl_wfc::{adjacency::TileEdges, narrate, semantic};
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
        "pixl_narrate_map" => handle_narrate_map(state, args),
        "pixl_render_sprite_gif" => handle_render_sprite_gif(state, args),
        "pixl_learn_style" => handle_learn_style(state, args),
        "pixl_check_style" => handle_check_style(state, args),
        "pixl_generate_context" => handle_generate_context(state, args),
        "pixl_vary_tile" => handle_vary_tile(state, args),
        "pixl_list_themes" => handle_list_themes(state),
        "pixl_list_stamps" => handle_list_stamps(state),
        "pixl_pack_atlas" => handle_pack_atlas(state, args),
        "pixl_load_source" => handle_load_source(state, args),
        "pixl_record_feedback" => handle_record_feedback(state, args),
        "pixl_feedback_stats" => handle_feedback_stats(state),
        "pixl_feedback_constraints" => handle_feedback_constraints(state),
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

    let target_layer: Option<String> = args
        .get("target_layer")
        .and_then(|v| v.as_str())
        .map(String::from);

    let edge_class_for_response = edge_class.clone();

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
            target_layer,
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

    // Build edge context: actual border pixels from compatible neighbors
    let (edge_n, edge_e, edge_s, edge_w) = edges::extract_edges(&parsed_grid);
    let mut compatible_neighbors = serde_json::Map::new();
    for (other_name, other_raw) in &st.file.tile {
        if other_name == &name || other_raw.grid.is_none() {
            continue;
        }
        if let (Some(other_ec), Some(our_ec)) = (&other_raw.edge_class, &edge_class_for_response) {
            let mut dirs = Vec::new();
            if our_ec.n == other_ec.s { dirs.push("can go north"); }
            if our_ec.s == other_ec.n { dirs.push("can go south"); }
            if our_ec.e == other_ec.w { dirs.push("can go east"); }
            if our_ec.w == other_ec.e { dirs.push("can go west"); }
            if !dirs.is_empty() {
                compatible_neighbors.insert(
                    other_name.clone(),
                    json!(dirs),
                );
            }
        }
    }

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
        "edge_pixels": {
            "n": edge_n,
            "e": edge_e,
            "s": edge_s,
            "w": edge_w,
        },
        "compatible_neighbors": compatible_neighbors,
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
                "target_layer": raw.target_layer,
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

fn handle_render_sprite_gif(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();
    let spriteset_name = args.get("spriteset").and_then(|v| v.as_str()).unwrap_or("");
    let sprite_name = args.get("sprite").and_then(|v| v.as_str()).unwrap_or("");
    let scale = args.get("scale").and_then(|v| v.as_u64()).unwrap_or(8) as u32;

    let spriteset = match st.file.spriteset.get(spriteset_name) {
        Some(ss) => ss,
        None => return json!({"error": format!("spriteset '{}' not found", spriteset_name)}),
    };

    let palette = match st.palettes.get(&spriteset.palette) {
        Some(p) => p,
        None => return json!({"error": format!("palette '{}' not found", spriteset.palette)}),
    };

    let (sw, sh) = parse_size(&spriteset.size).unwrap_or((16, 32));

    let sprite = match spriteset.sprite.iter().find(|s| s.name == sprite_name) {
        Some(s) => s,
        None => return json!({"error": format!("sprite '{}' not found in '{}'", sprite_name, spriteset_name)}),
    };

    if sprite.frames.is_empty() {
        return json!({"error": "sprite has no frames"});
    }

    // Resolve frames
    let mut resolved_grids: Vec<Vec<Vec<char>>> = Vec::new();
    let mut base_grids: std::collections::HashMap<u32, Vec<Vec<char>>> = std::collections::HashMap::new();

    for frame in &sprite.frames {
        let encoding = frame.encoding.as_deref().unwrap_or("grid");
        match encoding {
            "grid" => {
                if let Some(ref grid_str) = frame.grid {
                    match grid::parse_grid(grid_str, sw, sh, palette) {
                        Ok(g) => {
                            base_grids.insert(frame.index, g.clone());
                            resolved_grids.push(g);
                        }
                        Err(e) => return json!({"error": format!("frame {}: {}", frame.index, e)}),
                    }
                }
            }
            "delta" => {
                let base_idx = frame.base.unwrap_or(1);
                if let Some(base_grid) = base_grids.get(&base_idx) {
                    let mut new_grid = base_grid.clone();
                    for change in &frame.changes {
                        let ch = change.sym.chars().next().unwrap_or('.');
                        if (change.y as usize) < new_grid.len()
                            && (change.x as usize) < new_grid[0].len()
                        {
                            new_grid[change.y as usize][change.x as usize] = ch;
                        }
                    }
                    resolved_grids.push(new_grid);
                }
            }
            "linked" => {
                let link_idx = frame.link_to.unwrap_or(1);
                if let Some(linked_grid) = base_grids.get(&link_idx) {
                    resolved_grids.push(linked_grid.clone());
                }
            }
            _ => {}
        }
    }

    if resolved_grids.is_empty() {
        return json!({"error": "could not resolve any frames"});
    }

    // Render each frame
    let fps = sprite.fps.max(1);
    let frame_duration_ms = 1000 / fps;

    let gif_frames: Vec<(image::RgbaImage, u32)> = resolved_grids
        .iter()
        .enumerate()
        .map(|(i, grid)| {
            let dur = sprite.frames.get(i)
                .and_then(|f| f.duration_ms)
                .unwrap_or(frame_duration_ms);
            let img = renderer::render_grid(grid, palette, scale);
            (img, dur)
        })
        .collect();

    match pixl_render::gif::encode_gif(&gif_frames, sprite.r#loop) {
        Ok(gif_bytes) => {
            use base64::Engine;
            let gif_b64 = base64::engine::general_purpose::STANDARD.encode(&gif_bytes);
            json!({
                "ok": true,
                "spriteset": spriteset_name,
                "sprite": sprite_name,
                "frames": resolved_grids.len(),
                "fps": fps,
                "gif_b64": gif_b64,
            })
        }
        Err(e) => json!({"error": format!("GIF encode failed: {}", e)}),
    }
}

fn handle_narrate_map(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();

    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(12) as usize;
    let height = args.get("height").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);

    let rules_arr: Vec<String> = args
        .get("rules")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if rules_arr.is_empty() {
        return json!({
            "ok": false,
            "error": "no rules provided. Pass an array of predicate strings.",
            "examples": [
                "border:wall_solid",
                "region:chamber:floor_stone:3x3:southeast",
                "region:entrance:floor_moss:2x2:northwest",
                "path:0,3:11,3"
            ]
        });
    }

    // Get palette
    let palette_name = st.file.tile.values().next().map(|t| t.palette.clone()).unwrap_or_default();
    let palette = match st.palettes.get(&palette_name) {
        Some(p) => p.clone(),
        None => return json!({"ok": false, "error": "no palette found"}),
    };

    // Build tile edges, affordances, grids
    let mut tile_edges = Vec::new();
    let mut tile_affordances = Vec::new();
    let mut tile_names: Vec<String> = Vec::new();
    let mut tile_grids: Vec<Vec<Vec<char>>> = Vec::new();

    for (name, tile_raw) in &st.file.tile {
        if tile_raw.template.is_some() || tile_raw.grid.is_none() {
            continue;
        }
        let ec = tile_raw.edge_class.as_ref();
        tile_edges.push(TileEdges {
            name: name.clone(),
            n: ec.map(|e| e.n.clone()).unwrap_or_default(),
            e: ec.map(|e| e.e.clone()).unwrap_or_default(),
            s: ec.map(|e| e.s.clone()).unwrap_or_default(),
            w: ec.map(|e| e.w.clone()).unwrap_or_default(),
            weight: tile_raw.weight,
        });
        tile_affordances.push(semantic::TileAffordance {
            affordance: tile_raw.semantic.as_ref().and_then(|s| s.affordance.clone()),
        });
        tile_names.push(name.clone());

        let size_str = tile_raw.size.as_deref().unwrap_or("16x16");
        let (w, h) = parse_size(size_str).unwrap_or((16, 16));
        if let Some(ref grid_str) = tile_raw.grid {
            if let Ok(g) = grid::parse_grid(grid_str, w, h, &palette) {
                tile_grids.push(g);
            } else {
                tile_grids.push(vec![vec!['.'; w as usize]; h as usize]);
            }
        } else {
            tile_grids.push(vec![vec!['.'; w as usize]; h as usize]);
        }
    }

    if tile_edges.is_empty() {
        return json!({"ok": false, "error": "no tiles with edge classes found"});
    }

    // Build adjacency rules
    let variant_groups = st.file.wfc_rules.as_ref()
        .map(|r| r.variant_groups.clone()).unwrap_or_default();
    let adj_rules = pixl_wfc::adjacency::AdjacencyRules::build(&tile_edges, &variant_groups);

    // Parse semantic rules
    let forbids: Vec<semantic::SemanticRule> = st.file.wfc_rules.as_ref()
        .map(|r| r.forbids.iter().filter_map(|s| semantic::parse_forbids(s)).collect())
        .unwrap_or_default();
    let requires: Vec<semantic::SemanticRule> = st.file.wfc_rules.as_ref()
        .map(|r| r.requires.iter().filter_map(|s| semantic::parse_requires(s)).collect())
        .unwrap_or_default();
    let require_boost = st.file.wfc_rules.as_ref().map(|r| r.require_boost).unwrap_or(3.0);

    // Parse predicates
    let predicates: Vec<narrate::Predicate> = rules_arr
        .iter()
        .filter_map(|r| narrate::parse_predicate(r))
        .collect();

    let config = narrate::NarrateConfig {
        width,
        height,
        seed,
        max_retries: 5,
        predicates,
    };

    match narrate::narrate_map(&tile_edges, &tile_affordances, &adj_rules, &forbids, &requires, require_boost, &config) {
        Ok(result) => {
            // Render the map
            let tile_size = st.file.tile.values().next()
                .and_then(|t| t.size.as_deref())
                .and_then(|s| parse_size(s).ok())
                .unwrap_or((16, 16));

            let scale = 2u32;
            let img_w = width as u32 * tile_size.0 * scale;
            let img_h = height as u32 * tile_size.1 * scale;
            let mut img = image::ImageBuffer::new(img_w, img_h);

            for (ty, row) in result.grid.iter().enumerate() {
                for (tx, &tile_idx) in row.iter().enumerate() {
                    if tile_idx < tile_grids.len() {
                        let tile_img = renderer::render_grid(&tile_grids[tile_idx], &palette, scale);
                        let ox = tx as u32 * tile_size.0 * scale;
                        let oy = ty as u32 * tile_size.1 * scale;
                        for py in 0..tile_img.height() {
                            for px in 0..tile_img.width() {
                                let ax = ox + px;
                                let ay = oy + py;
                                if ax < img_w && ay < img_h {
                                    img.put_pixel(ax, ay, *tile_img.get_pixel(px, py));
                                }
                            }
                        }
                    }
                }
            }

            let preview_b64 = renderer::png_to_base64(&renderer::encode_png(&img));

            // Build tile name grid
            let tile_grid: Vec<Vec<&str>> = result.grid.iter()
                .map(|row| row.iter().map(|&idx| tile_names.get(idx).map(|s| s.as_str()).unwrap_or("?")).collect())
                .collect();

            json!({
                "ok": true,
                "width": width,
                "height": height,
                "seed": result.seed,
                "retries": result.retries,
                "pins_applied": result.pins_applied,
                "tile_grid": tile_grid,
                "preview_b64": preview_b64,
            })
        }
        Err(e) => json!({
            "ok": false,
            "error": format!("{}", e),
            "hint": "Try simpler predicates, add transition tiles, or increase tileset variety.",
        }),
    }
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

/// Build an enriched generation context for the Studio to send to Claude.
/// Returns the system prompt + constraints the Studio injects into its Anthropic call.
fn handle_vary_tile(state: &Mutex<McpState>, args: &Value) -> Value {
    let mut st = state.lock().unwrap();
    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(4) as usize;
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);

    let palette = {
        let tile_raw = match st.file.tile.get(name) {
            Some(t) => t,
            None => return json!({"error": format!("tile '{}' not found", name)}),
        };
        match st.palettes.get(&tile_raw.palette) {
            Some(p) => p.clone(),
            None => return json!({"error": "palette not found"}),
        }
    };

    let (base_grid, w, h) = match pixl_core::resolve::resolve_tile_grid(
        name,
        &st.file.tile,
        &st.palettes,
        &std::collections::HashMap::new(),
    ) {
        Ok(r) => r,
        Err(e) => return json!({"error": format!("{}", e)}),
    };

    let variants = pixl_core::vary::generate_variants(name, &base_grid, &palette, count, seed, '.');

    // Grab what we need before mutating
    let base_palette_name = st.file.tile.get(name).map(|t| t.palette.clone()).unwrap_or_default();
    let base_semantic = st.file.tile.get(name).and_then(|t| t.semantic.clone());

    // Store variants in session and render previews
    let mut results = Vec::new();
    for v in &variants {
        let preview_img = renderer::render_grid(&v.grid, &palette, 8);
        let b64 = renderer::png_to_base64(&renderer::encode_png(&preview_img));

        let grid_string: String = v.grid.iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        // Auto-classify edges
        let auto_edges = edges::auto_classify_edges(&v.grid);

        // Store in session
        st.file.tile.insert(
            v.name.clone(),
            pixl_core::types::TileRaw {
                palette: base_palette_name.clone(),
                size: Some(format!("{}x{}", w, h)),
                encoding: None,
                symmetry: None,
                auto_rotate: None,
                auto_rotate_weight: None,
                template: None,
                edge_class: Some(pixl_core::types::EdgeClassRaw {
                    n: auto_edges.n.clone(),
                    e: auto_edges.e.clone(),
                    s: auto_edges.s.clone(),
                    w: auto_edges.w.clone(),
                }),
                tags: vec!["variant".to_string(), format!("base:{}", name)],
                target_layer: None,
                weight: 1.0,
                palette_swaps: vec![],
                cycles: vec![],
                nine_slice: None,
                visual_height_extra: None,
                semantic: base_semantic.clone(),
                grid: Some(grid_string),
                rle: None,
                layout: None,
            },
        );

        results.push(json!({
            "name": v.name,
            "mutation": v.mutation,
            "preview_b64": b64,
        }));
    }

    json!({
        "ok": true,
        "base_tile": name,
        "count": variants.len(),
        "seed": seed,
        "variants": results,
    })
}

fn handle_generate_context(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();

    let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let tile_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("tile");
    let size_str = args.get("size").and_then(|v| v.as_str()).unwrap_or("16x16");

    // Build palette constraint text
    let mut palette_text = String::new();
    for (pal_name, palette) in &st.palettes {
        palette_text.push_str(&format!("Palette '{}':\n", pal_name));
        for (sym, rgba) in &palette.symbols {
            palette_text.push_str(&format!(
                "  '{}' = #{:02x}{:02x}{:02x}\n",
                sym, rgba.r, rgba.g, rgba.b
            ));
        }
    }

    // Build theme constraint text
    let mut theme_text = String::new();
    if let Some(theme_name) = st.active_theme() {
        if let Some(theme) = st.file.theme.get(theme_name) {
            theme_text.push_str(&format!("Active theme: {}\n", theme_name));
            if let Some(max) = theme.max_palette_size {
                theme_text.push_str(&format!("Max palette size: {} colors\n", max));
            }
            if let Some(ref light) = theme.light_source {
                theme_text.push_str(&format!("Light source: {}\n", light));
            }
            for (role, sym) in &theme.roles {
                theme_text.push_str(&format!("  Role '{}' = '{}'\n", role, sym));
            }
        }
    }

    // Build style latent text
    let style_text = st
        .style_latent
        .as_ref()
        .map(|l| l.describe())
        .unwrap_or_default();

    // Build edge context from existing tiles
    let mut edge_context = String::new();
    for (name, tile_raw) in &st.file.tile {
        if let Some(ref ec) = tile_raw.edge_class {
            edge_context.push_str(&format!(
                "  {}: n={}, e={}, s={}, w={}\n",
                name, ec.n, ec.e, ec.s, ec.w
            ));
        }
    }

    // Build target_layer context — list available layers and existing tile assignments
    let layer_roles = ["background", "terrain", "walls", "platform", "foreground", "effects"];
    let mut layer_context = String::from("Available target layers: ");
    layer_context.push_str(&layer_roles.join(", "));
    layer_context.push('\n');

    // Show existing tile → layer assignments
    let mut layer_assignments = String::new();
    for (name, tile_raw) in &st.file.tile {
        if let Some(ref tl) = tile_raw.target_layer {
            layer_assignments.push_str(&format!("  {} → {}\n", name, tl));
        }
    }
    if !layer_assignments.is_empty() {
        layer_context.push_str("Existing tile layer assignments:\n");
        layer_context.push_str(&layer_assignments);
    }

    // Get feedback constraints — structured, not prompt injection
    let constraints = st.feedback.constraints();

    // Build few-shot examples section from accepted tiles
    let mut examples_text = String::new();
    if !constraints.examples.is_empty() {
        examples_text.push_str("\nReference tiles (accepted by artist):\n");
        for ex in &constraints.examples {
            examples_text.push_str(&format!(
                "  {} [{}]:\n```\n{}\n```\n",
                ex.name,
                ex.tags.join(", "),
                ex.grid
            ));
        }
    }

    // Build avoid constraints from rejection patterns
    let mut avoid_text = String::new();
    if !constraints.avoid.is_empty() {
        avoid_text.push_str("\nLearned constraints (from artist feedback):\n");
        for c in &constraints.avoid {
            avoid_text.push_str(&format!("- {}\n", c));
        }
    }

    // Preferred style as structured features (not prose)
    let preference_text = if let Some(ref pref) = constraints.preferred_style {
        format!(
            "\nPreferred style profile (from accepted tiles):\n{}",
            pref.describe()
        )
    } else {
        String::new()
    };

    // Build the enriched system prompt
    let system_prompt = format!(
        "You are a pixel art expert generating PAX format tiles.\n\
         \n\
         {palette_text}\n\
         {theme_text}\n\
         {style_text}\n\
         {preference_text}\n\
         Canvas size: {size_str}\n\
         Type: {tile_type}\n\
         \n\
         {layer_context}\n\
         Existing tile edge classes:\n\
         {edge_context}\n\
         {examples_text}\
         {avoid_text}\
         \n\
         Rules:\n\
         - Use ONLY symbols from the palette above\n\
         - Output a raw character grid (one row per line)\n\
         - Shadows go bottom-right of structures\n\
         - Highlights go top-left of surfaces\n\
         - Grid must be exactly {size_str} characters\n\
         - For WFC compatibility, edges should match neighboring tiles\n\
         - Suggest a target_layer for this tile (background/terrain/walls/platform/foreground/effects)"
    );

    // Build the enriched user prompt
    let user_prompt = format!(
        "Generate a {size_str} {tile_type}: {prompt}"
    );

    let stats = st.feedback.stats();

    json!({
        "system_prompt": system_prompt,
        "user_prompt": user_prompt,
        "palette_symbols": palette_text.trim(),
        "theme": st.active_theme(),
        "size": size_str,
        "existing_tiles": st.tile_names(),
        "style_latent": style_text,
        "available_layers": layer_roles,
        "feedback": {
            "min_style_score": constraints.min_style_score,
            "acceptance_rate": stats.acceptance_rate,
            "total_feedback": stats.total_accepts + stats.total_rejects,
            "example_count": constraints.examples.len(),
            "avoid_count": constraints.avoid.len(),
        },
    })
}

fn handle_list_themes(state: &Mutex<McpState>) -> Value {
    let st = state.lock().unwrap();

    let themes: Vec<Value> = st
        .file
        .theme
        .iter()
        .map(|(name, theme)| {
            json!({
                "name": name,
                "palette": theme.palette,
                "scale": theme.scale,
                "canvas": theme.canvas,
                "max_palette_size": theme.max_palette_size,
                "light_source": theme.light_source,
                "roles": theme.roles,
            })
        })
        .collect();

    json!({"themes": themes})
}

fn handle_list_stamps(state: &Mutex<McpState>) -> Value {
    let st = state.lock().unwrap();

    let stamps: Vec<Value> = st
        .file
        .stamp
        .iter()
        .map(|(name, stamp)| {
            json!({
                "name": name,
                "palette": stamp.palette,
                "size": stamp.size,
            })
        })
        .collect();

    json!({"stamps": stamps})
}

fn handle_pack_atlas(state: &Mutex<McpState>, args: &Value) -> Value {
    let st = state.lock().unwrap();

    let columns = args.get("columns").and_then(|v| v.as_u64()).unwrap_or(8) as u32;
    let padding = args.get("padding").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
    let scale = args.get("scale").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

    let palette_name = st
        .file
        .tile
        .values()
        .next()
        .map(|t| t.palette.clone())
        .unwrap_or_default();
    let palette = match st.palettes.get(&palette_name) {
        Some(p) => p.clone(),
        None => return json!({"ok": false, "error": "no palette found"}),
    };

    let mut tiles = Vec::new();
    for (name, tile_raw) in &st.file.tile {
        if tile_raw.template.is_some() {
            continue;
        }
        match pixl_core::resolve::resolve_tile_grid(
            name,
            &st.file.tile,
            &st.palettes,
            &std::collections::HashMap::new(),
        ) {
            Ok((grid_data, w, h)) => {
                tiles.push(pixl_render::atlas::AtlasTile {
                    name: name.clone(),
                    grid: grid_data,
                    width: w,
                    height: h,
                });
            }
            Err(_) => continue,
        }
    }

    if tiles.is_empty() {
        return json!({"ok": false, "error": "no tiles to pack"});
    }

    match pixl_render::atlas::pack_atlas(&tiles, &palette, columns, padding, scale, "atlas.png") {
        Ok((img, atlas_json)) => {
            let b64 = renderer::png_to_base64(&renderer::encode_png(&img));
            let json_str = serde_json::to_string(&atlas_json).unwrap_or_default();
            json!({
                "ok": true,
                "atlas_b64": b64,
                "atlas_json": json_str,
                "tile_count": tiles.len(),
                "width": img.width(),
                "height": img.height(),
            })
        }
        Err(e) => json!({"ok": false, "error": e}),
    }
}

fn handle_load_source(state: &Mutex<McpState>, args: &Value) -> Value {
    let mut st = state.lock().unwrap();
    let source = match args.get("source").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return json!({"ok": false, "error": "missing 'source' field"}),
    };

    match pixl_core::parser::parse_pax(source) {
        Ok(file) => {
            match pixl_core::parser::resolve_all_palettes(&file) {
                Ok(palettes) => {
                    let tile_count = file.tile.len();
                    let theme_count = file.theme.len();
                    st.file = file;
                    st.palettes = palettes;
                    st.refinement_count.clear();
                    st.style_latent = None;
                    json!({
                        "ok": true,
                        "tiles": tile_count,
                        "themes": theme_count,
                    })
                }
                Err(e) => json!({"ok": false, "error": format!("{}", e)}),
            }
        }
        Err(e) => json!({"ok": false, "error": format!("{}", e)}),
    }
}

// ── Feedback ─────────────────────────────────────────────────────────

fn handle_record_feedback(state: &Mutex<McpState>, args: &Value) -> Value {
    let mut st = state.lock().unwrap();

    let tile_name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return json!({"error": "name required"}),
    };

    let action = match args.get("action").and_then(|v| v.as_str()) {
        Some("accept") => FeedbackAction::Accept,
        Some("reject") => FeedbackAction::Reject,
        Some("edit") => FeedbackAction::Edit,
        _ => return json!({"error": "action must be accept, reject, or edit"}),
    };

    let reject_reason = args.get("reject_reason").and_then(|v| v.as_str()).map(|r| {
        match r {
            "too_sparse" => RejectReason::TooSparse,
            "too_dense" => RejectReason::TooDense,
            "wrong_style" => RejectReason::WrongStyle,
            "bad_edges" => RejectReason::BadEdges,
            "palette_violation" => RejectReason::PaletteViolation,
            "bad_composition" => RejectReason::BadComposition,
            other => RejectReason::Other(other.to_string()),
        }
    });

    // Extract tile features + grid if the tile exists
    let (tile_features, grid, tags, target_layer) = if let Some(tile_raw) = st.file.tile.get(&tile_name) {
        let tags = tile_raw.tags.clone();
        let target_layer = tile_raw.target_layer.clone();

        // Resolve grid and compute features
        let resolved = pixl_core::resolve::resolve_tile_grid(
            &tile_name,
            &st.file.tile,
            &st.palettes,
            &std::collections::HashMap::new(),
        );
        match resolved {
            Ok((grid_data, _, _)) => {
                // Get palette for feature extraction
                let palette_name = &tile_raw.palette;
                let features = st.palettes.get(palette_name).map(|pal| {
                    let void_sym = pal.symbols.iter()
                        .find(|(_, rgba)| rgba.a == 0)
                        .map(|(c, _)| *c)
                        .unwrap_or('.');
                    StyleLatent::extract(&[&grid_data], pal, void_sym)
                });
                let style_score = features.as_ref().and_then(|f| {
                    st.style_latent.as_ref().map(|latent| {
                        let pal = st.palettes.get(palette_name)?;
                        let void_sym = pal.symbols.iter()
                            .find(|(_, rgba)| rgba.a == 0)
                            .map(|(c, _)| *c)
                            .unwrap_or('.');
                        Some(latent.score_tile(&grid_data, pal, void_sym))
                    }).flatten()
                });

                (features, Some(grid_data), tags, target_layer)
            }
            Err(_) => (None, None, tags, target_layer),
        }
    } else {
        (None, None, vec![], None)
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Compute style score
    let style_score = tile_features.as_ref().and_then(|features| {
        st.style_latent.as_ref().and_then(|latent| {
            grid.as_ref().and_then(|g| {
                let tile_raw = st.file.tile.get(&tile_name)?;
                let pal = st.palettes.get(&tile_raw.palette)?;
                let void_sym = pal.symbols.iter()
                    .find(|(_, rgba)| rgba.a == 0)
                    .map(|(c, _)| *c)
                    .unwrap_or('.');
                Some(latent.score_tile(g, pal, void_sym))
            })
        })
    });

    st.feedback.record(FeedbackEvent {
        tile_name: tile_name.clone(),
        action,
        tile_features,
        style_score,
        reject_reason,
        grid,
        tags,
        target_layer,
        timestamp,
    });

    // Auto-update style latent on accept
    if action == FeedbackAction::Accept || action == FeedbackAction::Edit {
        // Rebuild style latent from all accepted tiles
        let accepted_grids: Vec<Vec<Vec<char>>> = st.feedback.events().iter()
            .filter(|e| e.action == FeedbackAction::Accept || e.action == FeedbackAction::Edit)
            .filter_map(|e| e.grid.clone())
            .collect();

        if !accepted_grids.is_empty() {
            // Find a palette for extraction
            if let Some(first_pal) = st.palettes.values().next() {
                let void_sym = first_pal.symbols.iter()
                    .find(|(_, rgba)| rgba.a == 0)
                    .map(|(c, _)| *c)
                    .unwrap_or('.');
                let grid_refs: Vec<&Vec<Vec<char>>> = accepted_grids.iter().collect();
                st.style_latent = Some(StyleLatent::extract(
                    &grid_refs,
                    first_pal,
                    void_sym,
                ));
            }
        }
    }

    // Persist to disk
    st.save_feedback();

    let stats = st.feedback.stats();
    json!({
        "ok": true,
        "recorded": tile_name,
        "acceptance_rate": stats.acceptance_rate,
        "total_feedback": stats.total_accepts + stats.total_rejects + stats.total_edits,
        "style_score": style_score,
    })
}

fn handle_feedback_stats(state: &Mutex<McpState>) -> Value {
    let st = state.lock().unwrap();
    let stats = st.feedback.stats();
    json!({
        "total_accepts": stats.total_accepts,
        "total_rejects": stats.total_rejects,
        "total_edits": stats.total_edits,
        "acceptance_rate": stats.acceptance_rate,
        "avg_accepted_score": stats.avg_accepted_score,
        "avg_rejected_score": stats.avg_rejected_score,
        "top_reject_reasons": stats.top_reject_reasons,
    })
}

fn handle_feedback_constraints(state: &Mutex<McpState>) -> Value {
    let st = state.lock().unwrap();
    let constraints = st.feedback.constraints();
    json!({
        "avoid": constraints.avoid,
        "examples": constraints.examples,
        "min_style_score": constraints.min_style_score,
        "has_preferred_style": constraints.preferred_style.is_some(),
    })
}

/// Generate a tile using the local LoRA-powered model (async).
/// Builds context from session state, sends to mlx_lm.server, parses the
/// response grid, and creates the tile in-session.
pub async fn handle_generate_tile(
    state: &Mutex<McpState>,
    inference: &tokio::sync::Mutex<Option<InferenceServer>>,
    args: &Value,
) -> Value {
    let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let size_str = args.get("size").and_then(|v| v.as_str()).unwrap_or("16x16");

    if name.is_empty() {
        return json!({"ok": false, "error": "name is required"});
    }
    if prompt.is_empty() {
        return json!({"ok": false, "error": "prompt is required"});
    }

    // Build generation context from session state (sync, hold lock briefly)
    let context_args = json!({
        "prompt": prompt,
        "type": "tile",
        "size": size_str,
    });
    let context = handle_generate_context(state, &context_args);

    let system_prompt = context
        .get("system_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let user_prompt = context
        .get("user_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Ensure inference server is running and generate
    let mut inf_guard = inference.lock().await;
    let server = match inf_guard.as_mut() {
        Some(s) => s,
        None => {
            return json!({
                "ok": false,
                "error": "local inference not configured. Start with --model and --adapter flags.",
                "hint": "pixl serve --model mlx-community/Qwen2.5-3B-Instruct-4bit --adapter training/adapters/pixl-lora-v2"
            });
        }
    };

    if let Err(e) = server.ensure_running().await {
        return json!({"ok": false, "error": format!("inference server: {}", e)});
    }

    let raw_response = match server.generate(system_prompt, user_prompt).await {
        Ok(r) => r,
        Err(e) => return json!({"ok": false, "error": format!("generation failed: {}", e)}),
    };
    drop(inf_guard);

    // Extract the grid from the response — look for a code block or raw grid lines
    let grid_str = extract_grid(&raw_response, size_str);

    if grid_str.is_empty() {
        return json!({
            "ok": false,
            "error": "could not parse grid from model response",
            "raw_response": raw_response,
        });
    }

    // Determine the palette to use (first available)
    let palette_name = {
        let st = state.lock().unwrap();
        args.get("palette")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| st.palettes.keys().next().cloned())
            .unwrap_or_default()
    };

    // Create the tile via the existing handler
    let create_args = json!({
        "name": name,
        "palette": palette_name,
        "size": size_str,
        "grid": grid_str,
    });
    let mut result = handle_create_tile(state, &create_args);

    // Annotate with generation metadata
    if let Some(obj) = result.as_object_mut() {
        obj.insert("generated".to_string(), json!(true));
        obj.insert("model".to_string(), json!("local-lora"));
        obj.insert("prompt".to_string(), json!(prompt));
    }

    result
}

/// Extract a character grid from the model's raw text response.
/// Handles code fences, leading/trailing whitespace, and explanatory text.
fn extract_grid(response: &str, expected_size: &str) -> String {
    let (w, _h) = parse_size(expected_size).unwrap_or((16, 16));
    let w = w as usize;

    // Try to find a code block first
    if let Some(start) = response.find("```") {
        let after_fence = &response[start + 3..];
        // Skip optional language tag on the same line
        let content_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let content = &after_fence[content_start..];
        if let Some(end) = content.find("```") {
            let block = content[..end].trim();
            if !block.is_empty() {
                return block.to_string();
            }
        }
    }

    // Fallback: find consecutive lines that look like grid rows (length == w, ascii)
    let mut grid_lines = Vec::new();
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.len() == w && trimmed.chars().all(|c| !c.is_whitespace()) {
            grid_lines.push(trimmed);
        } else if !grid_lines.is_empty() {
            // Stop at first non-grid line after we started collecting
            break;
        }
    }

    grid_lines.join("\n")
}
