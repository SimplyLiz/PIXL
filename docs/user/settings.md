# Settings & Training

## LLM Providers

Open **Settings** (gear icon in top bar) to configure your AI provider.

### Cloud Providers

| Provider | Models | Key Format |
|----------|--------|------------|
| **Anthropic (Claude)** | Sonnet 4, Haiku 4.5, Opus 4.6 | `sk-ant-...` |
| **OpenAI (GPT)** | GPT-4o, GPT-4o Mini, o3-mini | `sk-...` |
| **Google (Gemini)** | Gemini 2.5 Flash/Pro, 2.0 Flash | `AIza...` |
| **Ollama (Local)** | Any installed model | No key needed |

Click a provider, enter your API key, and save. The model list is fetched dynamically from the provider's API.

### Ollama

For Ollama, configure the endpoint URL (default: `http://localhost:11434`). Studio fetches installed models automatically. You can pull new models directly from the Settings dialog.

### PIXL LoRA (On-Device)

Select "PIXL LoRA (On-Device)" for fully local AI generation using your trained LoRA adapter:

- **Base Model**: `mlx-community/Qwen2.5-3B-Instruct-4bit` (default)
- **LoRA Adapter Path**: path to the adapter directory (e.g. `training/adapters/pixl-lora-v2`)

The engine spawns `mlx_lm.server` automatically on first generation. The engine searches for Python with `mlx-lm` in `training/.venv`, `.venv`, and system Python (in that order). If found, the Settings dialog shows a green checkmark. If not found, an **Install mlx-lm** button appears to set it up automatically.

See [Local Inference Guide](../guides/local-inference.md) for full setup.

## Right Panel Tabs

The right panel has 4 icon tabs:

### Palette Tab
- **Pixel mode**: Color palette grid, foreground/background colors, HSV color picker, layers, canvas size
- **Tilemap mode**: Map size controls (width x height)

### Style Tab
- Theme selection (Dark Fantasy, Light Fantasy, Sci-Fi, Nature, Retro 8-bit, Game Boy)
- Mood (gritty, clean, vibrant, pastel, monochrome)
- Outline style (none, self-outline, drop-shadow, selective)
- Dithering mode (none, Bayer, ordered, selective)

### Generate Tab
- Quick tile generation with text prompt
- Backend connection status
- Knowledge base toggle

### Tiles Tab
- Tile list with inline thumbnails
- Tile preview (click to expand) with 3x3 tiling view
- Edge class display (N/E/S/W)
- Stamps listing + procedural stamp generator (8 patterns: `brick_bond`, `checkerboard`, `diagonal`, `dither_bayer`, `horizontal_stripe`, `dots`, `cross`, `noise`)
- Play Animation button for spriteset tiles (renders animated GIF)
- Edge compatibility checker (select two tiles + direction)
- Validation runner

## Training

Access via the **Training** button (graduation cap icon) in the top bar, or via "Open Training..." in Settings.

### Auto-Learn

When enabled, every tile you **accept** in the chat flow is recorded as training data. This builds a dataset of your curated tiles for LoRA fine-tuning.

- Toggle auto-learn in the Training dialog or Settings
- When active, accept messages show "Saved as training data"
- All data stays on your machine

### Training Data Stats

The Training dialog shows:
- Training pairs count (accepted tiles with grids)
- Total feedback events
- Acceptance rate
- Style scores (average accepted vs rejected)
- Top rejection reasons

### Feedback Insights

View aggregated rejection patterns to understand what the model gets wrong most often. This guides both prompt engineering and retraining.

### Export & Retrain

1. Click **Export Training Data** to generate JSONL
2. Run the training script:
   ```bash
   cd training && ./train_matched.sh
   ```
3. Update the adapter path in Settings > PIXL LoRA

### Adapter Info

When the local inference is configured, the Training dialog shows the current model and adapter path.

## Game Engine Export

The Export menu (download icon in top bar) supports:

| Format | Output |
|--------|--------|
| PNG (4x/8x) | Scaled canvas image |
| PAX Source | Raw .pax TOML file |
| Atlas | Packed spritesheet PNG |
| Tiled | .tmx map + .tsx tileset |
| Godot | Tileset resource files |
| TexturePacker | JSON metadata + spritesheet |
| GB Studio | GB Studio tilemap format |
| Unity | Unity tilemap asset |

For game engine exports, you'll be prompted to select an output directory.

## Keyboard Shortcuts

Press **Cmd+/** to see the full shortcuts overlay. Key ones:

| Shortcut | Action |
|----------|--------|
| `Cmd+S` | Quick save |
| `Cmd+Z` | Undo |
| `Cmd+Shift+Z` | Redo |
| `Cmd+C/V/X` | Copy/Paste/Cut selection |
| `B` | Pencil |
| `E` | Eraser |
| `G` | Fill |
| `I` | Eyedropper |
| `L` | Line |
| `R` | Rectangle |
| `S` | Select |
| `T` | Stamp (tilemap) |
| `H` | Toggle grid |
| `Space+drag` | Pan |
| `Escape` | Deselect |
