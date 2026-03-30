/// HTTP API server for PIXL Studio integration.
/// Exposes the same handlers as the MCP server over REST endpoints.
use crate::{
    adapters,
    handlers,
    inference::{InferenceConfig, InferenceServer},
    state::McpState,
};
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub mcp: Mutex<McpState>,
    pub inference: tokio::sync::Mutex<Option<InferenceServer>>,
    /// Currently activated adapter path (for future hot-swap).
    pub active_adapter: Mutex<Option<PathBuf>>,
    /// Training job state — tracks a background training process.
    pub training_job: Mutex<Option<TrainingJob>>,
}

/// Background training job state.
pub struct TrainingJob {
    pub total_iters: usize,
    pub current_iter: usize,
    pub last_loss: Option<f64>,
    pub adapter_path: PathBuf,
    pub pid: Option<u32>,
    pub done: bool,
    pub error: Option<String>,
    pub paused: bool,
    pub throttle: String,
    pub best_loss: Option<f64>,
    pub speed: Option<f64>,
    pub epoch: usize,
    pub total_epochs: usize,
    pub train_samples: usize,
}

type SharedState = Arc<AppState>;

/// Create the axum router with all endpoints.
pub fn create_router(state: McpState, inference_config: Option<InferenceConfig>) -> Router {
    let inference = inference_config.map(InferenceServer::new);
    let shared = Arc::new(AppState {
        mcp: Mutex::new(state),
        inference: tokio::sync::Mutex::new(inference),
        active_adapter: Mutex::new(None),
        training_job: Mutex::new(None),
    });

    Router::new()
        .route("/health", get(health))
        .route("/api/session", post(session_start))
        .route("/api/palette", post(get_palette))
        .route("/api/tile/create", post(create_tile))
        .route("/api/tile/render", post(render_tile))
        .route("/api/tile/export-png", post(export_png))
        .route("/api/tile/delete", post(delete_tile))
        .route("/api/tile/edge-check", post(check_edge_pair))
        .route("/api/tiles", get(list_tiles))
        .route("/api/validate", post(validate))
        .route("/api/narrate", post(narrate_map))
        .route("/api/style/learn", post(learn_style))
        .route("/api/style/check", post(check_style))
        .route("/api/blueprint", post(get_blueprint))
        .route("/api/sprite/gif", post(render_sprite_gif))
        .route("/api/file", get(get_file))
        .route("/api/generate/context", post(generate_context))
        .route("/api/generate/tile", post(generate_tile))
        .route("/api/tile/vary", post(vary_tile))
        .route("/api/themes", get(list_themes))
        .route("/api/stamps", get(list_stamps))
        .route("/api/atlas/pack", post(pack_atlas))
        .route("/api/load", post(load_source))
        .route("/api/feedback", post(record_feedback))
        .route("/api/feedback/stats", get(feedback_stats))
        .route("/api/feedback/constraints", get(feedback_constraints))
        .route("/api/training/export", post(export_training))
        .route("/api/training/stats", get(training_stats))
        .route("/api/new", post(new_from_template))
        .route("/api/export", post(export_engine))
        .route("/api/check/completeness", get(check_completeness))
        .route("/api/tile/generate-transition", post(generate_transition))
        .route("/api/convert", post(convert_sprite))
        .route("/api/backdrop/import", post(backdrop_import))
        .route("/api/backdrop/render", post(backdrop_render))
        .route("/api/composites", get(list_composites))
        .route("/api/composite/render", post(render_composite))
        .route("/api/composite/check-seams", get(check_composite_seams))
        .route("/api/tile/critique", post(critique_tile))
        .route("/api/tile/refine", post(refine_tile))
        .route("/api/tile/upscale", post(upscale_tile))
        .route("/api/tile/references", post(show_references))
        .route("/api/tile/generate-sprite", post(generate_sprite))
        .route("/api/tool", post(generic_tool_call))
        .route("/api/adapters", get(list_adapters))
        .route("/api/adapter/activate", post(activate_adapter))
        .route("/api/scan/start", post(scan_start))
        .route("/api/prepare", post(prepare))
        .route("/api/train/start", post(train_start))
        .route("/api/train/status", get(train_status))
        .route("/api/train/stop", post(train_stop))
        .route("/api/train/pause", post(train_pause))
        .route("/api/train/throttle", post(train_throttle))
        .route("/api/datasets", get(list_datasets).post(list_datasets_post))
        .with_state(shared)
}

async fn health() -> &'static str {
    "pixl ok"
}

async fn session_start(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_session_start",
        &Value::Null,
    ))
}

async fn get_palette(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_get_palette", &args))
}

async fn create_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_create_tile", &args))
}

async fn render_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_render_tile", &args))
}

async fn export_png(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_export_png", &args))
}

async fn delete_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_delete_tile", &args))
}

async fn check_edge_pair(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_check_edge_pair",
        &args,
    ))
}

async fn list_tiles(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_list_tiles",
        &Value::Null,
    ))
}

async fn validate(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_validate", &args))
}

async fn narrate_map(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_narrate_map", &args))
}

async fn learn_style(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_learn_style", &args))
}

async fn check_style(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_check_style", &args))
}

async fn get_blueprint(Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &Mutex::new(McpState::new()),
        "pixl_get_blueprint",
        &args,
    ))
}

async fn render_sprite_gif(
    State(state): State<SharedState>,
    Json(args): Json<Value>,
) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_render_sprite_gif",
        &args,
    ))
}

async fn get_file(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_get_file",
        &Value::Null,
    ))
}

async fn generate_context(
    State(state): State<SharedState>,
    Json(args): Json<Value>,
) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_generate_context",
        &args,
    ))
}

async fn generate_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_generate_tile(&state.mcp, &state.inference, &args).await)
}

async fn list_themes(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_list_themes",
        &Value::Null,
    ))
}

async fn list_stamps(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_list_stamps",
        &Value::Null,
    ))
}

async fn pack_atlas(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_pack_atlas", &args))
}

async fn vary_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_vary_tile", &args))
}

async fn load_source(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_load_source", &args))
}

async fn record_feedback(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_record_feedback",
        &args,
    ))
}

async fn feedback_stats(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_feedback_stats",
        &Value::Null,
    ))
}

async fn export_training(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_export_training",
        &args,
    ))
}

async fn training_stats(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_training_stats",
        &Value::Null,
    ))
}

async fn feedback_constraints(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_feedback_constraints",
        &Value::Null,
    ))
}

async fn generate_transition(
    State(state): State<SharedState>,
    Json(args): Json<Value>,
) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_generate_transition_context",
        &args,
    ))
}

async fn convert_sprite(Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &Mutex::new(McpState::new()),
        "pixl_convert_sprite",
        &args,
    ))
}

async fn backdrop_import(Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &Mutex::new(McpState::new()),
        "pixl_backdrop_import",
        &args,
    ))
}

async fn backdrop_render(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_backdrop_render",
        &args,
    ))
}

async fn check_completeness(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_check_completeness",
        &Value::Null,
    ))
}

async fn new_from_template(Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &Mutex::new(McpState::new()),
        "pixl_new_from_template",
        &args,
    ))
}

async fn export_engine(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_export", &args))
}

async fn list_composites(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_list_composites",
        &Value::Null,
    ))
}

async fn render_composite(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_render_composite",
        &args,
    ))
}

async fn check_composite_seams(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(
        &state.mcp,
        "pixl_check_seams",
        &Value::Null,
    ))
}

async fn generate_sprite(
    State(state): State<SharedState>,
    Json(args): Json<Value>,
) -> Json<Value> {
    Json(handlers::handle_generate_sprite(&state.mcp, &args).await)
}

async fn show_references(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_show_references", &args))
}

async fn upscale_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_upscale_tile", &args))
}

async fn critique_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_critique_tile", &args))
}

async fn refine_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state.mcp, "pixl_refine_tile", &args))
}

/// Generic tool call endpoint — accepts { "tool": "pixl_xxx", "args": {...} }
async fn generic_tool_call(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let tool_name = body
        .get("tool")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let args = body.get("args").cloned().unwrap_or(Value::Null);

    if tool_name == "pixl_generate_tile" {
        return Json(handlers::handle_generate_tile(&state.mcp, &state.inference, &args).await);
    }

    Json(handlers::handle_tool(&state.mcp, tool_name, &args))
}

// ─── Adapter & training pipeline endpoints ──────────────────────────────────

async fn list_adapters(State(_state): State<SharedState>) -> Json<Value> {
    // Search several candidate directories for adapters
    let candidates = [
        PathBuf::from("training/adapters"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("training/adapters")))
            .unwrap_or_default(),
    ];

    let mut all = Vec::new();
    for dir in &candidates {
        if dir.as_os_str().is_empty() || !dir.exists() {
            continue;
        }
        all.extend(adapters::list_adapters(dir));
    }

    // Deduplicate by path
    all.sort_by(|a, b| a.path.cmp(&b.path));
    all.dedup_by(|a, b| a.path == b.path);

    serde_json::to_value(&all)
        .map(Json)
        .unwrap_or_else(|e| Json(serde_json::json!({"error": e.to_string()})))
}

async fn activate_adapter(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let path_str = match body.get("path").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return Json(serde_json::json!({"error": "missing 'path' field"})),
    };

    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Json(serde_json::json!({"error": format!("path not found: {}", path.display())}));
    }

    // Validate it's a real adapter directory
    let has_safetensors = path.join("adapters.safetensors").exists();
    let has_config = path.join("adapter_config.json").exists();
    if !has_safetensors && !has_config {
        return Json(serde_json::json!({
            "error": format!("not a valid adapter: {} (no adapters.safetensors or adapter_config.json)", path.display())
        }));
    }

    // Store the active adapter
    if let Ok(mut guard) = state.active_adapter.lock() {
        *guard = Some(path.clone());
    }

    // Hot-swap: stop existing inference server and start with new adapter
    let mut inf_guard = state.inference.lock().await;
    if let Some(ref mut server) = *inf_guard {
        server.stop();
        // Reconfigure with new adapter path
        server.set_adapter(path.clone());
        match server.ensure_running().await {
            Ok(_) => Json(serde_json::json!({
                "ok": true,
                "active_adapter": path.to_string_lossy(),
                "hot_swapped": true,
            })),
            Err(e) => Json(serde_json::json!({
                "ok": true,
                "active_adapter": path.to_string_lossy(),
                "hot_swapped": false,
                "warning": format!("adapter stored but inference restart failed: {e}"),
            })),
        }
    } else {
        // No inference server running — just store the path
        Json(serde_json::json!({
            "ok": true,
            "active_adapter": path.to_string_lossy(),
            "hot_swapped": false,
        }))
    }
}

async fn scan_start(Json(body): Json<Value>) -> Json<Value> {
    use pixl_render::scan::{self, ScanConfig};

    let input = match body.get("input").and_then(|v| v.as_str()) {
        Some(s) => PathBuf::from(s),
        None => return Json(serde_json::json!({"error": "missing 'input' field"})),
    };
    let patch_size = body.get("patch_size").and_then(|v| v.as_u64()).unwrap_or(16) as u32;
    let stride = body.get("stride").and_then(|v| v.as_u64()).unwrap_or(8) as u32;

    if !input.exists() {
        return Json(serde_json::json!({"error": format!("input path not found: {}", input.display())}));
    }

    let config = ScanConfig {
        patch_size,
        stride,
        ..ScanConfig::default()
    };

    // Run the full scan pipeline
    let manifest = if input.is_dir() {
        match scan::scan_directory(&input, &config) {
            Ok(m) => m,
            Err(e) => return Json(serde_json::json!({"error": e})),
        }
    } else {
        match scan::scan_image(&input, &config) {
            Ok(result) => scan::ScanManifest {
                patch_size: config.patch_size,
                stride: config.stride,
                total_patches_raw: result.total_patches,
                total_patches_quality: result.quality_patches,
                total_filtered: result.total_patches - result.quality_patches,
                categories: result.patches.iter()
                    .fold(std::collections::HashMap::new(), |mut acc, p| {
                        *acc.entry(p.category.clone()).or_insert(0) += 1;
                        acc
                    }),
                sources: vec![result],
            },
            Err(e) => return Json(serde_json::json!({"error": e})),
        }
    };

    // Save patches to a temp scan directory
    let scan_dir = input.parent().unwrap_or(&input).join("pixl_scan");
    let mut source_images = vec![];
    for src in &manifest.sources {
        if let Ok(img) = image::open(&src.source) {
            source_images.push((src.source.clone(), img.to_rgba8()));
        }
    }
    let _ = scan::save_scan(&manifest, &source_images, &scan_dir);

    // Return manifest as JSON matching ScanSummary format
    serde_json::to_value(&serde_json::json!({
        "total_patches_raw": manifest.total_patches_raw,
        "total_patches_quality": manifest.total_patches_quality,
        "total_filtered": manifest.total_filtered,
        "categories": manifest.categories,
        "scan_dir": scan_dir.to_string_lossy(),
    }))
    .map(Json)
    .unwrap_or_else(|e| Json(serde_json::json!({"error": e.to_string()})))
}

async fn prepare(Json(body): Json<Value>) -> Json<Value> {
    let scan_dir = match body.get("scan_dir").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return Json(serde_json::json!({"error": "missing 'scan_dir' field"})),
    };
    let out_dir = body.get("out").and_then(|v| v.as_str())
        .unwrap_or("training/data_custom").to_string();
    let style = body.get("style").and_then(|v| v.as_str()).unwrap_or("custom").to_string();
    let aug = body.get("aug").and_then(|v| v.as_u64()).unwrap_or(4) as u8;
    let color_aug = body.get("color_aug").and_then(|v| v.as_bool()).unwrap_or(true);
    let max_per_bin = body.get("max_per_bin").and_then(|v| v.as_u64()).unwrap_or(150) as usize;
    let max_colors = body.get("max_colors").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    // Load scan manifest
    let manifest_path = PathBuf::from(&scan_dir).join("scan_manifest.json");
    let patches_dir = PathBuf::from(&scan_dir).join("patches");

    if !manifest_path.exists() {
        return Json(serde_json::json!({"error": format!("no scan_manifest.json in {scan_dir}")}));
    }

    let manifest_json = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"error": format!("cannot read manifest: {e}")})),
    };
    let manifest: pixl_render::scan::ScanManifest = match serde_json::from_str(&manifest_json) {
        Ok(m) => m,
        Err(e) => return Json(serde_json::json!({"error": format!("invalid manifest: {e}")})),
    };

    let patch_size = manifest.patch_size as usize;
    let symbol_pool = ".#+=~gorhwsABCDE";

    // Group patches by category
    let mut category_patches: std::collections::HashMap<String, Vec<pixl_render::scan::PatchInfo>> =
        std::collections::HashMap::new();
    for source in &manifest.sources {
        for patch in &source.patches {
            category_patches.entry(patch.category.clone()).or_default().push(patch.clone());
        }
    }

    // Extract palettes + quantize + augment
    let color_shifts: Vec<&str> = if color_aug { vec!["", "warm", "cool", "dark"] } else { vec![""] };
    let mut all_samples: Vec<(pixl_core::prepare::TrainingSample, pixl_core::prepare::GridFeatures)> = vec![];

    for (category, patches) in &category_patches {
        let mut all_pixels: Vec<Vec<u8>> = vec![];
        for p in patches {
            if let Ok(img) = image::open(patches_dir.join(&p.filename)) {
                all_pixels.push(img.to_rgba8().into_raw());
            }
        }
        let pixel_refs: Vec<&[u8]> = all_pixels.iter().map(|v| v.as_slice()).collect();
        let palette = pixl_core::prepare::extract_palette_from_pixels(&pixel_refs, max_colors, symbol_pool);
        if palette.is_empty() { continue; }

        for p in patches {
            if let Ok(img) = image::open(patches_dir.join(&p.filename)) {
                let raw = img.to_rgba8().into_raw();
                let grid = pixl_core::prepare::quantize_to_grid(&raw, patch_size, patch_size, &palette);
                let non_void: usize = grid.iter().flat_map(|r| r.iter()).filter(|&&c| c != '.').count();
                if non_void < patch_size * patch_size / 20 { continue; }

                let features = pixl_core::prepare::compute_features(&grid);
                for &shift in &color_shifts {
                    let shifted = if shift.is_empty() { palette.clone() } else {
                        pixl_core::prepare::shift_palette(&palette, shift)
                    };
                    let pal_desc = pixl_core::prepare::palette_to_desc(&shifted);
                    for (aug_grid, aug_tag) in pixl_core::prepare::augment_grid(&grid, aug) {
                        let label = pixl_core::prepare::make_label(&features, &style, category, aug_tag, shift);
                        let grid_str = pixl_core::prepare::grid_to_string(&aug_grid);
                        let sample = pixl_core::prepare::make_sample(&pal_desc, &label, &grid_str);
                        all_samples.push((sample, features.clone()));
                    }
                }
            }
        }
    }

    let total_augmented = all_samples.len();
    let (stratified, bins_filled) = pixl_core::prepare::stratified_sample(all_samples, max_per_bin, 42);
    let total_stratified = stratified.len();

    // Split and write
    let out_path = PathBuf::from(&out_dir);
    if let Err(e) = std::fs::create_dir_all(&out_path) {
        return Json(serde_json::json!({"error": format!("cannot create output dir: {e}")}));
    }

    let n = stratified.len();
    let train_end = (n as f64 * 0.9) as usize;
    let valid_end = (n as f64 * 0.95) as usize;
    let train = &stratified[..train_end];
    let valid = &stratified[train_end..valid_end];
    let test = &stratified[valid_end..];

    let _ = pixl_core::prepare::write_jsonl(train, &out_path.join("train.jsonl"));
    let _ = pixl_core::prepare::write_jsonl(valid, &out_path.join("valid.jsonl"));
    let _ = pixl_core::prepare::write_jsonl(test, &out_path.join("test.jsonl"));

    Json(serde_json::json!({
        "total_patches": manifest.total_patches_quality,
        "total_augmented": total_augmented,
        "total_stratified": total_stratified,
        "train_count": train.len(),
        "valid_count": valid.len(),
        "test_count": test.len(),
        "bins_filled": bins_filled,
        "data_dir": out_dir,
        "categories": manifest.categories,
    }))
}

async fn train_start(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let data_dir = match body.get("data_dir").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return Json(serde_json::json!({"error": "missing 'data_dir' field"})),
    };
    let adapter_path = match body.get("adapter").and_then(|v| v.as_str()) {
        Some(s) => PathBuf::from(s),
        None => return Json(serde_json::json!({"error": "missing 'adapter' field"})),
    };
    let model = body
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("mlx-community/Qwen2.5-3B-Instruct-4bit");
    let epochs = body.get("epochs").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
    let lr = body.get("lr").and_then(|v| v.as_f64()).unwrap_or(0.00002);
    let layers = body.get("layers").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
    let resume = body.get("resume").and_then(|v| v.as_bool()).unwrap_or(false);

    // Count training samples
    let train_path = PathBuf::from(&data_dir).join("train.jsonl");
    let train_count = match std::fs::read_to_string(&train_path) {
        Ok(content) => content.lines().filter(|l| !l.is_empty()).count(),
        Err(e) => return Json(serde_json::json!({"error": format!("Cannot read {}: {e}", train_path.display())})),
    };

    let total_iters = train_count * epochs;
    let python = crate::inference::find_python_with_mlx();

    // Create adapter directory
    if let Err(e) = std::fs::create_dir_all(&adapter_path) {
        return Json(serde_json::json!({"error": format!("Cannot create adapter dir: {e}")}));
    }

    // Spawn training process in background
    let mut cmd = std::process::Command::new(&python);
    cmd.args(["-m", "mlx_lm", "lora"])
        .arg("--model").arg(model)
        .arg("--train")
        .arg("--data").arg(&data_dir)
        .arg("--adapter-path").arg(&adapter_path)
        .arg("--fine-tune-type").arg("lora")
        .arg("--num-layers").arg(layers.to_string())
        .arg("--batch-size").arg("1")
        .arg("--learning-rate").arg(format!("{lr}"))
        .arg("--iters").arg(total_iters.to_string())
        .arg("--val-batches").arg("25")
        .arg("--steps-per-eval").arg("500")
        .arg("--save-every").arg("2000")
        .arg("--max-seq-length").arg("512")
        .arg("--seed").arg("42");

    // Resume from checkpoint if available
    if resume {
        let checkpoint = adapter_path.join("adapters.safetensors");
        if checkpoint.exists() {
            cmd.arg("--resume-adapter-file").arg(&checkpoint);
            eprintln!("resuming training from checkpoint: {}", checkpoint.display());
        }
    }

    let result = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match result {
        Ok(child) => {
            let pid = child.id();

            // Store job state
            let job = TrainingJob {
                total_iters,
                current_iter: 0,
                last_loss: None,
                adapter_path: adapter_path.clone(),
                pid: Some(pid),
                done: false,
                error: None,
                paused: false,
                throttle: "normal".into(),
                best_loss: None,
                speed: None,
                epoch: 0,
                total_epochs: epochs,
                train_samples: train_count,
            };
            *state.training_job.lock().unwrap() = Some(job);

            // Spawn a task to monitor the process
            let state_clone = state.clone();
            let adapter_clone = adapter_path.clone();
            tokio::spawn(async move {
                // Wait for process to finish
                let mut child_handle = child;
                let status = tokio::task::spawn_blocking(move || child_handle.wait()).await;

                let mut job_lock = state_clone.training_job.lock().unwrap();
                if let Some(ref mut job) = *job_lock {
                    job.done = true;
                    job.current_iter = job.total_iters;
                    match status {
                        Ok(Ok(s)) if s.success() => {
                            job.error = None;
                        }
                        Ok(Ok(s)) => {
                            job.error = Some(format!("Training exited with code {:?}", s.code()));
                        }
                        _ => {
                            job.error = Some("Training process monitoring failed".into());
                        }
                    }
                }
            });

            Json(serde_json::json!({
                "ok": true,
                "pid": pid,
                "total_iters": total_iters,
                "train_samples": train_count,
                "epochs": epochs,
                "adapter": adapter_path.display().to_string(),
                "est_minutes": total_iters / 2 / 60,
            }))
        }
        Err(e) => {
            Json(serde_json::json!({"error": format!("Failed to start training: {e}")}))
        }
    }
}

async fn train_status(State(state): State<SharedState>) -> Json<Value> {
    let job_lock = state.training_job.lock().unwrap();
    match &*job_lock {
        None => Json(serde_json::json!({"status": "idle"})),
        Some(job) => {
            let progress = if job.total_iters > 0 {
                job.current_iter as f64 / job.total_iters as f64
            } else {
                0.0
            };
            let remaining = job.total_iters.saturating_sub(job.current_iter);
            let eta_minutes = job.speed
                .filter(|&s| s > 0.0)
                .map(|s| remaining as f64 / s / 60.0);
            Json(serde_json::json!({
                "status": if job.done { "done" } else { "training" },
                "total_iters": job.total_iters,
                "current_iter": job.current_iter,
                "progress": progress,
                "loss": job.last_loss,
                "adapter": job.adapter_path.display().to_string(),
                "error": job.error,
                "paused": job.paused,
                "throttle": job.throttle,
                "best_loss": job.best_loss,
                "speed": job.speed,
                "epoch": job.epoch,
                "total_epochs": job.total_epochs,
                "train_samples": job.train_samples,
                "eta_minutes": eta_minutes,
            }))
        }
    }
}

async fn train_stop(State(state): State<SharedState>) -> Json<Value> {
    let mut job_lock = state.training_job.lock().unwrap();
    match &*job_lock {
        None => Json(serde_json::json!({"status": "idle", "message": "no training running"})),
        Some(job) => {
            if job.done {
                return Json(serde_json::json!({"status": "done", "message": "training already finished"}));
            }

            // Kill the training process and its children (Python + mlx_lm)
            if let Some(pid) = job.pid {
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .output();
                let _ = std::process::Command::new("pkill")
                    .args(["-f", "mlx_lm lora"])
                    .output();
                eprintln!("stopped training process (PID {})", pid);
            }

            // Mark job as done with cancellation
            drop(job_lock);
            let mut job_lock = state.training_job.lock().unwrap();
            if let Some(ref mut job) = *job_lock {
                job.done = true;
                job.error = Some("Training cancelled by user".into());
            }

            Json(serde_json::json!({"status": "cancelled", "message": "training stopped"}))
        }
    }
}

async fn train_pause(State(state): State<SharedState>) -> Json<Value> {
    let mut job_lock = state.training_job.lock().unwrap();
    match &mut *job_lock {
        None => Json(serde_json::json!({"error": "no training running"})),
        Some(job) => {
            if job.done {
                return Json(serde_json::json!({"error": "training already finished"}));
            }

            if job.paused {
                // Resume: restart training from checkpoint
                // mlx_lm saves checkpoints every 2000 iters. When we resume,
                // we pass --resume-adapter-file to continue from the last save.
                job.paused = false;
                Json(serde_json::json!({
                    "status": "resumed",
                    "message": "Call /api/train/start with resume=true to continue from checkpoint",
                }))
            } else {
                // Pause: gracefully stop the process (checkpoint is auto-saved)
                if let Some(pid) = job.pid {
                    let _ = std::process::Command::new("kill")
                        .args(["-TERM", &pid.to_string()])
                        .output();
                    let _ = std::process::Command::new("pkill")
                        .args(["-f", "mlx_lm lora"])
                        .output();
                    eprintln!("paused training (killed PID {}, checkpoint saved)", pid);
                }
                job.paused = true;
                // Don't mark as done — we're paused, not stopped
                Json(serde_json::json!({
                    "status": "paused",
                    "message": "Training paused. Checkpoint saved. Resume to continue.",
                }))
            }
        }
    }
}

async fn train_throttle(
    State(state): State<SharedState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let level = match body.get("level").and_then(|v| v.as_str()) {
        Some(l) => l.to_string(),
        None => return Json(serde_json::json!({"error": "missing 'level' field"})),
    };

    let nice_val = match level.as_str() {
        "full" => 0,
        "normal" => 5,
        "background" => 10,
        "minimal" => 19,
        _ => return Json(serde_json::json!({"error": format!("invalid level: {level}. Use full/normal/background/minimal")})),
    };

    let mut job_lock = state.training_job.lock().unwrap();
    match &mut *job_lock {
        None => Json(serde_json::json!({"error": "no training running"})),
        Some(job) => {
            if job.done {
                return Json(serde_json::json!({"error": "training already finished"}));
            }
            if let Some(pid) = job.pid {
                let _ = std::process::Command::new("renice")
                    .args([&nice_val.to_string(), "-p", &pid.to_string()])
                    .output();
            }
            job.throttle = level.clone();
            Json(serde_json::json!({"ok": true, "level": level}))
        }
    }
}

fn scan_dataset_dirs(search_dirs: &[PathBuf]) -> (Vec<Value>, usize) {
    let mut datasets = vec![];

    for base in search_dirs {
        if !base.exists() { continue; }

        // Scan base dir and all subdirs for data_* directories
        let mut dirs_to_check = vec![base.clone()];
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    dirs_to_check.push(p);
                }
            }
        }

        for dir in &dirs_to_check {
            let entries = match std::fs::read_dir(dir) {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                if !name.starts_with("data_") || !path.is_dir() { continue; }

                let train_path = path.join("train.jsonl");
                if !train_path.exists() { continue; }

                let sample_count = std::fs::read_to_string(&train_path)
                    .unwrap_or_default()
                    .lines()
                    .filter(|l| !l.is_empty())
                    .count();

                let suffix = name.strip_prefix("data_").unwrap_or(&name);

                let mut style = serde_json::Value::Null;
                let mut source = serde_json::Value::Null;
                if let Ok(info) = std::fs::read_to_string(path.join("dataset_info.json")) {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&info) {
                        if let Some(s) = parsed.get("style_tag").or(parsed.get("style")) {
                            style = s.clone();
                        }
                        if let Some(s) = parsed.get("sources").or(parsed.get("source")) {
                            source = s.clone();
                        }
                    }
                }

                datasets.push(serde_json::json!({
                    "name": suffix,
                    "path": path.to_string_lossy(),
                    "sample_count": sample_count,
                    "style": style,
                    "source": source,
                }));
            }
        }
    }

    // Canonicalize paths to catch duplicates from different relative paths
    for d in &mut datasets {
        if let Some(p) = d["path"].as_str() {
            if let Ok(canonical) = std::fs::canonicalize(p) {
                d["path"] = serde_json::Value::String(canonical.to_string_lossy().to_string());
            }
        }
    }
    datasets.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    datasets.dedup_by(|a, b| a["path"] == b["path"]);

    let total: usize = datasets.iter()
        .filter_map(|d| d["sample_count"].as_u64())
        .map(|n| n as usize)
        .sum();

    (datasets, total)
}

async fn list_datasets() -> Json<Value> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let search_dirs = vec![cwd.join("training"), cwd.join("../training")];
    let (datasets, total) = scan_dataset_dirs(&search_dirs);
    Json(serde_json::json!({ "datasets": datasets, "total_samples": total }))
}

async fn list_datasets_post(Json(body): Json<Value>) -> Json<Value> {
    let mut search_dirs = vec![];

    if let Some(dirs) = body.get("dirs").and_then(|v| v.as_array()) {
        for d in dirs {
            if let Some(s) = d.as_str() {
                let p = PathBuf::from(s);
                search_dirs.push(p.clone());
                // Also check as relative to cwd
                if let Ok(cwd) = std::env::current_dir() {
                    let abs = cwd.join(&p);
                    if abs != p { search_dirs.push(abs); }
                }
            }
        }
    }

    if search_dirs.is_empty() {
        let cwd = std::env::current_dir().unwrap_or_default();
        search_dirs.push(cwd.join("training"));
        search_dirs.push(cwd.join("../training"));
    }

    let (datasets, total) = scan_dataset_dirs(&search_dirs);
    Json(serde_json::json!({ "datasets": datasets, "total_samples": total }))
}

/// Start the HTTP server.
pub async fn run_http(
    state: McpState,
    port: u16,
    inference_config: Option<InferenceConfig>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router(state, inference_config);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    eprintln!("pixl http server listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
