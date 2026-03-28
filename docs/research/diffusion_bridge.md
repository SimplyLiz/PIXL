# Diffusion Import Bridge — Implemented

## Status: Shipped (2026-03-28)

Originally planned as V1.1 feature. Now fully implemented with DALL-E
integration, auto-palette extraction, and multi-stage image processing.

## Implementation

**Module:** `pixl-mcp/src/diffusion.rs` (~165 lines)
**Import pipeline:** `pixl-render/src/import.rs` (~500 lines)
**Palette tools:** `pixl-render/src/pixelize.rs` (extract_palette, remap_grid)

### Full Pipeline

```
Text prompt
  → DALL-E (gpt-image-1) generates 1024×1024 reference
  → Detect native pixel grid (scan for repeating block size)
  → Center-sample each pixel block (avoids AA at block edges)
  → Flood-fill background removal from corners (strips halos)
  → Auto-extract palette via median-cut (preserves darkest/lightest)
  → Quantize each pixel to nearest palette symbol (OKLab distance)
  → AA artifact cleanup (snap lone intermediate pixels to neighbors)
  → Conditional outline enforcement (darken light boundary pixels)
  → Structural quality critique
  → Optional remap to project palette
```

### Key Design Decisions

- **Always auto-palette:** Extracting colors from the generated image
  produces dramatically better results than forcing a session palette.
  Remap to project palette afterward via `pixl_remap_tile`.

- **Center-sampling over Lanczos:** AI pixel art has anti-aliased block
  edges. Center-sampling the middle of each block avoids blending.

- **Background flood-fill:** DALL-E often produces opaque grey halos
  even when asked for transparent backgrounds. Flood-fill from corners
  with color tolerance detects and strips these.

- **Palette extremes:** Median-cut averages within buckets, which can
  lose the actual darkest (outline) and lightest (highlight) pixels.
  Explicitly preserve these in the extracted palette.

### MCP Tool

`pixl_generate_sprite(prompt, name, size?, max_colors?, target_palette?)`

### CLI

`pixl generate-sprite FILE --prompt "..." --name NAME --out OUT`

### Original Research Context

- SD-piXL (2024): logit-per-pixel-per-palette-color formulation
- Our approach (quantization-after-generation) is more practical than
  palette-constrained diffusion — works with any image model

### Honest Limit

Extreme detail at 16×16 loses information during downsampling.
Native resolution detection mitigates this — auto-detect snaps to the
actual pixel art resolution (typically 24-32px) instead of forcing 16×16.
