---
title: "Procedural Sprite Generation Using Cellular Automata"
source: "https://ljvmiranda921.github.io/projects/2020/03/31/cellular-sprites/"
topic: "procedural-generation"
fetched: "2026-03-23"
---

# Procedural Sprite Generation Using Cellular Automata

Based on ljvmiranda921's "Sprites-as-a-Service" project

## Core Algorithm

The sprite generation follows a four-step process:

1. **Initialize**: Create a half-width sprite grid (e.g., 4x8 for an 8x8 sprite)
2. **Seed**: Add random noise to establish "live" cells (targeting 40–60% density)
3. **Simulate**: Run Conway's Game of Life for multiple iterations
4. **Symmetry**: Mirror the half-canvas horizontally to create the full sprite

## Conway's Game of Life Rules

The simulation operates under four governing principles:

- **Overpopulation**: Cells with 3+ living neighbors die
- **Stasis**: Cells with 2–3 living neighbors survive
- **Underpopulation**: Cells with <2 neighbors die
- **Reproduction**: Dead cells with exactly 3 neighbors become alive

## Key Parameters

Two critical variables control sprite appearance:

- **Survival Rate**: Higher values → "big, blocky sprites"
- **Extinction Rate**: Higher values → "mosquito-like thinner sprites"

The balance between these two parameters determines the character of generated sprites — adjusting them creates vastly different visual styles from the same algorithm.

## Why Symmetry Works

Mirroring the half-canvas creates sprites that read as humanoid or creature-like shapes. This works because:

- Bilateral symmetry is a fundamental property of living organisms
- Players expect characters/enemies to be roughly symmetric
- The mirror operation doubles visual complexity from half the computation

Supported sizes: 8x8, 16x16, 32x32, 64x64 pixels.

## Visual Enhancement Techniques

### Outlining

A solid black border surrounding cells near empty space improves visual distinction and recognizability. The outline is computed by detecting cells adjacent to the sprite boundary.

### Gradient Shading

Compute spatial gradients across the sprite, shift the matrix vertically by one pixel, and map values to custom colormaps. This adds dimensional depth beyond flat coloring.

### Color Harmony

Colors assigned using custom color harmony algorithms — not random colors. Harmonious palettes make procedurally generated sprites look intentional rather than noisy.

## Variations and Extensions

### Asymmetric Sprites

Not all generated content needs symmetry. Environmental objects (rocks, trees, debris) benefit from asymmetric cellular automata output.

### Multi-Layer Generation

Generate separate layers for:
- Body silhouette (main cellular automata pass)
- Detail overlay (second pass with different parameters)
- Color regions (seeded by body shape)

### Animation

Run additional simulation steps from the final sprite state to generate animation frames. The cellular automata naturally produces smooth transitions between states.

## Applications

- Procedural enemy/NPC generation for roguelikes
- Placeholder sprites during prototyping
- Background decoration variety
- Particle effect shapes
- Icon/collectible generation
