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
        "Pixel art sprite on a solid black background. Clean 1-pixel dark outline around the entire subject. \
         Limited color palette, flat shading, no anti-aliasing, no gradients. \
         The subject should be centered and fill most of the frame. \
         Style: 16-bit era SNES pixel art. \
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

    Ok(GenerateResult {
        grid: import_result.grid,
        grid_string: import_result.grid_string,
        width: target_width,
        height: target_height,
        color_accuracy: import_result.color_accuracy,
        clipped_colors: import_result.clipped_colors,
        generated_size: (gen_w, gen_h),
        reference_png: png_bytes,
    })
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
}
