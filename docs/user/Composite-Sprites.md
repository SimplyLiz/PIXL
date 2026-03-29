# Composite Sprites

Build 32×32 characters from reusable 16×16 parts. Mix and match heads, bodies, and weapons without redrawing anything.

## Why composites?

Drawing a 32×32 character in a text grid is hard — 1024 pixels to place correctly. But four 16×16 tiles? Each one is only 256 pixels, easy to get right. Compose them into a 2×2 grid and you have a clean 32×32 character.

The bonus: those parts are reusable. A wizard and a knight can share the same boots. A walk animation only needs new leg tiles — the head stays the same.

## How it works

Define 16×16 tiles for each body part, then assemble them:

```toml
[tile.knight_head_l]
palette = "hero"
size = "16x16"
grid = '''
...your head art...
'''

[composite.knight]
size = "32x32"
tile_size = "16x16"
layout = """
knight_head_l    knight_head_l!h
knight_body_l    knight_body_r
"""
```

The `!h` suffix flips the tile horizontally — draw one side of a face, flip it for the other.

## Variants

Swap parts without redefining the whole character:

```toml
[composite.knight.variant.shield]
slot = { "1_0" = "knight_shield_body" }
```

This replaces just the bottom-left tile with a shield-holding variant. Everything else stays the same.

## Animation

Walk cycles only change the tiles that move:

```toml
[composite.knight.anim.walk]
fps = 8
loop = true

[[composite.knight.anim.walk.frame]]
index = 1
# base layout

[[composite.knight.anim.walk.frame]]
index = 2
swap = { "1_0" = "knight_walk2_l", "1_1" = "knight_walk2_r" }
```

Frame 1 uses the base layout. Frame 2 only swaps the leg tiles. The head and torso don't move — no wasted data.

## Per-tile offsets

Add bounce or weapon extension without changing the tile art:

```toml
[composite.knight.offset]
"0_0" = [0, -1]    # head bobs up 1px
```

## Seam checking

PIXL validates that adjacent tiles line up at the boundary:

```bash
pixl validate tileset.pax --check-seams
```

Reports pixel-level discontinuities at tile edges — so you can fix alignment issues before they show up in-game.

## Rendering

```bash
pixl render-composite tileset.pax --composite knight --variant shield --scale 4 --out knight.png
```
