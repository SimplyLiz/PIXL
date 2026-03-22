# NVIDIA NTC → PAX Style Latent

## NTC Summary

Neural Texture Compression — compresses PBR texture bundles (albedo + normal +
roughness + AO) for 3D rendering. 272 MB → 11 MB (96% reduction). Requires RTX
tensor cores. Beta v0.5.

**Wrong for pixel art:** indexed palettes, 512-byte tiles, needs crisp edges.

## The Insight Worth Stealing

NTC encodes **correlated visual information** as compact latents. For PBR:
albedo and normal are correlated. For pixel art: tiles in a set share lighting
direction, outline weight, pixel density, dithering patterns, palette ratios.

## PAX Style Latent — 8 Properties

1. **Light direction** — dominant highlight vector
2. **Palette distribution** — color usage frequency, dark/mid/light contrast ratios
3. **Outline style** — self-outline weight, selective, drop-shadow, or none
4. **Dithering pattern** — none / Bayer / checkerboard (detected from pixel runs)
5. **Pixel density** — average detail density per 4×4 region
6. **Edge character** — hard vs. soft transitions, AA frequency
7. **Hue bias** — warm/cool, saturation envelope
8. **Symmetry tendency** — % of tiles using h/v symmetry

## Pipeline

Seed tiles (5–10, hand-drawn) → Style encoder (statistical analysis) →
Style token (stored in .pax) → Injected into generation prompts →
All new tiles match the style.

## Implementation

~300 lines of Rust. No ML. Pure signal processing on indexed color grids.
Scheduled for V1.2 alongside procedural variation engine.

## Novel Contribution

No pixel art tool does this. The NTC insight applied to authorship, not
compression.
