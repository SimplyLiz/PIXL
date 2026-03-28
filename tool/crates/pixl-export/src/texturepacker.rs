//! TexturePacker JSON Hash export.
//!
//! Re-exports the canonical types from `pixl_render::atlas` and provides
//! convenience constructors for game engine export workflows.

use std::collections::HashMap;

// ── Re-export canonical types from pixl-render ──────────────────────
// These are the single source of truth for TexturePacker JSON format.
pub use pixl_render::atlas::{
    AtlasJson, AtlasMeta, Border, FrameEntry, FrameTag, Pivot, Rect, Size,
    frame_tags_from_spritesets,
};

/// Generate TexturePacker JSON Hash from pre-built frame entries.
pub fn generate(
    frames: HashMap<String, FrameEntry>,
    image_name: &str,
    atlas_width: u32,
    atlas_height: u32,
    scale: u32,
) -> AtlasJson {
    generate_with_tags(frames, image_name, atlas_width, atlas_height, scale, vec![])
}

/// Generate TexturePacker JSON Hash with animation frame tags.
pub fn generate_with_tags(
    frames: HashMap<String, FrameEntry>,
    image_name: &str,
    atlas_width: u32,
    atlas_height: u32,
    scale: u32,
    frame_tags: Vec<FrameTag>,
) -> AtlasJson {
    AtlasJson {
        frames,
        meta: AtlasMeta {
            app: "pixl".to_string(),
            version: "0.1.0".to_string(),
            image: image_name.to_string(),
            format: "RGBA8888".to_string(),
            frame_tags,
            size: Size {
                w: atlas_width,
                h: atlas_height,
            },
            scale: scale.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_correctly() {
        let mut frames = HashMap::new();
        frames.insert(
            "wall_solid".to_string(),
            FrameEntry {
                frame: Rect {
                    x: 1,
                    y: 1,
                    w: 16,
                    h: 16,
                },
                rotated: false,
                trimmed: false,
                sprite_source_size: Rect {
                    x: 0,
                    y: 0,
                    w: 16,
                    h: 16,
                },
                source_size: Size { w: 16, h: 16 },
                pivot: Pivot { x: 0.5, y: 0.5 },
                border: None,
            },
        );

        let json = generate(frames, "atlas.png", 256, 128, 1);
        let serialized = serde_json::to_string_pretty(&json).unwrap();
        assert!(serialized.contains("spriteSourceSize"));
        assert!(serialized.contains("RGBA8888"));
        assert!(serialized.contains("\"app\": \"pixl\""));
        assert!(!serialized.contains("border")); // skipped when None
    }

    #[test]
    fn nine_slice_border_included() {
        let mut frames = HashMap::new();
        frames.insert(
            "ui_panel".to_string(),
            FrameEntry {
                frame: Rect {
                    x: 0,
                    y: 0,
                    w: 24,
                    h: 24,
                },
                rotated: false,
                trimmed: false,
                sprite_source_size: Rect {
                    x: 0,
                    y: 0,
                    w: 24,
                    h: 24,
                },
                source_size: Size { w: 24, h: 24 },
                pivot: Pivot { x: 0.5, y: 0.5 },
                border: Some(Border {
                    left: 8,
                    right: 8,
                    top: 8,
                    bottom: 8,
                }),
            },
        );

        let json = generate(frames, "ui.png", 24, 24, 1);
        let serialized = serde_json::to_string(&json).unwrap();
        assert!(serialized.contains("\"border\""));
        assert!(serialized.contains("\"left\":8"));
    }
}
