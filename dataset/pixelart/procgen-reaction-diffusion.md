---
title: "Reaction-Diffusion Pattern Generation"
source: "https://jasonwebb.github.io/reaction-diffusion-playground/"
topic: "procedural-generation"
fetched: "2026-03-23"
---

# Reaction-Diffusion Pattern Generation

## Overview

Reaction-diffusion is a mathematical model describing how two chemicals react as they diffuse through a medium. Proposed by Alan Turing in 1952 as a possible explanation for biological patterns (stripes on zebras, spots on leopards, labyrinthine patterns on corals).

## The Gray-Scott Model

The most commonly implemented reaction-diffusion system for pattern generation.

### Two Chemicals

- **Chemical A** (activator): Diffuses quickly, promotes growth
- **Chemical B** (inhibitor): Diffuses slowly, suppresses growth

### Update Equations

```
A' = A + (Da * ∇²A - A*B² + f*(1-A)) * dt
B' = B + (Db * ∇²B + A*B² - (k+f)*B) * dt
```

Where:
- `Da`, `Db` = diffusion rates (Da > Db, typically Da/Db ≈ 2)
- `∇²` = Laplacian (sum of neighbors minus center, approximated by convolution)
- `f` = feed rate (how fast A is replenished)
- `k` = kill rate (how fast B decays)
- `A*B²` = reaction term (A converts to B when they meet)

### Parameters and Patterns

By varying `f` (feed) and `k` (kill), vastly different patterns emerge:

| f | k | Pattern |
|---|---|---------|
| 0.0545 | 0.062 | Spots (mitosis-like) |
| 0.03 | 0.062 | Stripes/worms |
| 0.025 | 0.05 | Labyrinthine/maze |
| 0.04 | 0.06 | Coral/branching |
| 0.012 | 0.05 | Moving spots (solitons) |
| 0.025 | 0.06 | Pulsating patterns |

## Implementation

### Grid Setup

Store A and B concentrations as 2D arrays (or texture channels). Initialize:
- A = 1.0 everywhere
- B = 0.0 everywhere except seed regions (B = 1.0 in small circles/squares)

### Per-Frame Update

```pseudocode
for each cell (x, y):
    laplacian_A = neighbors_sum(A, x, y) - 4 * A[x][y]  // or use 3x3 kernel
    laplacian_B = neighbors_sum(B, x, y) - 4 * B[x][y]

    reaction = A[x][y] * B[x][y] * B[x][y]

    new_A = A[x][y] + (Da * laplacian_A - reaction + f * (1 - A[x][y])) * dt
    new_B = B[x][y] + (Db * laplacian_B + reaction - (k + f) * B[x][y]) * dt

    A_next[x][y] = clamp(new_A, 0, 1)
    B_next[x][y] = clamp(new_B, 0, 1)
```

### Laplacian Kernel (3×3)

```
0.05  0.2  0.05
0.2  -1.0  0.2
0.05  0.2  0.05
```

Center weight = -1.0, edges = 0.2, corners = 0.05.

### Rendering

Map chemical concentrations to colors:
- `color = palette_lookup(A - B)` — high A is background, high B is pattern

## Applications for Pixel Art

### Organic Textures

Generate natural-looking patterns for:
- Animal skin/fur patterns
- Coral, lichen, moss textures
- Terrain features (river deltas, cave systems)
- Magic/energy effects

### Low-Resolution Adaptation

At pixel art scales, the patterns become inherently quantized:
- Small grids (32×32, 64×64) produce chunky, pixel-appropriate patterns
- Threshold the output to get crisp 1-bit patterns
- Use the pattern as a dithering guide

### Seeding for Control

Place initial B concentrations strategically to guide pattern growth:
- Text or shapes as seeds → patterns grow from your design
- Edge seeds → border patterns
- Random point seeds → organic scatter

## Related Techniques

- **L-Systems**: Rule-based recursive growth (better for branching structures like trees)
- **Cellular Automata**: Discrete rule application (simpler, faster, but less organic)
- **Voronoi Noise**: Cell-based patterns (better for scales, tiles, stones)
