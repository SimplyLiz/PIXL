---
title: "Autotiling with Bitmask Encoding"
source: "https://excaliburjs.com/blog/Autotiling%20Technique/"
topic: "tilesets"
fetched: "2026-03-23"
---

# Autotiling with Bitmask Encoding

Based on Excalibur.js autotiling technique guide

## What is Autotiling?

Autotiling converts map data into properly rendered tilemaps by selecting appropriate tile sprites based on neighboring tiles. It examines each tile's neighbors to determine which visual variant should be displayed.

## Wang Tiles Foundation

Named after mathematician Hao Wang — square tiles with colored edges designed to match adjacent tiles. A Wang tileset provides multiple tile variations representing different neighbor configurations.

## Bitmask Encoding System

### Core Idea

A tile's 8 neighbors can be encoded into a single byte (8 bits). Each neighbor position gets a bit: 1 if that neighbor is solid, 0 otherwise.

### Bit Assignment (clockwise from top-left)

```
Position     | Bit | Value
-------------|-----|------
Top-Left     |  0  |   1
Top          |  1  |   2
Top-Right    |  2  |   4
Left         |  3  |   8
Right        |  4  |  16
Bottom-Left  |  5  |  32
Bottom       |  6  |  64
Bottom-Right |  7  | 128
```

Sum the values of all solid neighbors to get the bitmask index.

### Example

A tile with solid neighbors at top, right, and bottom:
- Top = 2, Right = 16, Bottom = 64
- Bitmask = 2 + 16 + 64 = 82
- Look up tile sprite for index 82

### Corner Optimization (47-tile reduction)

With 8 bits, there are 256 possible combinations. But corner tiles only matter visually when both adjacent edges are solid. This reduces unique visual cases to 47 tiles.

**Rule**: A corner bit only contributes to the bitmask if both adjacent edge bits are set.

```
Top-Left matters only if Top AND Left are both solid
Top-Right matters only if Top AND Right are both solid
Bottom-Left matters only if Bottom AND Left are both solid
Bottom-Right matters only if Bottom AND Right are both solid
```

## Implementation Steps

### 1. Build Tilemap Data

Store as 2D grid where each cell is solid (1) or empty (0).

### 2. Calculate Bitmasks

```pseudocode
for each tile at (x, y):
    bitmask = 0
    for each neighbor offset (dx, dy) with bit_index:
        nx, ny = x + dx, y + dy
        if out_of_bounds(nx, ny):
            solid = default_value  // typically true (walls at edges)
        else:
            solid = grid[ny][nx]
        if solid:
            bitmask |= (1 << bit_index)

    // Corner optimization
    if not (bitmask & TOP and bitmask & LEFT):
        bitmask &= ~TOP_LEFT
    if not (bitmask & TOP and bitmask & RIGHT):
        bitmask &= ~TOP_RIGHT
    if not (bitmask & BOTTOM and bitmask & LEFT):
        bitmask &= ~BOTTOM_LEFT
    if not (bitmask & BOTTOM and bitmask & RIGHT):
        bitmask &= ~BOTTOM_RIGHT

    tile_sprite = lookup_table[bitmask]
```

### 3. Build Lookup Table

Map each possible bitmask value to a sprite position in the tileset. This is the most labor-intensive step — requires testing each configuration visually.

### 4. Render

For each tile: retrieve bitmask → look up sprite → draw.

## Practical Considerations

- **Edge handling**: Out-of-bounds neighbors can default to solid (for enclosed maps) or empty (for open maps)
- **Multiple terrain types**: Run separate bitmask passes for each terrain type, or use multi-bit encoding
- **Performance**: Bitmask calculation is O(1) per tile; full map is O(n) where n = tile count
- **Tileset swapping**: Same bitmask system works with any 47-tile tileset — swap art without changing code
