use serde::Serialize;
use std::collections::HashMap;

/// Unity sprite atlas metadata.
/// Unity reads TexturePacker JSON Hash natively via its 2D Sprite package.
/// This module generates supplementary metadata for tilemap import.

#[derive(Debug, Serialize)]
pub struct UnityTilemapMeta {
    pub name: String,
    pub tile_width: u32,
    pub tile_height: u32,
    pub atlas_image: String,
    pub tiles: Vec<UnityTileEntry>,
}

#[derive(Debug, Serialize)]
pub struct UnityTileEntry {
    pub name: String,
    pub index: u32,
    pub collision: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, String>,
}

/// Generate Unity tilemap metadata JSON.
pub fn generate_unity_meta(
    name: &str,
    tile_names: &[String],
    tile_width: u32,
    tile_height: u32,
    atlas_image: &str,
    collision_map: &HashMap<String, String>,
) -> UnityTilemapMeta {
    let tiles = tile_names
        .iter()
        .enumerate()
        .map(|(i, tile_name)| {
            let collision = collision_map
                .get(tile_name)
                .cloned()
                .unwrap_or_else(|| "none".to_string());

            UnityTileEntry {
                name: tile_name.clone(),
                index: i as u32,
                collision,
                properties: HashMap::new(),
            }
        })
        .collect();

    UnityTilemapMeta {
        name: name.to_string(),
        tile_width,
        tile_height,
        atlas_image: atlas_image.to_string(),
        tiles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_unity_meta() {
        let mut collision = HashMap::new();
        collision.insert("wall".to_string(), "full".to_string());

        let meta = generate_unity_meta(
            "dungeon",
            &["wall".to_string(), "floor".to_string()],
            16, 16,
            "atlas.png",
            &collision,
        );

        assert_eq!(meta.tiles.len(), 2);
        assert_eq!(meta.tiles[0].collision, "full");
        assert_eq!(meta.tiles[1].collision, "none");

        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("\"collision\":\"full\""));
    }
}
