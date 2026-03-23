use rmcp::model::Tool;
use serde_json::json;
use std::sync::Arc;

pub fn tool_definitions() -> Vec<Tool> {
    vec![
        // ── Discovery (call session_start FIRST) ──
        tool(
            "pixl_session_start",
            "CALL THIS FIRST. Returns active theme, palette symbols (char -> hex color + role), \
             canvas size, light source direction, available stamps, and existing tiles. \
             You MUST examine the palette symbols before writing any tile grid.",
        ),
        tool(
            "pixl_get_palette",
            "Get the full symbol table for a theme's palette. Args: {theme: string}. \
             Returns each symbol with its hex color and semantic role (bg, fg, shadow, etc).",
        ),
        tool(
            "pixl_list_tiles",
            "List all tiles in the session with edge classes, tags, and template info.",
        ),
        tool(
            "pixl_list_themes",
            "List available themes with palette name, scale, canvas size, light source.",
        ),
        tool(
            "pixl_list_stamps",
            "List available stamps with sizes.",
        ),
        // ── Creation ──
        tool(
            "pixl_create_tile",
            "Create a tile from a character grid. Args: {name, palette, size (e.g. '16x16'), \
             grid (multi-line string, one char per pixel)}. Optional: edge_class, symmetry, tags. \
             Returns: preview PNG at 16x zoom, auto-classified edge classes, actual border pixel \
             strings, and compatible neighbor tiles. Examine the preview image before proceeding.",
        ),
        tool(
            "pixl_load_source",
            "Load a .pax source string into the session, replacing current state. \
             Args: {source: string}.",
        ),
        // ── Rendering ──
        tool(
            "pixl_render_tile",
            "Render a tile to PNG. Args: {name, scale? (default 16)}. Returns base64 PNG.",
        ),
        tool(
            "pixl_render_sprite_gif",
            "Render a sprite animation as animated GIF. Args: {spriteset, sprite, scale?}. \
             Returns base64 GIF. Examine this to judge animation smoothness and timing.",
        ),
        // ── Validation ──
        tool(
            "pixl_validate",
            "Validate the entire session. Args: {check_edges?: bool}. \
             Returns errors, warnings, and stats.",
        ),
        tool(
            "pixl_check_edge_pair",
            "Check if two tiles can be placed adjacent. Args: {tile_a, direction (north/east/south/west), tile_b}. \
             Returns compatible: bool + reason.",
        ),
        // ── Generation ──
        tool(
            "pixl_narrate_map",
            "Generate a dungeon map from spatial predicates. Args: {width, height, seed?, \
             rules: ['border:wall_solid', 'region:chamber:floor_stone:3x3:southeast', \
             'path:0,3:11,3']}. Returns rendered map PNG + tile name grid.",
        ),
        tool(
            "pixl_generate_context",
            "Build enriched AI generation context. Args: {prompt, type? ('tile'), size? ('16x16')}. \
             Returns system_prompt and user_prompt with palette, theme, style latent, and edge \
             context pre-filled. Use this to generate tiles via a separate AI call.",
        ),
        // ── Style ──
        tool(
            "pixl_learn_style",
            "Extract style latent from session tiles. Args: {tiles?: [names]}. \
             Returns 8-property fingerprint + text description. Stored in session for scoring.",
        ),
        tool(
            "pixl_check_style",
            "Score a tile against the style latent. Args: {name}. Returns 0-1 score + assessment.",
        ),
        // ── Blueprint ──
        tool(
            "pixl_get_blueprint",
            "Get anatomy blueprint for character sprites. Args: {width, height, model? ('humanoid_chibi')}. \
             Returns pixel-coordinate landmarks and eye size rules for the canvas size.",
        ),
        // ── Export ──
        tool(
            "pixl_pack_atlas",
            "Pack all session tiles into a sprite atlas. Args: {columns?, padding?, scale?}. \
             Returns base64 atlas PNG + TexturePacker JSON.",
        ),
        tool(
            "pixl_get_file",
            "Get the full .pax TOML source of the current session state.",
        ),
        // ── Variation ──
        tool(
            "pixl_vary_tile",
            "Generate N variants from a base tile. Args: {name, count? (default 4), seed? (default 42)}. \
             Applies mutations: pixel noise, cracks, row/col shifts, symbol swaps, edge erosion. \
             Edges are preserved. Returns variant grids + previews.",
        ),
        // ── Mutation ──
        tool(
            "pixl_delete_tile",
            "Delete a tile from the session. Args: {name}.",
        ),
    ]
}

fn tool(name: &str, description: &str) -> Tool {
    let schema: rmcp::model::JsonObject =
        serde_json::from_value(json!({"type": "object", "properties": {}})).unwrap();
    let mut t = Tool::default();
    t.name = std::borrow::Cow::Owned(name.to_string());
    t.description = Some(std::borrow::Cow::Owned(description.to_string()));
    t.input_schema = Arc::new(schema);
    t
}
