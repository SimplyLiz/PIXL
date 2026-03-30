#!/usr/bin/env python3
"""Generate Eye of the Beholder-compatible wall texture sets.

Creates wall tilesets that match EotB's exact format:
- 9 perspective variants per wall (close → far, front + side)
- Progressive darkening at distance
- Assembled into sprite sheet with correct layout
- Cyan (#57FFFF) background key

Usage:
    # Generate with trained LoRA adapter
    python generate_eotb_walls.py --name mossy_stone

    # Generate multiple wall themes
    python generate_eotb_walls.py --name cracked_brick --name vine_wall --name ice_cave

    # Use a specific adapter
    python generate_eotb_walls.py --name dark_stone --adapter training/adapters/pixl-eotb-optimal
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

PROJECT_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_ADAPTER = PROJECT_ROOT / "training" / "adapters" / "pixl-eotb-walls"
OUTPUT_DIR = PROJECT_ROOT / "reference" / "eotb-sprites" / "generated"
MODEL_ID = "mlx-community/Qwen2.5-3B-Instruct-4bit"

# EotB cyan background key
CYAN_BG = (87, 255, 255)

# Wall perspective variants: (width, height, role, darkness_factor)
# darkness_factor: 1.0 = full brightness (close), lower = darker (far)
WALL_VARIANTS = [
    (128, 96,  "front_close",    1.0),
    (80,  59,  "front_mid",      0.75),
    (48,  37,  "front_far",      0.55),
    (24,  35,  "front_vfar",     0.40),
    (24,  105, "side_close",     0.85),
    (24,  95,  "side_mid",       0.65),
    (16,  59,  "side_far",       0.50),
    (16,  43,  "side_narrow",    0.40),
    (8,   35,  "side_sliver",    0.30),
]

# Sheet layout: approximate positions matching original EotB format
# Row 0: plain wall, Row 1: wall variant, Row 2: special (door/decorated)
ROW_Y = [8, 136, 264]
ROW_HEIGHT = 120

SYSTEM_PROMPT = """You are a pixel art tile generator. Given a description, output a PAX-format character grid.
Rules:
- Use only the symbols from the palette provided
- Each row must be exactly the specified width
- Total rows must equal the specified height
- '.' means transparent/void
- Output ONLY the grid, no explanation"""

# Themed palettes — each theme gets its own color ramp
# 10 colors: dark → light, ordered by luminance
THEME_PALETTES = {
    "dark_stone": {
        '#': (12, 8, 16),      '+': (28, 20, 36),     '=': (48, 36, 56),
        '~': (68, 52, 72),     'g': (88, 72, 88),     'o': (108, 92, 100),
        'r': (32, 28, 24),     'h': (56, 48, 44),     'w': (80, 72, 68),
        's': (112, 104, 96),
    },
    "red_brick": {
        '#': (20, 12, 8),      '+': (49, 20, 32),     '=': (73, 28, 45),
        '~': (97, 36, 49),     'g': (121, 44, 53),    'o': (144, 60, 49),
        'r': (45, 36, 16),     'h': (80, 65, 40),     'w': (112, 96, 72),
        's': (160, 140, 100),
    },
    "mossy_brick": {
        '#': (8, 16, 8),       '+': (16, 32, 12),     '=': (28, 48, 20),
        '~': (44, 64, 32),     'g': (56, 80, 40),     'o': (72, 96, 48),
        'r': (32, 24, 16),     'h': (64, 56, 36),     'w': (96, 88, 56),
        's': (128, 120, 80),
    },
    "ice_cave": {
        '#': (8, 12, 24),      '+': (16, 24, 48),     '=': (28, 40, 72),
        '~': (44, 60, 96),     'g': (64, 84, 120),    'o': (88, 112, 144),
        'r': (24, 28, 40),     'h': (48, 56, 72),     'w': (80, 96, 120),
        's': (128, 152, 180),
    },
    "sandstone": {
        '#': (24, 16, 8),      '+': (48, 32, 16),     '=': (80, 56, 28),
        '~': (112, 80, 40),    'g': (144, 108, 56),   'o': (168, 132, 72),
        'r': (40, 32, 20),     'h': (72, 60, 40),     'w': (120, 100, 68),
        's': (176, 156, 112),
    },
    "sewer": {
        '#': (8, 12, 8),       '+': (16, 24, 16),     '=': (32, 40, 28),
        '~': (48, 56, 36),     'g': (64, 72, 48),     'o': (80, 88, 56),
        'r': (24, 20, 16),     'h': (48, 44, 32),     'w': (72, 68, 52),
        's': (100, 96, 72),
    },
    "obsidian": {
        '#': (4, 4, 8),        '+': (12, 12, 20),     '=': (24, 20, 36),
        '~': (36, 32, 52),     'g': (52, 44, 68),     'o': (68, 56, 84),
        'r': (20, 8, 12),      'h': (40, 16, 28),     'w': (60, 28, 44),
        's': (88, 48, 64),
    },
    "crypt": {
        '#': (12, 12, 12),     '+': (28, 24, 28),     '=': (44, 40, 44),
        '~': (64, 56, 60),     'g': (84, 76, 80),     'o': (104, 96, 100),
        'r': (36, 28, 20),     'h': (60, 48, 36),     'w': (88, 76, 60),
        's': (120, 108, 88),
    },
}

# Themed row descriptions for better variety
THEME_DESCRIPTIONS = {
    "dark_stone":  [
        "dark stone dungeon wall with rough hewn blocks",
        "dark stone wall with carved rune detail",
        "dark stone wall with iron bracket decoration",
    ],
    "red_brick":   [
        "red brick dungeon wall with mortar lines",
        "red brick wall with crumbling mortar detail",
        "red brick wall with archway stones",
    ],
    "mossy_brick": [
        "mossy overgrown stone wall with vines",
        "moss-covered brick wall with dripping water stains",
        "ancient mossy wall with cracked stone blocks",
    ],
    "ice_cave":    [
        "frozen ice cave wall with crystalline surface",
        "glacial ice wall with frost patterns",
        "frozen cavern wall with icicle formations",
    ],
    "sandstone":   [
        "sandy desert temple wall with carved blocks",
        "weathered sandstone wall with hieroglyph detail",
        "crumbling sandstone wall with exposed layers",
    ],
    "sewer":       [
        "damp sewer wall with slime stains",
        "wet stone sewer wall with moss and grime",
        "sewer tunnel wall with drainage grate",
    ],
    "obsidian":    [
        "dark obsidian volcanic wall with glassy surface",
        "obsidian wall with magma vein cracks",
        "smooth obsidian wall with purple crystal inlay",
    ],
    "crypt":       [
        "ancient crypt wall with dusty stone blocks",
        "crypt wall with skull alcove decoration",
        "tomb wall with cracked plaster over stone",
    ],
}


def generate_base_tile(model, tokenizer, name: str, palette_desc: str, palette_map: dict, size: int = 16) -> np.ndarray:
    """Generate a base 16x16 wall tile using the LoRA model."""
    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": f"Palette: {palette_desc}\nstyle:eye-of-the-beholder, type:{name}, density:solid, detail:complex, colors:rich"},
    ]

    prompt = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
    response = generate_fn(model, tokenizer, prompt=prompt, max_tokens=350)

    # Parse grid
    lines = [l.strip() for l in response.strip().split('\n') if l.strip() and not l.startswith('```')]
    grid = []
    for l in lines:
        row = []
        for ch in l[:size]:
            if ch in palette_map:
                row.append(palette_map[ch])
            else:
                row.append(palette_map.get('.', (0, 0, 0)))
        # Pad if short
        while len(row) < size:
            row.append(palette_map.get('.', (0, 0, 0)))
        grid.append(row)
        if len(grid) >= size:
            break

    # Pad rows if short
    while len(grid) < size:
        grid.append([palette_map.get('.', (0, 0, 0))] * size)

    return np.array(grid, dtype=np.uint8)


def tile_to_variant(base_tile: np.ndarray, target_w: int, target_h: int, darkness: float) -> Image.Image:
    """Scale and darken a base tile to create a perspective variant."""
    base_img = Image.fromarray(base_tile)

    # Tile the base to cover the target aspect ratio
    base_h, base_w = base_tile.shape[:2]

    # Figure out how many times to tile
    tiles_x = max(1, (target_w + base_w - 1) // base_w + 1)
    tiles_y = max(1, (target_h + base_h - 1) // base_h + 1)

    tiled = Image.new('RGB', (base_w * tiles_x, base_h * tiles_y))
    for ty in range(tiles_y):
        for tx in range(tiles_x):
            tiled.paste(base_img, (tx * base_w, ty * base_h))

    # Crop to target aspect, then resize
    crop_w = min(tiled.width, target_w * (base_w // min(base_w, 16)))
    crop_h = min(tiled.height, target_h * (base_h // min(base_h, 16)))
    cropped = tiled.crop((0, 0, crop_w, crop_h))

    # Resize with nearest neighbor (pixel art)
    variant = cropped.resize((target_w, target_h), Image.NEAREST)

    # Apply distance darkening
    arr = np.array(variant, dtype=np.float32)
    arr *= darkness
    arr = np.clip(arr, 0, 255).astype(np.uint8)

    return Image.fromarray(arr)


def assemble_sheet(variants_by_row: list[list[Image.Image]], sheet_w: int = 888, sheet_h: int = 392) -> Image.Image:
    """Assemble wall variants into an EotB-format sprite sheet."""
    sheet = Image.new('RGB', (sheet_w, sheet_h), CYAN_BG)

    for row_idx, row_variants in enumerate(variants_by_row):
        if row_idx >= len(ROW_Y):
            break

        y_base = ROW_Y[row_idx]

        # Layout: front_close, front_mid, front_far, front_vfar, side_close, side_mid, side_far, side_narrow, side_sliver
        # Approximate x positions matching original layout
        x_positions = [8, 144, 232, 416, 288, 320, 352, 392, 376]

        for i, variant_img in enumerate(row_variants):
            if i >= len(x_positions):
                break
            x = x_positions[i]
            y = y_base + (2 if i >= 4 else 0)  # slight y offset for side panels
            sheet.paste(variant_img, (x, y))

    return sheet


def generate_wall_set(model, tokenizer, wall_name: str, palette_map: dict):
    """Generate a complete wall texture set with all perspective variants."""
    palette_desc = " ".join(f"'{s}'=({r},{g},{b})" for s, (r, g, b) in palette_map.items() if s != '.')
    descriptions = THEME_DESCRIPTIONS.get(wall_name, [
        f"{wall_name} dungeon stone wall with block pattern",
        f"{wall_name} dungeon wall with decorative detail",
        f"{wall_name} dungeon wall with architectural feature",
    ])

    print(f"\n  Palette: {wall_name}")
    print(f"  Generating base tiles...")

    rows = []
    for row_idx, desc in enumerate(descriptions):
        print(f"    Row {row_idx + 1}/3: {desc}")

        # Generate candidates, reject flat tiles (< 3 colors), keep best
        best_base = None
        best_unique = 0
        max_attempts = 8
        for attempt in range(max_attempts):
            base = generate_base_tile(model, tokenizer, desc, palette_desc, palette_map)
            unique = len(set(tuple(row) for row in base.reshape(-1, 3).tolist()))
            if unique > best_unique:
                best_unique = unique
                best_base = base
            if unique >= 5:
                break  # good enough, stop early

        if best_unique < 3:
            print(f"      -> WARNING: only {best_unique} colors after {max_attempts} attempts (flat tile)")
        else:
            print(f"      -> {best_unique} unique colors (attempt {min(attempt+1, max_attempts)})")

        # Create all 9 perspective variants
        row_variants = []
        for w, h, role, darkness in WALL_VARIANTS:
            variant = tile_to_variant(best_base, w, h, darkness)
            row_variants.append(variant)

        rows.append(row_variants)

    # Assemble into sheet
    sheet = assemble_sheet(rows)
    return sheet, rows


def main():
    parser = argparse.ArgumentParser(description="Generate EotB-compatible wall texture sets")
    parser.add_argument("--name", action="append", required=True, help="Wall theme name(s)")
    parser.add_argument("--adapter", type=str, default=str(DEFAULT_ADAPTER), help="LoRA adapter path")
    parser.add_argument("--model", type=str, default=MODEL_ID, help="Base model ID")
    parser.add_argument("--version", type=str, default=None, help="Output version folder (e.g. v1, v2, v3). Auto-increments if not set.")
    args = parser.parse_args()

    try:
        from mlx_lm import load, generate
        global generate_fn
        generate_fn = generate
    except ImportError:
        print("ERROR: mlx-lm required. Install: pip install mlx-lm")
        sys.exit(1)

    print(f"Loading model + adapter...")
    model, tokenizer = load(args.model, adapter_path=args.adapter)

    # Determine version folder
    if args.version:
        version = args.version
    else:
        # Auto-increment: find highest existing vN
        existing = [d.name for d in OUTPUT_DIR.iterdir() if d.is_dir() and d.name.startswith("v")] if OUTPUT_DIR.exists() else []
        nums = [int(v[1:]) for v in existing if v[1:].isdigit()]
        version = f"v{max(nums, default=0) + 1}"

    version_dir = OUTPUT_DIR / version
    version_dir.mkdir(parents=True, exist_ok=True)

    print(f"Output: {version_dir}")
    print(f"Available themes: {', '.join(sorted(THEME_PALETTES.keys()))}")
    print(f"(Unknown themes will use 'red_brick' palette)\n")

    for wall_name in args.name:
        # Resolve palette for this theme
        palette_colors = THEME_PALETTES.get(wall_name, THEME_PALETTES["red_brick"])
        palette_map = {'.': (0, 0, 0)}
        palette_map.update(palette_colors)

        print(f"\n{'='*60}")
        print(f"Generating wall set: {wall_name}")
        print(f"{'='*60}")

        sheet, rows = generate_wall_set(model, tokenizer, wall_name, palette_map)

        # Save sheet
        sheet_path = version_dir / f"walls_{wall_name}.png"
        sheet.save(sheet_path)
        print(f"\n  Sheet saved: {sheet_path}")

        # Save individual variants for inspection
        variant_dir = version_dir / wall_name
        variant_dir.mkdir(exist_ok=True)
        for row_idx, row_variants in enumerate(rows):
            for var_idx, (w, h, role, _) in enumerate(WALL_VARIANTS):
                if var_idx < len(row_variants):
                    var_path = variant_dir / f"row{row_idx}_{role}_{w}x{h}.png"
                    row_variants[var_idx].save(var_path)

        # Save a preview (scaled up 4x)
        preview = sheet.resize((sheet.width * 2, sheet.height * 2), Image.NEAREST)
        preview_path = version_dir / f"walls_{wall_name}_preview.png"
        preview.save(preview_path)
        print(f"  Preview:    {preview_path}")

    # Side-by-side comparison with original
    print(f"\n{'='*60}")
    print("Comparison with original EotB tiles")
    print(f"{'='*60}")

    original_path = PROJECT_ROOT / "reference" / "eotb-sprites" / "eotb1" / "walls" / "level_01-03_walls.png"
    if original_path.exists():
        original = Image.open(original_path)
        first_gen = Image.open(version_dir / f"walls_{args.name[0]}.png")

        # Resize both to same height for comparison
        comp_h = 200
        orig_resized = original.resize((int(original.width * comp_h / original.height), comp_h), Image.NEAREST)
        gen_resized = first_gen.resize((int(first_gen.width * comp_h / first_gen.height), comp_h), Image.NEAREST)

        comparison = Image.new('RGB', (orig_resized.width + gen_resized.width + 16, comp_h + 30), (30, 30, 30))
        comparison.paste(orig_resized, (0, 25))
        comparison.paste(gen_resized, (orig_resized.width + 16, 25))

        # Add labels
        draw = ImageDraw.Draw(comparison)
        draw.text((4, 4), "ORIGINAL", fill=(200, 200, 200))
        draw.text((orig_resized.width + 20, 4), "GENERATED", fill=(200, 200, 200))

        comp_path = version_dir / "comparison.png"
        comparison.save(comp_path)
        print(f"\n  Comparison: {comp_path}")


if __name__ == "__main__":
    main()
