# Map Generation Training — TileGPT-Style Pipeline

PIXL includes a data-driven map generation pipeline inspired by [TileGPT](https://tilegpt.github.io/) (Autodesk Research, 2024). It uses MAP-Elites quality-diversity search to synthesize diverse training data, fine-tunes an LLM on tile-name grid prediction, and generates maps via LM + WFC refinement.

## Architecture

```
MAP-Elites (pyribs)           LM (MLX LoRA)              WFC (Rust)
┌─────────────────┐     ┌─────────────────────┐     ┌──────────────┐
│ Search WFC       │     │ Qwen2.5-3B +        │     │ Constraint   │
│ parameter space  │────►│ pixl-mapgen adapter  │────►│ refinement   │
│ → labeled maps   │     │ → coarse tile grid   │     │ → valid map  │
└─────────────────┘     └─────────────────────┘     └──────────────┘
     Phase 1                  Phase 2/3                  Phase 3
```

The key insight from TileGPT: **dataset diversity matters more than size**. MAP-Elites ensures uniform coverage of the feature space (wall ratio × room count), which dramatically outperforms random sampling.

## Prerequisites

```bash
cd training
source .venv/bin/activate
pip install ribs[all]   # MAP-Elites library
```

The base model (`mlx-community/Qwen2.5-3B-Instruct-4bit`) downloads automatically on first use (~2GB).

## Phase 1: Data Synthesis with MAP-Elites

MAP-Elites searches the WFC parameter space (tile weights, seeds, predicates) to fill a 2D archive binned on wall ratio and room count.

```bash
# Dry run — validate one theme works
python map_elites.py --theme dark_fantasy --dry-run

# Generate data for one theme (500 iterations, ~5 min)
python map_elites.py --theme dark_fantasy --iterations 500

# Generate data for all 8 themes (~40 min total)
python map_elites.py --all --iterations 500
```

Output: `data_me/archive_{theme}.jsonl` — one labeled map per archive cell.

### Feature Dimensions

| Axis | Bins | Range | Description |
|------|------|-------|-------------|
| wall_ratio | 20 | 0.05–0.95 | Fraction of obstacle tiles |
| room_count | 10 | 0.5–10.5 | BFS-counted connected walkable regions |

Archive = 200 cells per theme. Typical coverage: 75-80% (150-160 maps per theme).

### Label Format

Each map gets a structured text label:
```
theme:dark_fantasy, size:12x8, layout:open, rooms:few, border:enclosed
```

Labels encode layout density, room count, and border enclosure — the conditioning signal the LM learns.

## Phase 2: Training

### Prepare Training Data

```bash
python prepare_me_data.py
# -> data_me/train.jsonl, valid.jsonl, test.jsonl
```

### Train (Epoch-by-Epoch)

Training runs one epoch at a time so you can pause, test, and resume:

```bash
# Epoch 1 (~40 min on M4 Pro)
bash train_me.sh 1

# Test after epoch 1
.venv/bin/python generate_map.py --prompt "open dungeon" --theme dark_fantasy --no-refine

# Continue if results look good
bash train_me.sh 2
bash train_me.sh 3
# ... up to 5
```

Checkpoints save every 250 iterations. Ctrl+C is safe — resume picks up from the last checkpoint.

### Training Config

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Model | Qwen2.5-3B-Instruct-4bit | Fits 24GB, strong instruction following |
| LoRA rank | 16 | Higher than tile training (8) — maps have more spatial structure |
| LoRA layers | 24 | More layers for spatial reasoning |
| Learning rate | 2e-5 | Standard for domain adaptation |
| Max seq length | 1024 | 16x12 maps need ~500-700 tokens |
| Batch size | 1 | VRAM constraint |

Config file: `lora_config_mapgen.yaml`

## Phase 3: Generation

```bash
# Full pipeline: prompt → LM → WFC refinement → valid map
python generate_map.py --prompt "dark fantasy dungeon, mostly open, 2-3 rooms" \
                       --theme dark_fantasy --out map.png

# Raw LM output (skip WFC)
python generate_map.py --prompt "dense maze" --theme gameboy --no-refine

# Custom size
python generate_map.py --prompt "open field" --theme nature --width 16 --height 12
```

### How Generation Works

1. **Prompt → Label**: Free-text prompt is parsed into the structured label format the model was trained on
2. **LM Inference**: Fine-tuned model generates a tile-name grid (e.g., `floor_stone wall_solid wall_floor_n ...`)
3. **Validation**: Valid tile names become pins; invalid/unknown names are marked for WFC to fill
4. **WFC Refinement**: Pins are fed to the WFC engine which fills remaining cells while enforcing edge compatibility
5. **Graceful Degradation**: If pins contradict, retry with fewer pins; worst case falls back to pure WFC

## CLI Extensions

The narrate subcommand gained three flags to support this pipeline:

```bash
# Override tile weights (for MAP-Elites search)
pixl narrate tileset.pax -w floor_stone:5.0 -w wall_solid:0.5

# Pin specific cells (for LM output → WFC refinement)
pixl narrate tileset.pax --pin 0,0:wall_solid --pin 5,3:floor_stone

# Machine-readable JSON output (for Python pipeline)
pixl narrate tileset.pax --format json
```

## File Reference

```
training/
├── map_elites.py           MAP-Elites QD search
├── me_features.py          Feature computation (BFS rooms, wall ratio)
├── prepare_me_data.py      Archive → training JSONL
├── train_me.sh             Epoch-by-epoch training wrapper
├── lora_config_mapgen.yaml LoRA config for map generation
├── generate_map.py         LM + WFC generation pipeline
├── data_me/                Generated data + training splits
│   ├── archive_*.jsonl     MAP-Elites archives (per theme)
│   ├── train.jsonl         Training split
│   ├── valid.jsonl         Validation split
│   └── test.jsonl          Test split
└── adapters/
    └── pixl-mapgen/        Trained LoRA adapter
```

## Improving the Image Training Pipeline

The existing image-based training (`prepare_matched.py`) was also improved based on TileGPT findings:

- **Rich feature labels** instead of generic descriptions: `tileset:Terrain, density:dense, symmetry:high, detail:simple, colors:few`
- **Visual feature extraction**: density, symmetry (H/V), edge complexity, color distribution
- **Stratified sampling** (`--stratify` flag): bins on density × edge_complexity for uniform coverage

```bash
# Regenerate image training data with rich labels
python prepare_matched.py --stratify

# Train with improved data
bash train_matched.sh
```

## Research Background

This pipeline implements the core ideas from:
- **TileGPT** (Gaier et al., Autodesk 2024): MAP-Elites + LLM + WFC decomposition
- **MarioGPT** (NeurIPS 2023): Fine-tuned GPT-2 on compact level representations
- **PIXL research synthesis** (docs/research/llm_tile_generation.md): Narrate→resolve is the empirically dominant architecture

The key TileGPT finding driving this implementation: MAP-Elites produces uniformly distributed datasets (Gini coefficient ~0) vs. random sampling (Gini ~1), and models trained on MAP-Elites data show superior fidelity and prompt adherence.
