"""Ingest Cytopia tile data for PIXL training.

Loads isometric city-builder tiles from Cytopia's TileData.json,
quantizes each variant to a PAX symbol grid, and generates rich
training labels from the metadata (category, title, description, tags,
game properties like pollution/happiness/inhabitants).

Usage:
    python ingest_cytopia.py
    python ingest_cytopia.py --cytopia-root /path/to/Cytopia/data
    python ingest_cytopia.py --merge   # merge into existing training data

Requires: pip install Pillow
"""

from __future__ import annotations

import argparse
import json
import os
import random
from collections import Counter
from pathlib import Path

from PIL import Image

# --- Constants ---

CYTOPIA_ROOT = Path("/Users/lisa/Work/Cytopia/src/Cytopia/data")
TILEDATA_PATH = CYTOPIA_ROOT / "resources" / "data" / "TileData.json"
OUTPUT_DIR = Path(__file__).resolve().parent / "data_cytopia"

# Symbols ordered by brightness (same pool as prepare_matched.py)
SYMBOL_POOL = ".#+=~gorhwsABCDE"

SYSTEM_PROMPT = (
    "You are a pixel art tile generator. Given a description, output a PAX-format character grid.\n"
    "Rules:\n"
    "- Use only the symbols from the palette provided\n"
    "- Each row must be exactly the specified width\n"
    "- Total rows must equal the specified height\n"
    "- '.' means transparent/void\n"
    "- Output ONLY the grid, no explanation"
)


# --- Image processing ---

def extract_palette(pixels: list[tuple], max_colors: int = 8, void_threshold: int = 128) -> list[tuple[str, tuple]]:
    """Extract top N colors sorted by brightness, assign symbols."""
    counts: dict[tuple, int] = {}
    for r, g, b, a in pixels:
        if a < void_threshold:
            continue
        qr = (r // 16) * 16
        qg = (g // 16) * 16
        qb = (b // 16) * 16
        key = (qr, qg, qb)
        counts[key] = counts.get(key, 0) + 1

    if not counts:
        return [(".", (0, 0, 0, 0))]

    top = sorted(counts.items(), key=lambda x: -x[1])[:max_colors]
    top_colors = sorted([c for c, _ in top], key=lambda c: c[0] + c[1] + c[2])

    palette = [(".", (0, 0, 0, 0))]
    for i, (r, g, b) in enumerate(top_colors):
        if i + 1 < len(SYMBOL_POOL):
            palette.append((SYMBOL_POOL[i + 1], (r, g, b, 255)))
    return palette


def quantize_tile(img: Image.Image, palette: list[tuple[str, tuple]], size: int = 16) -> list[list[str]]:
    """Resize to size×size and quantize to palette symbols."""
    resized = img.convert("RGBA").resize((size, size), Image.LANCZOS)
    pixels = list(resized.getdata())

    grid = []
    for y in range(size):
        row = []
        for x in range(size):
            r, g, b, a = pixels[y * size + x]
            if a < 128:
                row.append(".")
                continue
            best_sym = "."
            best_dist = float("inf")
            for sym, (pr, pg, pb, pa) in palette:
                if sym == ".":
                    continue
                d = (r - pr) ** 2 * 0.30 + (g - pg) ** 2 * 0.59 + (b - pb) ** 2 * 0.11
                if d < best_dist:
                    best_dist = d
                    best_sym = sym
            row.append(best_sym)
        grid.append(row)
    return grid


def grid_to_string(grid: list[list[str]]) -> str:
    return "\n".join("".join(row) for row in grid)


# --- Label generation ---

def build_label(entry: dict) -> str:
    """Build a rich structured label from Cytopia metadata.

    Combines category, title, description snippet, tags, and game
    properties into a conditioning string.
    """
    parts = []

    # Category
    cat = entry.get("category", "").lower().replace(" ", "_")
    if cat:
        parts.append(f"category:{cat}")

    # Title (cleaned)
    title = entry.get("title", "").strip()
    if title:
        parts.append(f"title:{title.lower()}")

    # Size footprint
    req = entry.get("RequiredTiles", {})
    w, h = req.get("width", 1), req.get("height", 1)
    parts.append(f"footprint:{w}x{h}")

    # Tags (first 5)
    tags = entry.get("tags", [])
    if tags:
        tag_str = " ".join(t.lower() for t in tags[:5])
        parts.append(f"tags:{tag_str}")

    # Game properties (only non-zero ones)
    props = []
    if entry.get("pollutionLevel", 0) > 0:
        props.append("polluting")
    if entry.get("happiness", 0) > 0:
        props.append("happy")
    if entry.get("inhabitants", 0) > 0:
        props.append(f"pop:{entry['inhabitants']}")
    if entry.get("fireHazardLevel", 0) > 0:
        props.append("fire_risk")
    if entry.get("power", 0) > 0:
        props.append("powered")
    if entry.get("placeOnWater", False):
        props.append("aquatic")
    if props:
        parts.append(f"props:{' '.join(props)}")

    # Description snippet (first sentence, max 60 chars)
    desc = entry.get("description", "").strip()
    if desc:
        first_sentence = desc.split(".")[0].strip()
        if len(first_sentence) > 60:
            first_sentence = first_sentence[:57] + "..."
        parts.append(f"desc:{first_sentence.lower()}")

    return ", ".join(parts)


def to_chat(label: str, grid_str: str, palette_desc: str) -> dict:
    """Convert to chat-format training entry."""
    user_msg = f"Palette: {palette_desc}\n{label}"
    return {
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_msg},
            {"role": "assistant", "content": grid_str},
        ]
    }


# --- Augmentation ---

def rotate_90(grid: list[list[str]]) -> list[list[str]]:
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    return [[grid[h - 1 - x][y] for x in range(h)] for y in range(w)]


def augment(grid: list[list[str]]) -> list[tuple[str, list[list[str]]]]:
    """Return 4 augmented versions with rotation labels."""
    r90 = rotate_90(grid)
    r180 = rotate_90(r90)
    r270 = rotate_90(r180)
    return [("", grid), ("90", r90), ("180", r180), ("270", r270)]


# --- Main ---

def main():
    parser = argparse.ArgumentParser(description="Ingest Cytopia tiles for PIXL training")
    parser.add_argument("--cytopia-root", type=str, default=str(CYTOPIA_ROOT))
    parser.add_argument("--tile-size", type=int, default=16, help="Quantization target size")
    parser.add_argument("--merge", action="store_true", help="Merge into existing data_me/ training splits")
    parser.add_argument("--no-augment", action="store_true", help="Skip rotation augmentation")
    args = parser.parse_args()

    cytopia_root = Path(args.cytopia_root)
    tiledata_path = cytopia_root / "resources" / "data" / "TileData.json"

    if not tiledata_path.exists():
        print(f"error: TileData.json not found at {tiledata_path}")
        return

    with open(tiledata_path) as f:
        data = json.load(f)

    print(f"Loaded {len(data)} tile entries from Cytopia")

    os.makedirs(OUTPUT_DIR, exist_ok=True)

    all_pairs = []
    total_variants = 0
    skipped = 0
    category_counts: Counter = Counter()

    for entry in data:
        tiles_info = entry.get("tiles", {})
        filename = tiles_info.get("fileName", "")
        clip_w = tiles_info.get("clip_width", 32)
        clip_h = tiles_info.get("clip_height", 32)
        count = tiles_info.get("count", 1)

        img_path = cytopia_root / filename
        if not img_path.exists():
            skipped += 1
            continue

        try:
            sheet = Image.open(img_path).convert("RGBA")
        except Exception:
            skipped += 1
            continue

        # Extract palette from all variants in the sheet
        all_pixels = list(sheet.getdata())
        palette = extract_palette(all_pixels, max_colors=8)
        palette_desc = " ".join(
            f"'{s}'=({r},{g},{b})" for s, (r, g, b, a) in palette if s != "."
        )

        # Build label from metadata
        label = build_label(entry)
        category_counts[entry.get("category", "?")] += 1

        # Extract each variant from the sprite sheet
        for var_idx in range(count):
            x_offset = var_idx * clip_w
            if x_offset + clip_w > sheet.width:
                break

            variant = sheet.crop((x_offset, 0, x_offset + clip_w, clip_h))

            # Skip mostly-transparent variants
            pixels = list(variant.getdata())
            non_void = sum(1 for _, _, _, a in pixels if a >= 128)
            if non_void < clip_w * clip_h * 0.05:
                skipped += 1
                continue

            grid = quantize_tile(variant, palette, args.tile_size)
            total_variants += 1

            # Check density — skip if >95% void after quantization
            non_void_grid = sum(1 for row in grid for c in row if c != ".")
            if non_void_grid < args.tile_size * args.tile_size * 0.05:
                skipped += 1
                continue

            if args.no_augment:
                grid_str = grid_to_string(grid)
                pair = to_chat(label, grid_str, palette_desc)
                all_pairs.append(pair)
            else:
                for rot_label, aug_grid in augment(grid):
                    grid_str = grid_to_string(aug_grid)
                    full_label = label
                    if rot_label:
                        full_label += f", rotation:{rot_label}"
                    pair = to_chat(full_label, grid_str, palette_desc)
                    all_pairs.append(pair)

    print(f"\nProcessed: {total_variants} tile variants -> {len(all_pairs)} training pairs ({skipped} skipped)")
    print(f"Categories: {dict(category_counts)}")

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

    for name, split_data in [("train", train), ("valid", valid), ("test", test)]:
        path = OUTPUT_DIR / f"{name}.jsonl"
        with open(path, "w") as f:
            for entry in split_data:
                f.write(json.dumps(entry) + "\n")
        print(f"Wrote {path}")

    # Merge into data_me if requested
    if args.merge:
        merge_into_training(train, valid, test)

    # Sample output
    if all_pairs:
        sample = all_pairs[0]
        print(f"\nSample label:\n  {sample['messages'][1]['content'][:200]}")
        print(f"\nSample grid:\n  {sample['messages'][2]['content'][:200]}")


def merge_into_training(
    cytopia_train: list[dict],
    cytopia_valid: list[dict],
    cytopia_test: list[dict],
):
    """Merge Cytopia data into the existing data_me/ training splits."""
    data_me = Path(__file__).resolve().parent / "data_me"

    if not data_me.exists():
        print(f"\nwarning: {data_me} not found — run map_elites.py + prepare_me_data.py first")
        print("Cytopia data saved to data_cytopia/ only")
        return

    for name, new_data in [("train", cytopia_train), ("valid", cytopia_valid), ("test", cytopia_test)]:
        path = data_me / f"{name}.jsonl"
        if not path.exists():
            print(f"  skipping {path} — not found")
            continue

        # Read existing
        with open(path) as f:
            existing = [line.strip() for line in f if line.strip()]

        # Append new
        with open(path, "a") as f:
            for entry in new_data:
                f.write(json.dumps(entry) + "\n")

        print(f"  merged {len(new_data)} Cytopia entries into {path} (was {len(existing)}, now {len(existing) + len(new_data)})")

    # Reshuffle each file to interleave Cytopia with MAP-Elites data
    rng = random.Random(42)
    for name in ["train", "valid", "test"]:
        path = data_me / f"{name}.jsonl"
        if not path.exists():
            continue
        with open(path) as f:
            lines = [line.strip() for line in f if line.strip()]
        rng.shuffle(lines)
        with open(path, "w") as f:
            for line in lines:
                f.write(line + "\n")
        print(f"  reshuffled {path} ({len(lines)} total)")


if __name__ == "__main__":
    main()
