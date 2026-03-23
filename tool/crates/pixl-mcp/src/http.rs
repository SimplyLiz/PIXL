/// HTTP API server for PIXL Studio integration.
/// Exposes the same handlers as the MCP server over REST endpoints.

use crate::{handlers, state::McpState};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;
use std::sync::{Arc, Mutex};

type SharedState = Arc<Mutex<McpState>>;

/// Create the axum router with all endpoints.
pub fn create_router(state: McpState) -> Router {
    let shared = Arc::new(Mutex::new(state));

    Router::new()
        .route("/health", get(health))
        .route("/api/session", post(session_start))
        .route("/api/palette", post(get_palette))
        .route("/api/tile/create", post(create_tile))
        .route("/api/tile/render", post(render_tile))
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
        .route("/api/tile/vary", post(vary_tile))
        .route("/api/themes", get(list_themes))
        .route("/api/stamps", get(list_stamps))
        .route("/api/atlas/pack", post(pack_atlas))
        .route("/api/load", post(load_source))
        .route("/api/tool", post(generic_tool_call))
        .with_state(shared)
}

async fn health() -> &'static str {
    "pixl ok"
}

async fn session_start(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_session_start", &Value::Null))
}

async fn get_palette(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_get_palette", &args))
}

async fn create_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_create_tile", &args))
}

async fn render_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_render_tile", &args))
}

async fn delete_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_delete_tile", &args))
}

async fn check_edge_pair(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_check_edge_pair", &args))
}

async fn list_tiles(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_list_tiles", &Value::Null))
}

async fn validate(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_validate", &args))
}

async fn narrate_map(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_narrate_map", &args))
}

async fn learn_style(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_learn_style", &args))
}

async fn check_style(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_check_style", &args))
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
    Json(handlers::handle_tool(&state, "pixl_render_sprite_gif", &args))
}

async fn get_file(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_get_file", &Value::Null))
}

async fn generate_context(
    State(state): State<SharedState>,
    Json(args): Json<Value>,
) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_generate_context", &args))
}

async fn list_themes(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_list_themes", &Value::Null))
}

async fn list_stamps(State(state): State<SharedState>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_list_stamps", &Value::Null))
}

async fn pack_atlas(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_pack_atlas", &args))
}

async fn vary_tile(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_vary_tile", &args))
}

async fn load_source(State(state): State<SharedState>, Json(args): Json<Value>) -> Json<Value> {
    Json(handlers::handle_tool(&state, "pixl_load_source", &args))
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
    Json(handlers::handle_tool(&state, tool_name, &args))
}

/// Start the HTTP server.
pub async fn run_http(
    state: McpState,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    eprintln!("pixl http server listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
