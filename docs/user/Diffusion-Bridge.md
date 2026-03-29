# Diffusion Bridge

The diffusion bridge connects AI image generators (DALL-E, Stable Diffusion, Midjourney) to the PAX pixel art pipeline. Generate a reference image, and PIXL converts it into a clean, palette-constrained tile.

## Pipeline

```
Text prompt
  → Image model generates 1024×1024 reference
  → Detect native pixel grid
  → Center-sample each pixel block
  → Strip background halos
  → Extract palette (or remap to project palette)
  → Quantize to indexed colors
  → Clean up anti-aliasing artifacts
  → Enforce dark outlines
  → Structural quality check
```

## Pixel grid detection

AI generators render "pixel art" at high resolution — each art pixel is a 20-40px block. PIXL scans horizontal and vertical runs of identical pixels to find the actual grid size, then samples the center of each block instead of blurring with traditional downscaling.

## Background removal

Even when asked for transparency, generators often add glowing halos around sprites. PIXL flood-fills from the image corners with color tolerance to detect and strip these opaque-but-unwanted background pixels.

## Auto-palette

Colors are extracted directly from the generated image using median-cut quantization. The actual darkest pixel (for outlines) and lightest pixel (for highlights) are always preserved — median-cut alone would average them away.

Default: 32 colors. Adjustable with `max_colors`.

## Supported image sources

Any PNG or JPEG works as input:

- **DALL-E / GPT Image** — built-in via `OPENAI_API_KEY`
- **Stable Diffusion** — generate externally, import with `pixl convert`
- **Midjourney** — download the image, import with `pixl convert`
- **Any pixel art image** — screenshots, references, existing assets

## CLI usage

```bash
# Generate via DALL-E
pixl generate-sprite tileset.pax \
  --prompt "wizard with staff" \
  --name wizard \
  --out wizard.png

# Import existing image
pixl convert reference.png --width 32 --colors 32
```
