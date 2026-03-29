# PAX Format

PAX (Pixel Art eXchange) is a plain text file format for pixel art. It uses TOML syntax with character grids — every pixel is a single character mapped to a color in a palette.

## Why plain text?

- **Readable** — open any `.pax` file in a text editor and see exactly what's there
- **Diffable** — Git shows which pixels changed, line by line
- **No lock-in** — it's TOML, an open standard. Your art is never trapped
- **AI-writable** — LLMs can generate valid `.pax` files directly

## File structure

A `.pax` file contains sections for palettes, themes, tiles, sprites, animations, and more:

```toml
[pax]
version = "2.0"
name = "my_tileset"
theme = "dark_fantasy"

[palette.dungeon]
"." = "#00000000"    # transparent
"#" = "#2a1f3d"      # dark wall
"+" = "#4a3a6d"      # lit surface
"h" = "#8070a8"      # highlight

[tile.wall]
palette = "dungeon"
size = "16x16"
grid = '''
################
#++++++++++++++#
#+#++#++#++#+++#
#++++++++++++++#
################
'''
```

## Palettes

Each character maps to a hex color with alpha:

```toml
[palette.forest]
"." = "#00000000"    # transparent
"#" = "#1a3a1a"      # dark trunk
"g" = "#2d5a27"      # dark leaf
"G" = "#4a8c3f"      # light leaf
```

Up to 94 single-character symbols per palette. For larger palettes (17-48 colors), use `[palette_ext]` with two-character symbols.

## Tile sizes

Supported sizes: 8×8, 16×16, 24×24, 32×32, 48×48, 64×64. Tiles can also use non-square dimensions.

## Encoding options

- **Grid** — raw character matrix, best for tiles up to 16×16
- **RLE** — run-length encoded (`"5# 3+ 2."`), saves space for larger tiles
- **Compose** — assemble from reusable stamps (`"@brick @mortar"`)

## Everything in one file

A single `.pax` file can hold your entire game's art:

- Palettes and palette swaps
- Tiles with edge classes for WFC
- Sprite animations (frame-by-frame, delta, linked, mirrored)
- Color cycling effects
- Composite character sprites
- Tilemap layouts with layers
- Backdrop scenes with parallax
- Export configuration
