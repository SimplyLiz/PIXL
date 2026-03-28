"""Prepare palette-matched training data with rich feature labels.

Improvements over V1 (informed by TileGPT research):
1. Auto-extract a palette from each tileset's actual colors
2. Quantize each tileset with its own palette
3. Compute visual features per tile (density, symmetry, edge complexity, dominant color)
4. Generate structured labels from features (not just "a tile from X")
5. Augment with rotations (4x data)
6. Feature-stratified sampling to ensure uniform coverage (TileGPT's key finding)

Usage:
    python prepare_matched.py
    python prepare_matched.py --assets /path/to/GameTileNet/Assets --stratify
"""

import json
import math
import os
import random
from collections import Counter
from pathlib import Path

ASSETS_DIR = "/tmp/GameTileNet/DataAndAnnotations/Assets"
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "data_matched")

# Symbols we assign to extracted palette colors (ordered by brightness)
SYMBOL_POOL = ".#+=~gorhwsABCDE"

SYSTEM_PROMPT = """You are a pixel art tile generator. Given a description, output a PAX-format character grid.
Rules:
- Use only the symbols from the palette provided
- Each row must be exactly the specified width
- Total rows must equal the specified height
- '.' means transparent/void
- Output ONLY the grid, no explanation"""


def load_image(path, size=16):
    """Load and resize a PNG to size x size, return RGBA pixels."""
    from PIL import Image
    img = Image.open(path).convert("RGBA").resize((size, size), Image.LANCZOS)
    return list(img.getdata()), img


def extract_palette(pixels, max_colors=10, void_threshold=128):
    """Extract the top N most frequent colors from pixel data."""
    # Group by RGB, skip transparent
    color_counts = Counter()
    for r, g, b, a in pixels:
        if a < void_threshold:
            continue
        # Quantize to reduce noise (round to nearest 16)
        qr = (r // 16) * 16
        qg = (g // 16) * 16
        qb = (b // 16) * 16
        color_counts[(qr, qg, qb)] = color_counts.get((qr, qg, qb), 0) + 1

    if not color_counts:
        return [(".", (0, 0, 0, 0))]

    # Top N colors sorted by brightness
    top = sorted(color_counts.items(), key=lambda x: -x[1])[:max_colors]
    top_colors = sorted([c for c, _ in top], key=lambda c: c[0] + c[1] + c[2])

    # Assign symbols
    palette = [(".", (0, 0, 0, 0))]  # void is always .
    for i, (r, g, b) in enumerate(top_colors):
        if i + 1 < len(SYMBOL_POOL):
            palette.append((SYMBOL_POOL[i + 1], (r, g, b, 255)))

    return palette


def quantize_with_palette(pixels, palette, size, void_threshold=128):
    """Quantize pixels to the nearest palette color."""
    grid = []
    for y in range(size):
        row = []
        for x in range(size):
            idx = y * size + x
            r, g, b, a = pixels[idx]

            if a < void_threshold:
                row.append(".")
                continue

            # Find nearest palette color (skip void)
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


def grid_to_string(grid):
    return "\n".join("".join(row) for row in grid)


def rotate_90(grid):
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    return [[grid[h - 1 - x][y] for x in range(h)] for y in range(w)]


def flip_h(grid):
    return [row[::-1] for row in grid]


def augment(grid):
    """Return 4 augmented versions: original, 90, 180, 270."""
    r90 = rotate_90(grid)
    r180 = rotate_90(r90)
    r270 = rotate_90(r180)
    return [grid, r90, r180, r270]


def compute_tile_features(grid):
    """Compute visual features from a character grid.

    Returns dict with: density, symmetry_h, symmetry_v, edge_complexity,
    unique_symbols, dominant_symbol, dominant_ratio.
    """
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    total = h * w
    if total == 0:
        return {}

    # Density: fraction of non-void cells
    non_void = sum(1 for row in grid for c in row if c != ".")
    density = non_void / total

    # Symmetry: horizontal and vertical
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

    # Edge complexity: count transitions along borders
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

    # Symbol distribution
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


def features_to_label(features, tileset_name, rotation_suffix=""):
    """Convert computed features to a structured label string.

    TileGPT finding: structured labels with feature values dramatically
    improve prompt adherence vs. generic descriptions.
    """
    parts = [f"tileset:{tileset_name}"]

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

    if rotation_suffix:
        parts.append(f"rotation:{rotation_suffix}")

    return ", ".join(parts)


def to_chat(desc, grid_str, palette_desc):
    """Convert to chat format with palette context."""
    user_msg = f"Palette: {palette_desc}\n{desc}"
    return {
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_msg},
            {"role": "assistant", "content": grid_str},
        ]
    }


def stratified_sample(pairs_with_features, max_per_bin=50, seed=42):
    """Stratified sampling to ensure uniform feature coverage.

    TileGPT's key finding: MAP-Elites (uniform coverage, Gini~0) dramatically
    outperforms random sampling (Gini~1). We approximate this by binning on
    density × edge_complexity and capping per-bin count.
    """
    bins = {}
    for pair, features in pairs_with_features:
        d_bin = min(int(features.get("density", 0) * 5), 4)      # 5 bins
        e_bin = min(int(features.get("edge_complexity", 0) * 5), 4)  # 5 bins
        key = (d_bin, e_bin)
        if key not in bins:
            bins[key] = []
        bins[key].append(pair)

    # Report coverage
    total_bins = 25  # 5x5
    filled = sum(1 for v in bins.values() if len(v) > 0)
    print(f"  Feature coverage: {filled}/{total_bins} bins filled")
    for key in sorted(bins.keys()):
        print(f"    bin {key}: {len(bins[key])} samples")

    # Cap each bin, shuffle within bin
    rng = random.Random(seed)
    result = []
    for key in sorted(bins.keys()):
        items = bins[key]
        rng.shuffle(items)
        result.extend(items[:max_per_bin])

    rng.shuffle(result)
    return result


def main():
    import argparse
    parser = argparse.ArgumentParser(description="Prepare palette-matched training data")
    parser.add_argument("--assets", type=str, default=ASSETS_DIR, help="Path to GameTileNet Assets dir")
    parser.add_argument("--stratify", action="store_true", help="Use stratified sampling for uniform coverage")
    parser.add_argument("--max-per-bin", type=int, default=200, help="Max samples per feature bin (with --stratify)")
    args = parser.parse_args()

    try:
        from PIL import Image
    except ImportError:
        print("Installing Pillow...")
        import subprocess
        subprocess.check_call(["pip", "install", "Pillow"])
        from PIL import Image

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    all_pairs = []          # (chat_dict, features_dict) tuples
    total_images = 0
    skipped = 0

    assets_dir = args.assets
    tileset_dirs = sorted(Path(assets_dir).iterdir())
    print(f"Processing {len(tileset_dirs)} tilesets...")

    # Feature distribution tracking
    density_hist = Counter()
    detail_hist = Counter()

    for tileset_dir in tileset_dirs:
        if not tileset_dir.is_dir():
            continue

        png_files = sorted(tileset_dir.glob("*.png"))
        if not png_files:
            continue

        # Collect all pixels from this tileset to extract a shared palette
        all_pixels = []
        loaded = []
        for png in png_files:
            try:
                pixels, img = load_image(str(png), 16)
                all_pixels.extend(pixels)
                loaded.append((png, pixels))
            except Exception:
                skipped += 1
                continue

        if not loaded:
            continue

        # Extract palette from this tileset's actual colors
        palette = extract_palette(all_pixels, max_colors=8)
        palette_desc = " ".join(f"'{s}'=({r},{g},{b})" for s, (r, g, b, a) in palette if s != ".")

        print(f"  {tileset_dir.name}: {len(loaded)} images, {len(palette)} colors")

        for png, pixels in loaded:
            total_images += 1
            grid = quantize_with_palette(pixels, palette, 16)

            # Check density — skip if >90% void
            non_void = sum(1 for row in grid for c in row if c != ".")
            if non_void < 16 * 16 * 0.1:
                skipped += 1
                continue

            # Compute features for the original (un-augmented) grid
            features = compute_tile_features(grid)

            # Track distribution
            density_hist[features_to_label(features, "").split("density:")[1].split(",")[0]] += 1

            # Augment: original + 3 rotations
            rotation_labels = ["", "90", "180", "270"]
            for i, aug_grid in enumerate(augment(grid)):
                grid_str = grid_to_string(aug_grid)
                # Rich label instead of generic description
                label = features_to_label(features, tileset_dir.name, rotation_labels[i])
                pair = to_chat(label, grid_str, palette_desc)
                all_pairs.append((pair, features))

    print(f"\nTotal: {total_images} images -> {len(all_pairs)} training pairs ({skipped} skipped)")
    print(f"Density distribution: {dict(density_hist)}")

    # Extract just the chat dicts
    if args.stratify:
        print("\nApplying stratified sampling...")
        chat_data = stratified_sample(all_pairs, max_per_bin=args.max_per_bin)
        print(f"After stratification: {len(chat_data)} samples")
    else:
        chat_data = [pair for pair, _ in all_pairs]

    # Shuffle and split
    random.seed(42)
    random.shuffle(chat_data)

    n = len(chat_data)
    train_end = int(n * 0.9)
    valid_end = int(n * 0.95)

    train = chat_data[:train_end]
    valid = chat_data[train_end:valid_end]
    test = chat_data[valid_end:]

    print(f"Split: {len(train)} train, {len(valid)} valid, {len(test)} test")

    for name, data in [("train", train), ("valid", valid), ("test", test)]:
        path = os.path.join(OUTPUT_DIR, f"{name}.jsonl")
        with open(path, "w") as f:
            for entry in data:
                f.write(json.dumps(entry) + "\n")
        print(f"Wrote {path}")

    # Stats
    sample = chat_data[0]["messages"][2]["content"]
    non_void = sum(1 for c in sample if c not in (".", "\n"))
    total_chars = sum(1 for c in sample if c != "\n")
    print(f"\nSample density: {non_void}/{total_chars} ({100*non_void/max(total_chars,1):.0f}% non-void)")
    print(f"Sample label: {chat_data[0]['messages'][1]['content'][:200]}")
    print(f"Sample grid:\n{sample[:200]}")


if __name__ == "__main__":
    main()
