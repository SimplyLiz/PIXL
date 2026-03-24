# Local Inference — LoRA-Powered Tile Generation

PIXL supports local AI tile generation using a fine-tuned LoRA adapter served via `mlx_lm.server`. This runs entirely on-device using Apple Silicon's unified memory — no cloud API needed.

## Architecture

```
Studio (Flutter) ──► pixl serve (Rust, HTTP :3742)
                        ├── mlx_lm.server (Python sidecar, :8099)
                        │     └── Qwen2.5-3B-Instruct-4bit + pixl-lora-v2
                        └── Cloud LLMs (Claude, OpenAI, etc.)
```

The Rust server (`pixl-mcp`) manages the `mlx_lm.server` process as a sidecar:
- Spawns it on first `pixl_generate_tile` call
- Health-checks before each request
- Sends the LoRA adapter path per-request (no restart needed to swap adapters)
- Cleans up the process on shutdown

## Prerequisites

```bash
# Python environment with mlx-lm
pip install mlx-lm

# Verify it works
python -m mlx_lm.server --help
```

The base model (`mlx-community/Qwen2.5-3B-Instruct-4bit`, ~2GB) is downloaded automatically on first use.

## Usage

### HTTP Server (Studio integration)

```bash
# With the trained adapter
pixl serve \
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --adapter training/adapters/pixl-lora-v2 \
  --file examples/dungeon.pax

# Custom inference port
pixl serve --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --adapter training/adapters/pixl-lora-v2 \
  --inference-port 9000
```

### MCP Server (Claude Desktop / CLI integration)

```bash
pixl mcp \
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --adapter training/adapters/pixl-lora-v2 \
  --file examples/dungeon.pax
```

### Environment Variables

Instead of CLI flags, you can set:

```bash
export PIXL_MODEL="mlx-community/Qwen2.5-3B-Instruct-4bit"
export PIXL_ADAPTER="training/adapters/pixl-lora-v2"

pixl serve --file examples/dungeon.pax
```

## API

### MCP Tool: `pixl_generate_tile`

Generate a tile from a text description using the local model.

**Arguments:**
| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | yes | — | Tile name for the session |
| `prompt` | string | yes | — | Text description of the tile |
| `size` | string | no | `"16x16"` | Tile dimensions |
| `palette` | string | no | auto-detected | Palette to use |

**Example:**
```json
{
  "name": "mossy_floor",
  "prompt": "a stone floor tile with scattered moss patches",
  "size": "16x16"
}
```

**Response:** Same as `pixl_create_tile`, plus:
```json
{
  "ok": true,
  "generated": true,
  "model": "local-lora",
  "prompt": "a stone floor tile with scattered moss patches",
  "preview_b64": "...",
  "edges": { "n": "open", "e": "open", "s": "open", "w": "open" }
}
```

### HTTP Endpoint

```
POST /api/generate/tile
Content-Type: application/json

{"name": "mossy_floor", "prompt": "stone floor with moss", "size": "16x16"}
```

## How It Works

1. **Context building** — `pixl_generate_tile` calls `pixl_generate_context` internally to build a prompt enriched with:
   - Current palette symbols and theme constraints
   - Style latent from `pixl_learn_style`
   - Edge classes from existing tiles
   - Feedback constraints (learned from accept/reject decisions)
   - Few-shot examples from accepted tiles

2. **Inference** — The enriched prompt is sent to `mlx_lm.server` with the LoRA adapter path. The adapter was trained on 9,864 palette-matched PAX tile pairs.

3. **Grid extraction** — The model's response is parsed to extract the character grid (handles code fences and raw grid formats).

4. **Tile creation** — The grid is passed through the standard `pixl_create_tile` pipeline: grid validation, auto edge classification, preview rendering.

## Training

The adapter at `training/adapters/pixl-lora-v2/` was trained with:

| Parameter | Value |
|-----------|-------|
| Base model | `mlx-community/Qwen2.5-3B-Instruct-4bit` |
| Training data | 8,877 palette-matched pairs with 4x rotation augmentation |
| Learning rate | 2e-5 |
| LoRA layers | 16 |
| Epochs | ~3 (26,631 iterations) |
| Adapter size | ~25MB |

To retrain with updated data:

```bash
cd training
./train_matched.sh
```

The feedback loop (`pixl_record_feedback`) collects accept/reject signals that can be used to prepare new training data for the next adapter version.

## Adapter Management

### Switching adapters

The adapter is specified per-request, so you can test different versions without restarting:

```bash
# Start with v2
pixl serve --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --adapter training/adapters/pixl-lora-v2

# Or without an adapter (base model only)
pixl serve --model mlx-community/Qwen2.5-3B-Instruct-4bit
```

### Merging for distribution

For shipping a standalone model (e.g. via Ollama), merge the adapter:

```bash
python -m mlx_lm fuse \
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --adapter-path training/adapters/pixl-lora-v2 \
  --save-path training/merged/pixl-v2
```

## Performance

On Apple Silicon with a 3B-4bit model:

| Chip | Tokens/sec | 16x16 tile (~80 tokens) |
|------|-----------|------------------------|
| M2 Pro | ~40 | ~2s |
| M4 Pro | ~60-80 | ~1-1.3s |

The adapter (~25MB) loads into unified memory with negligible overhead.

## Without Local Inference

If `--model` is not specified, `pixl_generate_tile` returns an error with setup instructions. All other tools work normally — you can still use `pixl_generate_context` to build prompts for cloud LLMs.
