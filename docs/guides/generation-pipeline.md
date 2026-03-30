# Sprite Generation Pipeline

How PIXL generates pixel art sprites from text prompts, including the
diffusion bridge, structural quality system, and refinement loop.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Diffusion Bridge](#2-diffusion-bridge)
3. [Structural Validators](#3-structural-validators)
4. [SELF-REFINE Loop](#4-self-refine-loop)
5. [8→16 Upscale Workflow](#5-816-upscale-workflow)
6. [Visual References](#6-visual-references)
7. [Palette Management](#7-palette-management)
8. [MCP Tools Reference](#8-mcp-tools-reference)
9. [CLI Commands](#9-cli-commands)

---

## 1. Overview

PIXL has two paths for generating pixel art sprites:

**Path A — Diffusion Bridge (recommended):**
Text prompt → DALL-E generates reference image → detect pixel grid →
center-sample → auto-extract palette → quantize → structural critique →
optional refinement → optional remap to project palette.

**Path B — LLM Text Grid + Upscale:**
LLM generates 8×8 grid (high accuracy) → upscale to 16×16 → refine
detail → optional composite assembly for 32×32+.

Path A produces better results because image models handle spatial
layout, proportions, and shading natively. Path B is useful when no
image API is available or for very small tiles.

---

## 2. Diffusion Bridge

### How It Works

`pixl_generate_sprite` sends a prompt to DALL-E, receives a 1024×1024
reference image, then converts it to a PAX tile:

1. **Generate** — DALL-E creates pixel art at 1024×1024
2. **Detect pixel grid** — scans for repeating block size (typically 20-40px)
3. **Center-sample** — samples the center pixel of each block (avoids AA artifacts at block edges)
4. **Background removal** — flood-fill from corners strips grey halos
5. **Auto-palette extraction** — median-cut extracts dominant colors, preserving darkest/lightest extremes
6. **Quantize** — each pixel mapped to nearest palette symbol via OKLab distance
7. **AA cleanup** — lone pixels differing from all neighbors snapped to dominant neighbor
8. **Outline enforcement** — light boundary pixels darkened for readable silhouettes
9. **Structural critique** — automated quality checks

### Requirements

- `OPENAI_API_KEY` environment variable
- Optional: `PIXL_IMAGE_MODEL` (defaults to `gpt-image-1`)

### MCP Usage

```
pixl_generate_sprite({
  prompt: "wizard with purple hat and staff",
  name: "wizard",
  // size: "auto" (default) — detects from generated image
  // max_colors: 32 (default)
  // target_palette: "dungeon" (optional — remaps to project palette)
})
```

Returns: preview PNG, reference PNG, PAX grid, palette TOML, structural critique.

### CLI Usage

```bash
pixl generate-sprite tileset.pax \
  --prompt "wizard with purple hat and staff" \
  --name wizard \
  --max-colors 32 \
  --out wizard.png
```

### Palette Integration

The tool always extracts colors from the generated image for maximum
fidelity. To integrate with an existing project palette:

- **At generation time:** pass `target_palette: "your_palette"` to auto-remap
- **After generation:** use `pixl_remap_tile` to remap from auto-palette to project palette
- **Manual:** copy the `palette_toml` from the response into your .pax file

---

## 3. Structural Validators

Module: `pixl-core/src/structural.rs`

Automated quality checks that catch common pixel art defects:

| Validator | What It Checks | Threshold |
|-----------|---------------|-----------|
| **Outline coverage** | % of boundary pixels that are dark | <30% = ERROR, <70% = WARN |
| **Canvas utilization** | % of canvas occupied by bounding box | <25% = ERROR, <40% = WARN |
| **Centering** | Distance from subject center to canvas center | <70% = WARN |
| **Contrast** | Mean OKLab ΔE between adjacent pixels | <0.03 = WARN |
| **Connected components** | Number of disconnected pixel regions | >3 = WARN |
| **Pixel density** | Fraction of non-void pixels | reported, not enforced |

### Severity Levels

- **ERROR** — auto-reject, regenerate the tile
- **WARNING** — flag for refinement pass
- **INFO** — informational, no action needed

### Critique Text

`critique_text()` generates human-readable fix instructions:

```
Tile 'wizard' structural critique:
  [WARN] Only 67% of boundary pixels are dark — outline is incomplete.
         Fill gaps in the silhouette border.
  Metrics: outline=67% center=91% util=66% contrast=0.274 components=2
```

### Refinement Prompts

`refinement_prompt()` generates LLM-targeted fix instructions with
specific row references for use with `pixl_refine_tile`.

---

## 4. SELF-REFINE Loop

The generate → see → fix cycle for iterative quality improvement.

### Protocol (MCP)

```
1. pixl_generate_sprite or pixl_create_tile
   → generates tile, returns preview + critique

2. pixl_critique_tile({name: "wizard"})
   → renders preview, runs structural analysis
   → returns: preview PNG, critique text, refinement_prompt,
     should_refine (bool), should_reject (bool)

3. If should_reject: regenerate
   If should_refine: read the refinement_prompt, fix specific rows

4. pixl_refine_tile({name: "wizard", start_row: 3, rows: "..."})
   → patches rows, re-renders, returns new critique
   → tracks iteration count (max 3)

5. Repeat 2-4 until should_refine=false or iteration count = 3
```

### CLI

```bash
# Critique a tile
pixl critique tileset.pax --tile wizard

# Output:
# pixl critique: 'wizard' (16x16)
#   Outline coverage:    67.0%
#   Centering:           91.0%
#   ✗ Only 67% of boundary pixels are dark...
#   Verdict: REFINE
```

---

## 5. 8→16 Upscale Workflow

For text-grid generation (no image API), LLMs are 85-95% accurate at
8×8 but struggle at 16×16+. The upscale workflow bridges this gap.

### Pipeline

```
1. Generate at 8×8 (64 pixels — LLM gets structure right)
2. pixl_upscale_tile (nearest-neighbor 2× — each pixel becomes 2×2 block)
3. pixl_critique_tile (check structural quality)
4. pixl_refine_tile (add sub-pixel detail: AA, dithering, texture)
```

### MCP Usage

```
pixl_create_tile({name: "potion_8", size: "8x8", grid: "...", palette: "items"})
pixl_upscale_tile({name: "potion_8", factor: 2, new_name: "potion_16"})
pixl_critique_tile({name: "potion_16"})
// Refine the blocky 2×2 regions into detailed pixel art
pixl_refine_tile({name: "potion_16", start_row: 2, rows: "..."})
```

### CLI

```bash
pixl upscale tileset.pax --tile potion_8 --factor 2 --out potion_16.png
```

---

## 6. Visual References

Before generating, show the LLM rendered examples of existing tiles at
the target size. This grounds generation in visual reality.

### MCP Usage

```
pixl_show_references({
  query: "wall",       // search by name/tags
  count: 4,            // max results
  size: "16x16"        // filter by size
})
```

Returns multiple rendered preview PNGs that appear as images in the MCP
response. The LLM sees actual pixel art to match.

### In generate_context

When `pixl_generate_context` is called, accepted few-shot examples are
automatically rendered as preview images alongside their text grids.

---

## 7. Palette Management

### Auto-Palette Extraction

`pixl_generate_sprite` always extracts a palette from the generated
image. This ensures maximum color fidelity — the quantized result
uses colors that actually exist in the reference.

The extracted palette:
- Uses median-cut quantization on non-transparent pixels
- Preserves the actual darkest and lightest colors (not averages)
- Skips semi-transparent halo pixels (alpha < 200)
- Assigns PAX symbols sorted by lightness (dark → light)

### Palette Remapping

To convert auto-palette tiles to a project palette:

```
pixl_remap_tile({
  name: "wizard",
  target_palette: "dungeon"    // project palette name
})
```

Maps each symbol to the perceptually closest symbol in the target
palette using OKLab color distance.

### TOML Output

Every `pixl_generate_sprite` call returns `palette_toml` — a
copy-pasteable palette block:

```toml
[palette.auto]
"." = "#00000000"
"#" = "#1a1020ff"
"a" = "#4a3668ff"
...
```

---

## 8. MCP Tools Reference

### Generation

| Tool | Purpose |
|------|---------|
| `pixl_generate_sprite` | DALL-E → quantize → PAX tile (recommended) |
| `pixl_create_tile` | Create tile from LLM-generated text grid |
| `pixl_upscale_tile` | Nearest-neighbor grid upscale (8→16, 16→32) |

### Quality

| Tool | Purpose |
|------|---------|
| `pixl_critique_tile` | Structural analysis + rendered preview + fix instructions |
| `pixl_refine_tile` | Patch specific rows + re-critique |
| `pixl_check_seams` | Seam continuity across composite tile boundaries |

### References

| Tool | Purpose |
|------|---------|
| `pixl_show_references` | Render matching tiles as visual examples |
| `pixl_generate_context` | Build full generation prompt with rendered few-shot examples |

### Palette

| Tool | Purpose |
|------|---------|
| `pixl_remap_tile` | OKLab nearest-color remap between palettes |

---

## 9. CLI Commands

| Command | Purpose |
|---------|---------|
| `pixl generate-sprite` | DALL-E + quantize pipeline |
| `pixl critique` | Structural quality analysis |
| `pixl upscale` | Grid upscale (8→16, 16→32) |
| `pixl convert` | AI image → pixel art (Lanczos + median-cut) |
| `pixl import` | Reference image → PAX grid with specific palette |
| `pixl validate --check-seams` | Validate with composite seam checking |

### HTTP Endpoints

| Endpoint | Method | Tool |
|----------|--------|------|
| `/api/tile/generate-sprite` | POST | pixl_generate_sprite |
| `/api/tile/critique` | POST | pixl_critique_tile |
| `/api/tile/refine` | POST | pixl_refine_tile |
| `/api/tile/upscale` | POST | pixl_upscale_tile |
| `/api/tile/references` | POST | pixl_show_references |
| `/api/tile/remap` | POST | pixl_remap_tile (via /api/tool) |
