use crate::renderer::render_grid;
/// GBStudio export — 160x144 PNG grid layout for Game Boy style games.
///
/// GBStudio uses a specific tileset format:
/// - Background tiles: 8x8, arranged in a grid PNG
/// - Max 192 unique background tiles per scene
/// - 4 colors per tile (Game Boy palette)
/// - Tileset PNG width = 128px (16 tiles per row)
///
/// This module generates the correctly-sized PNG grid that GBStudio expects.
use image::{ImageBuffer, Rgba, RgbaImage};
use pixl_core::types::Palette;

/// Pack tiles into a GBStudio-compatible tileset PNG.
/// Width fixed at 128px (16 x 8px tiles per row).
pub fn pack_gbstudio(
    tile_grids: &[Vec<Vec<char>>],
    palette: &Palette,
) -> Result<RgbaImage, String> {
    if tile_grids.is_empty() {
        return Err("no tiles to pack".to_string());
    }

    let tile_size = 8u32;
    let cols = 16u32;
    let rows = (tile_grids.len() as u32).div_ceil(cols);
    let img_w = cols * tile_size; // always 128
    let img_h = rows * tile_size;

    let mut atlas: RgbaImage = ImageBuffer::from_pixel(img_w, img_h, Rgba([0, 0, 0, 0]));

    for (i, grid) in tile_grids.iter().enumerate() {
        let col = i as u32 % cols;
        let row = i as u32 / cols;
        let x = col * tile_size;
        let y = row * tile_size;

        let tile_img = render_grid(grid, palette, 1);

        for py in 0..tile_img.height().min(tile_size) {
            for px in 0..tile_img.width().min(tile_size) {
                atlas.put_pixel(x + px, y + py, *tile_img.get_pixel(px, py));
            }
        }
    }

    Ok(atlas)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixl_core::types::Rgba as PaxRgba;
    use std::collections::HashMap;

    fn gb_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert(
            '.',
            PaxRgba {
                r: 15,
                g: 56,
                b: 15,
                a: 255,
            },
        ); // darkest
        symbols.insert(
            '1',
            PaxRgba {
                r: 48,
                g: 98,
                b: 48,
                a: 255,
            },
        );
        symbols.insert(
            '2',
            PaxRgba {
                r: 139,
                g: 172,
                b: 15,
                a: 255,
            },
        );
        symbols.insert(
            '3',
            PaxRgba {
                r: 155,
                g: 188,
                b: 15,
                a: 255,
            },
        ); // lightest
        Palette { symbols }
    }

    #[test]
    fn packs_into_128_wide_grid() {
        let palette = gb_palette();
        let tile = vec![
            vec!['.', '1', '2', '3', '.', '1', '2', '3'],
            vec!['3', '2', '1', '.', '3', '2', '1', '.'],
            vec!['.', '1', '2', '3', '.', '1', '2', '3'],
            vec!['3', '2', '1', '.', '3', '2', '1', '.'],
            vec!['.', '1', '2', '3', '.', '1', '2', '3'],
            vec!['3', '2', '1', '.', '3', '2', '1', '.'],
            vec!['.', '1', '2', '3', '.', '1', '2', '3'],
            vec!['3', '2', '1', '.', '3', '2', '1', '.'],
        ];

        let tiles = vec![tile.clone(); 20]; // 20 tiles
        let img = pack_gbstudio(&tiles, &palette).unwrap();
        assert_eq!(img.width(), 128); // 16 tiles x 8px
        assert_eq!(img.height(), 16); // 2 rows (ceil(20/16) = 2)
    }
}
