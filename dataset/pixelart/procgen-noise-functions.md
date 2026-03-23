---
title: "Noise Functions for 2D Terrain and Texture Generation"
source: "https://www.redblobgames.com/maps/terrain-from-noise/"
topic: "procedural-generation"
fetched: "2026-03-23"
---

# Noise Functions for 2D Terrain and Texture Generation

Based on Red Blob Games and multiple sources

## Core Concepts

Bandwidth-limited gradient noise functions (Simplex or Perlin) assign values from 0.0 to 1.0 across a 2D map. By layering and transforming these values, you can generate terrain, textures, and natural-looking patterns.

## Perlin Noise

Developed by Ken Perlin in 1983. A type of gradient noise that:

1. Defines a grid of random gradient vectors
2. For each point, finds the surrounding grid cell
3. Computes dot products between gradient vectors and distance vectors
4. Interpolates results using a smooth fade function

**Properties**: Continuous, band-limited, reproducible from seed, tileable with proper setup.

## Simplex Noise

Also by Ken Perlin, an improvement over classic Perlin:

- Uses a simplex grid (triangles in 2D, tetrahedra in 3D) instead of a square grid
- Fewer multiplications per point
- No directional artifacts
- Scales better to higher dimensions
- Better visual quality overall

## Frequency and Wavelength

Frequency determines oscillation density:
- `wavelength = map_size / frequency`
- Doubling frequency halves feature size
- Low frequency = large terrain features (continents)
- High frequency = fine detail (pebbles)

## Building Complexity with Octaves (Fractal Brownian Motion)

Layer multiple noise samples at different frequencies:

```
Layer 1 (freq 1x, amp 1.0):   Large hills and valleys
Layer 2 (freq 2x, amp 0.5):   Mid-sized features
Layer 3 (freq 4x, amp 0.25):  Fine detail
Layer 4 (freq 8x, amp 0.125): Micro-detail
```

Each octave doubles frequency and halves amplitude. The ratio between amplitudes is called **persistence** (typically 0.5).

**Normalization**: Sum all weighted noise values, divide by sum of amplitudes to keep results in 0–1 range.

### Lacunarity

The frequency multiplier between octaves. Default is 2.0 (double each octave). Higher values create more high-frequency detail.

## Elevation Redistribution

Raw noise produces continuous rolling terrain. Apply mathematical transformations:

### Power Function
`elevation = pow(noise_sum, exponent)`
- Exponent > 1: Flatter lowlands, steeper peaks
- Exponent < 1: Flatter highlands, steeper valleys

### Absolute Value Ridging
`elevation = abs(noise)` creates sharp mountain ridges instead of smooth hills.

### Terracing
Round elevation to discrete levels for stepped terrain.

## Biome Assignment

### Elevation-Only
Threshold values into biome types: water < 0.3, beach < 0.35, forest < 0.7, mountain < 0.9, snow > 0.9.

### Two-Parameter System (Elevation + Moisture)
Combine two independent noise maps:
- Elevation controls vertical geography
- Moisture controls precipitation/vegetation

This creates realistic biome grids where tropical forests appear at low, wet areas and tundra at high, dry areas.

## Advanced Techniques

### Island Shaping
Apply distance-from-center functions to force map borders toward water:
`shaped = noise - distance_from_center * factor`

### Wraparound Maps
Sample higher-dimensional noise with trigonometric coordinate conversion for seamless cylindrical/toroidal wrapping.

### Domain Warping
Feed noise output as input coordinates to another noise function. Creates organic, swirling patterns useful for alien terrain or magical effects.

### Tree/Object Placement
Use high-frequency noise and place objects at local maxima within search radius R. Creates natural-looking distribution without grid artifacts.

## Implementation Pattern

```pseudocode
for each pixel (x, y):
    nx = x / map_width - 0.5    // normalize to -0.5..+0.5
    ny = y / map_height - 0.5

    elevation = 0
    for each octave i:
        freq = base_freq * lacunarity^i
        amp = base_amp * persistence^i
        elevation += amp * noise(nx * freq, ny * freq)

    elevation = redistribute(elevation)
    biome = lookup_biome(elevation, moisture)
```

## Strengths and Limitations

**Strengths**: Simple (~50 lines), fast, supports infinite generation, parallelizable, deterministic from seed.

**Limitations**: All locations are independent (no river systems, no guaranteed lake counts). Regions feel similar without post-processing. Heavy parameter tuning needed.
