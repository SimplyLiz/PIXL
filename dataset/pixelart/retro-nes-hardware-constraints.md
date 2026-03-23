---
title: "Retro Hardware Constraints: NES, SNES, Game Boy"
source: "https://www.nesdev.org/wiki/Limitations"
topic: "retro-constraints"
fetched: "2026-03-23"
---

# Retro Hardware Constraints

Compiled from nesdev.org, snes.nesdev.org, and multiple sources

## NES (Nintendo Entertainment System)

### Color System

- **Master palette**: 54 usable colors (from 64 entries, some duplicates/unusable)
- **Background palettes**: 4 palettes × 4 colors each = 13 unique colors + shared background color
- **Sprite palettes**: 4 palettes × 3 colors each + transparent = 12 unique sprite colors
- **Total on screen**: Up to 25 simultaneous colors (13 BG + 12 sprite)
- **Attribute tiles**: 16×16 pixel regions share one palette — limits color mixing granularity

### Sprite Constraints

- **Sprite size**: 8×8 or 8×16 pixels (system-wide setting, not per-sprite)
- **Max sprites on screen**: 64
- **Max sprites per scanline**: 8 — more than 8 causes flickering
- **Colors per sprite**: 3 + transparent (from one of 4 sprite palettes)
- **Practical character width**: Max ~32px before scanline limit becomes a constant issue
- Wider characters (64px) must be rendered as background tiles, not sprites

### Background/Tile Constraints

- **Pattern tables**: 2 × 256 tiles (one for BG, one for sprites), each tile 8×8
- **Nametable**: 32×30 tiles visible (256×240 pixels)
- **CHR RAM bandwidth**: ~8 tiles can be updated per vblank (60Hz), limiting real-time tile animation

## SNES (Super Nintendo)

### Color System

- **Full palette**: 256 colors selectable from 32,768 (15-bit RGB)
- **Background palettes**: 8 palettes × up to 16 colors each (varies by BG mode)
- **Sprite palettes**: 8 palettes × 16 colors each (from last half of color RAM)
- **Mode 7**: Single 256-color palette for rotation/scaling layer

### Sprite Constraints

- **Sprite sizes**: 8×8, 16×16, 32×32, 64×64, plus 16×32 and 32×64
- **Two size settings**: System selects 2 of the above sizes; each sprite picks one
- **Max sprites on screen**: 128
- **Max sprites per scanline**: 32
- **Colors per sprite**: 16 (from one of 8 sprite palettes, all 4bpp)
- **VRAM for sprites**: 32KB shared between both sprite sizes

### Background Modes

- **Mode 0**: 4 BG layers, 4 colors each
- **Mode 1**: 2 BG layers at 16 colors + 1 at 4 colors (most common)
- **Mode 3**: 1 BG at 256 colors + 1 at 16 colors (direct color)
- **Mode 7**: 1 rotatable/scalable BG at 256 colors

## Game Boy (Original)

### Color System

- **Palette**: 4 shades of green (actually gray on later models)
- **Background palette**: 1 palette × 4 shades
- **Sprite palettes**: 2 palettes × 3 shades + transparent
- **Total**: 4 shades only

### Sprite Constraints

- **Sprite size**: 8×8 or 8×16 (system-wide)
- **Max sprites on screen**: 40
- **Max sprites per scanline**: 10
- **Screen resolution**: 160×144 pixels

## Game Boy Color

### Color System

- **Colors available**: 32,768 (15-bit RGB)
- **Background palettes**: 8 palettes × 4 colors = up to 32 BG colors
- **Sprite palettes**: 8 palettes × 3 colors + transparent = up to 24 sprite colors
- **Total on screen**: Up to 56 simultaneous colors
- **Key detail**: Palette assigned per-tile, not per-pixel — same tile can display with different palettes

## How Constraints Shaped Art Style

### Economy of Expression

Limited palettes forced artists to:
- Use hue shifting within 3–4 color ramps
- Rely on dithering patterns for intermediate tones
- Design readable silhouettes that work with minimal color
- Share palettes between similar-colored sprites

### Scanline Tricks

Developers changed palettes mid-frame (per-scanline) to exceed normal color limits. This technique:
- Enabled gradient skies
- Allowed different color schemes for HUD vs. game area
- Created the illusion of more colors than hardware supported

### Tile Reuse

With limited tile memory, artists designed tiles that worked in multiple contexts:
- Mirrored/rotated versions for symmetrical structures
- Modular pieces that combine into varied environments
- Shared tiles between characters with palette swaps

### Size Constraints as Design Language

- 8×8 sprites: Iconic, abstract characters (early RPGs, puzzle games)
- 16×16 sprites: Detailed enough for readable characters (most NES platformers)
- 32×32 sprites: "Large" characters reserved for bosses or starring roles (SNES era)
