# Atlas Packing

Pack all your tiles into optimized sprite atlases with JSON metadata for your game engine.

## How it works

PIXL arranges all tiles into a grid-layout atlas image with configurable padding, then generates TexturePacker-compatible JSON metadata with pixel-perfect frame coordinates.

```bash
pixl atlas tileset.pax --out atlas.png --map atlas.json --columns 8 --scale 2
```

## Options

| Flag | Default | What it does |
|------|---------|-------------|
| `--columns` | 8 | Number of columns in the atlas grid |
| `--padding` | 1 | Pixels between tiles (prevents bleeding) |
| `--scale` | 1 | Render scale factor |
| `--map` | — | Output path for JSON metadata |

## Uniform tile sizes

The atlas packer requires all tiles to have the same dimensions. If your tileset has mixed sizes (16×16 tiles + 32×32 composites), PIXL automatically creates separate atlases:

- `atlas.png` — regular tiles (e.g., all 16×16)
- `atlas_composites.png` — composed sprites (e.g., all 32×32)

Each gets its own JSON metadata file.

## Animation frame tags

When your tileset includes spritesets with animations, the atlas JSON includes Aseprite-compatible frame tags:

```json
"meta": {
  "frameTags": [
    { "name": "hero_idle", "from": 0, "to": 3, "direction": "forward" },
    { "name": "hero_walk", "from": 4, "to": 9, "direction": "forward" }
  ]
}
```

## Composite entries

Composite sprites are packed with all variants and animation frames as separate atlas entries:

```
knight           → base layout
knight:shield    → shield variant
knight:walk:1    → walk animation frame 1
knight:walk:2    → walk animation frame 2
```

Each entry has its own frame coordinates in the JSON metadata, ready for your sprite system to look up by name.
