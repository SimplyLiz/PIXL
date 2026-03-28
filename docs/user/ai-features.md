# AI Features

PIXL Studio integrates AI at every step — from generating tiles to refining style to auto-tagging. All AI features work through the chat panel.

## Tile Generation

Type a description in the chat to generate a tile:

```
generate a 16x16 dungeon wall tile
create a mossy stone floor
draw me a treasure chest
```

The system enriches your prompt with palette constraints, theme context, style latent, edge compatibility info, and feedback from your accept/reject history.

### Accept / Reject Flow

After generation, you see a preview with three options:
- **Accept** — keeps the tile, records positive feedback
- **Reject** — opens a reason picker (too sparse, too dense, wrong style, bad edges, palette issue, bad composition), records negative feedback
- **Variations** — generates 3 alternative versions to choose from

Feedback improves future generations: accepted tiles refine the style latent, rejected patterns become constraints.

## AI Chat Commands

Beyond generation, these special commands trigger focused AI operations:

### Make it Tile
```
make it tile
fix edges
make tileable
```
Takes the current/last tile and adjusts border pixels for seamless horizontal and vertical repetition. Interior design stays intact. Creates a new `_tileable` version.

### Style Transfer
```
restyle to Game Boy
make this look like sci-fi
shift to warmer palette
```
Restyles the current tile to match a different aesthetic while using the same palette symbols. Creates a `_restyled` version.

### Inpaint Region
```
inpaint with moss patches
fill this with water ripples
```
**Requires an active selection** (use Select tool, `S`). Replaces the pixels in the selected region with the described content. Everything outside the selection stays unchanged. Creates an `_inpainted` version.

### Auto-Tag
```
auto-tag
tag all tiles
```
AI analyzes all tiles in the session and suggests semantic tags (wall, floor, corner, decoration), target layers, and descriptions. Useful for organizing a tileset before export.

## Generate Tileset

Use the **Generate Tilegroup** button in the top bar to batch-generate a complete autotile set (up to 16 variants: solid, corners, edges, transitions). Enter a base name and description, and the AI generates each variant with edge compatibility.

## Tileset Completeness Analyzer

After generating tiles, use the **Validate** button in the Tiles tab (or `pixl validate --completeness` from the CLI) to analyze edge class connectivity. The analyzer reports:

- **Disconnected tile pairs** — edges that can't connect to anything
- **Missing transition tiles** — specific tile types needed to bridge incompatible edges (e.g., "need a tile with N=wall, S=floor")
- **Coverage score** — percentage of possible edge combinations that have valid tile placements

## Transition Tile Generator

When the completeness analyzer identifies gaps, use the **Generate Transition** button (or the `pixl_generate_transition_context` MCP tool) to create knowledge-enriched prompts for the missing transition tiles. The system pre-fills edge constraints and palette context so the AI generates tiles that fit the gap.

## Full PAX Source Generation

If the AI returns a complete PAX TOML document (with `[theme]`, `[[tiles]]` sections), Studio detects this and loads the entire tileset into the session — not just a single tile.

## Local LoRA Model

When "PIXL LoRA (On-Device)" is selected as the LLM provider:
- Generation runs entirely on your machine via `mlx_lm.server`
- Uses the fine-tuned LoRA adapter trained on your accepted tiles
- No API key needed, no cloud calls
- The engine auto-detects `mlx-lm` in `training/.venv`, `.venv`, or system Python
- If `mlx-lm` isn't found, the Settings dialog shows an **Install mlx-lm** button
- See [Local Inference Guide](../guides/local-inference.md) for setup

## Auto-Learn

When enabled (Settings or Training dialog), every accepted tile is automatically collected as training data. You can later export this data and retrain the LoRA adapter for better results. See [Training](settings.md#training).

## Knowledge Base

The engine includes a built-in pixel art knowledge base (indexed with BM25 search). When generating tiles, relevant knowledge passages are automatically injected into the prompt for better results. Toggle this in the Generate tab.

## OKLab Color Space

Image import and style matching use **OKLab** perceptual color distance instead of raw RGB. This means:
- **Sprite conversion** (`pixl convert`) quantizes images to the nearest palette color using perceptual similarity, not Euclidean RGB distance
- **Style latent extraction** (`pixl learn-style`) measures lightness and hue in OKLab space, producing more accurate style fingerprints
- The result is better palette matching that aligns with how humans perceive color differences

## CLI: `pixl new --generate`

The `--generate` flag on `pixl new` loads the knowledge base and generates enriched system/user prompts for each tile type in the template. You can pipe these directly to any LLM for automated tileset generation:

```bash
pixl new --theme dark_fantasy --generate | your-llm-cli
```
