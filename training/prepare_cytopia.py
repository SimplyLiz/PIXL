"""Prepare training data from Cytopia isometric building assets.

Uses TileData.json for rich descriptions and metadata.
Converts Cytopia's isometric sprites into PAX-format training pairs:
1. Resize to target tile size (16x16 default)
2. Extract palette from each image
3. Quantize to PAX symbol grid
4. Use TileData.json descriptions (with category, size, title)
5. Output as mlx-lm compatible JSONL with 4x rotation augmentation

Usage:
    cd training && .venv/bin/python prepare_cytopia.py [--size 16]
"""

import json
import os
import random
from collections import Counter
from pathlib import Path

CYTOPIA_BASE = os.path.expanduser("~/Work/Cytopia/src/Cytopia/data")
TILE_DATA = os.path.join(CYTOPIA_BASE, "resources/data/TileData.json")
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "data_cytopia")

SYMBOL_POOL = ".#+=~gorhwsABCDE"

SYSTEM_PROMPT = """You are a pixel art tile generator specializing in isometric buildings and terrain.
Given a description, output a PAX-format character grid.
Rules:
- Use only the symbols from the palette provided
- Each row must be exactly the specified width
- Total rows must equal the specified height
- '.' means transparent/void
- Output ONLY the grid, no explanation"""


def load_image(path, size=16):
    from PIL import Image
    img = Image.open(path).convert("RGBA").resize((size, size), Image.LANCZOS)
    return list(img.getdata()), img


def extract_palette(pixels, max_colors=12, void_threshold=128):
    color_counts = Counter()
    for r, g, b, a in pixels:
        if a < void_threshold:
            continue
        qr = (r // 16) * 16
        qg = (g // 16) * 16
        qb = (b // 16) * 16
        color_counts[(qr, qg, qb)] = color_counts.get((qr, qg, qb), 0) + 1

    if not color_counts:
        return [(".", (0, 0, 0, 0))]

    top = sorted(color_counts.items(), key=lambda x: -x[1])[:max_colors]
    top_colors = sorted([c for c, _ in top], key=lambda c: c[0] + c[1] + c[2])

    palette = [(".", (0, 0, 0, 0))]
    for i, (r, g, b) in enumerate(top_colors):
        if i < len(SYMBOL_POOL) - 1:
            palette.append((SYMBOL_POOL[i + 1], (r, g, b, 255)))
    return palette


def quantize_pixel(r, g, b, a, palette, void_threshold=128):
    if a < void_threshold:
        return "."
    qr = (r // 16) * 16
    qg = (g // 16) * 16
    qb = (b // 16) * 16

    best_sym = "."
    best_dist = float("inf")
    for sym, (pr, pg, pb, pa) in palette:
        if sym == ".":
            continue
        dist = (qr - pr) ** 2 + (qg - pg) ** 2 + (qb - pb) ** 2
        if dist < best_dist:
            best_dist = dist
            best_sym = sym
    return best_sym


def pixels_to_grid(pixels, width, height, palette):
    rows = []
    for y in range(height):
        row = ""
        for x in range(width):
            r, g, b, a = pixels[y * width + x]
            row += quantize_pixel(r, g, b, a, palette)
        rows.append(row)
    return "\n".join(rows)


def palette_to_prompt(palette):
    lines = []
    for sym, (r, g, b, a) in palette:
        if sym == ".":
            lines.append("  '.' = transparent")
        else:
            lines.append(f"  '{sym}' = ({r},{g},{b})")
    return "\n".join(lines)


def rotate_grid(grid_str, times=1):
    lines = grid_str.split("\n")
    grid = [list(row) for row in lines]
    for _ in range(times):
        h = len(grid)
        w = len(grid[0]) if h > 0 else 0
        rotated = [[grid[h - 1 - y][x] for y in range(h)] for x in range(w)]
        grid = rotated
    return "\n".join("".join(row) for row in grid)


def build_description(tile_info):
    """Build a rich description from TileData.json entry."""
    title = tile_info.get("title", tile_info["id"])
    category = tile_info.get("category", "building")
    desc = tile_info.get("description", "")
    req = tile_info.get("RequiredTiles", {})
    w = req.get("width", 1)
    h = req.get("height", 1)
    size = f"{w}x{h} " if w > 1 or h > 1 else ""

    # Build rich description
    parts = [f"a {size}isometric {category.lower()} tile: {title.lower()}"]
    if desc:
        # Take first sentence of description
        first_sentence = desc.split(".")[0].strip()
        if first_sentence and len(first_sentence) < 150:
            parts.append(first_sentence)

    # Add gameplay tags if interesting
    tags = []
    if tile_info.get("pollutionLevel", 0) > 0:
        tags.append("polluting")
    if tile_info.get("happiness", 0) > 0:
        tags.append("makes people happy")
    if tile_info.get("fireHazardLevel", 0) > 0:
        tags.append("fire hazard")
    if tile_info.get("placeOnWater", False):
        tags.append("water building")
    if tile_info.get("powerProduction", 0) > 0:
        tags.append("power plant")
    if tags:
        parts.append(f"({', '.join(tags)})")

    return ". ".join(parts)


def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--size", type=int, default=16)
    parser.add_argument("--out", default=OUTPUT_DIR)
    parser.add_argument("--augment", type=int, default=4)
    args = parser.parse_args()

    os.makedirs(args.out, exist_ok=True)

    # Load tile metadata
    with open(TILE_DATA) as f:
        tile_data = json.load(f)

    print(f"Loaded {len(tile_data)} tile definitions from TileData.json")

    # Build lookup: id → tile_info
    tile_lookup = {}
    for tile in tile_data:
        tile_lookup[tile["id"]] = tile

    # Process each tile
    pairs = []
    found = 0
    skipped = 0
    no_image = 0

    for tile_info in tile_data:
        tile_id = tile_info["id"]
        tiles_block = tile_info.get("tiles", {})
        filename = tiles_block.get("fileName", "")

        if not filename:
            no_image += 1
            continue

        image_path = os.path.join(CYTOPIA_BASE, filename)
        if not os.path.exists(image_path):
            no_image += 1
            continue

        try:
            pixels, img = load_image(image_path, args.size)
        except Exception:
            skipped += 1
            continue

        # Skip mostly-transparent images
        opaque = sum(1 for _, _, _, a in pixels if a >= 128)
        if opaque < (args.size * args.size) * 0.08:
            skipped += 1
            continue

        palette = extract_palette(pixels)
        if len(palette) < 3:
            skipped += 1
            continue

        grid = pixels_to_grid(pixels, args.size, args.size, palette)
        description = build_description(tile_info)
        palette_prompt = palette_to_prompt(palette)

        user_prompt = f"Palette:\n{palette_prompt}\n\nGenerate a {args.size}x{args.size} pixel art tile: {description}"

        pair = {
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_prompt},
                {"role": "assistant", "content": grid},
            ]
        }
        pairs.append(pair)
        found += 1

        # Augment with rotations
        if args.augment > 1:
            for rot in range(1, min(args.augment, 4)):
                rotated = rotate_grid(grid, rot)
                pairs.append({
                    "messages": [
                        {"role": "system", "content": SYSTEM_PROMPT},
                        {"role": "user", "content": user_prompt},
                        {"role": "assistant", "content": rotated},
                    ]
                })

    print(f"Processed: {found} tiles, skipped: {skipped}, no image: {no_image}")
    print(f"Total pairs (with {args.augment}x augmentation): {len(pairs)}")

    # Shuffle and split 90/5/5
    random.seed(42)
    random.shuffle(pairs)

    n = len(pairs)
    train_end = int(n * 0.9)
    valid_end = int(n * 0.95)

    splits = {
        "train": pairs[:train_end],
        "valid": pairs[train_end:valid_end],
        "test": pairs[valid_end:],
    }

    for name, data in splits.items():
        path = os.path.join(args.out, f"{name}.jsonl")
        with open(path, "w") as f:
            for pair in data:
                f.write(json.dumps(pair) + "\n")
        print(f"  {name}: {len(data)} -> {path}")


if __name__ == "__main__":
    main()
