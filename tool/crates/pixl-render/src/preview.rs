use image::{Rgba, RgbaImage};

/// Render a 16x zoom preview with optional grid lines for SELF-REFINE loop.
/// Grid lines are drawn at tile-pixel boundaries to help the LLM see
/// individual pixels clearly.
pub fn render_preview(
    base_image: &RgbaImage,
    tile_width: u32,
    tile_height: u32,
    scale: u32,
    show_grid: bool,
) -> RgbaImage {
    let mut img = base_image.clone();

    if !show_grid {
        return img;
    }

    let grid_color = Rgba([128, 128, 128, 80]); // semi-transparent gray

    // Draw vertical grid lines at each tile-pixel boundary
    for tx in 1..tile_width {
        let px = tx * scale;
        if px < img.width() {
            for py in 0..img.height() {
                blend_pixel(&mut img, px, py, &grid_color);
            }
        }
    }

    // Draw horizontal grid lines
    for ty in 1..tile_height {
        let py = ty * scale;
        if py < img.height() {
            for px in 0..img.width() {
                blend_pixel(&mut img, px, py, &grid_color);
            }
        }
    }

    img
}

/// Alpha-blend a pixel on top of existing pixel.
fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, overlay: &Rgba<u8>) {
    let dst = img.get_pixel(x, y);
    let alpha = overlay.0[3] as f32 / 255.0;
    let inv = 1.0 - alpha;

    let r = (overlay.0[0] as f32 * alpha + dst.0[0] as f32 * inv) as u8;
    let g = (overlay.0[1] as f32 * alpha + dst.0[1] as f32 * inv) as u8;
    let b = (overlay.0[2] as f32 * alpha + dst.0[2] as f32 * inv) as u8;
    let a = (overlay.0[3] as f32 + dst.0[3] as f32 * inv).min(255.0) as u8;

    img.put_pixel(x, y, Rgba([r, g, b, a]));
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageBuffer;

    #[test]
    fn preview_without_grid_is_identity() {
        let img = ImageBuffer::from_pixel(32, 32, Rgba([42, 31, 61, 255]));
        let preview = render_preview(&img, 2, 2, 16, false);
        assert_eq!(preview.get_pixel(0, 0), img.get_pixel(0, 0));
    }

    #[test]
    fn preview_with_grid_adds_lines() {
        let img = ImageBuffer::from_pixel(32, 32, Rgba([42, 31, 61, 255]));
        let preview = render_preview(&img, 2, 2, 16, true);
        // Grid line at x=16 should differ from original
        let original = img.get_pixel(16, 0);
        let with_grid = preview.get_pixel(16, 0);
        assert_ne!(original, with_grid);
    }

    #[test]
    fn preview_dimensions_unchanged() {
        let img = ImageBuffer::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
        let preview = render_preview(&img, 4, 4, 16, true);
        assert_eq!(preview.width(), 64);
        assert_eq!(preview.height(), 64);
    }
}
