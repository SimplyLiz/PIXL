#!/usr/bin/env python3
"""Optimal ML data preparation for Eye of the Beholder tiles.

Extracts maximum training signal from the sliced sprite sheets:

1. Patch extraction — cut large tiles into 16x16 patches (stride-8 overlap)
2. Quality filtering — drop empty, single-color, and low-information patches
3. Per-group palette extraction with finer quantization
4. Color augmentation — warm/cool shifts, brightness, constrained to palette
5. Geometric augmentation — rotations + flips (8x)
6. Feature-stratified sampling — uniform coverage across density × complexity bins
7. Metadata export — full per-patch annotations for downstream ML

Output:
  training/data_eotb_optimal/  — train/valid/test JSONL (LoRA-ready)
  training/data_eotb_optimal/patches/  — raw 16x16 PNGs (for CNN/VAE/diffusion)
  training/data_eotb_optimal/metadata.json  — full dataset catalog

Usage:
    python prepare_eotb_optimal.py                    # default (stride-8, 4x aug)
    python prepare_eotb_optimal.py --stride 16        # non-overlapping only
    python prepare_eotb_optimal.py --aug 8            # full 8x augmentation
    python prepare_eotb_optimal.py --max-per-bin 100  # cap per stratification bin
"""

import argparse
import json
import math
import os
import random
from collections import Counter
from pathlib import Path

import numpy as np
from PIL import Image

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
REF_DIR = PROJECT_ROOT / "reference" / "eotb-sprites"
SLICED_DIR = REF_DIR / "dataset" / "sliced"
METADATA_PATH = REF_DIR / "dataset" / "metadata.json"
OUTPUT_DIR = SCRIPT_DIR / "data_eotb_optimal"
PATCH_DIR = OUTPUT_DIR / "patches"

SYMBOL_POOL = ".#+=~gorhwsABCDE"
PATCH_SIZE = 16

# All data sources with their style tags and licenses
SOURCES = [
    # (directory, glob_pattern, style_tag, tile_size_hint, license)
    (REF_DIR / "dataset" / "sliced", "*.png", "eye-of-the-beholder", None, "reference"),
    (REF_DIR / "tiles" / "first person dungeon crawl tiles", "*.png", "heroine-dusk-dungeon", None, "CC-BY-SA-3.0"),
    (REF_DIR / "enemies" / "first person dungeon crawl enemies", "*.png", "heroine-dusk-enemy", None, "CC-BY-SA-3.0"),
    (REF_DIR / "open-source" / "industrial", "*.png", "industrial-dungeon", None, "CC-BY-SA-3.0"),
    (REF_DIR / "open-source" / "dungeon-crawl-more", "**/*.png", "dungeon-variant", None, "CC-BY-SA-3.0"),
    (REF_DIR / "open-source" / "dungeon-16x16", "*.png", "dungeon-16x16", 16, "CC-BY-3.0"),
    (REF_DIR / "open-source" / "puny-dungeon", "*.png", "puny-dungeon", 16, "CC0"),
    (REF_DIR / "open-source" / "dcss-32x32", "*.png", "dcss", 32, "CC0"),
]

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
# Step 1: Patch extraction
# ---------------------------------------------------------------------------

def is_bg(r, g, b):
    return (r, g, b) in BG_COLORS


def extract_patches(img: Image.Image, stride: int = 8) -> list[tuple[Image.Image, int, int]]:
    """Extract 16x16 patches from an image with given stride.
    Returns list of (patch_image, x_offset, y_offset)."""
    arr = np.array(img.convert("RGB"))
    h, w = arr.shape[:2]

    patches = []

    if w < PATCH_SIZE or h < PATCH_SIZE:
        # Too small — resize to 16x16 directly
        resized = img.convert("RGB").resize((PATCH_SIZE, PATCH_SIZE), Image.NEAREST)
        patches.append((resized, 0, 0))
        return patches

    for y in range(0, h - PATCH_SIZE + 1, stride):
        for x in range(0, w - PATCH_SIZE + 1, stride):
            patch = img.crop((x, y, x + PATCH_SIZE, y + PATCH_SIZE)).convert("RGB")
            patches.append((patch, x, y))

    return patches


# ---------------------------------------------------------------------------
# Step 2: Quality filtering
# ---------------------------------------------------------------------------

def patch_quality(patch_img: Image.Image) -> dict:
    """Compute quality metrics for a patch. Returns dict with scores."""
    arr = np.array(patch_img)
    total = PATCH_SIZE * PATCH_SIZE

    # Count background pixels
    bg_count = 0
    for y in range(PATCH_SIZE):
        for x in range(PATCH_SIZE):
            r, g, b = arr[y, x]
            if is_bg(int(r), int(g), int(b)):
                bg_count += 1

    bg_ratio = bg_count / total

    # Unique colors (non-bg)
    colors = set()
    for y in range(PATCH_SIZE):
        for x in range(PATCH_SIZE):
            r, g, b = arr[y, x]
            if not is_bg(int(r), int(g), int(b)):
                colors.add((int(r) // 8, int(g) // 8, int(b) // 8))

    # Edge density (transitions between different colors)
    edges = 0
    edge_total = 0
    for y in range(PATCH_SIZE):
        for x in range(PATCH_SIZE):
            if x < PATCH_SIZE - 1:
                edge_total += 1
                if not np.array_equal(arr[y, x], arr[y, x + 1]):
                    edges += 1
            if y < PATCH_SIZE - 1:
                edge_total += 1
                if not np.array_equal(arr[y, x], arr[y + 1, x]):
                    edges += 1

    edge_density = edges / max(edge_total, 1)

    # Luminance variance (information content)
    lum = 0.299 * arr[:, :, 0].astype(float) + 0.587 * arr[:, :, 1].astype(float) + 0.114 * arr[:, :, 2].astype(float)
    lum_var = float(np.var(lum))

    return {
        "bg_ratio": bg_ratio,
        "unique_colors": len(colors),
        "edge_density": edge_density,
        "lum_variance": lum_var,
    }


def passes_quality(quality: dict, min_colors: int = 2, max_bg: float = 0.85, min_variance: float = 10.0) -> bool:
    """Filter out low-information patches."""
    if quality["bg_ratio"] > max_bg:
        return False
    if quality["unique_colors"] < min_colors:
        return False
    if quality["lum_variance"] < min_variance:
        return False
    return True


# ---------------------------------------------------------------------------
# Step 3: Palette & quantization
# ---------------------------------------------------------------------------

def extract_palette(pixels_list: list[np.ndarray], max_colors: int = 10) -> list:
    """Extract shared palette from a group of patch arrays."""
    color_counts = Counter()
    for arr in pixels_list:
        for y in range(arr.shape[0]):
            for x in range(arr.shape[1]):
                r, g, b = int(arr[y, x, 0]), int(arr[y, x, 1]), int(arr[y, x, 2])
                if is_bg(r, g, b):
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


def quantize_array(arr: np.ndarray, palette: list) -> list[list[str]]:
    """Quantize a 16x16 RGB array to PAX character grid."""
    grid = []
    for y in range(arr.shape[0]):
        row = []
        for x in range(arr.shape[1]):
            r, g, b = int(arr[y, x, 0]), int(arr[y, x, 1]), int(arr[y, x, 2])
            if is_bg(r, g, b):
                row.append(".")
                continue
            best_sym, best_dist = ".", float("inf")
            for sym, (pr, pg, pb, _) in palette:
                if sym == ".":
                    continue
                d = (r - pr) ** 2 * 0.30 + (g - pg) ** 2 * 0.59 + (b - pb) ** 2 * 0.11
                if d < best_dist:
                    best_dist, best_sym = d, sym
            row.append(best_sym)
        grid.append(row)
    return grid


# ---------------------------------------------------------------------------
# Step 4: Augmentation
# ---------------------------------------------------------------------------

def rotate_90(grid):
    h = len(grid)
    return [[grid[h - 1 - x][y] for x in range(h)] for y in range(len(grid[0]))]


def augment_grid(grid, aug_level: int = 4):
    """Augment a grid. aug_level=4: rotations only. aug_level=8: + flips."""
    r90 = rotate_90(grid)
    r180 = rotate_90(r90)
    r270 = rotate_90(r180)
    variants = [
        (grid, "orig"), (r90, "r90"), (r180, "r180"), (r270, "r270"),
    ]
    if aug_level >= 8:
        flipped = [row[::-1] for row in grid]
        fr90 = rotate_90(flipped)
        fr180 = rotate_90(fr90)
        fr270 = rotate_90(fr180)
        variants.extend([
            (flipped, "flip"), (fr90, "flip_r90"), (fr180, "flip_r180"), (fr270, "flip_r270"),
        ])
    return variants


def color_shift_grid(grid, palette, shift_name: str) -> tuple[list[list[str]], list, str]:
    """Create a color-shifted version by rotating palette assignments.
    Returns (new_grid, new_palette, shift_label)."""
    # Build symbol remapping — shift non-void symbols by 1 position
    symbols = [s for s, _ in palette if s != "."]
    if len(symbols) < 3:
        return grid, palette, shift_name

    if shift_name == "warm":
        # Shift dark colors darker, light colors warmer
        new_palette = [(".", (0, 0, 0, 0))]
        for sym, (r, g, b, a) in palette:
            if sym == ".":
                continue
            nr = min(255, int(r * 1.1 + 8))
            ng = int(g * 0.95)
            nb = max(0, int(b * 0.85 - 5))
            new_palette.append((sym, (nr, ng, nb, a)))
        return grid, new_palette, "warm"

    elif shift_name == "cool":
        new_palette = [(".", (0, 0, 0, 0))]
        for sym, (r, g, b, a) in palette:
            if sym == ".":
                continue
            nr = max(0, int(r * 0.85 - 5))
            ng = int(g * 0.95)
            nb = min(255, int(b * 1.1 + 8))
            new_palette.append((sym, (nr, ng, nb, a)))
        return grid, new_palette, "cool"

    elif shift_name == "dark":
        new_palette = [(".", (0, 0, 0, 0))]
        for sym, (r, g, b, a) in palette:
            if sym == ".":
                continue
            nr = int(r * 0.75)
            ng = int(g * 0.75)
            nb = int(b * 0.75)
            new_palette.append((sym, (nr, ng, nb, a)))
        return grid, new_palette, "dark"

    return grid, palette, shift_name


# ---------------------------------------------------------------------------
# Step 5: Feature computation & labeling
# ---------------------------------------------------------------------------

def compute_features(grid) -> dict:
    h, w = len(grid), len(grid[0])
    total = h * w
    non_void = sum(1 for row in grid for c in row if c != ".")
    density = non_void / total if total else 0

    h_matches = sum(1 for y in range(h) for x in range(w // 2) if grid[y][x] == grid[y][w - 1 - x])
    v_matches = sum(1 for y in range(h // 2) for x in range(w) if grid[y][x] == grid[h - 1 - y][x])
    sym_h = h_matches / max(h * (w // 2), 1)
    sym_v = v_matches / max((h // 2) * w, 1)

    edges, edge_total = 0, 0
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

    counts = Counter(c for row in grid for c in row if c != ".")
    return {
        "density": density,
        "symmetry": max(sym_h, sym_v),
        "edge_complexity": edges / max(edge_total, 1),
        "unique_symbols": len(counts),
    }


def classify_source(filename: str, style_tag: str = "") -> str:
    name = filename.lower()
    if "ceiling" in name:
        return "dungeon ceiling"
    if "floor" in name or "cobble" in name:
        return "dungeon floor"
    if "door" in name:
        return "dungeon door"
    if "pillar" in name or "support" in name:
        return "dungeon pillar"
    if "chest" in name or "container" in name:
        return "chest"
    if "water" in name or "slime" in name or "underwater" in name:
        return "liquid surface"
    if "tree" in name or "grass" in name:
        return "vegetation"
    if "skull" in name or "grave" in name:
        return "dungeon decoration"
    if "machine" in name:
        return "machinery"
    if any(e in name for e in ["skeleton", "zombie", "imp", "druid", "mimic", "shadow", "bone", "death"]):
        return "enemy sprite"
    if "metallic" in name or "metal" in name:
        return "metal wall"
    level_map = {
        "01-03": "upper dungeon wall", "04-06": "mid dungeon wall",
        "07-09": "lower dungeon wall", "10-11": "deep dungeon wall",
    }
    for key, desc in level_map.items():
        if key in name:
            return desc
    if "wall" in name:
        return "dungeon wall"
    if "dcss" in style_tag:
        return "dungeon tile"
    return "dungeon tile"


def make_label(features, tile_type, aug_tag, color_tag="", style_tag="eye-of-the-beholder"):
    parts = [f"style:{style_tag}", f"type:{tile_type}"]

    d = features["density"]
    parts.append(f"density:{'sparse' if d < 0.2 else 'moderate' if d < 0.5 else 'dense' if d < 0.8 else 'solid'}")

    s = features["symmetry"]
    parts.append(f"symmetry:{'high' if s > 0.85 else 'medium' if s > 0.6 else 'low'}")

    e = features["edge_complexity"]
    parts.append(f"detail:{'flat' if e < 0.15 else 'simple' if e < 0.35 else 'moderate' if e < 0.55 else 'complex'}")

    u = features["unique_symbols"]
    parts.append(f"colors:{'minimal' if u <= 2 else 'few' if u <= 4 else 'rich'}")

    if aug_tag and aug_tag != "orig":
        parts.append(f"aug:{aug_tag}")
    if color_tag:
        parts.append(f"palette:{color_tag}")

    return ", ".join(parts)


def grid_to_string(grid):
    return "\n".join("".join(row) for row in grid)


def palette_to_desc(palette):
    return " ".join(f"'{s}'=({r},{g},{b})" for s, (r, g, b, _) in palette if s != ".")


# ---------------------------------------------------------------------------
# Step 6: Stratified sampling
# ---------------------------------------------------------------------------

def stratified_sample(pairs_with_features, max_per_bin=100, seed=42):
    """Bin by density × edge_complexity (5x5 grid), cap per bin."""
    bins = {}
    for pair, features in pairs_with_features:
        d_bin = min(int(features.get("density", 0) * 5), 4)
        e_bin = min(int(features.get("edge_complexity", 0) * 5), 4)
        key = (d_bin, e_bin)
        bins.setdefault(key, []).append(pair)

    filled = sum(1 for v in bins.values() if v)
    print(f"  Stratification: {filled}/25 bins filled")
    for key in sorted(bins.keys()):
        print(f"    bin d={key[0]} e={key[1]}: {len(bins[key])} samples")

    rng = random.Random(seed)
    result = []
    for key in sorted(bins.keys()):
        items = bins[key]
        rng.shuffle(items)
        result.extend(items[:max_per_bin])

    rng.shuffle(result)
    return result


# ---------------------------------------------------------------------------
# Main pipeline
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Optimal EotB ML data preparation")
    parser.add_argument("--stride", type=int, default=8, help="Patch extraction stride (default: 8, use 16 for non-overlapping)")
    parser.add_argument("--aug", type=int, default=4, choices=[4, 8], help="Augmentation level: 4=rotations, 8=+flips")
    parser.add_argument("--color-aug", action="store_true", default=True, help="Enable warm/cool/dark palette shifts (3x more data)")
    parser.add_argument("--no-color-aug", action="store_true", help="Disable color augmentation")
    parser.add_argument("--max-per-bin", type=int, default=150, help="Max samples per stratification bin")
    parser.add_argument("--max-colors", type=int, default=10, help="Max palette colors per group")
    parser.add_argument("--save-patches", action="store_true", help="Also save raw 16x16 PNGs for CNN/VAE training")
    args = parser.parse_args()

    if args.no_color_aug:
        args.color_aug = False

    print(f"{'='*60}")
    print(f"Step 1: Multi-source patch extraction (stride={args.stride})")
    print(f"{'='*60}\n")

    all_patches = []
    total_raw = 0
    total_filtered = 0
    source_stats = {}

    for source_dir, glob_pattern, style_tag, tile_size_hint, license_tag in SOURCES:
        if not source_dir.exists():
            print(f"  SKIP {source_dir.name}: not found")
            continue

        files = sorted(source_dir.glob(glob_pattern))
        if not files:
            print(f"  SKIP {source_dir.name}: no matching files")
            continue

        source_patches = []
        for f in files:
            try:
                img = Image.open(f).convert("RGB")
            except Exception:
                continue

            w, h = img.size

            # For grid-based tilesets (DCSS, puny), cut into individual tiles first
            if tile_size_hint and w > tile_size_hint * 2 and h > tile_size_hint * 2:
                for ty in range(0, h - tile_size_hint + 1, tile_size_hint):
                    for tx in range(0, w - tile_size_hint + 1, tile_size_hint):
                        tile_img = img.crop((tx, ty, tx + tile_size_hint, ty + tile_size_hint))
                        # Resize to PATCH_SIZE if needed
                        if tile_size_hint != PATCH_SIZE:
                            tile_img = tile_img.resize((PATCH_SIZE, PATCH_SIZE), Image.NEAREST)

                        total_raw += 1
                        quality = patch_quality(tile_img)
                        if not passes_quality(quality):
                            total_filtered += 1
                            continue

                        source_patches.append({
                            "array": np.array(tile_img),
                            "group": style_tag,
                            "source": f.name,
                            "style_tag": style_tag,
                            "quality": quality,
                            "offset": (tx, ty),
                        })
            else:
                # Standard patch extraction with stride
                stride = args.stride
                # For small images, use non-overlapping
                if w <= PATCH_SIZE * 3 or h <= PATCH_SIZE * 3:
                    stride = PATCH_SIZE

                patches = extract_patches(img, stride=stride)
                for patch_img, px, py in patches:
                    total_raw += 1
                    quality = patch_quality(patch_img)
                    if not passes_quality(quality):
                        total_filtered += 1
                        continue

                    source_patches.append({
                        "array": np.array(patch_img),
                        "group": style_tag,
                        "source": f.name,
                        "style_tag": style_tag,
                        "quality": quality,
                        "offset": (px, py),
                    })

        all_patches.extend(source_patches)
        source_stats[style_tag] = source_stats.get(style_tag, 0) + len(source_patches)
        print(f"  {style_tag}: {len(files)} files -> {len(source_patches)} quality patches [{license_tag}]")

    print(f"\n  Total raw patches: {total_raw}")
    print(f"  Filtered out:      {total_filtered} ({100*total_filtered/max(total_raw,1):.0f}% low quality)")
    print(f"  Kept:              {len(all_patches)}")
    print(f"\n  By source:")
    for tag, count in sorted(source_stats.items(), key=lambda x: -x[1]):
        print(f"    {tag}: {count}")

    # --- Step 2: Per-group palette extraction ---
    print(f"\n{'='*60}")
    print("Step 2: Palette extraction")
    print(f"{'='*60}\n")

    group_palettes = {}
    for group_name in sorted(set(p["group"] for p in all_patches)):
        group_arrays = [p["array"] for p in all_patches if p["group"] == group_name]
        palette = extract_palette(group_arrays, max_colors=args.max_colors)
        group_palettes[group_name] = palette
        print(f"  {group_name}: {len(palette)} colors")

    # --- Step 3: Quantize + augment + color shift ---
    print(f"\n{'='*60}")
    print(f"Step 3: Quantize + augment ({args.aug}x geo" + (" + 3x color)" if args.color_aug else ")"))
    print(f"{'='*60}\n")

    all_pairs = []  # (chat_dict, features_dict)
    patch_id = 0

    if args.save_patches:
        PATCH_DIR.mkdir(parents=True, exist_ok=True)

    for patch_info in all_patches:
        arr = patch_info["array"]
        group = patch_info["group"]
        style_tag = patch_info.get("style_tag", "eye-of-the-beholder")
        palette = group_palettes[group]
        tile_type = classify_source(patch_info["source"], style_tag)

        grid = quantize_array(arr, palette)

        # Check grid isn't degenerate after quantization
        non_void = sum(1 for row in grid for c in row if c != ".")
        if non_void < PATCH_SIZE * PATCH_SIZE * 0.05:
            continue

        features = compute_features(grid)

        # Color variants: original + warm + cool + dark
        color_variants = [("", grid, palette)]
        if args.color_aug:
            for shift in ["warm", "cool", "dark"]:
                shifted_grid, shifted_palette, tag = color_shift_grid(grid, palette, shift)
                color_variants.append((tag, shifted_grid, shifted_palette))

        for color_tag, c_grid, c_palette in color_variants:
            pal_desc = palette_to_desc(c_palette)

            for aug_grid, aug_tag in augment_grid(c_grid, args.aug):
                grid_str = grid_to_string(aug_grid)
                label = make_label(features, tile_type, aug_tag, color_tag, style_tag)
                user_msg = f"Palette: {pal_desc}\n{label}"

                pair = {
                    "messages": [
                        {"role": "system", "content": SYSTEM_PROMPT},
                        {"role": "user", "content": user_msg},
                        {"role": "assistant", "content": grid_str},
                    ]
                }
                all_pairs.append((pair, features))

                if args.save_patches:
                    patch_img = Image.fromarray(arr)
                    patch_img.save(PATCH_DIR / f"patch_{patch_id:05d}.png")

                patch_id += 1

    print(f"  Total training pairs: {len(all_pairs)}")

    # --- Step 4: Stratified sampling ---
    print(f"\n{'='*60}")
    print("Step 4: Stratified sampling")
    print(f"{'='*60}\n")

    sampled = stratified_sample(all_pairs, max_per_bin=args.max_per_bin)
    print(f"\n  After stratification: {len(sampled)} samples")

    # --- Step 5: Split and write ---
    print(f"\n{'='*60}")
    print("Step 5: Train/valid/test split")
    print(f"{'='*60}\n")

    random.seed(42)
    random.shuffle(sampled)
    n = len(sampled)
    train_end = int(n * 0.9)
    valid_end = int(n * 0.95)

    splits = {
        "train": sampled[:train_end],
        "valid": sampled[train_end:valid_end],
        "test": sampled[valid_end:],
    }

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    for name, data in splits.items():
        path = OUTPUT_DIR / f"{name}.jsonl"
        with open(path, "w") as f:
            for entry in data:
                f.write(json.dumps(entry) + "\n")
        print(f"  {name}: {len(data)} samples -> {path}")

    # --- Summary ---
    train_count = len(splits["train"])
    iters_3ep = train_count * 3
    iters_5ep = train_count * 5

    print(f"\n{'='*60}")
    print("Summary")
    print(f"{'='*60}")
    print(f"  Sources:             {len(source_stats)} datasets")
    print(f"  Raw patches:         {total_raw}")
    print(f"  Quality-filtered:    {len(all_patches)}")
    print(f"  After augmentation:  {len(all_pairs)}")
    print(f"  After stratification:{len(sampled)}")
    print(f"  Train / valid / test:{len(splits['train'])} / {len(splits['valid'])} / {len(splits['test'])}")
    print(f"\n  Estimated training time (M4 Pro, ~2 it/sec):")
    print(f"    3 epochs: {iters_3ep} iters -> ~{iters_3ep / 2 / 60:.0f} min")
    print(f"    5 epochs: {iters_5ep} iters -> ~{iters_5ep / 2 / 60:.0f} min")

    print(f"\n  To train:")
    print(f"    cd {SCRIPT_DIR}")
    print(f"    source .venv/bin/activate")
    print(f"    python -m mlx_lm lora \\")
    print(f"      --model mlx-community/Qwen2.5-3B-Instruct-4bit \\")
    print(f"      --train --data data_eotb_optimal \\")
    print(f"      --adapter-path adapters/pixl-eotb-optimal \\")
    print(f"      --fine-tune-type lora --num-layers 16 \\")
    print(f"      --batch-size 1 --learning-rate 2e-5 \\")
    print(f"      --iters {iters_3ep} --val-batches 25 \\")
    print(f"      --steps-per-eval 500 --save-every 1000 \\")
    print(f"      --max-seq-length 512 --seed 42")

    # Save dataset metadata
    ds_meta = {
        "sources": source_stats,
        "patch_stride": args.stride,
        "patch_size": PATCH_SIZE,
        "raw_patches": total_raw,
        "quality_patches": len(all_patches),
        "augmentation": f"{args.aug}x geo" + (" + 3x color" if args.color_aug else ""),
        "total_pairs": len(all_pairs),
        "stratified": len(sampled),
        "splits": {k: len(v) for k, v in splits.items()},
    }
    with open(OUTPUT_DIR / "dataset_info.json", "w") as f:
        json.dump(ds_meta, f, indent=2)


if __name__ == "__main__":
    main()
