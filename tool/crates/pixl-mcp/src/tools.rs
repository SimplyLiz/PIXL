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
        tool("pixl_list_stamps", "List available stamps with sizes."),
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
            "Validate the entire session. Args: {check_edges?: bool, quality?: bool}. \
             Returns errors, warnings, and stats. When quality=true, also runs per-tile \
             structural analysis (outline, contrast, centering) and cross-tile style \
             consistency checks, with knowledge base advice for each issue.",
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
        tool(
            "pixl_generate_wang",
            "Generate a complete Wang tileset for terrain transitions. \
             Args: {terrain_a, terrain_b, method?: 'dual_grid'|'blob_47', size?: 16, \
             palette?, sym_a?: '+', sym_b?: '~', sym_border?: '#'}. \
             Creates all transition tiles with correct edge classes for WFC. \
             dual_grid = 15 tiles (simpler, top-down). blob_47 = 47 tiles (complex, walls/caves).",
        ),
        tool(
            "pixl_rate_tile",
            "Rate a tile aesthetically (1-5) on readability, appeal, and consistency. \
             Args: {name, criteria?: ['readability','appeal','consistency']}. \
             Returns per-axis scores, overall rating, and a suggested WFC weight. \
             Use after generating multiple variants to rank them.",
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
        // ── Backdrop ──
        tool(
            "pixl_backdrop_import",
            "Import a pixelized image as a PAX backdrop (tile-decomposed animated background). \
             Args: {input: path}. Optional: name (default 'scene'), colors (default 32), \
             tile_size (default 16), out (output .pax path). Slices the image into tiles, \
             deduplicates, builds extended palette, writes PAX file with tilemap.",
        ),
        tool(
            "pixl_backdrop_render",
            "Render a backdrop from a .pax file. Args: {file: path, name: string}. \
             Optional: frames (0=static PNG, >0=animated GIF), scale (default 1), \
             duration (frame ms, default 120). Returns base64 PNG or GIF.",
        ),
        // ── Sprite Conversion ──
        tool(
            "pixl_convert_sprite",
            "Convert AI-generated images to true 1:1 pixel art. Args: {input: path}. \
             Optional: out_dir (default: pixl_convert/), width (single-res mode), \
             colors (default 32). Without width, produces 3 presets: small (128px, 16 colors), \
             medium (160px, 32 colors), large (256px, 48 colors). Copies original to originals/.",
        ),
        // ── Feedback & Training ──
        tool(
            "pixl_record_feedback",
            "Record accept/reject/edit feedback for a tile. Args: {name, action ('accept'|'reject'|'edit')}. \
             Optional: reject_reason. Builds training signal for LoRA fine-tuning.",
        ),
        tool(
            "pixl_feedback_stats",
            "Get feedback statistics: total accepts, rejects, acceptance rate, avg style scores.",
        ),
        tool(
            "pixl_feedback_constraints",
            "Get learned constraints from feedback history. Returns avoid-patterns derived from \
             repeated rejections.",
        ),
        tool(
            "pixl_export_training",
            "Export accepted tiles as JSONL training data for LoRA fine-tuning. \
             Args: {path?: string}. Returns exported pair count.",
        ),
        tool(
            "pixl_training_stats",
            "Get training data statistics: pair count, adapter info, model path.",
        ),
        // ── Templates & Export ──
        tool(
            "pixl_new_from_template",
            "Create a new PAX file from a built-in theme template. Args: {theme: string}. \
             Themes: dark_fantasy, light_fantasy, sci_fi, nature, gameboy, nes. \
             Returns the PAX source string.",
        ),
        tool(
            "pixl_export",
            "Export the session to a game engine format. Args: {format ('tiled'|'godot'|'texturepacker'|\
             'gbstudio'|'unity'), out_dir: path}. Writes export files to the directory.",
        ),
        tool(
            "pixl_check_completeness",
            "Analyze tileset completeness for WFC. Identifies missing transition tiles \
             needed for seamless map generation. Returns gaps with suggested edge classes.",
        ),
        tool(
            "pixl_generate_transition_context",
            "Build enriched AI prompts for creating a missing transition tile between two tiles. \
             Args: {tile_a, tile_b}. Returns system_prompt + user_prompt with edge context.",
        ),
        // ── Composites ──
        tool(
            "pixl_list_composites",
            "List all composites in the session with their layout dimensions, variants, \
             and animation names.",
        ),
        tool(
            "pixl_render_composite",
            "Render a composite sprite to PNG. Args: {name, variant? (string), anim? (string), \
             frame? (1-based integer), scale? (default 8)}. Returns base64 PNG preview. \
             Without anim/frame, renders the base layout (or variant if specified).",
        ),
        tool(
            "pixl_check_seams",
            "Check seam continuity across tile boundaries in composites. Returns warnings for \
             pixel discontinuities at adjacent tile edges. No args — checks all composites.",
        ),
        // ── SELF-REFINE Loop ──
        tool(
            "pixl_critique_tile",
            "Structural quality critique of a tile. Args: {name, scale? (default 16)}. \
             Renders the tile to PNG, then runs structural validators: outline coverage, \
             centering, canvas utilization, contrast, fragmentation. Returns the rendered \
             preview PNG + a text critique with specific issues and fix instructions. \
             ALWAYS examine the preview image alongside the critique text. \
             This is the 'look at what you drew' step in the SELF-REFINE loop.",
        ),
        tool(
            "pixl_refine_tile",
            "Patch a sub-region of a tile grid. Args: {name, start_row (0-based), \
             rows (multi-line string — replacement rows)}. Replaces rows starting at \
             start_row with the provided rows. The replacement rows must be the same width \
             as the tile. Returns updated preview PNG + new structural critique. \
             Use this after pixl_critique_tile identifies specific row/region issues.",
        ),
        tool(
            "pixl_show_references",
            "Show rendered reference tiles as visual examples. Args: {query (search term like \
             'wall', 'character', 'potion'), count? (default 4), size? (filter by size e.g. '16x16')}. \
             Searches all tiles in the session by name and tags, renders the best matches as \
             preview images at 16x zoom. CALL THIS BEFORE generating a new tile — seeing real \
             rendered pixel art at the target size dramatically improves generation quality. \
             The returned images are your visual reference for style, proportions, and technique.",
        ),
        tool(
            "pixl_upscale_tile",
            "Upscale a tile's character grid by an integer factor (nearest-neighbor). \
             Args: {name, factor? (default 2), new_name? (default: name_upscaled)}. \
             Creates a new tile in the session with the upscaled grid. \
             A factor of 2 turns 8x8 → 16x16 (each pixel becomes a 2x2 block). \
             Use this as step 2 of the progressive resolution workflow: \
             (1) generate at 8x8, (2) upscale to 16x16, (3) refine detail with pixl_refine_tile. \
             Returns preview PNG of the upscaled result for visual inspection.",
        ),
        // ── Diffusion Bridge ──
        tool(
            "pixl_generate_sprite",
            "Generate a pixel art sprite via image AI (DALL-E) + palette quantization. \
             Args: {prompt (description of the sprite), name (tile name to create), \
             size? (default 'auto' — detects native resolution), max_colors? (default 32), \
             target_palette? (remap to this project palette after generation), dither? (default false)}. \
             Pipeline: DALL-E generates reference → detect native pixel grid → center-sample → \
             auto-extract palette from image → quantize → background removal → AA cleanup → \
             outline enforcement → optional remap to target_palette. \
             Always extracts colors from the generated image for maximum fidelity. \
             The palette_toml in the response can be pasted into your .pax file. \
             To integrate with an existing project palette, pass target_palette. \
             Requires OPENAI_API_KEY environment variable.",
        ),
        tool(
            "pixl_remap_tile",
            "Remap a tile from one palette to another using OKLab nearest-color matching. \
             Args: {name, target_palette (name of palette in session)}. Maps each symbol \
             to the perceptually closest symbol in the target palette. Use this after \
             pixl_generate_sprite with auto_palette to convert the tile to your project palette.",
        ),
        // ── Local AI Generation ──
        tool(
            "pixl_generate_tile",
            "Generate a tile using the local LoRA-trained model. Args: {name, prompt, size? ('16x16'), \
             palette? (auto-detected)}. Uses the fine-tuned PAX model to generate a character grid \
             from a text description, then creates the tile in-session with auto-classified edges \
             and a preview. Requires local inference to be configured (--model + --adapter flags).",
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
