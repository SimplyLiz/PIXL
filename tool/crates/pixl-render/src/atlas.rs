use image::{ImageBuffer, Rgba, RgbaImage};
use pixl_core::types::Palette;
use crate::renderer::render_grid;
use serde::Serialize;
use std::collections::HashMap;

/// Atlas metadata in TexturePacker JSON Hash format.
#[derive(Debug, Serialize)]
pub struct AtlasJson {
    pub frames: HashMap<String, FrameEntry>,
    pub meta: AtlasMeta,
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
pub struct AtlasMeta {
    pub app: String,
    pub version: String,
    pub image: String,
    pub format: String,
    pub size: Size,
    pub scale: String,
}

/// A tile to be packed into an atlas.
pub struct AtlasTile {
    pub name: String,
    pub grid: Vec<Vec<char>>,
    pub width: u32,
    pub height: u32,
}

/// Pack tiles into a grid-layout atlas.
/// All tiles must share dimensions (validated before calling).
pub fn pack_atlas(
    tiles: &[AtlasTile],
    palette: &Palette,
    columns: u32,
    padding: u32,
    scale: u32,
    output_name: &str,
) -> Result<(RgbaImage, AtlasJson), String> {
    if tiles.is_empty() {
        return Err("no tiles to pack".to_string());
    }

    // Validate uniform size
    let (tw, th) = (tiles[0].width, tiles[0].height);
    for t in tiles {
        if t.width != tw || t.height != th {
            return Err(format!(
                "mixed tile sizes: '{}' is {}x{}, expected {}x{}. \
                 Use --include to filter by size.",
                t.name, t.width, t.height, tw, th
            ));
        }
    }

    let cell_w = tw * scale + padding;
    let cell_h = th * scale + padding;
    let rows = (tiles.len() as u32 + columns - 1) / columns;
    let atlas_w = columns * cell_w + padding;
    let atlas_h = rows * cell_h + padding;

    let mut atlas: RgbaImage = ImageBuffer::from_pixel(atlas_w, atlas_h, Rgba([0, 0, 0, 0]));
    let mut frames = HashMap::new();

    for (i, tile) in tiles.iter().enumerate() {
        let col = i as u32 % columns;
        let row = i as u32 / columns;
        let x = padding + col * cell_w;
        let y = padding + row * cell_h;

        let tile_img = render_grid(&tile.grid, palette, scale);

        // Blit tile onto atlas
        for py in 0..tile_img.height() {
            for px in 0..tile_img.width() {
                let ax = x + px;
                let ay = y + py;
                if ax < atlas_w && ay < atlas_h {
                    atlas.put_pixel(ax, ay, *tile_img.get_pixel(px, py));
                }
            }
        }

        frames.insert(
            tile.name.clone(),
            FrameEntry {
                frame: Rect {
                    x,
                    y,
                    w: tw * scale,
                    h: th * scale,
                },
                rotated: false,
                trimmed: false,
                sprite_source_size: Rect {
                    x: 0,
                    y: 0,
                    w: tw * scale,
                    h: th * scale,
                },
                source_size: Size {
                    w: tw * scale,
                    h: th * scale,
                },
                pivot: Pivot { x: 0.5, y: 0.5 },
            },
        );
    }

    let meta = AtlasMeta {
        app: "pixl".to_string(),
        version: "0.1.0".to_string(),
        image: output_name.to_string(),
        format: "RGBA8888".to_string(),
        size: Size {
            w: atlas_w,
            h: atlas_h,
        },
        scale: scale.to_string(),
    };

    Ok((atlas, AtlasJson { frames, meta }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixl_core::types::Rgba as PaxRgba;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert('#', PaxRgba { r: 42, g: 31, b: 61, a: 255 });
        symbols.insert('+', PaxRgba { r: 74, g: 58, b: 109, a: 255 });
        Palette { symbols }
    }

    #[test]
    fn pack_two_tiles() {
        let palette = test_palette();
        let tiles = vec![
            AtlasTile {
                name: "a".to_string(),
                grid: vec![vec!['#', '#'], vec!['#', '#']],
                width: 2,
                height: 2,
            },
            AtlasTile {
                name: "b".to_string(),
                grid: vec![vec!['+', '+'], vec!['+', '+']],
                width: 2,
                height: 2,
            },
        ];

        let (img, json) = pack_atlas(&tiles, &palette, 4, 1, 1, "test.png").unwrap();
        assert!(img.width() > 0);
        assert!(img.height() > 0);
        assert_eq!(json.frames.len(), 2);
        assert!(json.frames.contains_key("a"));
        assert!(json.frames.contains_key("b"));
        assert_eq!(json.meta.app, "pixl");
    }

    #[test]
    fn mixed_sizes_rejected() {
        let palette = test_palette();
        let tiles = vec![
            AtlasTile {
                name: "a".to_string(),
                grid: vec![vec!['#', '#'], vec!['#', '#']],
                width: 2,
                height: 2,
            },
            AtlasTile {
                name: "b".to_string(),
                grid: vec![vec!['+', '+', '+'], vec!['+', '+', '+']],
                width: 3,
                height: 2,
            },
        ];

        let err = pack_atlas(&tiles, &palette, 4, 1, 1, "test.png").unwrap_err();
        assert!(err.contains("mixed tile sizes"));
    }

    #[test]
    fn json_has_texturepacker_fields() {
        let palette = test_palette();
        let tiles = vec![AtlasTile {
            name: "t".to_string(),
            grid: vec![vec!['#']],
            width: 1,
            height: 1,
        }];

        let (_, json) = pack_atlas(&tiles, &palette, 4, 0, 2, "out.png").unwrap();
        let serialized = serde_json::to_string(&json).unwrap();
        assert!(serialized.contains("spriteSourceSize"));
        assert!(serialized.contains("sourceSize"));
        assert!(serialized.contains("RGBA8888"));
    }
}
