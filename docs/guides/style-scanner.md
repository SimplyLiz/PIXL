# Style Scanner — Reference Art → Trained Model Pipeline

## Overview

The Style Scanner is a 4-phase pipeline that lets users scan reference pixel art (from any game or art style), train a LoRA adapter on it, and generate new assets that match that style.

```
SCAN → LEARN → GENERATE → REFINE
  │       │        │          │
  │       │        │          └─ Accept/reject feeds back
  │       │        └─ Generate new tiles with trained adapter
  │       └─ Train LoRA on prepared data
  └─ Import sprites, slice, filter, classify
```

## Phase 1: Scan (`pixl scan`)

Scans reference images (sprite sheets, tilesets, individual sprites) and extracts quality-filtered patches.

### CLI

```bash
# Scan a directory of sprite sheets
pixl scan reference/sprites/ --out my_scan --stride 8

# Scan a single image with custom patch size
pixl scan boss_spritesheet.png --out boss_scan --patch-size 32

# Grid-based tileset (e.g., 32x32 tiles)
pixl scan dcss_tiles.png --out dcss_scan --tile-size 32
```

### What it does

1. **Loads images** — PNG, JPG, BMP, GIF, WebP. Recursively scans directories.
2. **Detects background colors** — Auto-detects cyan, magenta, and other key colors used as transparency in sprite sheets. Also detects the most dominant saturated color.
3. **Slices sprite sheets** — Finds tile boundaries by detecting background-colored gutter rows/columns. Falls back to sliding window for images without clear gutters.
4. **Extracts patches** — Cuts images into NxN patches (default 16x16) with configurable stride for overlap.
5. **Quality filters** — Rejects patches that are:
   - Mostly background (>85% BG pixels)
   - Single-color (< 2 unique colors)
   - Low information (luminance variance < 10)
6. **Auto-classifies** — Labels patches by type (wall, floor, enemy, item, door, etc.) from filenames and visual features.
7. **Saves output** — Writes patches as individual PNGs and a `scan_manifest.json` with per-patch metadata.

### Output

```
my_scan/
├── patches/          # Individual NxN PNGs
│   ├── wall_0000.png
│   ├── wall_0001.png
│   └── ...
└── scan_manifest.json  # Metadata: source, bbox, quality, category
```

### Implementation

- **Rust module:** `tool/crates/pixl-render/src/scan.rs`
- **Key types:** `ScanConfig`, `ScanResult`, `ScanManifest`, `PatchInfo`, `PatchQuality`
- **Key functions:**
  - `scan_image()` — Scan a single image
  - `scan_directory()` — Scan recursively
  - `detect_bg_colors()` — Auto-detect background key colors
  - `find_tile_bboxes()` — Sprite sheet gutter detection
  - `extract_patches()` — Sliding window extraction
  - `assess_patch()` — Quality metrics computation
  - `classify_patch()` — Filename + feature based classification

## Phase 2: Prepare (`pixl prepare`)

Converts scanned patches into LoRA training data.

### CLI

```bash
# Prepare with default settings
pixl prepare my_scan/ --out training/data_custom --palette project.pax

# With augmentation and stratification
pixl prepare my_scan/ --out training/data_custom \
  --palette project.pax --aug 8 --color-aug --max-per-bin 150
```

### What it does

1. **Reads scan manifest** — Loads patches and metadata from Phase 1.
2. **Extracts palettes** — Per-category palette extraction from patch pixel data.
3. **Quantizes to PAX** — Converts RGB patches to PAX character grids using perceptual color matching (OKLab distance).
4. **Computes features** — Density, symmetry, edge complexity, unique symbols per tile.
5. **Generates structured labels** — `style:my-game, type:wall, density:solid, detail:complex, colors:rich`
6. **Augments** — Geometric (4x rotations, optional 8x with flips) + color shifts (warm/cool/dark).
7. **Stratifies** — Bins by density × complexity (5×5 grid), caps per bin for uniform coverage.
8. **Writes JSONL** — `train.jsonl`, `valid.jsonl`, `test.jsonl` in chat format for mlx_lm.

### Output

```
training/data_custom/
├── train.jsonl      # 90% of stratified samples
├── valid.jsonl      # 5%
├── test.jsonl       # 5%
└── dataset_info.json  # Stats: sources, counts, augmentation
```

## Phase 3: Train (`pixl train`)

Fine-tunes a LoRA adapter on the prepared data.

### CLI

```bash
# Train with defaults (3 epochs, ~30-60 min on M4 Pro)
pixl train training/data_custom --adapter training/adapters/my-style

# Custom training
pixl train training/data_custom --adapter training/adapters/my-style \
  --epochs 10 --lr 0.0003 --layers 16
```

### What it does

1. **Finds Python** — Locates a Python environment with `mlx-lm` installed (checks `.venv`, `training/.venv`, system).
2. **Spawns training** — Runs `python -m mlx_lm lora` as a child process with the prepared data.
3. **Streams progress** — Real-time iteration count, loss values, ETA.
4. **Saves adapter** — safetensors format + metadata JSON (source, params, loss history).

### Time estimates (M4 Pro, ~2 it/sec)

| Samples | Epochs | Iters | Time |
|---------|--------|-------|------|
| 1,000 | 3 | 3,000 | ~25 min |
| 2,000 | 3 | 6,000 | ~50 min |
| 2,000 | 5 | 10,000 | ~83 min |
| 2,000 | 10 | 20,000 | ~2.7 hours |

## Phase 4: Generate & Refine

Use the trained adapter to generate new tiles:

```bash
# Start the server with your adapter
pixl serve \
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --adapter training/adapters/my-style \
  --file project.pax

# Generate via MCP
pixl_generate_tile(name: "wall_01", prompt: "stone wall with cracks")
```

The feedback system (`pixl_record_feedback`) captures accept/reject signals that can be used to retrain the adapter with improved data.

### Rejection Sampling

`pixl_generate_tile` automatically retries up to 5 times if the generated grid has fewer than 3 unique symbols (flat/single-color tiles). This catches the most common failure mode of small LoRA adapters. The `attempts` field in the response shows how many tries were needed.

## Convenience Commands

### `pixl retrain` — One-command feedback loop

Takes a .pax file, exports all tiles as training pairs (with 4× rotation augmentation), and trains a new adapter:

```bash
pixl retrain dungeon.pax --adapter training/adapters/retrained --style my-game --epochs 5
```

This replaces the manual `export → prepare → train` cycle. Every tile in the .pax file becomes a training sample.

### `pixl generate-set` — Batch coherent generation

Generates a set of related tiles using the trained adapter:

```bash
# Generate 5 wall variants
pixl generate-set dungeon.pax --set-type walls --theme dark_fantasy --count 5 \
  --adapter training/adapters/my-style --out generated/

# Generate enemy sprites
pixl generate-set dungeon.pax --set-type enemies --theme dark_fantasy --count 3 \
  --out generated/
```

Supported set types: `walls`, `floors`, `enemies`, `items`. Each has themed descriptions that produce coherent variants. Rejection sampling ensures quality (≥3 unique colors per tile).

## Architecture

```
User drops sprite sheets
     │
     ▼
┌──────────┐     ┌───────────┐     ┌──────────┐     ┌──────────┐
│ pixl scan │ ──► │pixl prepare│ ──► │pixl train│ ──► │pixl serve│
│           │     │           │     │          │     │+ adapter │
│ Rust      │     │ Rust      │     │ Python   │     │ Rust     │
│ scan.rs   │     │ prepare.rs│     │ mlx_lm   │     │ inference│
└──────────┘     └───────────┘     └──────────┘     └──────────┘
     │                 │                │                 │
     ▼                 ▼                ▼                 ▼
scan_manifest    train/valid/test   adapters/         pixl_generate_tile
  .json            .jsonl          .safetensors         ──► new tiles
```

## Dataset Management

### Directory structure

Training datasets follow the `training/data_*` naming convention:

```
training/
├── data_eotb/            # Eye of the Beholder reference art
│   ├── train.jsonl
│   ├── valid.jsonl
│   ├── test.jsonl
│   └── dataset_info.json
├── data_eotb_optimal/    # Curated subset of EotB
│   └── ...
├── data_matched/         # Style-matched custom tiles
│   └── ...
└── adapters/             # Trained LoRA adapters
    └── ...
```

### `pixl datasets`

Lists all `data_*` directories that contain a `train.jsonl`. For each, it reads `dataset_info.json` (if present) to display the `style` tag and `source`/`sources` metadata.

### `pixl train --sources` / `--exclude`

When `--sources` is provided, `cmd_train` calls `discover_datasets()` to find all `data_*` directories under the given base path. It filters by the comma-separated suffixes (the part after `data_`), then calls `merge_datasets()` to:

1. Read all `train.jsonl`, `valid.jsonl`, and `test.jsonl` files from the selected directories.
2. Deduplicate by exact line content using a `HashSet`.
3. Write the merged files to a temporary directory.
4. Train on the merged data.

When `--exclude` is provided instead, all discovered datasets are included except the excluded names. Both flags can be combined: `--sources` selects the initial set, `--exclude` removes from it.

Helper functions:
- `discover_datasets(base)` -- finds `data_*` dirs containing `train.jsonl`
- `dataset_suffix(dir)` -- extracts the name after `data_` prefix
- `merge_datasets(dirs)` -- deduplicates and writes to temp dir

## Key Design Decisions

- **Training stays in Python** — mlx_lm requires it. Rust invokes it as a subprocess.
- **Scan manifest as intermediate** — Users can inspect, prune, and re-prepare without re-scanning.
- **Stratified sampling** — MAP-Elites-inspired uniform coverage prevents overfitting on dominant tile types.
- **Per-category palettes** — Each tile category gets its own palette extraction for better quantization.
- **Background auto-detection** — Handles any sprite sheet format (cyan key, magenta key, alpha, etc.).
