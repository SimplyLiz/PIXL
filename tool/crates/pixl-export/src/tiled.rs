use serde::Serialize;
use std::collections::HashMap;

/// Tiled TMJ (JSON) tileset + tilemap export.
/// Tiled is the most widely supported tilemap editor — exports to Godot 4
/// (.tscn via plugin), Unity (SuperTiled2Unity), and virtually every 2D engine.

#[derive(Debug, Serialize)]
pub struct TiledMap {
    #[serde(rename = "compressionlevel")]
    pub compression_level: i32,
    pub height: u32,
    pub width: u32,
    pub infinite: bool,
    pub orientation: String,
    #[serde(rename = "renderorder")]
    pub render_order: String,
    #[serde(rename = "tileheight")]
    pub tile_height: u32,
    #[serde(rename = "tilewidth")]
    pub tile_width: u32,
    #[serde(rename = "tiledversion")]
    pub tiled_version: String,
    #[serde(rename = "type")]
    pub map_type: String,
    pub version: String,
    pub layers: Vec<TiledLayer>,
    pub tilesets: Vec<TiledTilesetRef>,
}

#[derive(Debug, Serialize)]
pub struct TiledLayer {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub layer_type: String,
    pub visible: bool,
    pub opacity: f32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct TiledTilesetRef {
    #[serde(rename = "firstgid")]
    pub first_gid: u32,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct TiledTileset {
    pub name: String,
    #[serde(rename = "tilewidth")]
    pub tile_width: u32,
    #[serde(rename = "tileheight")]
    pub tile_height: u32,
    #[serde(rename = "tilecount")]
    pub tile_count: u32,
    pub columns: u32,
    pub image: String,
    #[serde(rename = "imagewidth")]
    pub image_width: u32,
    #[serde(rename = "imageheight")]
    pub image_height: u32,
    pub tiles: Vec<TiledTileEntry>,
}

#[derive(Debug, Serialize)]
pub struct TiledTileEntry {
    pub id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<TiledProperty>>,
    #[serde(rename = "objectgroup", skip_serializing_if = "Option::is_none")]
    pub object_group: Option<TiledObjectGroup>,
}

#[derive(Debug, Serialize)]
pub struct TiledProperty {
    pub name: String,
    #[serde(rename = "type")]
    pub prop_type: String,
    pub value: serde_json::Value,
}

/// Collision object group for a tile.
#[derive(Debug, Serialize)]
pub struct TiledObjectGroup {
    #[serde(rename = "draworder")]
    pub draw_order: String,
    pub objects: Vec<TiledCollisionObject>,
}

#[derive(Debug, Serialize)]
pub struct TiledCollisionObject {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Generate a Tiled tileset JSON (.tsj) from PAX tile data.
pub fn generate_tileset(
    name: &str,
    tile_names: &[String],
    tile_width: u32,
    tile_height: u32,
    atlas_image: &str,
    atlas_width: u32,
    atlas_height: u32,
    columns: u32,
    collision_map: &HashMap<String, String>, // tile_name -> collision type
) -> TiledTileset {
    let tiles: Vec<TiledTileEntry> = tile_names
        .iter()
        .enumerate()
        .map(|(i, tile_name)| {
            let collision = collision_map.get(tile_name).map(|c| c.as_str());
            let object_group = match collision {
                Some("full") => Some(TiledObjectGroup {
                    draw_order: "topdown".to_string(),
                    objects: vec![TiledCollisionObject {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        width: tile_width as f32,
                        height: tile_height as f32,
                    }],
                }),
                Some("half_top") => Some(TiledObjectGroup {
                    draw_order: "topdown".to_string(),
                    objects: vec![TiledCollisionObject {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        width: tile_width as f32,
                        height: tile_height as f32 / 2.0,
                    }],
                }),
                _ => None,
            };

            TiledTileEntry {
                id: i as u32,
                properties: Some(vec![TiledProperty {
                    name: "pax_name".to_string(),
                    prop_type: "string".to_string(),
                    value: serde_json::Value::String(tile_name.clone()),
                }]),
                object_group,
            }
        })
        .collect();

    TiledTileset {
        name: name.to_string(),
        tile_width,
        tile_height,
        tile_count: tile_names.len() as u32,
        columns,
        image: atlas_image.to_string(),
        image_width: atlas_width,
        image_height: atlas_height,
        tiles,
    }
}

/// Generate a Tiled map JSON (.tmj) from a WFC result grid.
pub fn generate_map(
    tile_grid: &[Vec<usize>],
    tile_width: u32,
    tile_height: u32,
    tileset_source: &str,
) -> TiledMap {
    let h = tile_grid.len() as u32;
    let w = if h > 0 { tile_grid[0].len() as u32 } else { 0 };

    // Convert tile indices to Tiled GIDs (1-based, 0 = empty)
    let data: Vec<u32> = tile_grid
        .iter()
        .flat_map(|row| row.iter().map(|&idx| idx as u32 + 1))
        .collect();

    TiledMap {
        compression_level: -1,
        height: h,
        width: w,
        infinite: false,
        orientation: "orthogonal".to_string(),
        render_order: "right-down".to_string(),
        tile_height,
        tile_width,
        tiled_version: "1.11.0".to_string(),
        map_type: "map".to_string(),
        version: "1.10".to_string(),
        layers: vec![TiledLayer {
            id: 1,
            name: "terrain".to_string(),
            layer_type: "tilelayer".to_string(),
            visible: true,
            opacity: 1.0,
            x: 0,
            y: 0,
            width: w,
            height: h,
            data,
        }],
        tilesets: vec![TiledTilesetRef {
            first_gid: 1,
            source: tileset_source.to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tileset_has_collision() {
        let mut collision = HashMap::new();
        collision.insert("wall".to_string(), "full".to_string());

        let tileset = generate_tileset(
            "test",
            &["wall".to_string(), "floor".to_string()],
            16, 16,
            "atlas.png", 32, 16, 2,
            &collision,
        );

        assert_eq!(tileset.tile_count, 2);
        assert!(tileset.tiles[0].object_group.is_some()); // wall has collision
        assert!(tileset.tiles[1].object_group.is_none()); // floor has no collision
    }

    #[test]
    fn map_from_wfc_grid() {
        let grid = vec![
            vec![0, 1, 0],
            vec![1, 0, 1],
        ];
        let map = generate_map(&grid, 16, 16, "tileset.tsj");
        assert_eq!(map.width, 3);
        assert_eq!(map.height, 2);
        assert_eq!(map.layers[0].data, vec![1, 2, 1, 2, 1, 2]); // 1-based GIDs
    }

    #[test]
    fn tileset_json_serializes() {
        let tileset = generate_tileset(
            "dungeon", &["wall".to_string()],
            16, 16, "atlas.png", 16, 16, 1,
            &HashMap::new(),
        );
        let json = serde_json::to_string_pretty(&tileset).unwrap();
        assert!(json.contains("tilewidth"));
        assert!(json.contains("tileheight"));
        assert!(json.contains("pax_name"));
    }
}
