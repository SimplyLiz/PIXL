use pixl_core::types::ObjectRaw;
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
    #[serde(rename = "type")]
    pub tileset_type: String,
    #[serde(rename = "tiledversion")]
    pub tiled_version: String,
    pub version: String,
    pub name: String,
    #[serde(rename = "tilewidth")]
    pub tile_width: u32,
    #[serde(rename = "tileheight")]
    pub tile_height: u32,
    #[serde(rename = "tilecount")]
    pub tile_count: u32,
    pub columns: u32,
    pub spacing: u32,
    pub margin: u32,
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
    spacing: u32,
    margin: u32,
    collision_map: &HashMap<String, String>, // tile_name -> collision type
) -> TiledTileset {
    let tiles: Vec<TiledTileEntry> = tile_names
        .iter()
        .enumerate()
        .map(|(i, tile_name)| {
            let collision = collision_map.get(tile_name).map(|c| c.as_str());
            let tw = tile_width as f32;
            let th = tile_height as f32;
            let object_group = match collision {
                Some("full") => Some(TiledObjectGroup {
                    draw_order: "topdown".to_string(),
                    objects: vec![TiledCollisionObject {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        width: tw,
                        height: th,
                    }],
                }),
                Some("top_half") | Some("half_top") => Some(TiledObjectGroup {
                    draw_order: "topdown".to_string(),
                    objects: vec![TiledCollisionObject {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        width: tw,
                        height: th / 2.0,
                    }],
                }),
                Some("bottom_half") => Some(TiledObjectGroup {
                    draw_order: "topdown".to_string(),
                    objects: vec![TiledCollisionObject {
                        id: 1,
                        x: 0.0,
                        y: th / 2.0,
                        width: tw,
                        height: th / 2.0,
                    }],
                }),
                Some("center") => Some(TiledObjectGroup {
                    draw_order: "topdown".to_string(),
                    objects: vec![TiledCollisionObject {
                        id: 1,
                        x: tw * 0.25,
                        y: th * 0.25,
                        width: tw * 0.5,
                        height: th * 0.5,
                    }],
                }),
                Some("custom") => Some(TiledObjectGroup {
                    draw_order: "topdown".to_string(),
                    objects: vec![TiledCollisionObject {
                        id: 1,
                        x: 0.0,
                        y: 0.0,
                        width: tw,
                        height: th,
                    }],
                }),
                Some("none") => None,
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
        tileset_type: "tileset".to_string(),
        tiled_version: "1.11.0".to_string(),
        version: "1.10".to_string(),
        name: name.to_string(),
        tile_width,
        tile_height,
        tile_count: tile_names.len() as u32,
        columns,
        spacing,
        margin,
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

/// Parse an object's `tiles` string into a row-major grid of tile names.
fn parse_object_tile_grid(tiles: &str) -> Vec<Vec<&str>> {
    tiles
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|line| line.split_whitespace().collect())
        .collect()
}

/// Look up a tile name in the sorted tile_names list, returning its Tiled GID (1-based).
fn tile_name_to_gid(name: &str, tile_names: &[String]) -> Option<u32> {
    tile_names
        .binary_search_by(|n| n.as_str().cmp(name))
        .ok()
        .map(|idx| idx as u32 + 1)
}

/// Generate a Tiled map with 3 depth layers: below_player, terrain, above_player.
///
/// Objects are split across the below/above layers based on their
/// `above_player_rows` / `below_player_rows` fields. Rows not listed in
/// either default to above_player.
pub fn generate_map_with_objects(
    tile_grid: &[Vec<usize>],
    tile_names: &[String],
    placements: &[(String, u32, u32)],
    object_defs: &HashMap<String, &ObjectRaw>,
    tile_width: u32,
    tile_height: u32,
    tileset_source: &str,
) -> Result<TiledMap, String> {
    let h = tile_grid.len() as u32;
    let w = if h > 0 { tile_grid[0].len() as u32 } else { 0 };
    let size = (w * h) as usize;

    // Three layer buffers (0 = empty in Tiled)
    let mut below_data = vec![0u32; size];
    let mut terrain_data = vec![0u32; size];
    let mut above_data = vec![0u32; size];

    // Fill terrain from tile_grid (usize::MAX = empty cell → GID 0)
    for (row_idx, row) in tile_grid.iter().enumerate() {
        for (col_idx, &idx) in row.iter().enumerate() {
            terrain_data[row_idx * w as usize + col_idx] = if idx == usize::MAX {
                0
            } else {
                idx as u32 + 1
            };
        }
    }

    // Stamp objects into the appropriate layers
    for (obj_name, obj_x, obj_y) in placements {
        let obj_def = object_defs
            .get(obj_name.as_str())
            .ok_or_else(|| format!("unknown object '{}'", obj_name))?;

        let (obj_cols, obj_rows) =
            pixl_core::types::parse_size(&obj_def.size_tiles)?;
        let obj_grid = parse_object_tile_grid(&obj_def.tiles);

        // Stamp base_tile into terrain under the object footprint
        if let Some(ref base) = obj_def.base_tile {
            let base_gid = tile_name_to_gid(base, tile_names).ok_or_else(|| {
                format!("object '{}' base_tile '{}' not found in tileset", obj_name, base)
            })?;
            for row in 0..obj_rows {
                for col in 0..obj_cols {
                    let mx = *obj_x + col;
                    let my = *obj_y + row;
                    if mx < w && my < h {
                        terrain_data[(my * w + mx) as usize] = base_gid;
                    }
                }
            }
        }

        // Stamp object tiles into above/below layers
        for (row_idx, row) in obj_grid.iter().enumerate() {
            let row_num = row_idx as u32;
            for (col_idx, tile_name) in row.iter().enumerate() {
                if *tile_name == "." || tile_name.is_empty() {
                    continue;
                }
                let mx = *obj_x + col_idx as u32;
                let my = *obj_y + row_num;
                if mx >= w || my >= h {
                    continue;
                }
                let gid = tile_name_to_gid(tile_name, tile_names).ok_or_else(|| {
                    format!(
                        "object '{}' references unknown tile '{}'",
                        obj_name, tile_name
                    )
                })?;
                let pos = (my * w + mx) as usize;
                if obj_def.below_player_rows.contains(&row_num) {
                    below_data[pos] = gid;
                } else {
                    // above_player_rows, or default when neither list claims this row
                    above_data[pos] = gid;
                }
            }
        }
    }

    Ok(TiledMap {
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
        layers: vec![
            TiledLayer {
                id: 1,
                name: "below_player".to_string(),
                layer_type: "tilelayer".to_string(),
                visible: true,
                opacity: 1.0,
                x: 0,
                y: 0,
                width: w,
                height: h,
                data: below_data,
            },
            TiledLayer {
                id: 2,
                name: "terrain".to_string(),
                layer_type: "tilelayer".to_string(),
                visible: true,
                opacity: 1.0,
                x: 0,
                y: 0,
                width: w,
                height: h,
                data: terrain_data,
            },
            TiledLayer {
                id: 3,
                name: "above_player".to_string(),
                layer_type: "tilelayer".to_string(),
                visible: true,
                opacity: 1.0,
                x: 0,
                y: 0,
                width: w,
                height: h,
                data: above_data,
            },
        ],
        tilesets: vec![TiledTilesetRef {
            first_gid: 1,
            source: tileset_source.to_string(),
        }],
    })
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
            16,
            16,
            "atlas.png",
            32,
            16,
            2,
            1,
            1,
            &collision,
        );

        assert_eq!(tileset.tile_count, 2);
        assert_eq!(tileset.tileset_type, "tileset");
        assert_eq!(tileset.spacing, 1);
        assert_eq!(tileset.margin, 1);
        assert!(tileset.tiles[0].object_group.is_some()); // wall has collision
        assert!(tileset.tiles[1].object_group.is_none()); // floor has no collision
    }

    #[test]
    fn map_from_wfc_grid() {
        let grid = vec![vec![0, 1, 0], vec![1, 0, 1]];
        let map = generate_map(&grid, 16, 16, "tileset.tsj");
        assert_eq!(map.width, 3);
        assert_eq!(map.height, 2);
        assert_eq!(map.layers[0].data, vec![1, 2, 1, 2, 1, 2]); // 1-based GIDs
    }

    #[test]
    fn tileset_json_serializes() {
        let tileset = generate_tileset(
            "dungeon",
            &["wall".to_string()],
            16,
            16,
            "atlas.png",
            16,
            16,
            1,
            0,
            0,
            &HashMap::new(),
        );
        let json = serde_json::to_string_pretty(&tileset).unwrap();
        assert!(json.contains("tilewidth"));
        assert!(json.contains("tileheight"));
        assert!(json.contains("pax_name"));
        assert!(json.contains("\"type\": \"tileset\""));
        assert!(json.contains("\"tiledversion\""));
        assert!(json.contains("\"spacing\""));
        assert!(json.contains("\"margin\""));
    }

    #[test]
    fn parse_object_grid_cottage() {
        let tiles = "roof_l      roof_c      roof_r\nwall_win_l  wall_door   wall_win_r\nwall_base_l wall_base_c wall_base_r\nshadow_l    shadow_c    shadow_r";
        let grid = parse_object_tile_grid(tiles);
        assert_eq!(grid.len(), 4);
        assert_eq!(grid[0].len(), 3);
        assert_eq!(grid[0][0], "roof_l");
        assert_eq!(grid[1][1], "wall_door");
        assert_eq!(grid[3][2], "shadow_r");
    }

    #[test]
    fn tile_name_gid_lookup() {
        let names = vec![
            "floor".to_string(),
            "grass".to_string(),
            "wall".to_string(),
        ];
        assert_eq!(tile_name_to_gid("floor", &names), Some(1));
        assert_eq!(tile_name_to_gid("grass", &names), Some(2));
        assert_eq!(tile_name_to_gid("wall", &names), Some(3));
        assert_eq!(tile_name_to_gid("missing", &names), None);
    }

    #[test]
    fn map_with_objects_simple() {
        // 4x4 terrain of tile index 0 ("floor" = GID 1)
        // tile_names sorted: floor(0), roof(1), trunk(2)
        let tile_names = vec![
            "floor".to_string(),
            "roof".to_string(),
            "trunk".to_string(),
        ];
        let grid = vec![vec![0; 4]; 4]; // all floor

        let obj = ObjectRaw {
            size_tiles: "2x2".to_string(),
            base_tile: None,
            above_player_rows: vec![0],
            below_player_rows: vec![1],
            tiles: "roof roof\ntrunk trunk".to_string(),
            collision: None,
        };
        let mut defs = HashMap::new();
        defs.insert("tree".to_string(), &obj);

        let placements = vec![("tree".to_string(), 1, 1)];
        let map = generate_map_with_objects(
            &grid,
            &tile_names,
            &placements,
            &defs,
            16,
            16,
            "tileset.tsj",
        )
        .unwrap();

        assert_eq!(map.layers.len(), 3);
        assert_eq!(map.layers[0].name, "below_player");
        assert_eq!(map.layers[1].name, "terrain");
        assert_eq!(map.layers[2].name, "above_player");

        // above_player: row 0 of object at (1,1) and (2,1) = "roof" = GID 2
        assert_eq!(map.layers[2].data[1 * 4 + 1], 2); // (1,1)
        assert_eq!(map.layers[2].data[1 * 4 + 2], 2); // (2,1)

        // below_player: row 1 of object at (1,2) and (2,2) = "trunk" = GID 3
        assert_eq!(map.layers[0].data[2 * 4 + 1], 3); // (1,2)
        assert_eq!(map.layers[0].data[2 * 4 + 2], 3); // (2,2)

        // terrain still has floor everywhere
        assert_eq!(map.layers[1].data[0], 1);
    }

    #[test]
    fn map_with_objects_default_above() {
        let tile_names = vec!["floor".to_string(), "leaf".to_string()];
        let grid = vec![vec![0; 2]; 2];

        let obj = ObjectRaw {
            size_tiles: "1x1".to_string(),
            base_tile: None,
            above_player_rows: vec![],
            below_player_rows: vec![],
            tiles: "leaf".to_string(),
            collision: None,
        };
        let mut defs = HashMap::new();
        defs.insert("bush".to_string(), &obj);

        let placements = vec![("bush".to_string(), 0, 0)];
        let map = generate_map_with_objects(
            &grid,
            &tile_names,
            &placements,
            &defs,
            16,
            16,
            "t.tsj",
        )
        .unwrap();

        // With neither list populated, defaults to above_player
        assert_eq!(map.layers[2].data[0], 2); // leaf GID
        assert_eq!(map.layers[0].data[0], 0); // below_player empty
    }

    #[test]
    fn map_with_objects_clipping() {
        let tile_names = vec!["floor".to_string(), "top".to_string()];
        let grid = vec![vec![0; 2]; 2]; // 2x2 map

        let obj = ObjectRaw {
            size_tiles: "2x2".to_string(),
            base_tile: None,
            above_player_rows: vec![0, 1],
            below_player_rows: vec![],
            tiles: "top top\ntop top".to_string(),
            collision: None,
        };
        let mut defs = HashMap::new();
        defs.insert("big".to_string(), &obj);

        // Place at (1,1) — only (1,1) is in bounds, (2,1), (1,2), (2,2) clip
        let placements = vec![("big".to_string(), 1, 1)];
        let map = generate_map_with_objects(
            &grid,
            &tile_names,
            &placements,
            &defs,
            16,
            16,
            "t.tsj",
        )
        .unwrap();

        assert_eq!(map.layers[2].data[1 * 2 + 1], 2); // (1,1) in bounds
        // no panic from out-of-bounds cells
    }

    #[test]
    fn map_with_objects_unknown_tile() {
        let tile_names = vec!["floor".to_string()];
        let grid = vec![vec![0; 2]; 2];

        let obj = ObjectRaw {
            size_tiles: "1x1".to_string(),
            base_tile: None,
            above_player_rows: vec![],
            below_player_rows: vec![],
            tiles: "nonexistent".to_string(),
            collision: None,
        };
        let mut defs = HashMap::new();
        defs.insert("bad".to_string(), &obj);

        let placements = vec![("bad".to_string(), 0, 0)];
        let result = generate_map_with_objects(
            &grid,
            &tile_names,
            &placements,
            &defs,
            16,
            16,
            "t.tsj",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nonexistent"));
    }

    #[test]
    fn map_with_objects_base_tile() {
        let tile_names = vec![
            "floor".to_string(),
            "grass".to_string(),
            "roof".to_string(),
        ];
        let grid = vec![vec![0; 3]; 3]; // all floor

        let obj = ObjectRaw {
            size_tiles: "2x1".to_string(),
            base_tile: Some("grass".to_string()),
            above_player_rows: vec![0],
            below_player_rows: vec![],
            tiles: "roof roof".to_string(),
            collision: None,
        };
        let mut defs = HashMap::new();
        defs.insert("awning".to_string(), &obj);

        let placements = vec![("awning".to_string(), 0, 0)];
        let map = generate_map_with_objects(
            &grid,
            &tile_names,
            &placements,
            &defs,
            16,
            16,
            "t.tsj",
        )
        .unwrap();

        // base_tile "grass" (GID 2) stamped into terrain under the object
        assert_eq!(map.layers[1].data[0], 2); // (0,0)
        assert_eq!(map.layers[1].data[1], 2); // (1,0)
        // rest of terrain is still floor
        assert_eq!(map.layers[1].data[2], 1); // (2,0)
    }

    #[test]
    fn map_with_empty_cells_in_terrain() {
        // usize::MAX signals an empty cell ("." in tilemap grids)
        let grid = vec![
            vec![0, usize::MAX],
            vec![usize::MAX, 0],
        ];
        let tile_names = vec!["floor".to_string()];
        let map = generate_map_with_objects(
            &grid,
            &tile_names,
            &[],
            &HashMap::new(),
            16,
            16,
            "t.tsj",
        )
        .unwrap();

        assert_eq!(map.layers[1].data[0], 1); // floor
        assert_eq!(map.layers[1].data[1], 0); // empty
        assert_eq!(map.layers[1].data[2], 0); // empty
        assert_eq!(map.layers[1].data[3], 1); // floor
    }
}
