#!/usr/bin/env python3
"""Prepare Eye of the Beholder tiles for LoRA training.

Takes the sliced EotB wall/ceiling tiles, quantizes them to PAX format,
computes features, augments, and produces training data that can be
merged with existing data or used standalone.

Usage:
    # Prepare EotB data only
    python prepare_eotb.py

    # Merge with existing GameTileNet data and retrain
    python prepare_eotb.py --merge

    # Custom tile size (default 16x16)
    python prepare_eotb.py --size 16
"""

import json
import math
import os
import random
from collections import Counter
from pathlib import Path

from PIL import Image

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
SLICED_DIR = PROJECT_ROOT / "reference" / "eotb-sprites" / "dataset" / "sliced"
OUTPUT_DIR = SCRIPT_DIR / "data_eotb"
MERGED_DIR = SCRIPT_DIR / "data_merged"
EXISTING_DIR = SCRIPT_DIR / "data_matched"

SYMBOL_POOL = ".#+=~gorhwsABCDE"

SYSTEM_PROMPT = """You are a pixel art tile generator. Given a description, output a PAX-format character grid.
Rules:
- Use only the symbols from the palette provided
- Each row must be exactly the specified width
- Total rows must equal the specified height
- '.' means transparent/void
- Output ONLY the grid, no explanation"""

# EotB background colors to treat as transparent
BG_COLORS = {
    (87, 255, 255),
    (0, 255, 255),
    (255, 0, 255),
    (0, 128, 128),
    (0, 0, 0),  # black background in sliced tiles
}


def load_tile(path: Path, size: int = 16) -> tuple[list, Image.Image]:
    """Load a sliced tile, resize to target, return RGBA pixels."""
    img = Image.open(path).convert("RGBA")
    img = img.resize((size, size), Image.NEAREST)  # nearest-neighbor for pixel art
    return list(img.getdata()), img


def extract_palette_eotb(pixels: list, max_colors: int = 10) -> list:
    """Extract palette from EotB tile pixels, filtering BG colors."""
    color_counts = Counter()
    for r, g, b, a in pixels:
        if a < 128:
            continue
        if (r, g, b) in BG_COLORS:
            continue
        # Quantize to reduce noise
        qr = (r // 8) * 8  # finer quantization for EotB's rich palette
        qg = (g // 8) * 8
        qb = (b // 8) * 8
        color_counts[(qr, qg, qb)] += 1

    if not color_counts:
        return [(".", (0, 0, 0, 0))]

    top = sorted(color_counts.items(), key=lambda x: -x[1])[:max_colors]
    top_colors = sorted([c for c, _ in top], key=lambda c: c[0] + c[1] + c[2])

    palette = [(".", (0, 0, 0, 0))]
    for i, (r, g, b) in enumerate(top_colors):
        if i + 1 < len(SYMBOL_POOL):
            palette.append((SYMBOL_POOL[i + 1], (r, g, b, 255)))

    return palette


def quantize_tile(pixels: list, palette: list, size: int) -> list[list[str]]:
    """Quantize pixel data to nearest palette color."""
    grid = []
    for y in range(size):
        row = []
        for x in range(size):
            r, g, b, a = pixels[y * size + x]

            if a < 128 or (r, g, b) in BG_COLORS:
                row.append(".")
                continue

            best_sym = "."
            best_dist = float("inf")
            for sym, (pr, pg, pb, pa) in palette:
                if sym == ".":
                    continue
                dr = r - pr
                dg = g - pg
                db = b - pb
                d = dr * dr * 0.30 + dg * dg * 0.59 + db * db * 0.11
                if d < best_dist:
                    best_dist = d
                    best_sym = sym

            row.append(best_sym)
        grid.append(row)
    return grid


def grid_to_string(grid: list[list[str]]) -> str:
    return "\n".join("".join(row) for row in grid)


def rotate_90(grid):
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    return [[grid[h - 1 - x][y] for x in range(h)] for y in range(w)]


def flip_h(grid):
    return [row[::-1] for row in grid]


def augment(grid):
    """8x augmentation: 4 rotations × 2 flips."""
    r90 = rotate_90(grid)
    r180 = rotate_90(r90)
    r270 = rotate_90(r180)
    variants = [grid, r90, r180, r270]
    flipped = [flip_h(g) for g in variants]
    return variants + flipped


def compute_tile_features(grid):
    """Compute visual features from a character grid."""
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    total = h * w
    if total == 0:
        return {}

    non_void = sum(1 for row in grid for c in row if c != ".")
    density = non_void / total

    h_matches = 0
    v_matches = 0
    for y in range(h):
        for x in range(w // 2):
            if grid[y][x] == grid[y][w - 1 - x]:
                h_matches += 1
    for y in range(h // 2):
        for x in range(w):
            if grid[y][x] == grid[h - 1 - y][x]:
                v_matches += 1
    half_cells = (h * (w // 2)) or 1
    symmetry_h = h_matches / half_cells
    half_cells_v = ((h // 2) * w) or 1
    symmetry_v = v_matches / half_cells_v

    edges = 0
    edge_total = 0
    for y in range(h):
        for x in range(w):
            if x < w - 1:
                edge_total += 1
                if grid[y][x] != grid[y][x + 1]:
                    edges += 1
            if y < h - 1:
                edge_total += 1
                if grid[y][x] != grid[y + 1][x]:
                    edges += 1
    edge_complexity = edges / edge_total if edge_total else 0

    counts = Counter(c for row in grid for c in row if c != ".")
    unique_symbols = len(counts)
    if counts:
        dominant_sym, dominant_count = counts.most_common(1)[0]
        dominant_ratio = dominant_count / non_void if non_void else 0
    else:
        dominant_sym = "."
        dominant_ratio = 1.0

    return {
        "density": density,
        "symmetry_h": symmetry_h,
        "symmetry_v": symmetry_v,
        "edge_complexity": edge_complexity,
        "unique_symbols": unique_symbols,
        "dominant_symbol": dominant_sym,
        "dominant_ratio": dominant_ratio,
    }


def classify_tile(filename: str) -> str:
    """Derive a descriptive label from the EotB tile filename."""
    name = filename.lower()
    if "floor" in name or "ceiling" in name:
        if "ceiling" in name:
            return "dungeon ceiling"
        return "dungeon floor"
    # Extract level range
    level = ""
    if "01-03" in name:
        level = "upper dungeon"
    elif "04-06" in name:
        level = "middle dungeon"
    elif "07-09" in name:
        level = "lower dungeon"
    elif "10-11" in name:
        level = "deep dungeon"
    return f"{level} stone wall".strip()


def features_to_label(features: dict, tile_type: str, aug_label: str = "") -> str:
    """Build structured prompt label from features."""
    parts = [f"style:eye-of-the-beholder", f"type:{tile_type}"]

    d = features.get("density", 0)
    if d < 0.2:
        parts.append("density:sparse")
    elif d < 0.5:
        parts.append("density:moderate")
    elif d < 0.8:
        parts.append("density:dense")
    else:
        parts.append("density:solid")

    sym = max(features.get("symmetry_h", 0), features.get("symmetry_v", 0))
    if sym > 0.85:
        parts.append("symmetry:high")
    elif sym > 0.6:
        parts.append("symmetry:medium")
    else:
        parts.append("symmetry:low")

    ec = features.get("edge_complexity", 0)
    if ec < 0.15:
        parts.append("detail:flat")
    elif ec < 0.35:
        parts.append("detail:simple")
    elif ec < 0.55:
        parts.append("detail:moderate")
    else:
        parts.append("detail:complex")

    uc = features.get("unique_symbols", 0)
    if uc <= 2:
        parts.append("colors:minimal")
    elif uc <= 4:
        parts.append("colors:few")
    else:
        parts.append("colors:rich")

    if aug_label:
        parts.append(f"aug:{aug_label}")

    return ", ".join(parts)


def to_chat(desc: str, grid_str: str, palette_desc: str) -> dict:
    """Convert to chat training format."""
    user_msg = f"Palette: {palette_desc}\n{desc}"
    return {
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_msg},
            {"role": "assistant", "content": grid_str},
        ]
    }


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Prepare EotB tiles for LoRA training")
    parser.add_argument("--size", type=int, default=16, help="Tile size (default: 16)")
    parser.add_argument("--merge", action="store_true", help="Merge with existing data_matched")
    parser.add_argument("--max-colors", type=int, default=10, help="Max palette colors per group")
    args = parser.parse_args()

    if not SLICED_DIR.exists():
        print(f"ERROR: No sliced tiles at {SLICED_DIR}")
        print("Run train_tiles.py --slice-only first.")
        return

    tile_files = sorted(SLICED_DIR.glob("*.png"))
    print(f"Found {len(tile_files)} sliced EotB tiles")

    # Group tiles by source sheet (shared palette per group)
    groups: dict[str, list[Path]] = {}
    for f in tile_files:
        # Group by prefix before _tile_
        prefix = f.stem.rsplit("_tile_", 1)[0]
        groups.setdefault(prefix, []).append(f)

    print(f"Tile groups: {list(groups.keys())}")

    all_pairs = []
    aug_labels = ["orig", "r90", "r180", "r270", "flip", "flip_r90", "flip_r180", "flip_r270"]

    for group_name, files in sorted(groups.items()):
        # Collect all pixels from this group to build shared palette
        all_pixels = []
        loaded = []
        for f in files:
            pixels, img = load_tile(f, args.size)
            all_pixels.extend(pixels)
            loaded.append((f, pixels))

        palette = extract_palette_eotb(all_pixels, max_colors=args.max_colors)
        palette_desc = " ".join(
            f"'{s}'=({r},{g},{b})" for s, (r, g, b, a) in palette if s != "."
        )
        print(f"  {group_name}: {len(loaded)} tiles, {len(palette)} palette colors")

        for tile_path, pixels in loaded:
            grid = quantize_tile(pixels, palette, args.size)

            # Skip mostly-void tiles
            non_void = sum(1 for row in grid for c in row if c != ".")
            if non_void < args.size * args.size * 0.05:
                continue

            features = compute_tile_features(grid)
            tile_type = classify_tile(tile_path.stem)

            # 8x augmentation
            for i, aug_grid in enumerate(augment(grid)):
                grid_str = grid_to_string(aug_grid)
                label = features_to_label(features, tile_type, aug_labels[i])
                pair = to_chat(label, grid_str, palette_desc)
                all_pairs.append((pair, features))

    print(f"\nTotal: {len(all_pairs)} training pairs (after 8x augmentation)")

    # Write EotB-only dataset
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    chat_data = [pair for pair, _ in all_pairs]

    random.seed(42)
    random.shuffle(chat_data)

    n = len(chat_data)
    train_end = int(n * 0.9)
    valid_end = int(n * 0.95)

    splits = {
        "train": chat_data[:train_end],
        "valid": chat_data[train_end:valid_end],
        "test": chat_data[valid_end:],
    }

    for name, data in splits.items():
        path = OUTPUT_DIR / f"{name}.jsonl"
        with open(path, "w") as f:
            for entry in data:
                f.write(json.dumps(entry) + "\n")
        print(f"  {name}: {len(data)} -> {path}")

    # Show sample
    sample = chat_data[0]
    print(f"\nSample prompt:\n  {sample['messages'][1]['content'][:200]}")
    print(f"\nSample grid:\n  {sample['messages'][2]['content'][:200]}")

    # Merge with existing data if requested
    if args.merge:
        print(f"\n{'='*60}")
        print("Merging with existing training data...")
        print(f"{'='*60}")

        if not EXISTING_DIR.exists():
            print(f"WARNING: No existing data at {EXISTING_DIR}, writing EotB-only")
            merged_data = chat_data
        else:
            existing = []
            for split_file in ["train.jsonl", "valid.jsonl", "test.jsonl"]:
                p = EXISTING_DIR / split_file
                if p.exists():
                    with open(p) as f:
                        for line in f:
                            existing.append(json.loads(line))
            print(f"  Existing data: {len(existing)} samples")
            print(f"  EotB data:     {len(chat_data)} samples")

            merged_data = existing + chat_data
            random.seed(42)
            random.shuffle(merged_data)
            print(f"  Merged total:  {len(merged_data)} samples")

        MERGED_DIR.mkdir(parents=True, exist_ok=True)
        mn = len(merged_data)
        mt_end = int(mn * 0.9)
        mv_end = int(mn * 0.95)

        merged_splits = {
            "train": merged_data[:mt_end],
            "valid": merged_data[mt_end:mv_end],
            "test": merged_data[mv_end:],
        }

        for name, data in merged_splits.items():
            path = MERGED_DIR / f"{name}.jsonl"
            with open(path, "w") as f:
                for entry in data:
                    f.write(json.dumps(entry) + "\n")
            print(f"  {name}: {len(data)} -> {path}")

    # Print training command
    data_dir = "data_merged" if args.merge else "data_eotb"
    total_train = splits["train"] if not args.merge else merged_splits["train"]
    iters = len(total_train) * 3  # 3 epochs

    print(f"\n{'='*60}")
    print("To train the LoRA adapter:")
    print(f"{'='*60}")
    print(f"""
cd {SCRIPT_DIR}
source .venv/bin/activate

python -m mlx_lm lora \\
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \\
  --train \\
  --data {data_dir} \\
  --adapter-path adapters/pixl-lora-v3 \\
  --fine-tune-type lora \\
  --num-layers 16 \\
  --batch-size 1 \\
  --learning-rate 2e-5 \\
  --iters {iters} \\
  --val-batches 25 \\
  --steps-per-eval 500 \\
  --save-every 2000 \\
  --max-seq-length 512 \\
  --seed 42
""")


if __name__ == "__main__":
    main()
