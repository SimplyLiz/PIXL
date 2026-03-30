#!/usr/bin/env python3
"""End-to-end EotB LoRA training: slice → quantize → train.

Prepares Eye of the Beholder tiles as PAX training data and fine-tunes
a LoRA adapter on them. Runs entirely on Apple Silicon via mlx_lm.

Usage:
    # Train on walls only, 3 epochs (~39 min on M4 Pro)
    python train_eotb.py --category walls

    # Train on everything, 5 epochs (~67 min)
    python train_eotb.py --category all --epochs 5

    # Just prepare data, don't train
    python train_eotb.py --prep-only

    # Resume from a checkpoint
    python train_eotb.py --category walls --resume

Categories:
    walls   — 196 wall tiles across 4 dungeon level groups
    floors  — 5 floor/ceiling tiles (very small, best combined with walls)
    all     — everything combined (recommended)
"""

import argparse
import json
import os
import random
import subprocess
import sys
from collections import Counter
from pathlib import Path

from PIL import Image

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
SLICED_DIR = PROJECT_ROOT / "reference" / "eotb-sprites" / "dataset" / "sliced"
METADATA_PATH = PROJECT_ROOT / "reference" / "eotb-sprites" / "dataset" / "metadata.json"

SYMBOL_POOL = ".#+=~gorhwsABCDE"
MODEL_ID = "mlx-community/Qwen2.5-3B-Instruct-4bit"

SYSTEM_PROMPT = """You are a pixel art tile generator. Given a description, output a PAX-format character grid.
Rules:
- Use only the symbols from the palette provided
- Each row must be exactly the specified width
- Total rows must equal the specified height
- '.' means transparent/void
- Output ONLY the grid, no explanation"""

BG_COLORS = {
    (87, 255, 255), (0, 255, 255), (255, 0, 255),
    (0, 128, 128), (0, 0, 0),
}

# ---------------------------------------------------------------------------
# Tile processing
# ---------------------------------------------------------------------------

def load_tile(path: Path, size: int = 16):
    img = Image.open(path).convert("RGBA")
    img = img.resize((size, size), Image.NEAREST)
    return list(img.getdata()), img


def extract_palette(pixels, max_colors=10):
    color_counts = Counter()
    for r, g, b, a in pixels:
        if a < 128 or (r, g, b) in BG_COLORS:
            continue
        qr, qg, qb = (r // 8) * 8, (g // 8) * 8, (b // 8) * 8
        color_counts[(qr, qg, qb)] += 1

    if not color_counts:
        return [(".", (0, 0, 0, 0))]

    top = sorted(color_counts.items(), key=lambda x: -x[1])[:max_colors]
    top_colors = sorted([c for c, _ in top], key=lambda c: sum(c))

    palette = [(".", (0, 0, 0, 0))]
    for i, (r, g, b) in enumerate(top_colors):
        if i + 1 < len(SYMBOL_POOL):
            palette.append((SYMBOL_POOL[i + 1], (r, g, b, 255)))
    return palette


def quantize_tile(pixels, palette, size):
    grid = []
    for y in range(size):
        row = []
        for x in range(size):
            r, g, b, a = pixels[y * size + x]
            if a < 128 or (r, g, b) in BG_COLORS:
                row.append(".")
                continue
            best_sym, best_dist = ".", float("inf")
            for sym, (pr, pg, pb, _) in palette:
                if sym == ".":
                    continue
                d = (r-pr)**2 * 0.30 + (g-pg)**2 * 0.59 + (b-pb)**2 * 0.11
                if d < best_dist:
                    best_dist, best_sym = d, sym
            row.append(best_sym)
        grid.append(row)
    return grid


def grid_to_string(grid):
    return "\n".join("".join(row) for row in grid)


def rotate_90(grid):
    h = len(grid)
    return [[grid[h - 1 - x][y] for x in range(h)] for y in range(len(grid[0]))]


def augment(grid):
    r90 = rotate_90(grid)
    r180 = rotate_90(r90)
    r270 = rotate_90(r180)
    flipped = [row[::-1] for row in grid]
    fr90 = rotate_90(flipped)
    fr180 = rotate_90(fr90)
    fr270 = rotate_90(fr180)
    return [
        (grid, "orig"), (r90, "r90"), (r180, "r180"), (r270, "r270"),
        (flipped, "flip"), (fr90, "flip_r90"), (fr180, "flip_r180"), (fr270, "flip_r270"),
    ]


def compute_features(grid):
    h, w = len(grid), len(grid[0])
    total = h * w
    non_void = sum(1 for row in grid for c in row if c != ".")
    density = non_void / total if total else 0

    h_matches = sum(1 for y in range(h) for x in range(w // 2) if grid[y][x] == grid[y][w-1-x])
    v_matches = sum(1 for y in range(h // 2) for x in range(w) if grid[y][x] == grid[h-1-y][x])
    sym_h = h_matches / max(h * (w // 2), 1)
    sym_v = v_matches / max((h // 2) * w, 1)

    edges, edge_total = 0, 0
    for y in range(h):
        for x in range(w):
            if x < w-1:
                edge_total += 1
                if grid[y][x] != grid[y][x+1]: edges += 1
            if y < h-1:
                edge_total += 1
                if grid[y][x] != grid[y+1][x]: edges += 1

    counts = Counter(c for row in grid for c in row if c != ".")
    return {
        "density": density,
        "symmetry": max(sym_h, sym_v),
        "edge_complexity": edges / max(edge_total, 1),
        "unique_symbols": len(counts),
    }


def classify_tile(filename: str) -> str:
    name = filename.lower()
    if "ceiling" in name:
        return "dungeon ceiling"
    if "floor" in name:
        return "dungeon floor"
    level_map = {
        "01-03": "upper dungeon", "04-06": "mid dungeon",
        "07-09": "lower dungeon", "10-11": "deep dungeon",
    }
    for key, desc in level_map.items():
        if key in name:
            return f"{desc} stone wall"
    return "dungeon wall"


def make_label(features, tile_type, aug_tag):
    parts = ["style:eye-of-the-beholder", f"type:{tile_type}"]

    d = features["density"]
    parts.append(f"density:{'sparse' if d < 0.2 else 'moderate' if d < 0.5 else 'dense' if d < 0.8 else 'solid'}")

    s = features["symmetry"]
    parts.append(f"symmetry:{'high' if s > 0.85 else 'medium' if s > 0.6 else 'low'}")

    e = features["edge_complexity"]
    parts.append(f"detail:{'flat' if e < 0.15 else 'simple' if e < 0.35 else 'moderate' if e < 0.55 else 'complex'}")

    u = features["unique_symbols"]
    parts.append(f"colors:{'minimal' if u <= 2 else 'few' if u <= 4 else 'rich'}")

    if aug_tag != "orig":
        parts.append(f"aug:{aug_tag}")

    return ", ".join(parts)


# ---------------------------------------------------------------------------
# Data preparation
# ---------------------------------------------------------------------------

def prepare_data(category: str, tile_size: int = 16) -> dict[str, Path]:
    """Prepare training data. Returns dict of split_name -> path."""

    if not SLICED_DIR.exists():
        print(f"ERROR: No sliced tiles at {SLICED_DIR}")
        print("Run: python tool/scripts/train_tiles.py --slice-only")
        sys.exit(1)

    # Load metadata to get categories
    with open(METADATA_PATH) as f:
        metadata = json.load(f)

    # Filter tiles by category
    tile_entries = metadata["tiles"]
    if category == "walls":
        tile_entries = [t for t in tile_entries if t["category"] == "wall"]
    elif category == "floors":
        tile_entries = [t for t in tile_entries if t["category"] == "floor_ceiling"]
    # "all" keeps everything

    tile_files = [SLICED_DIR / t["filename"] for t in tile_entries]
    tile_files = [f for f in tile_files if f.exists()]
    print(f"Category '{category}': {len(tile_files)} tiles")

    if len(tile_files) < 5:
        print(f"WARNING: Only {len(tile_files)} tiles — model will likely memorize, not generalize.")

    # Group by source sheet for shared palettes
    groups: dict[str, list[Path]] = {}
    for f in tile_files:
        prefix = f.stem.rsplit("_tile_", 1)[0]
        groups.setdefault(prefix, []).append(f)

    all_pairs = []
    for group_name, files in sorted(groups.items()):
        all_pixels = []
        loaded = []
        for f in files:
            pixels, _ = load_tile(f, tile_size)
            all_pixels.extend(pixels)
            loaded.append((f, pixels))

        palette = extract_palette(all_pixels)
        palette_desc = " ".join(f"'{s}'=({r},{g},{b})" for s, (r, g, b, _) in palette if s != ".")
        print(f"  {group_name}: {len(loaded)} tiles, {len(palette)} colors")

        for tile_path, pixels in loaded:
            grid = quantize_tile(pixels, palette, tile_size)

            non_void = sum(1 for row in grid for c in row if c != ".")
            if non_void < tile_size * tile_size * 0.05:
                continue

            features = compute_features(grid)
            tile_type = classify_tile(tile_path.stem)

            for aug_grid, aug_tag in augment(grid):
                grid_str = grid_to_string(aug_grid)
                label = make_label(features, tile_type, aug_tag)
                user_msg = f"Palette: {palette_desc}\n{label}"
                pair = {
                    "messages": [
                        {"role": "system", "content": SYSTEM_PROMPT},
                        {"role": "user", "content": user_msg},
                        {"role": "assistant", "content": grid_str},
                    ]
                }
                all_pairs.append(pair)

    print(f"\nTotal: {len(all_pairs)} training pairs")

    # Split
    random.seed(42)
    random.shuffle(all_pairs)
    n = len(all_pairs)
    train_end = int(n * 0.9)
    valid_end = int(n * 0.95)

    splits = {
        "train": all_pairs[:train_end],
        "valid": all_pairs[train_end:valid_end],
        "test": all_pairs[valid_end:],
    }

    out_dir = SCRIPT_DIR / f"data_eotb_{category}"
    out_dir.mkdir(parents=True, exist_ok=True)

    paths = {}
    for name, data in splits.items():
        p = out_dir / f"{name}.jsonl"
        with open(p, "w") as f:
            for entry in data:
                f.write(json.dumps(entry) + "\n")
        paths[name] = p
        print(f"  {name}: {len(data)} samples")

    return paths


# ---------------------------------------------------------------------------
# Training
# ---------------------------------------------------------------------------

def train_lora(category: str, epochs: int, resume: bool):
    """Run LoRA fine-tuning via mlx_lm."""

    data_dir = SCRIPT_DIR / f"data_eotb_{category}"
    adapter_dir = SCRIPT_DIR / "adapters" / f"pixl-eotb-{category}"

    if not data_dir.exists():
        print(f"ERROR: No data at {data_dir}. Run with --prep-only first.")
        sys.exit(1)

    # Count training samples
    train_path = data_dir / "train.jsonl"
    with open(train_path) as f:
        train_count = sum(1 for _ in f)

    iters = train_count * epochs
    est_minutes = iters / 2 / 60  # ~2 it/sec on M4 Pro

    print(f"\n{'='*60}")
    print(f"LoRA Training — EotB {category}")
    print(f"{'='*60}")
    print(f"  Model:    {MODEL_ID}")
    print(f"  Data:     {train_count} train samples")
    print(f"  Epochs:   {epochs}")
    print(f"  Iters:    {iters}")
    print(f"  Est time: ~{est_minutes:.0f} min on M4 Pro")
    print(f"  Adapter:  {adapter_dir}")
    if resume:
        print(f"  Resuming from existing adapter")
    print()

    adapter_dir.mkdir(parents=True, exist_ok=True)

    # Use training venv's mlx_lm
    venv_python = SCRIPT_DIR / ".venv" / "bin" / "python"
    if not venv_python.exists():
        print(f"ERROR: Training venv not found at {venv_python}")
        print("Create it: cd training && python3 -m venv .venv && source .venv/bin/activate && pip install mlx-lm")
        sys.exit(1)

    cmd = [
        str(venv_python), "-m", "mlx_lm", "lora",
        "--model", MODEL_ID,
        "--train",
        "--data", str(data_dir),
        "--adapter-path", str(adapter_dir),
        "--fine-tune-type", "lora",
        "--num-layers", "16",
        "--batch-size", "1",
        "--learning-rate", "2e-5",
        "--iters", str(iters),
        "--val-batches", "25",
        "--steps-per-eval", "500",
        "--save-every", "1000",
        "--max-seq-length", "512",
        "--seed", "42",
    ]

    if resume and (adapter_dir / "adapters.safetensors").exists():
        cmd.extend(["--resume-adapter-file", str(adapter_dir / "adapters.safetensors")])

    print(f"Running: {' '.join(cmd[:6])} ...")
    print()

    result = subprocess.run(cmd, cwd=str(SCRIPT_DIR))

    if result.returncode == 0:
        print(f"\n{'='*60}")
        print("Training complete!")
        print(f"{'='*60}")
        print(f"\nAdapter saved to: {adapter_dir}")
        print(f"\nTo use it:")
        print(f"  pixl serve \\")
        print(f"    --model {MODEL_ID} \\")
        print(f"    --adapter {adapter_dir}")
        print(f"\nThen generate tiles with prompts containing 'style:eye-of-the-beholder'")
    else:
        print(f"\nTraining failed with exit code {result.returncode}")
        sys.exit(result.returncode)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="EotB LoRA training pipeline",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Categories:
  walls    196 tiles, 3ep ~39min, 5ep ~65min
  floors     5 tiles, too small alone — use 'all'
  all      201 tiles, 3ep ~40min, 5ep ~67min

Examples:
  python train_eotb.py --category walls              # 3 epochs, ~39 min
  python train_eotb.py --category walls --epochs 5    # 5 epochs, ~65 min
  python train_eotb.py --category all                 # everything, ~40 min
  python train_eotb.py --prep-only --category walls   # just make the data
  python train_eotb.py --category walls --resume      # continue training
""")
    parser.add_argument("--category", choices=["walls", "floors", "all"], default="walls",
                        help="Tile category to train on (default: walls)")
    parser.add_argument("--epochs", type=int, default=3,
                        help="Training epochs (default: 3, ~39min for walls)")
    parser.add_argument("--prep-only", action="store_true",
                        help="Only prepare data, don't train")
    parser.add_argument("--train-only", action="store_true",
                        help="Skip data prep, train on existing data")
    parser.add_argument("--resume", action="store_true",
                        help="Resume training from existing adapter checkpoint")
    parser.add_argument("--tile-size", type=int, default=16,
                        help="Quantize tiles to NxN (default: 16)")
    args = parser.parse_args()

    if not args.train_only:
        print(f"{'='*60}")
        print(f"Preparing data — category: {args.category}")
        print(f"{'='*60}\n")
        prepare_data(args.category, args.tile_size)

    if not args.prep_only:
        train_lora(args.category, args.epochs, args.resume)


if __name__ == "__main__":
    main()
