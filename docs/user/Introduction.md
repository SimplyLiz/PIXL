# Introduction

PIXL is a complete pixel art toolchain — from blank canvas to game-ready assets. Draw tiles by hand, generate them with AI, build maps, and export to your game engine. Everything lives in plain text `.pax` files that you own.

## Who is PIXL for?

- **Pixel artists** who want AI to handle the tedious parts — generating variations, filling maps, checking consistency — while they focus on the creative work.
- **Game developers** who need a fast path from concept to tileset, with exports for Godot, Unity, Tiled, and TexturePacker.
- **Solo creators** building their first game who want professional-quality pixel art without years of practice.

## What's included

| Component | What it does |
|-----------|-------------|
| **PIXL Studio** | Visual editor — draw, paint, animate, compose |
| **CLI** | Render, validate, convert, generate, export |
| **MCP Server** | Connect Claude or any AI to your tileset |
| **Rust Engine** | Fast parser, renderer, and validator |

## Quick example

A `.pax` file is just text:

```toml
[palette.dungeon]
"." = "#00000000"
"#" = "#2a1f3d"
"+" = "#4a3a6d"

[tile.wall]
palette = "dungeon"
size = "8x8"
grid = '''
########
#++++++#
#+#++#+#
#++++++#
########
'''
```

That's a complete tile. Render it, validate it, export it — all from this one file.

## Next steps

- [Getting Started](./Getting-Started) — install and create your first tileset
- [Sprite Generation](./Sprite-Generation) — generate art from text descriptions
- [PAX Format](./PAX-Format) — learn the file format
