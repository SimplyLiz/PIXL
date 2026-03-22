use image::RgbaImage;

/// Encode a sequence of frames as an animated GIF.
/// Each frame has a duration in milliseconds.
/// Returns the GIF bytes.
pub fn encode_gif(
    frames: &[(RgbaImage, u32)], // (image, duration_ms)
    loop_anim: bool,
) -> Result<Vec<u8>, String> {
    if frames.is_empty() {
        return Err("no frames to encode".to_string());
    }

    let (first, _) = &frames[0];
    let width = first.width() as u16;
    let height = first.height() as u16;

    let mut buf = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut buf, width, height, &[])
            .map_err(|e| format!("GIF encoder init failed: {}", e))?;

        if loop_anim {
            encoder
                .set_repeat(gif::Repeat::Infinite)
                .map_err(|e| format!("GIF repeat failed: {}", e))?;
        }

        for (img, duration_ms) in frames {
            // Quantize RGBA to 256-color indexed
            let (palette_flat, indices) = quantize_frame(img);

            let delay = (*duration_ms / 10).max(1) as u16; // centiseconds

            let mut frame = gif::Frame::from_palette_pixels(
                img.width() as u16,
                img.height() as u16,
                indices,
                palette_flat,
                None,
            );
            frame.delay = delay;

            encoder
                .write_frame(&frame)
                .map_err(|e| format!("GIF frame write failed: {}", e))?;
        }
    }

    Ok(buf)
}

/// Quantize an RGBA image to a 256-color indexed palette.
/// Returns (flat_palette [r,g,b,...], indices).
fn quantize_frame(img: &RgbaImage) -> (Vec<u8>, Vec<u8>) {
    let mut colors: Vec<[u8; 3]> = Vec::new();
    let mut indices = Vec::with_capacity((img.width() * img.height()) as usize);
    let mut color_map: std::collections::HashMap<[u8; 3], u8> = std::collections::HashMap::new();

    for pixel in img.pixels() {
        let rgb = [pixel.0[0], pixel.0[1], pixel.0[2]];
        let idx = if let Some(&i) = color_map.get(&rgb) {
            i
        } else if colors.len() < 256 {
            let i = colors.len() as u8;
            colors.push(rgb);
            color_map.insert(rgb, i);
            i
        } else {
            nearest_color(&colors, &rgb)
        };
        indices.push(idx);
    }

    // Pad to power of 2 (GIF requirement)
    let target = colors.len().next_power_of_two().max(2);
    while colors.len() < target {
        colors.push([0, 0, 0]);
    }

    let flat: Vec<u8> = colors.into_iter().flat_map(|c| c).collect();
    (flat, indices)
}

fn nearest_color(palette: &[[u8; 3]], target: &[u8; 3]) -> u8 {
    palette
        .iter()
        .enumerate()
        .min_by_key(|(_, c)| {
            let dr = c[0] as i32 - target[0] as i32;
            let dg = c[1] as i32 - target[1] as i32;
            let db = c[2] as i32 - target[2] as i32;
            (dr * dr + dg * dg + db * db) as u32
        })
        .map(|(i, _)| i as u8)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn encode_single_frame() {
        let img = ImageBuffer::from_pixel(4, 4, Rgba([42, 31, 61, 255]));
        let frames = vec![(img, 100)];
        let gif_bytes = encode_gif(&frames, false).unwrap();
        assert!(!gif_bytes.is_empty());
        assert_eq!(&gif_bytes[0..3], b"GIF");
    }

    #[test]
    fn encode_two_frames_looping() {
        let f1 = ImageBuffer::from_pixel(4, 4, Rgba([42, 31, 61, 255]));
        let f2 = ImageBuffer::from_pixel(4, 4, Rgba([74, 58, 109, 255]));
        let frames = vec![(f1, 100), (f2, 100)];
        let gif_bytes = encode_gif(&frames, true).unwrap();
        assert!(gif_bytes.len() > 20);
    }

    #[test]
    fn empty_frames_error() {
        let frames: Vec<(RgbaImage, u32)> = vec![];
        assert!(encode_gif(&frames, false).is_err());
    }
}
