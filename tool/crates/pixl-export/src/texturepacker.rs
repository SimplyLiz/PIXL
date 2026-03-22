use serde::Serialize;
use std::collections::HashMap;

/// TexturePacker JSON Hash format — de facto sprite atlas standard.
/// Understood by 48+ game engines: Unity, Godot, Phaser, libGDX, Bevy,
/// Defold, Cocos2d, GDevelop, and more.
///
/// NOTE: TexturePacker does NOT include animationTags in its meta section.
/// Animation tags use Aseprite-compatible frameTags in a separate structure.

#[derive(Debug, Serialize)]
pub struct TexturePackerJson {
    pub frames: HashMap<String, FrameEntry>,
    pub meta: Meta,
}

#[derive(Debug, Serialize)]
pub struct FrameEntry {
    pub frame: Rect,
    pub rotated: bool,
    pub trimmed: bool,
    #[serde(rename = "spriteSourceSize")]
    pub sprite_source_size: Rect,
    #[serde(rename = "sourceSize")]
    pub source_size: Size,
    pub pivot: Pivot,
    /// 9-slice border (only present if tile has nine_slice)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<Border>,
}

#[derive(Debug, Serialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Serialize)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Serialize)]
pub struct Pivot {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Serialize)]
pub struct Border {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Serialize)]
pub struct Meta {
    pub app: String,
    pub version: String,
    pub image: String,
    pub format: String,
    pub size: Size,
    pub scale: String,
}

/// Aseprite-compatible frameTags for animation data.
#[derive(Debug, Serialize)]
pub struct FrameTag {
    pub name: String,
    pub from: u32,
    pub to: u32,
    pub direction: String,
}

/// Generate TexturePacker JSON Hash from atlas data.
pub fn generate(
    frames: HashMap<String, FrameEntry>,
    image_name: &str,
    atlas_width: u32,
    atlas_height: u32,
    scale: u32,
) -> TexturePackerJson {
    TexturePackerJson {
        frames,
        meta: Meta {
            app: "pixl".to_string(),
            version: "0.1.0".to_string(),
            image: image_name.to_string(),
            format: "RGBA8888".to_string(),
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
