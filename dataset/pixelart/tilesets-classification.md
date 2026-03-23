---
title: "Classification of Tilesets"
source: "https://www.boristhebrave.com/2021/11/14/classification-of-tilesets/"
topic: "tilesets"
fetched: "2026-03-23"
---

# Classification of Tilesets

By Boris the Brave

## Core Concepts

A systematic framework for categorizing tilesets used in game development. A **tileset** comprises a small collection of reusable tiles placed in a grid to form a tilemap.

## Classification Framework

The system breaks down tilesets into four dimensions:

### 1. Cell Type

- **S** — Square
- **C** — Cube
- **H** — Hexagon
- **T** — Triangle

### 2. Tile Identification

What minimal information uniquely identifies each tile:

- **V (Vertices)**: Information stored at corners
- **E (Edges)**: Information stored on sides
- **F (Faces)**: Used for 3D tiles
- **C (Cell)**: One value per tile

Numbers indicate possible values (V2 = binary vertex, V3 = ternary, etc.)

### 3. Symmetry

- **R**: Rotational symmetry
- **M**: Mirror/reflection symmetry
- Axes specified when relevant (x, y, z)

### 4. Restrictions

Additional constraints on which combinations are valid. The blob pattern exemplifies this — corners and adjacent edges must follow specific rules.

## Key Tileset Types

| Tileset | Code | Tiles Needed | Description |
|---------|------|--------------|-------------|
| Marching Squares | S-V2 | 16 | Binary corner flags, standard autotiling |
| Wang Tiles | S-E2 | 16 | Edge-based matching, borders at tile center |
| Blob Pattern | S-V2E2-Blob | 47 | Both vertex + edge data, most common for terrain |
| Corner Wang | S-V2-RM | 6 | With rotational + mirror symmetry |

## Autotiling Foundation

The classification builds on marching cubes/squares logic: each cell corner receives a binary flag; summing powers of 2 identifies the appropriate tile.

### Blob Tileset (47-Tile)

The most common approach for terrain autotiling in games:

- Index calculated by summing binary weights (1, 2, 4, 8, 16, 32, 64, 128) for edges/corners clockwise
- Complete set would be 2^8 = 256 tiles
- But corner tiles only matter when both adjacent edges are set → reduces to 47 unique tiles
- Covers all possible configurations for floor/terrain rendering

### Wang Tiles (16-Tile)

- Borders located at tile center (not edges)
- Usually suited for top-down artwork
- Provides autotiling equivalent to blob set with considerably fewer tiles
- Trade-off: less visual variety at tile boundaries

### Marching Squares (16-Tile)

- Simplest approach: only considers 4 corners (N/E/S/W)
- 4 bits → 16 possible tiles
- Easy to implement but limited expressiveness

## Comparative Analysis

Triangle grids require fewer tiles than square variants:
- S-V3: 81 tiles vs. T-V3: 54 tiles
- S-V2-RM: 6 tiles vs. T-V2-RM: 4 tiles

## Practical Implications

The classification enables:
- Systematic exploration of tileset variations
- Understanding minimum tile counts for desired expressiveness
- Choosing the right tileset type for your game's visual needs
- Inventing new tileset classifications for specific requirements
