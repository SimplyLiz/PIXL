//! Diffusion bridge — generate sprite reference images via OpenAI DALL-E,
//! then quantize into PAX palette grids.
//!
//! Flow: text prompt → DALL-E 3 → PNG → import_reference() → PAX grid

use base64::Engine;
use image::GenericImageView;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug)]
pub struct DiffusionConfig {
    pub api_key: String,
    pub model: String,
}

impl DiffusionConfig {
    /// Load config from environment. Returns None if OPENAI_API_KEY is not set.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").ok()?;
        if api_key.is_empty() {
            return None;
        }
        Some(Self {
            api_key,
            model: std::env::var("PIXL_IMAGE_MODEL")
                .unwrap_or_else(|_| "gpt-image-1".to_string()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct ImageResponse {
    data: Vec<ImageData>,
}

#[derive(Debug, Deserialize)]
struct ImageData {
    b64_json: Option<String>,
    url: Option<String>,
}

/// Generate an image via OpenAI's image generation API.
/// Returns raw PNG bytes.
pub async fn generate_image(
    config: &DiffusionConfig,
    prompt: &str,
    size: &str,
) -> Result<Vec<u8>, String> {
    let pixel_prompt = format!(
        "Pixel art sprite on a completely transparent background (alpha checkerboard). \
         Hard 1-pixel dark outline around the ENTIRE subject. The outline must be a single \
         unbroken ring of dark pixels — no gaps, no glow, no halo, no soft edges, no bloom. \
         Every outline pixel must be exactly 1 pixel wide and directly adjacent to transparent pixels. \
         Strictly limited color palette (max 8-10 colors). Completely flat shading with NO gradients, \
         NO anti-aliasing, NO sub-pixel blending, NO dithering. Each color area is a solid block. \
         The subject must be perfectly centered and fill 70-85% of the canvas. \
         NO background elements, NO ground plane, NO shadow on ground — subject only. \
         Style: 16-bit era SNES/GBA pixel art game sprite, clean professional quality. \
         Subject: {}",
        prompt
    );

    let body = serde_json::json!({
        "model": config.model,
        "prompt": pixel_prompt,
        "n": 1,
        "size": size,
        "background": "transparent",
        "output_format": "png",
    });

    let resp = reqwest::Client::new()
        .post("https://api.openai.com/v1/images/generations")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|e| format!("OpenAI API request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let error_body = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error {}: {}", status, error_body));
    }

    let image_resp: ImageResponse = resp
        .json()
        .await
        .map_err(|e| format!("failed to parse OpenAI response: {}", e))?;

    let data = image_resp
        .data
        .first()
        .ok_or("no image data in response")?;

    if let Some(ref b64) = data.b64_json {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("base64 decode failed: {}", e))?;
        return Ok(bytes);
    }

    if let Some(ref url) = data.url {
        let img_resp = reqwest::Client::new()
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("failed to download image: {}", e))?;

        let bytes = img_resp
            .bytes()
            .await
            .map_err(|e| format!("failed to read image bytes: {}", e))?;
        return Ok(bytes.to_vec());
    }

    Err("response contained neither b64_json nor url".to_string())
}

/// Full pipeline: generate image → quantize to palette → return PAX grid.
pub async fn generate_and_quantize(
    config: &DiffusionConfig,
    prompt: &str,
    target_width: u32,
    target_height: u32,
    palette: &pixl_core::types::Palette,
    dither: bool,
) -> Result<GenerateResult, String> {
    // Generate reference image at a reasonable resolution for DALL-E
    let dalle_size = "1024x1024";

    let png_bytes = generate_image(config, prompt, dalle_size).await?;

    // Decode PNG into image
    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| format!("failed to decode generated image: {}", e))?;

    let (gen_w, gen_h) = img.dimensions();

    // Quantize to palette at target resolution
    let import_result =
        pixl_render::import::import_reference(&img, target_width, target_height, palette, dither);

    // Post-processing: enforce outlines
    let grid = enforce_outline(&import_result.grid, palette, '.');
    let grid_string = grid
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(GenerateResult {
        grid,
        grid_string,
        width: target_width,
        height: target_height,
        color_accuracy: import_result.color_accuracy,
        clipped_colors: import_result.clipped_colors,
        generated_size: (gen_w, gen_h),
        reference_png: png_bytes,
        extracted_palette: None,
        palette_toml: None,
        detected_pixel_size: import_result.detected_pixel_size,
        native_resolution: import_result.native_resolution,
    })
}

/// Full pipeline with auto-palette: generate image → extract colors → quantize → PAX grid.
/// Produces a palette matched to the generated image for maximum color fidelity.
///
/// If `target_width`/`target_height` are None, automatically uses the native pixel art
/// resolution snapped to the next valid PAX canvas size (8, 16, 24, 32, 48, 64).
/// This preserves maximum detail from the AI-generated reference.
pub async fn generate_with_auto_palette(
    config: &DiffusionConfig,
    prompt: &str,
    target_width: Option<u32>,
    target_height: Option<u32>,
    max_colors: u32,
    dither: bool,
) -> Result<GenerateResult, String> {
    let dalle_size = "1024x1024";
    let png_bytes = generate_image(config, prompt, dalle_size).await?;

    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| format!("failed to decode generated image: {}", e))?;

    let (gen_w, gen_h) = img.dimensions();

    // Detect native pixel art resolution
    let detected = pixl_render::import::detect_pixel_size_pub(&img);
    let native_w = (gen_w / detected).max(1);
    let native_h = (gen_h / detected).max(1);

    // If no target specified, snap native to nearest valid PAX canvas size
    let (tw, th) = match (target_width, target_height) {
        (Some(w), Some(h)) => (w, h),
        _ => {
            let snapped = snap_to_canvas_size(native_w.max(native_h));
            (snapped, snapped)
        }
    };

    // Extract palette from the generated image
    let colors = pixl_render::pixelize::extract_palette(&img, max_colors);
    if colors.is_empty() {
        return Err("no colors extracted from generated image (fully transparent?)".to_string());
    }

    let palette = pixl_render::pixelize::build_pax_palette(&colors, [0, 0, 0, 0]);
    let palette_toml = pixl_render::pixelize::palette_to_toml("auto", &colors);

    // Quantize using the extracted palette at the resolved target size
    let import_result =
        pixl_render::import::import_reference(&img, tw, th, &palette, dither);

    // Post-processing: enforce outlines on boundary pixels
    let grid = enforce_outline(&import_result.grid, &palette, '.');
    let grid_string = grid
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(GenerateResult {
        grid,
        grid_string,
        width: tw,
        height: th,
        color_accuracy: import_result.color_accuracy,
        clipped_colors: import_result.clipped_colors,
        generated_size: (gen_w, gen_h),
        reference_png: png_bytes,
        extracted_palette: Some(palette),
        palette_toml: Some(palette_toml),
        detected_pixel_size: import_result.detected_pixel_size,
        native_resolution: import_result.native_resolution,
    })
}

/// Post-process a quantized grid to enforce dark outlines on boundary pixels.
///
/// Any non-void pixel adjacent to a void pixel gets replaced with the darkest
/// non-void color in the palette (the "outline" color). This ensures clean
/// silhouettes even when the AI-generated image has soft/missing edges.
fn enforce_outline(
    grid: &[Vec<char>],
    palette: &pixl_core::types::Palette,
    void_sym: char,
) -> Vec<Vec<char>> {
    if grid.is_empty() {
        return grid.to_vec();
    }
    let h = grid.len();
    let w = grid[0].len();

    // Find the darkest non-void symbol
    let darkest_sym = palette
        .symbols
        .iter()
        .filter(|(sym, _)| **sym != void_sym)
        .min_by_key(|(_, rgba)| {
            (rgba.r as u32 + rgba.g as u32 + rgba.b as u32)
        })
        .map(|(sym, _)| *sym);

    let darkest = match darkest_sym {
        Some(s) => s,
        None => return grid.to_vec(), // No non-void colors
    };

    let mut result = grid.to_vec();

    for y in 0..h {
        for x in 0..w {
            if grid[y][x] == void_sym || grid[y][x] == darkest {
                continue; // Skip void pixels and already-dark pixels
            }
            // Check if this pixel is on the boundary (adjacent to void)
            let neighbors = [(0i32, -1i32), (0, 1), (-1, 0), (1, 0)];
            let on_boundary = neighbors.iter().any(|&(dy, dx)| {
                let ny = y as i32 + dy;
                let nx = x as i32 + dx;
                if ny < 0 || ny >= h as i32 || nx < 0 || nx >= w as i32 {
                    true // Edge of canvas counts as void
                } else {
                    grid[ny as usize][nx as usize] == void_sym
                }
            });
            if on_boundary {
                result[y][x] = darkest;
            }
        }
    }

    result
}

/// Snap a pixel dimension to the nearest valid PAX canvas size.
fn snap_to_canvas_size(n: u32) -> u32 {
    const SIZES: &[u32] = &[8, 16, 24, 32, 48, 64];
    SIZES
        .iter()
        .min_by_key(|&&s| (s as i32 - n as i32).unsigned_abs())
        .copied()
        .unwrap_or(32)
}

pub struct GenerateResult {
    pub grid: Vec<Vec<char>>,
    pub grid_string: String,
    pub width: u32,
    pub height: u32,
    pub color_accuracy: f64,
    pub clipped_colors: u32,
    pub generated_size: (u32, u32),
    pub reference_png: Vec<u8>,
    pub extracted_palette: Option<pixl_core::types::Palette>,
    pub palette_toml: Option<String>,
    /// Detected pixel block size in the AI-generated image.
    pub detected_pixel_size: u32,
    /// Native resolution of the pixel art (before resizing to target).
    pub native_resolution: (u32, u32),
}
