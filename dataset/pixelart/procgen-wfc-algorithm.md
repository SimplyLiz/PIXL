---
title: "Wave Function Collapse: Algorithm and Tips"
source: "https://www.boristhebrave.com/2020/02/08/wave-function-collapse-tips-and-tricks/"
topic: "procedural-generation"
fetched: "2026-03-23"
---

# Wave Function Collapse: Algorithm and Tips

By Boris the Brave (with reference to Maxim Gumin's original WFC)

## Overview

Wave Function Collapse (WFC) is a constraint-based procedural generation algorithm created by Maxim Gumin in 2016. It produces images by arranging tiles according to adjacency rules, creating output that resembles input samples through local pattern reuse.

Despite the quantum physics name, WFC is fundamentally a constraint solver — it has virtually nothing in common with the physics concept.

## Core Algorithm

### Three Phases

1. **Initialization**: For each cell in the output grid, maintain a set of all possible tiles that could be placed there (the "superposition")

2. **Observation/Collapse**: Select the cell with the lowest entropy (fewest remaining possibilities). Choose one tile for that cell based on weighted probabilities.

3. **Propagation**: After placing a tile, propagate constraints to neighboring cells. Remove any tile options that would violate adjacency rules. This may cascade — removing options from one cell can force removals in adjacent cells.

Repeat observation + propagation until all cells are collapsed (solved) or a contradiction is reached.

### Two Models

- **Simple Tiled Model**: User provides a tileset with explicit adjacency rules. The algorithm selects and places tiles respecting those constraints.
- **Overlapping Model**: Extracts patterns from an example image and generates new images with the same local patterns. More flexible but harder to control.

### Contradiction Handling

When propagation leaves a cell with zero valid options, the algorithm has reached a contradiction. Solutions:
- **Restart**: Clear and regenerate (simple but wasteful)
- **Backtracking**: Undo recent collapses and try different choices

## Tileset Design Strategies

### Marching Cubes Approach

Design tiles where vertex behavior (black/white corners) determines connectivity. "If you line up tiles so the black and white corners always match, the red lines always connect together nicely." Reduces obscure tile combinations.

### Room Generation

Simple four-tile combinations (empty, wall, corner, corridor) generate varied room layouts by adjusting tile weights. Adding door tiles creates realistic floor plans.

### Foundation Constraints

Tiles designed where base width exceeds top width prevent unsupported floating structures. WFC automatically avoids impossible arrangements.

### Big Tiles

Multi-cell tile units enable smoother curves, larger set pieces, and disguise the underlying grid. Expand design possibilities significantly.

## Constraint Enhancement

### Fixed Tiles

Pre-place specific tiles to integrate handcrafted content (entrances, exits, landmarks) with generated areas. Seamless blending of authored + procedural.

### Path Constraint

A global constraint forcing connectivity between annotated tiles. Ensures a single connected component — prevents disconnected rooms that break playability.

### Tile Weights

Adjust relative frequency of tiles to control density. Low-weight T-junctions + fixed path endpoints produce natural-looking roads/rivers.

## Variety Techniques

### Alternant Variants

Interchangeable tiles with identical adjacency rules but different visuals. Adds variety without affecting generation logic.

### Biome Limitation

Disable contextually inappropriate tiles per area. Prevents incongruous mixing (lava next to snow).

### Spatial Subdivision

Divide large maps into regions with different tile sets and templates. Prevents the "everything looks the same" problem.

## Limitations

WFC generates locally-coherent output but lacks global structure. It cannot guarantee:
- Specific room counts or sizes
- River systems that flow downhill
- Narrative-meaningful spatial relationships

**Solutions**: Combine WFC with higher-level planning (handcrafted floorplans filled by WFC, path constraints, region subdivision).
