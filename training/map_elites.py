"""MAP-Elites quality-diversity search over WFC parameter space.

Generates diverse, labeled tilemap training data by searching the space
of WFC configurations (weights, seeds, predicates) for each theme.

Usage:
    python map_elites.py --theme dark_fantasy --iterations 5000
    python map_elites.py --all --iterations 5000
    python map_elites.py --theme gameboy --iterations 1000 --dry-run

Requires: pip install ribs[all]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

import numpy as np

import tomllib

def _import_ribs():
    """Lazy import of pyribs — only needed when running the search."""
    try:
        from ribs.archives import GridArchive
        from ribs.emitters import EvolutionStrategyEmitter
        from ribs.schedulers import Scheduler
        return GridArchive, EvolutionStrategyEmitter, Scheduler
    except ImportError:
        sys.exit("error: missing 'ribs' package. Install with: pip install ribs[all]")

from me_features import compute_features, path_connectivity

# --- Paths ---
PROJECT_ROOT = Path(__file__).resolve().parent.parent
PIXL_BIN = PROJECT_ROOT / "tool" / "target" / "release" / "pixl"
PIXL_BIN_DEBUG = PROJECT_ROOT / "tool" / "target" / "debug" / "pixl"
DATA_DIR = Path(__file__).resolve().parent / "data_me"

# Dogfood tilesets (these have proper edge_class and semantics)
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

# Per-theme predicate pools — predicates are selected by genotype flags
PREDICATE_POOLS: dict[str, list[str]] = {
    "_default": [
        "border:wall_solid",
        "region:room1:walkable:3x3:center",
        "region:room2:walkable:2x3:northwest",
        "region:room3:walkable:2x2:southeast",
        "region:room4:walkable:3x2:southwest",
        "region:room5:walkable:2x2:northeast",
    ],
}

# Map sizes to search over
MAP_SIZES = [
    (8, 8),
    (10, 8),
    (12, 8),
    (12, 10),
    (16, 12),
]


def get_pixl_bin() -> Path:
    """Find the pixl binary (release preferred, debug fallback)."""
    if PIXL_BIN.exists():
        return PIXL_BIN
    if PIXL_BIN_DEBUG.exists():
        return PIXL_BIN_DEBUG
    sys.exit(f"error: pixl binary not found. Run 'cargo build --release' in {PROJECT_ROOT / 'tool'}")


def load_theme_info(pax_path: Path) -> tuple[list[str], dict[str, str]]:
    """Parse tile names and affordance map from a PAX file.

    Returns:
        (tile_names, affordance_map) where affordance_map maps
        tile_name -> affordance string (e.g. "walkable", "obstacle").
    """
    with open(pax_path, "rb") as f:
        data = tomllib.load(f)
    tile_names = []
    affordance_map = {}
    for name, tile_def in data.get("tile", {}).items():
        tile_names.append(name)
        sem = tile_def.get("semantic", {})
        if isinstance(sem, dict) and "affordance" in sem:
            affordance_map[name] = sem["affordance"]
    return sorted(tile_names), affordance_map


def run_wfc(
    pixl_bin: Path,
    pax_path: Path,
    width: int,
    height: int,
    seed: int,
    weight_overrides: dict[str, float] | None = None,
    predicates: list[str] | None = None,
) -> dict | None:
    """Run pixl narrate and return parsed JSON, or None on failure."""
    cmd = [
        str(pixl_bin), "narrate", str(pax_path),
        "--width", str(width),
        "--height", str(height),
        "--seed", str(seed),
        "--format", "json",
        "--out", "/dev/null",
    ]
    if weight_overrides:
        for name, val in weight_overrides.items():
            cmd.extend(["-w", f"{name}:{val:.3f}"])
    if predicates:
        for p in predicates:
            cmd.extend(["-r", p])
    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True, timeout=10,
        )
        if result.returncode != 0:
            return None
        return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError):
        return None


def decode_genotype(
    genotype: np.ndarray,
    tile_names: list[str],
    predicate_pool: list[str],
) -> tuple[dict[str, float], int, list[str], tuple[int, int]]:
    """Decode a flat float array into WFC parameters.

    Layout:
        [0..N]        tile weight multipliers (N = len(tile_names))
        [N]           seed (float -> int)
        [N+1..N+1+P]  predicate activation flags (P = len(predicate_pool))
        [N+1+P]       map size selector

    Returns: (weight_overrides, seed, active_predicates, (width, height))
    """
    n = len(tile_names)
    p = len(predicate_pool)

    # Tile weights: softmax to ensure positive, then scale
    raw_weights = genotype[:n]
    exp_w = np.exp(raw_weights - raw_weights.max())  # numerical stability
    weights = exp_w / exp_w.sum() * n  # normalize so mean weight = 1.0
    weight_overrides = {name: max(0.01, float(w)) for name, w in zip(tile_names, weights)}

    # Seed
    seed = int(abs(genotype[n]) * 10000) % 100000

    # Predicate flags: > 0.0 means active
    pred_flags = genotype[n + 1 : n + 1 + p]
    active_preds = [pred for pred, flag in zip(predicate_pool, pred_flags) if flag > 0.0]

    # Map size selector
    size_idx = int(abs(genotype[n + 1 + p]) * 100) % len(MAP_SIZES)
    map_size = MAP_SIZES[size_idx]

    return weight_overrides, seed, active_preds, map_size


def label_map(
    features: dict[str, float],
    theme: str,
    width: int,
    height: int,
) -> str:
    """Convert numeric features to a structured text label."""
    parts = [f"theme:{theme}", f"size:{width}x{height}"]

    wr = features["wall_ratio"]
    if wr < 0.3:
        parts.append("layout:open")
    elif wr < 0.6:
        parts.append("layout:mixed")
    else:
        parts.append("layout:dense")

    rc = features["room_count"]
    if rc <= 1:
        parts.append("rooms:single")
    elif rc <= 3:
        parts.append("rooms:few")
    elif rc <= 6:
        parts.append("rooms:several")
    else:
        parts.append("rooms:many")

    bs = features["border_solidity"]
    if bs > 0.8:
        parts.append("border:enclosed")
    elif bs > 0.4:
        parts.append("border:partial")
    else:
        parts.append("border:open")

    return ", ".join(parts)


def run_map_elites(
    theme: str,
    pax_path: Path,
    iterations: int,
    batch_size: int = 16,
    dry_run: bool = False,
) -> int:
    """Run MAP-Elites for a single theme. Returns number of archive entries."""
    pixl_bin = get_pixl_bin()
    tile_names, affordance_map = load_theme_info(pax_path)
    predicate_pool = PREDICATE_POOLS.get(theme, PREDICATE_POOLS["_default"])

    n_tiles = len(tile_names)
    n_preds = len(predicate_pool)
    genotype_dim = n_tiles + 1 + n_preds + 1  # weights + seed + preds + size_selector

    print(f"[{theme}] tiles={n_tiles}, predicates={n_preds}, genotype_dim={genotype_dim}")
    print(f"[{theme}] tile names: {tile_names}")
    print(f"[{theme}] affordances: {affordance_map}")

    if dry_run:
        # Single evaluation test
        test_geno = np.zeros(genotype_dim)
        weights, seed, preds, (w, h) = decode_genotype(test_geno, tile_names, predicate_pool)
        result = run_wfc(pixl_bin, pax_path, w, h, seed, weights, preds)
        if result:
            features = compute_features(result["grid"], affordance_map)
            lbl = label_map(features, theme, w, h)
            print(f"[{theme}] dry run OK: {w}x{h}, features={features}")
            print(f"[{theme}] label: {lbl}")
        else:
            print(f"[{theme}] dry run FAILED")
        return 0

    # Feature dimensions: wall_ratio (20 bins), room_count (10 bins)
    GridArchive, EvolutionStrategyEmitter, Scheduler = _import_ribs()
    archive = GridArchive(
        solution_dim=genotype_dim,
        dims=[20, 10],
        ranges=[(0.05, 0.95), (0.5, 10.5)],
    )

    emitters = [
        EvolutionStrategyEmitter(
            archive,
            x0=np.zeros(genotype_dim),
            sigma0=0.5,
            batch_size=batch_size,
        )
        for _ in range(4)
    ]

    scheduler = Scheduler(archive, emitters)

    # Track progress
    total_evals = 0
    wfc_failures = 0

    for iteration in range(iterations):
        solutions = scheduler.ask()
        objectives = []
        measures_list = []

        for sol in solutions:
            weights, seed, preds, (w, h) = decode_genotype(sol, tile_names, predicate_pool)
            result = run_wfc(pixl_bin, pax_path, w, h, seed, weights, preds)
            total_evals += 1

            if result is None:
                # WFC failed — give minimum quality, place at edge of archive
                objectives.append(0.0)
                measures_list.append([0.5, 1.0])
                wfc_failures += 1
            else:
                features = compute_features(result["grid"], affordance_map)
                conn = path_connectivity(result["grid"], affordance_map)

                # Quality: 1.0 for valid + connectivity bonus
                quality = 1.0 + conn * 0.5

                objectives.append(quality)
                measures_list.append([
                    features["wall_ratio"],
                    features["room_count"],
                ])

        scheduler.tell(objectives, np.array(measures_list))

        # Progress report every 10 iterations
        if (iteration + 1) % 10 == 0:
            coverage = archive.stats.coverage
            print(
                f"[{theme}] iter {iteration+1}/{iterations} "
                f"| evals={total_evals} failures={wfc_failures} "
                f"| archive coverage={coverage:.1%} ({archive.stats.num_elites} elites)"
            )

    # Export archive
    return export_archive(archive, theme, tile_names, affordance_map, predicate_pool, pixl_bin, pax_path)


def export_archive(
    archive: GridArchive,
    theme: str,
    tile_names: list[str],
    affordance_map: dict[str, str],
    predicate_pool: list[str],
    pixl_bin: Path,
    pax_path: Path,
) -> int:
    """Re-evaluate all archive elites and export as labeled JSONL."""
    DATA_DIR.mkdir(parents=True, exist_ok=True)
    out_path = DATA_DIR / f"archive_{theme}.jsonl"

    count = 0
    with open(out_path, "w") as f:
        # Get all occupied cells from the archive
        data = archive.data()
        solutions = data["solution"]   # shape (n_elites, genotype_dim)
        objectives = data["objective"]  # shape (n_elites,)
        n_elites = len(objectives)

        for i in range(n_elites):
            sol = solutions[i]
            weights, seed, preds, (w, h) = decode_genotype(
                sol, tile_names, predicate_pool
            )
            result = run_wfc(pixl_bin, pax_path, w, h, seed, weights, preds)
            if result is None:
                continue

            features = compute_features(result["grid"], affordance_map)
            lbl = label_map(features, theme, w, h)

            entry = {
                "label": lbl,
                "grid": result["grid"],
                "features": features,
                "quality": float(objectives[i]),
                "theme": theme,
                "width": w,
                "height": h,
                "seed": seed,
            }
            f.write(json.dumps(entry) + "\n")
            count += 1

    print(f"[{theme}] exported {count} maps to {out_path}")
    return count


def main():
    parser = argparse.ArgumentParser(description="MAP-Elites QD search for tilemap training data")
    parser.add_argument("--theme", type=str, help="Theme name (e.g. dark_fantasy)")
    parser.add_argument("--all", action="store_true", help="Run for all themes")
    parser.add_argument("--iterations", type=int, default=5000, help="MAP-Elites iterations")
    parser.add_argument("--batch-size", type=int, default=16, help="Solutions per iteration")
    parser.add_argument("--dry-run", action="store_true", help="Single evaluation test")
    args = parser.parse_args()

    if not args.theme and not args.all:
        parser.error("specify --theme NAME or --all")

    themes = THEMES if args.all else {args.theme: THEMES[args.theme]}

    total = 0
    for theme, pax_path in themes.items():
        if not pax_path.exists():
            print(f"[{theme}] skipping — {pax_path} not found")
            continue
        n = run_map_elites(theme, pax_path, args.iterations, args.batch_size, args.dry_run)
        total += n

    if not args.dry_run:
        print(f"\nTotal: {total} maps exported to {DATA_DIR}")


if __name__ == "__main__":
    main()
