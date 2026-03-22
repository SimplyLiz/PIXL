# Diffusion Import Bridge Research — V1.1 Feature

## Concept

Two-stage pipeline:
1. Diffusion model (FLUX.2, SD3.5 + pixel art LoRA) generates reference image
2. PAX quantizes reference into project palette + LLM refines

## Quantization Pipeline

1. Downscale with Lanczos3 (preserves edges better than bilinear)
2. For each pixel: find nearest palette color by perceptual weighted distance
   - Weighted RGB: dr*dr*30 + dg*dg*59 + db*db*11 (luminance-weighted)
3. Optional Bayer dithering for smoother gradients
4. LLM examines 16x preview, fixes: outline cleanup, light correction,
   palette discipline, detail sharpening

## Implementation

~200 lines in pixl-render/src/import.rs
Dependencies: image crate (already included) for resize + pixel access

## MCP Tool

pixl.import_reference(reference_b64, target_size, palette, dither_mode)
Returns: quantized_grid, preview_b64, color_accuracy score

## Honest Limit

Extreme detail density (faces with individual pixel features at 16x16) loses
information during quantization that only artistic judgment can restore.
Information-theoretic constraint, not format limitation.

## Status: V1.1 feature
