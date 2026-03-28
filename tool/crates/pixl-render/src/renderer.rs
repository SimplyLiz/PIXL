use image::{ImageBuffer, Rgba, RgbaImage};
use pixl_core::types::{Palette, Rgba as PaxRgba};

/// Render a resolved tile grid to an image at the given scale.
/// Nearest-neighbor upscaling only — no bilinear, no anti-aliasing.
pub fn render_grid(grid: &[Vec<char>], palette: &Palette, scale: u32) -> RgbaImage {
    let h = grid.len() as u32;
    let w = if h > 0 { grid[0].len() as u32 } else { 0 };

    let img_w = w * scale;
    let img_h = h * scale;

    let mut img = ImageBuffer::new(img_w, img_h);

    for (ty, row) in grid.iter().enumerate() {
        for (tx, &sym) in row.iter().enumerate() {
            let color = palette.symbols.get(&sym).copied().unwrap_or(ERROR_COLOR);
            let pixel = to_image_rgba(&color);

            for dy in 0..scale {
                for dx in 0..scale {
                    let px = tx as u32 * scale + dx;
                    let py = ty as u32 * scale + dy;
                    img.put_pixel(px, py, pixel);
                }
            }
        }
    }

    img
}

/// Render a grid with palette swap applied.
pub fn render_grid_with_swap(
    grid: &[Vec<char>],
    palette: &Palette,
    swap_map: &std::collections::HashMap<char, PaxRgba>,
    scale: u32,
) -> RgbaImage {
    let h = grid.len() as u32;
    let w = if h > 0 { grid[0].len() as u32 } else { 0 };

    let img_w = w * scale;
    let img_h = h * scale;

    let mut img = ImageBuffer::new(img_w, img_h);

    for (ty, row) in grid.iter().enumerate() {
        for (tx, &sym) in row.iter().enumerate() {
            let color = swap_map
                .get(&sym)
                .or_else(|| palette.symbols.get(&sym))
                .copied()
                .unwrap_or(ERROR_COLOR);
            let pixel = to_image_rgba(&color);

            for dy in 0..scale {
                for dx in 0..scale {
                    let px = tx as u32 * scale + dx;
                    let py = ty as u32 * scale + dy;
                    img.put_pixel(px, py, pixel);
                }
            }
        }
    }

    img
}

/// Render a composite sprite to an image.
///
/// Resolves the composite grid (with optional variant and animation frame),
/// then renders it like any other tile grid.
pub fn render_composite(
    composite: &pixl_core::types::Composite,
    variant: Option<&str>,
    anim_name: Option<&str>,
    frame_index: Option<u32>,
    tiles: &std::collections::HashMap<String, pixl_core::types::Tile>,
    palette: &Palette,
    scale: u32,
) -> Result<RgbaImage, pixl_core::composite::CompositeError> {
    let grid = if let Some(anim) = anim_name {
        pixl_core::composite::compose_anim_frame(
            composite,
            anim,
            frame_index.unwrap_or(1),
            variant,
            tiles,
            '.',
        )?
    } else {
        pixl_core::composite::compose_grid(composite, variant, frame_index, tiles, '.')?
    };

    Ok(render_grid(&grid, palette, scale))
}

/// Error color for unknown symbols — hot pink (#FF00FF).
const ERROR_COLOR: PaxRgba = PaxRgba {
    r: 255,
    g: 0,
    b: 255,
    a: 255,
};

fn to_image_rgba(c: &PaxRgba) -> Rgba<u8> {
    Rgba([c.r, c.g, c.b, c.a])
}

/// Encode an image to PNG bytes in memory.
pub fn encode_png(img: &RgbaImage) -> Vec<u8> {
    let mut buf = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut buf);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )
    .expect("PNG encoding should not fail on valid ImageBuffer");
    buf
}

/// Encode PNG bytes to base64 string (for MCP responses).
pub fn png_to_base64(png_bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(png_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert(
            '.',
            PaxRgba {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        );
        symbols.insert(
            '#',
            PaxRgba {
                r: 42,
                g: 31,
                b: 61,
                a: 255,
            },
        );
        symbols.insert(
            '+',
            PaxRgba {
                r: 74,
                g: 58,
                b: 109,
                a: 255,
            },
        );
        Palette { symbols }
    }

    #[test]
    fn render_2x2_scale1() {
        let palette = test_palette();
        let grid = vec![vec!['#', '+'], vec!['+', '#']];
        let img = render_grid(&grid, &palette, 1);
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);
        assert_eq!(img.get_pixel(0, 0), &Rgba([42, 31, 61, 255]));
        assert_eq!(img.get_pixel(1, 0), &Rgba([74, 58, 109, 255]));
    }

    #[test]
    fn render_scale2_doubles_dimensions() {
        let palette = test_palette();
        let grid = vec![vec!['#', '+'], vec!['+', '#']];
        let img = render_grid(&grid, &palette, 2);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);
        // Top-left 2x2 block should all be '#' color
        assert_eq!(img.get_pixel(0, 0), &Rgba([42, 31, 61, 255]));
        assert_eq!(img.get_pixel(1, 0), &Rgba([42, 31, 61, 255]));
        assert_eq!(img.get_pixel(0, 1), &Rgba([42, 31, 61, 255]));
        assert_eq!(img.get_pixel(1, 1), &Rgba([42, 31, 61, 255]));
    }

    #[test]
    fn unknown_symbol_renders_hot_pink() {
        let palette = test_palette();
        let grid = vec![vec!['X']]; // not in palette
        let img = render_grid(&grid, &palette, 1);
        assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 255, 255]));
    }

    #[test]
    fn empty_grid_produces_empty_image() {
        let palette = test_palette();
        let grid: Vec<Vec<char>> = vec![];
        let img = render_grid(&grid, &palette, 1);
        assert_eq!(img.width(), 0);
        assert_eq!(img.height(), 0);
    }

    #[test]
    fn png_roundtrip() {
        let palette = test_palette();
        let grid = vec![vec!['#', '+'], vec!['+', '#']];
        let img = render_grid(&grid, &palette, 1);
        let png_bytes = encode_png(&img);
        assert!(!png_bytes.is_empty());
        // PNG magic bytes
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn base64_output() {
        let palette = test_palette();
        let grid = vec![vec!['#']];
        let img = render_grid(&grid, &palette, 1);
        let b64 = png_to_base64(&encode_png(&img));
        assert!(b64.starts_with("iVBOR")); // PNG in base64 always starts with this
    }
}
