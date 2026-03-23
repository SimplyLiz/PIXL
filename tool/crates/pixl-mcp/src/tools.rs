use rmcp::model::Tool;
use serde_json::json;
use std::sync::Arc;

pub fn tool_definitions() -> Vec<Tool> {
    vec![
        tool(
            "pixl_session_start",
            "Start a PIXL session. Returns theme, palette, stamps, workflow.",
        ),
        tool(
            "pixl_get_palette",
            "Get symbol table for a palette with hex colors and roles.",
        ),
        tool(
            "pixl_create_tile",
            "Create a tile from a character grid. Returns validation + 16x preview.",
        ),
        tool("pixl_validate", "Validate the in-memory PAX file."),
        tool(
            "pixl_render_tile",
            "Render a tile to PNG at specified scale.",
        ),
        tool(
            "pixl_check_edge_pair",
            "Check if two tiles can be placed adjacent.",
        ),
        tool(
            "pixl_list_tiles",
            "List all tiles with edge classes and tags.",
        ),
        tool(
            "pixl_get_file",
            "Get the .pax source of the current session.",
        ),
        tool("pixl_delete_tile", "Delete a tile from the session."),
        tool("pixl_learn_style", "Extract style latent from reference tiles. Returns style description for prompt injection."),
        tool("pixl_check_style", "Score a tile against the session style latent. Returns 0-1 match score."),
        tool(
            "pixl_get_blueprint",
            "Get anatomy blueprint for a canvas size.",
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
