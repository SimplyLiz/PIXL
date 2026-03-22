use std::collections::HashMap;

/// Generate a Godot .tres TileSet resource from PAX tile data.
/// Godot 4 uses .tres (text resource) format for tilesets.
pub fn generate_tres(
    name: &str,
    tile_names: &[String],
    tile_width: u32,
    tile_height: u32,
    atlas_image: &str,
    collision_map: &HashMap<String, String>,
) -> String {
    let mut lines = Vec::new();

    // Header
    lines.push("[gd_resource type=\"TileSet\" format=3]".to_string());
    lines.push(String::new());

    // External resource: the atlas texture
    lines.push(format!(
        "[ext_resource type=\"Texture2D\" path=\"res://{}\" id=\"1\"]",
        atlas_image
    ));
    lines.push(String::new());

    // TileSet resource
    lines.push("[resource]".to_string());
    lines.push(format!("tile_size = Vector2i({}, {})", tile_width, tile_height));
    lines.push(String::new());

    // Physics layer for collision
    lines.push("physics_layer_0/collision_layer = 1".to_string());
    lines.push("physics_layer_0/collision_mask = 1".to_string());
    lines.push(String::new());

    // TileSetAtlasSource
    lines.push("[sub_resource type=\"TileSetAtlasSource\" id=\"1\"]".to_string());
    lines.push("texture = ExtResource(\"1\")".to_string());
    lines.push(format!(
        "texture_region_size = Vector2i({}, {})",
        tile_width, tile_height
    ));
    lines.push(String::new());

    // Individual tiles
    for (i, tile_name) in tile_names.iter().enumerate() {
        let col = i as u32;
        let atlas_x = col; // assumes single-row atlas for simplicity
        lines.push(format!("# {}", tile_name));
        lines.push(format!("{}/0 = 0", atlas_x));

        // Add collision polygon if tile has collision
        if let Some(collision) = collision_map.get(tile_name) {
            if collision == "full" {
                lines.push(format!(
                    "{}/0/physics_layer_0/polygon_0/points = PackedVector2Array(0, 0, {}, 0, {}, {}, 0, {})",
                    atlas_x, tile_width, tile_width, tile_height, tile_height
                ));
            }
        }
    }

    lines.push(String::new());
    lines.push("sources/0 = SubResource(\"1\")".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_tres() {
        let mut collision = HashMap::new();
        collision.insert("wall".to_string(), "full".to_string());

        let tres = generate_tres(
            "dungeon",
            &["wall".to_string(), "floor".to_string()],
            16, 16,
            "dungeon_atlas.png",
            &collision,
        );

        assert!(tres.contains("[gd_resource type=\"TileSet\""));
        assert!(tres.contains("tile_size = Vector2i(16, 16)"));
        assert!(tres.contains("physics_layer_0"));
        assert!(tres.contains("# wall"));
        assert!(tres.contains("polygon_0/points"));
        assert!(tres.contains("# floor"));
    }
}
