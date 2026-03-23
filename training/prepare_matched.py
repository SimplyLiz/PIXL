"""Prepare palette-matched training data.

Instead of forcing everything into dark_fantasy, we:
1. Auto-extract a palette from each tileset's actual colors
2. Quantize each tileset with its own palette
3. Augment with rotations/flips (4x data)
4. Merge into training JSONL
"""

import json
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


def main():
    try:
        from PIL import Image
    except ImportError:
        print("Installing Pillow...")
        import subprocess
        subprocess.check_call(["pip", "install", "Pillow"])
        from PIL import Image

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    all_pairs = []
    total_images = 0
    skipped = 0

    tileset_dirs = sorted(Path(ASSETS_DIR).iterdir())
    print(f"Processing {len(tileset_dirs)} tilesets...")

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
            except Exception as e:
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

            name = png.stem
            desc = f"a 16x16 pixel art tile from {tileset_dir.name}"

            # Augment: original + 3 rotations
            for i, aug_grid in enumerate(augment(grid)):
                suffix = ["", " (rotated 90)", " (rotated 180)", " (rotated 270)"][i]
                grid_str = grid_to_string(aug_grid)
                pair = to_chat(desc + suffix, grid_str, palette_desc)
                all_pairs.append(pair)

    print(f"\nTotal: {total_images} images -> {len(all_pairs)} training pairs ({skipped} skipped)")

    # Shuffle and split
    random.seed(42)
    random.shuffle(all_pairs)

    n = len(all_pairs)
    train_end = int(n * 0.9)
    valid_end = int(n * 0.95)

    train = all_pairs[:train_end]
    valid = all_pairs[train_end:valid_end]
    test = all_pairs[valid_end:]

    print(f"Split: {len(train)} train, {len(valid)} valid, {len(test)} test")

    for name, data in [("train", train), ("valid", valid), ("test", test)]:
        path = os.path.join(OUTPUT_DIR, f"{name}.jsonl")
        with open(path, "w") as f:
            for entry in data:
                f.write(json.dumps(entry) + "\n")
        print(f"Wrote {path}")

    # Stats
    sample = all_pairs[0]["messages"][2]["content"]
    non_void = sum(1 for c in sample if c not in (".", "\n"))
    total_chars = sum(1 for c in sample if c != "\n")
    print(f"\nSample density: {non_void}/{total_chars} ({100*non_void/max(total_chars,1):.0f}% non-void)")
    print(f"Sample grid:\n{sample[:200]}")


if __name__ == "__main__":
    main()
