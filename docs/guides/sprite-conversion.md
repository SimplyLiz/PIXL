# Sprite Conversion & Backdrop Import Guide

Converting AI-generated "pixel art" into true 1:1 pixel art, and building
animated PAX backdrops from the results.

---

## Table of Contents

1. [Why Convert?](#1-why-convert)
2. [The Convert Tool](#2-the-convert-tool)
3. [Backdrop Import Pipeline](#3-backdrop-import-pipeline)
4. [Animation Zones](#4-animation-zones)
5. [Integration Points](#5-integration-points)
6. [Tips for Best Results](#6-tips-for-best-results)

---

## 1. Why Convert?

AI image generators (Midjourney, DALL-E, Stable Diffusion) can produce images
that *look* like pixel art but are not. They ship at high resolution
(512px-1024px+), contain thousands of unique colors, and use anti-aliased edges
that break the pixel grid. This means they cannot be used directly in a
tile-based engine -- they need to be downsampled and palette-quantized to become
real pixel art.

`pixl convert` automates this process: Lanczos downsampling brings the image
down to a true pixel resolution, and palette quantization maps the colors to a
constrained palette using **OKLab perceptual color distance** (not Euclidean
RGB). This ensures the nearest-palette-color mapping aligns with human color
perception — e.g., a dark blue won't incorrectly snap to a perceptually distant
dark green just because their RGB values are close. The result is a clean 1:1
pixel art image ready for PAX import.

---

## 2. The Convert Tool

### 2.1 Three-Preset Mode (Default)

Running `pixl convert` without explicit dimensions produces three output
variants:

| Preset   | Max Width | Colors | Use Case                        |
|----------|-----------|--------|---------------------------------|
| `small`  | 128px     | 16     | Icons, inventory items, UI      |
| `medium` | 160px     | 32     | Character sprites, small scenes |
| `large`  | 256px     | 48     | Backdrops, large tilesets       |

```bash
# Convert a single image -- outputs to ./pixl_convert/
pixl convert image.png

# Custom output directory
pixl convert image.png -o sprites/

# Batch convert every image in a folder
pixl convert images_dir/ -o out/
```

Output structure:

```
pixl_convert/
  originals/          # untouched source copies
  small/              # 128px, 16 colors
  medium/             # 160px, 32 colors
  large/              # 256px, 48 colors
```

Aspect ratio is preserved. The max-width constraint applies to the longer axis;
the shorter axis scales proportionally.

### 2.2 Single-Resolution Mode

When you know exactly what you need:

```bash
# 160px wide, 32-color palette
pixl convert image.png --width 160 --colors 32

# Same, with a 4x nearest-neighbor preview for visual QA
pixl convert image.png --width 160 --colors 32 --preview 4
```

The `--preview` flag generates an additional upscaled PNG using nearest-neighbor
interpolation so you can inspect the pixel grid at a comfortable size without
opening an editor.

### 2.3 Batch Convert

Point `convert` at a directory to process every PNG/JPEG inside it:

```bash
pixl convert ai_renders/ -o converted/
```

Each source image gets its own subfolder under the output directory with the
same three-preset structure.

---

## 3. Backdrop Import Pipeline

Large animated backgrounds (160x240 and up) use the backdrop format extension
in PAX. The typical workflow is: convert the source image, import it as a
tile-decomposed backdrop, render a static proof, then add animation zones.

### Step 1 -- Convert the Source

Start with a high-res AI-generated scene and convert it at the `large` preset
or a custom resolution that matches your target backdrop size:

```bash
pixl convert waterfall_scene.png --width 160 --colors 32
```

### Step 2 -- Import as Backdrop

`backdrop-import` decomposes the converted image into tiles and writes the PAX
sections:

```bash
pixl backdrop-import waterfall_160.png \
  --name moonlit_waterfall \
  --colors 32 \
  --tile-size 16 \
  -o scene.pax
```

This produces:
- A `[palette]` (and `[palette_ext]` if colors > 16)
- One `[backdrop_tile.*]` per unique tile
- A `[backdrop.*]` section with the tilemap layout

### Step 3 -- Render a Static Proof

Verify the decomposition looks correct before adding animation:

```bash
pixl backdrop-render scene.pax --name moonlit_waterfall -o static.png --scale 4
```

### Step 4 -- Add Animation Zones

Edit `scene.pax` to define `[[backdrop.*.zone]]` entries (see section 4 below),
then render an animated proof:

```bash
pixl backdrop-render scene.pax \
  --name moonlit_waterfall \
  -o anim.gif \
  --frames 8 \
  --scale 4
```

---

## 4. Animation Zones

Zones define rectangular regions of a backdrop that animate procedurally at
runtime. Each zone references a behavior and (usually) a `[cycle.*]` palette
rotation.

### 4.1 Behavior Reference

| Behavior       | Effect                                              | Required Fields              |
|----------------|-----------------------------------------------------|------------------------------|
| `cycle`        | Rotate symbol colors per the referenced cycle       | `cycle`                      |
| `wave`         | Cycle with per-row phase offset (water reflections) | `cycle`, `phase_rows`        |
| `flicker`      | Random subset of cycle pixels active per frame      | `cycle`                      |
| `scroll_down`    | Shift pixels downward, wrap at zone boundary        | (none beyond `rect`)         |
| `hscroll_sine`   | SNES HDMA-style per-scanline horizontal distortion  | `amplitude`, `period`        |
| `vscroll_sine`   | Genesis VSRAM-style per-column vertical distortion  | `amplitude`, `period`        |
| `color_gradient` | Per-pixel tint interpolation (atmospheric)          | `from`, `to`                 |
| `palette_ramp`   | Konami raster per-scanline palette replacement      | `symbol`, `from`, `to`       |
| `mosaic`         | GBA-style pixelation with independent X/Y blocks    | `size_x`, `size_y`           |
| `window`         | GBA WIN0/WIN1 layer visibility control              | `layers_visible`             |

### 4.2 Zone Definition

Zones are TOML array-of-tables under the backdrop:

```toml
[[backdrop.moonlit_waterfall.zone]]
name = "water_surface"
rect = { x = 16, y = 144, w = 128, h = 96 }
behavior = "cycle"
cycle = "water_shimmer"

[[backdrop.moonlit_waterfall.zone]]
name = "moon_reflection"
rect = { x = 32, y = 160, w = 32, h = 64 }
behavior = "wave"
cycle = "moonlight_pulse"
phase_rows = 4
```

- `name` -- Human-readable label. Must be unique within the backdrop.
- `rect` -- Pixel rectangle relative to the backdrop's top-left origin.
- `behavior` -- One of the four behavior types above.
- `cycle` -- Name of a `[cycle.*]` section that defines the color rotation.
- `phase_rows` -- For `wave` behavior: number of pixel rows per phase step.

### 4.3 Combining Zones

Zones may overlap. When they do, the last zone in document order wins for any
given pixel. Use this to layer effects -- for example, a broad `cycle` zone for
general water shimmer with a narrower `wave` zone on top for a moonlight
reflection streak.

---

## 5. Integration Points

### MCP

| Tool                    | Arguments                              |
|-------------------------|----------------------------------------|
| `pixl_convert_sprite`   | `input`, `out_dir?`, `width?`, `colors?` |
| `pixl_backdrop_import`  | `input`, `name`, `colors?`, `tile_size?`, `out?` |
| `pixl_backdrop_render`  | `input`, `name`, `out`, `frames?`, `scale?` |

### HTTP API

| Endpoint                   | Method | Description          |
|----------------------------|--------|----------------------|
| `/api/convert`             | POST   | Convert sprite       |
| `/api/backdrop/import`     | POST   | Import backdrop      |
| `/api/backdrop/render`     | POST   | Render backdrop      |

### Studio

The **Convert** dialog (magic wand icon) is accessible from the toolbar.
Browse for an image, pick a preset or enter custom dimensions, and the
converted results appear in the output directory.

The **Backdrop** dialog (landscape icon) provides a two-tab interface:
- **Import tab** — select an image, name the scene, choose palette size, and
  import as a tile-decomposed PAX backdrop.
- **Render tab** — select a .pax file, specify backdrop name, frame count, and
  scale. Renders inline as PNG or animated GIF with a live preview.

---

## 6. Tips for Best Results

**Source image quality matters.** The cleaner the AI output, the better the
conversion. Images with strong, well-defined shapes and limited color variation
convert more faithfully than painterly or noisy images.

**Match palette size to content complexity.** 16 colors is enough for a
single-subject sprite. Scenic backdrops with sky gradients, water, and foliage
typically need 32-48 colors to avoid banding.

**Use `--preview` for quick QA.** A 4x preview catches palette banding and
downsampling artifacts faster than inspecting a 128px PNG at 1:1.

**Tile size affects deduplication.** Smaller tiles (8x8) deduplicate better but
produce more tilemap entries. 16x16 is a good default for backdrops; drop to
8x8 only if you need aggressive size savings.

**Add animation zones after verifying the static render.** Getting the
tile decomposition right first avoids debugging zone rects against a broken
tilemap.

**Keep zone rects tile-aligned.** While the engine supports arbitrary pixel
rects, aligning zone boundaries to the tile grid avoids visual seams in
`scroll_down` behavior and simplifies manual PAX editing.
