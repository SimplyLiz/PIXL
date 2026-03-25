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

## Full PAX Source Generation

If the AI returns a complete PAX TOML document (with `[theme]`, `[[tiles]]` sections), Studio detects this and loads the entire tileset into the session — not just a single tile.

## Local LoRA Model

When "PIXL LoRA (On-Device)" is selected as the LLM provider:
- Generation runs entirely on your machine via `mlx_lm.server`
- Uses the fine-tuned LoRA adapter trained on your accepted tiles
- No API key needed, no cloud calls
- See [Local Inference Guide](../guides/local-inference.md) for setup

## Auto-Learn

When enabled (Settings or Training dialog), every accepted tile is automatically collected as training data. You can later export this data and retrain the LoRA adapter for better results. See [Training](settings.md#training).

## Knowledge Base

The engine includes a built-in pixel art knowledge base (indexed with BM25 search). When generating tiles, relevant knowledge passages are automatically injected into the prompt for better results. Toggle this in the Generate tab.
