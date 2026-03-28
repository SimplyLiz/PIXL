"""Generate tilemaps from text descriptions using the fine-tuned LM + WFC pipeline.

End-to-end: prompt -> label -> LM inference -> coarse grid -> WFC refinement -> valid tilemap.

Usage:
    python generate_map.py --prompt "dark fantasy dungeon, mostly open" --theme dark_fantasy
    python generate_map.py --prompt "dense maze" --theme gameboy --width 10 --height 10
    python generate_map.py --prompt "open field" --theme nature --out map.png
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path

MODEL = "mlx-community/Qwen2.5-3B-Instruct-4bit"
ADAPTER_PATH = Path(__file__).resolve().parent / "adapters" / "pixl-mapgen"

PROJECT_ROOT = Path(__file__).resolve().parent.parent
PIXL_BIN = PROJECT_ROOT / "tool" / "target" / "release" / "pixl"
PIXL_BIN_DEBUG = PROJECT_ROOT / "tool" / "target" / "debug" / "pixl"

THEMES = {
    "dark_fantasy": PROJECT_ROOT / "dogfood" / "final" / "dark_fantasy" / "tileset.pax",
    "light_fantasy": PROJECT_ROOT / "dogfood" / "final" / "light_fantasy" / "tileset.pax",
    "sci_fi": PROJECT_ROOT / "dogfood" / "final" / "sci_fi" / "tileset.pax",
    "nature": PROJECT_ROOT / "dogfood" / "final" / "nature" / "tileset.pax",
    "gameboy": PROJECT_ROOT / "dogfood" / "final" / "gameboy" / "tileset.pax",
    "nes": PROJECT_ROOT / "dogfood" / "final" / "nes" / "tileset.pax",
    "gba": PROJECT_ROOT / "dogfood" / "final" / "gba" / "tileset.pax",
    "snes": PROJECT_ROOT / "dogfood" / "final" / "snes" / "tileset.pax",
}

SYSTEM_PROMPT = (
    "You are a tilemap layout generator. Given a description of map properties, "
    "output a grid of tile names.\n"
    "Rules:\n"
    "- Each cell contains exactly one tile name from the tileset\n"
    "- Rows are separated by newlines\n"
    "- Tile names within a row are separated by spaces\n"
    "- The grid must match the specified dimensions\n"
    "- Output ONLY the grid, no explanation"
)


def get_pixl_bin() -> Path:
    if PIXL_BIN.exists():
        return PIXL_BIN
    if PIXL_BIN_DEBUG.exists():
        return PIXL_BIN_DEBUG
    sys.exit(f"error: pixl binary not found. Run 'cargo build' in {PROJECT_ROOT / 'tool'}")


def load_theme_tiles(pax_path: Path) -> list[str]:
    """Get tile names from a PAX file."""
    with open(pax_path, "rb") as f:
        data = tomllib.load(f)
    return sorted(data.get("tile", {}).keys())


def parse_prompt_to_label(
    prompt: str, theme: str, width: int, height: int
) -> str:
    """Convert a free-form prompt to the structured label format the model was trained on."""
    parts = [f"theme:{theme}", f"size:{width}x{height}"]

    lower = prompt.lower()

    # Layout
    if any(w in lower for w in ("open", "spacious", "wide", "empty", "field")):
        parts.append("layout:open")
    elif any(w in lower for w in ("dense", "tight", "maze", "narrow", "packed")):
        parts.append("layout:dense")
    else:
        parts.append("layout:mixed")

    # Rooms
    if any(w in lower for w in ("single", "one room", "1 room", "arena")):
        parts.append("rooms:single")
    elif any(w in lower for w in ("many", "lots", "complex", "labyrinth")):
        parts.append("rooms:many")
    elif any(w in lower for w in ("few", "2", "3", "couple")):
        parts.append("rooms:few")
    else:
        parts.append("rooms:several")

    # Border
    if any(w in lower for w in ("enclosed", "walled", "dungeon", "indoor")):
        parts.append("border:enclosed")
    elif any(w in lower for w in ("open border", "outdoor", "field")):
        parts.append("border:open")
    else:
        parts.append("border:partial")

    return ", ".join(parts)


def generate_coarse_grid(label: str) -> str:
    """Use the fine-tuned LM to generate a tile-name grid."""
    from mlx_lm import load, generate

    adapter = str(ADAPTER_PATH) if ADAPTER_PATH.exists() else None
    if adapter:
        print(f"adapter: {adapter}")
    else:
        print("warning: no adapter found, using base model")

    model, tokenizer = load(MODEL, adapter_path=adapter)

    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        {"role": "user", "content": label},
    ]
    formatted = tokenizer.apply_chat_template(
        messages, tokenize=False, add_generation_prompt=True
    )

    response = generate(
        model, tokenizer,
        prompt=formatted,
        max_tokens=1024,
        verbose=False,
    )
    return response


def parse_grid_response(
    response: str, valid_tiles: set[str], width: int, height: int
) -> list[list[str | None]]:
    """Parse the LM response into a grid, marking invalid tiles as None."""
    grid = []
    for line in response.strip().split("\n"):
        line = line.strip()
        if not line:
            continue
        row = []
        for token in line.split():
            row.append(token if token in valid_tiles else None)
        grid.append(row)

    # Truncate or pad to expected dimensions
    while len(grid) < height:
        grid.append([None] * width)
    grid = grid[:height]
    for i in range(len(grid)):
        while len(grid[i]) < width:
            grid[i].append(None)
        grid[i] = grid[i][:width]

    return grid


def wfc_refine(
    coarse_grid: list[list[str | None]],
    pax_path: Path,
    width: int,
    height: int,
    out_path: Path | None = None,
) -> dict | None:
    """Use WFC to refine the LM's coarse grid.

    Valid tile placements become pins; invalid/missing cells are left for WFC to fill.
    """
    pixl_bin = get_pixl_bin()

    cmd = [
        str(pixl_bin), "narrate", str(pax_path),
        "--width", str(width),
        "--height", str(height),
        "--seed", "42",
        "--format", "json",
        "--out", str(out_path or "/dev/null"),
    ]

    # Convert valid cells to pins
    pin_count = 0
    for y, row in enumerate(coarse_grid):
        for x, tile in enumerate(row):
            if tile is not None:
                cmd.extend(["--pin", f"{x},{y}:{tile}"])
                pin_count += 1

    total = width * height
    print(f"pins: {pin_count}/{total} cells ({pin_count/total:.0%} from LM)")

    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        if result.returncode != 0:
            # Retry with fewer pins (drop every other pin)
            print("WFC contradiction — retrying with reduced pins...")
            cmd2 = [
                str(pixl_bin), "narrate", str(pax_path),
                "--width", str(width),
                "--height", str(height),
                "--seed", "43",
                "--format", "json",
                "--out", str(out_path or "/dev/null"),
            ]
            # Keep only every other valid pin
            skip = True
            for y, row in enumerate(coarse_grid):
                for x, tile in enumerate(row):
                    if tile is not None:
                        skip = not skip
                        if not skip:
                            cmd2.extend(["--pin", f"{x},{y}:{tile}"])

            result = subprocess.run(cmd2, capture_output=True, text=True, timeout=30)
            if result.returncode != 0:
                # Final fallback: no pins, pure WFC
                print("still contradicting — falling back to pure WFC")
                cmd3 = [
                    str(pixl_bin), "narrate", str(pax_path),
                    "--width", str(width),
                    "--height", str(height),
                    "--seed", "44",
                    "--format", "json",
                    "--out", str(out_path or "/dev/null"),
                ]
                result = subprocess.run(cmd3, capture_output=True, text=True, timeout=30)
                if result.returncode != 0:
                    return None

        return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError):
        return None


def render_map(pax_path: Path, grid: list[list[str]], out_path: Path):
    """Render the final grid to PNG using pixl narrate with all cells pinned."""
    pixl_bin = get_pixl_bin()
    width = len(grid[0])
    height = len(grid)

    cmd = [
        str(pixl_bin), "narrate", str(pax_path),
        "--width", str(width),
        "--height", str(height),
        "--seed", "42",
        "--out", str(out_path),
    ]
    for y, row in enumerate(grid):
        for x, tile in enumerate(row):
            cmd.extend(["--pin", f"{x},{y}:{tile}"])

    subprocess.run(cmd, capture_output=True, text=True, timeout=30)


def main():
    parser = argparse.ArgumentParser(description="Generate tilemaps from text descriptions")
    parser.add_argument("--prompt", type=str, required=True, help="Natural language map description")
    parser.add_argument("--theme", type=str, required=True, choices=list(THEMES.keys()))
    parser.add_argument("--width", type=int, default=12)
    parser.add_argument("--height", type=int, default=8)
    parser.add_argument("--out", type=str, default=None, help="Output PNG path")
    parser.add_argument("--no-refine", action="store_true", help="Skip WFC refinement, show raw LM output")
    args = parser.parse_args()

    pax_path = THEMES[args.theme]
    if not pax_path.exists():
        sys.exit(f"error: theme PAX not found at {pax_path}")

    valid_tiles = set(load_theme_tiles(pax_path))
    # Also include rotation variants
    for tile in list(valid_tiles):
        for suffix in ("_90", "_180", "_270"):
            valid_tiles.add(tile + suffix)

    # Step 1: Convert prompt to label
    label = parse_prompt_to_label(args.prompt, args.theme, args.width, args.height)
    print(f"label: {label}")

    # Step 2: LM generates coarse grid
    print("generating coarse grid...")
    raw_response = generate_coarse_grid(label)
    print(f"\n--- LM output ---\n{raw_response}\n---\n")

    coarse_grid = parse_grid_response(raw_response, valid_tiles, args.width, args.height)

    # Count valid cells
    valid_count = sum(1 for row in coarse_grid for t in row if t is not None)
    total = args.width * args.height
    print(f"valid tiles: {valid_count}/{total} ({valid_count/total:.0%})")

    if args.no_refine:
        # Print raw grid
        for row in coarse_grid:
            print(" ".join(t or "???" for t in row))
        return

    # Step 3: WFC refinement
    out_path = Path(args.out) if args.out else Path(f"/tmp/pixl_mapgen_{args.theme}.png")
    print("refining with WFC...")
    result = wfc_refine(coarse_grid, pax_path, args.width, args.height, out_path)

    if result is None:
        print("error: WFC refinement failed")
        sys.exit(1)

    # Print final grid
    print(f"\n--- Final grid ({args.width}x{args.height}) ---")
    for row in result["grid"]:
        print(" ".join(row))

    # Render PNG if output format is text (JSON already skips rendering)
    if args.out:
        render_map(pax_path, result["grid"], out_path)
        print(f"\nmap -> {out_path}")


if __name__ == "__main__":
    main()
