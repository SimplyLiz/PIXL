//! Core tilemap types — game-level tilemaps with layers, objects, and WFC constraints.
//!
//! Implements PAX spec section 10: multi-layer tilemaps with z-ordering,
//! blend modes, collision, color cycling, and WFC constraint painting.

use crate::types::{BlendMode, TileRef};
use serde::Deserialize;
use std::collections::HashMap;

// ── Raw types (TOML deserialization) ────────────────────────────────

/// A game-level tilemap with multiple layers, objects, and WFC constraints.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TilemapRaw {
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_tile_width")]
    pub tile_width: u32,
    #[serde(default = "default_tile_height")]
    pub tile_height: u32,
    #[serde(default)]
    pub layer: HashMap<String, TilemapLayerRaw>,
    #[serde(default)]
    pub constraints: Option<TilemapConstraintsRaw>,
    #[serde(default)]
    pub objects: Vec<TilemapObjectPlacement>,
}

fn default_tile_width() -> u32 {
    16
}
fn default_tile_height() -> u32 {
    16
}

/// A single layer in a tilemap.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TilemapLayerRaw {
    #[serde(default)]
    pub z_order: i32,
    #[serde(default = "default_blend")]
    pub blend: String,
    #[serde(default)]
    pub collision: bool,
    #[serde(default)]
    pub collision_mode: Option<String>,
    #[serde(default)]
    pub layer_role: Option<String>,
    #[serde(default)]
    pub cycles: Vec<String>,
    #[serde(default)]
    pub scroll_factor: Option<f64>,
    #[serde(default)]
    pub grid: Option<String>,
}

fn default_blend() -> String {
    "normal".to_string()
}

/// WFC constraint painting for tilemaps.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TilemapConstraintsRaw {
    #[serde(default)]
    pub pins: Vec<TilemapPin>,
    #[serde(default)]
    pub zones: Vec<TilemapConstraintZone>,
    #[serde(default)]
    pub paths: Vec<TilemapPath>,
}

/// Pin a tile to a specific cell or range.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TilemapPin {
    pub x: u32,
    pub y: u32,
    #[serde(default)]
    pub to_x: Option<u32>,
    #[serde(default)]
    pub to_y: Option<u32>,
    pub tile: String,
}

/// Force a rectangular zone to a tile type.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TilemapConstraintZone {
    pub x1: u32,
    pub y1: u32,
    pub x2: u32,
    pub y2: u32,
    #[serde(rename = "type")]
    pub zone_type: String,
}

/// Require a passable path between two points.
#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TilemapPath {
    pub from: TilemapPoint,
    pub to: TilemapPoint,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct TilemapPoint {
    pub x: u32,
    pub y: u32,
}

/// Object placement in a tilemap.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct TilemapObjectPlacement {
    pub object: String,
    pub x: u32,
    pub y: u32,
}

// ── Resolved types ──────────────────────────────────────────────────

/// Resolved tilemap ready for rendering.
#[derive(Debug, Clone)]
pub struct Tilemap {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub layers: Vec<TilemapLayer>,
    pub objects: Vec<TilemapObjectPlacement>,
}

/// Resolved tilemap layer.
#[derive(Debug, Clone)]
pub struct TilemapLayer {
    pub name: String,
    pub z_order: i32,
    pub blend: BlendMode,
    pub collision: bool,
    pub collision_mode: CollisionMode,
    pub layer_role: LayerRole,
    pub cycles: Vec<String>,
    pub scroll_factor: f64,
    /// Tile references, row-major. "." = empty cell.
    pub grid: Vec<Vec<TileRef>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionMode {
    Full,
    TopOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerRole {
    Background,
    Platform,
    Foreground,
    Effects,
    Custom,
}

impl LayerRole {
    pub fn from_str(s: &str) -> Self {
        match s {
            "background" => LayerRole::Background,
            "platform" => LayerRole::Platform,
            "foreground" => LayerRole::Foreground,
            "effects" => LayerRole::Effects,
            _ => LayerRole::Custom,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LayerRole::Background => "background",
            LayerRole::Platform => "platform",
            LayerRole::Foreground => "foreground",
            LayerRole::Effects => "effects",
            LayerRole::Custom => "custom",
        }
    }
}

/// Resolve a TilemapRaw into a Tilemap.
pub fn resolve_tilemap(raw: &TilemapRaw) -> Tilemap {
    let mut layers: Vec<TilemapLayer> = raw
        .layer
        .iter()
        .map(|(name, lr)| {
            let grid = lr
                .grid
                .as_deref()
                .map(|g| {
                    g.lines()
                        .map(|l| l.trim())
                        .filter(|l| !l.is_empty())
                        .map(|line| line.split_whitespace().map(|s| TileRef::parse(s)).collect())
                        .collect()
                })
                .unwrap_or_default();

            TilemapLayer {
                name: name.clone(),
                z_order: lr.z_order,
                blend: BlendMode::from_str(&lr.blend),
                collision: lr.collision,
                collision_mode: match lr.collision_mode.as_deref() {
                    Some("top_only") => CollisionMode::TopOnly,
                    _ => CollisionMode::Full,
                },
                layer_role: lr
                    .layer_role
                    .as_deref()
                    .map(LayerRole::from_str)
                    .unwrap_or(LayerRole::Custom),
                cycles: lr.cycles.clone(),
                scroll_factor: lr.scroll_factor.unwrap_or(1.0),
                grid,
            }
        })
        .collect();

    // Sort by z_order (lowest = back)
    layers.sort_by_key(|l| l.z_order);

    Tilemap {
        width: raw.width,
        height: raw.height,
        tile_width: raw.tile_width,
        tile_height: raw.tile_height,
        layers,
        objects: raw.objects.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_simple_tilemap() {
        let mut layers = HashMap::new();
        layers.insert(
            "terrain".to_string(),
            TilemapLayerRaw {
                z_order: 0,
                blend: "normal".to_string(),
                collision: true,
                collision_mode: None,
                layer_role: Some("platform".to_string()),
                cycles: vec![],
                scroll_factor: None,
                grid: Some("floor_a floor_b\nfloor_c floor_d".to_string()),
            },
        );

        let raw = TilemapRaw {
            width: 2,
            height: 2,
            tile_width: 16,
            tile_height: 16,
            layer: layers,
            constraints: None,
            objects: vec![],
        };

        let tm = resolve_tilemap(&raw);
        assert_eq!(tm.layers.len(), 1);
        assert_eq!(tm.layers[0].name, "terrain");
        assert_eq!(tm.layers[0].grid.len(), 2);
        assert_eq!(tm.layers[0].grid[0][0].name, "floor_a");
        assert!(tm.layers[0].collision);
        assert_eq!(tm.layers[0].layer_role, LayerRole::Platform);
    }

    #[test]
    fn layers_sorted_by_z_order() {
        let mut layers = HashMap::new();
        layers.insert(
            "fg".to_string(),
            TilemapLayerRaw {
                z_order: 2,
                blend: "normal".to_string(),
                collision: false,
                collision_mode: None,
                layer_role: Some("foreground".to_string()),
                cycles: vec![],
                scroll_factor: None,
                grid: Some("a".to_string()),
            },
        );
        layers.insert(
            "bg".to_string(),
            TilemapLayerRaw {
                z_order: 0,
                blend: "normal".to_string(),
                collision: false,
                collision_mode: None,
                layer_role: Some("background".to_string()),
                cycles: vec![],
                scroll_factor: None,
                grid: Some("b".to_string()),
            },
        );

        let raw = TilemapRaw {
            width: 1,
            height: 1,
            tile_width: 16,
            tile_height: 16,
            layer: layers,
            constraints: None,
            objects: vec![],
        };

        let tm = resolve_tilemap(&raw);
        assert_eq!(tm.layers[0].name, "bg");
        assert_eq!(tm.layers[1].name, "fg");
    }

    #[test]
    fn tile_ref_with_flips_in_grid() {
        let mut layers = HashMap::new();
        layers.insert(
            "main".to_string(),
            TilemapLayerRaw {
                z_order: 0,
                blend: "normal".to_string(),
                collision: false,
                collision_mode: None,
                layer_role: None,
                cycles: vec![],
                scroll_factor: None,
                grid: Some("wall!h wall!v wall!hv:shadow".to_string()),
            },
        );

        let raw = TilemapRaw {
            width: 3,
            height: 1,
            tile_width: 16,
            tile_height: 16,
            layer: layers,
            constraints: None,
            objects: vec![],
        };

        let tm = resolve_tilemap(&raw);
        let row = &tm.layers[0].grid[0];
        assert!(row[0].flip_h);
        assert!(!row[0].flip_v);
        assert!(!row[1].flip_h);
        assert!(row[1].flip_v);
        assert!(row[2].flip_h);
        assert!(row[2].flip_v);
    }
}
