# PIXL Studio

A desktop visual editor for pixel art tilesets. Built with Flutter, runs on Mac, Windows, and Linux.

## Editor modes

Switch between modes using the toggle in the top bar:

- **Pixel** — draw and paint individual tiles
- **Tilemap** — paint 2D maps using your tiles
- **Backdrop** — view and edit parallax scenes with animation zones
- **Composite** — preview assembled multi-tile characters with variants and animations

## Interface layout

The workspace has four areas:

- **Left panel** — AI chat for tile generation, critique, and refinement
- **Tool strip** — drawing and editing tools (changes per mode)
- **Canvas** — the main editing area with zoom, pan, and grid overlay
- **Right panel** — layers, tile browser, palette, and properties

## Layers

Each tile can have multiple layers with independent opacity (0-100%), blend modes (normal, multiply, screen, add), and target layer assignments for tilemaps.

## Getting the engine

Studio bundles the Rust engine inside the app — no separate install needed. Or connect to a standalone engine:

```bash
pixl serve --port 3742 --file tileset.pax
```

## Learn more

- [Drawing & Painting](./Studio-Drawing) — tools, shortcuts, symmetry, color picker
- [Tilemap Mode](./Studio-Tilemaps) — painting maps, play mode, WFC validation
- [AI Chat & Generation](./Studio-AI) — generating tiles, critique, refinement
