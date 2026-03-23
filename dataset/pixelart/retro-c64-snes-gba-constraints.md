---
title: "Retro Hardware Constraints: C64, SNES Details, GBA"
source: "https://www.c64-wiki.com/wiki/Graphics_Modes"
topic: "retro-constraints"
fetched: "2026-03-23"
---

# Additional Retro Hardware Constraints

## Commodore 64 (VIC-II)

### Fixed 16-Color Palette

The C64 has a hardwired palette — no palette RAM, no custom colors:

| # | Color | Hex |
|---|-------|-----|
| 0 | Black | #000000 |
| 1 | White | #FFFFFF |
| 2 | Red | #68372B |
| 3 | Cyan | #70A4B2 |
| 4 | Purple | #6F3D86 |
| 5 | Green | #588D43 |
| 6 | Blue | #352879 |
| 7 | Yellow | #B8C76F |
| 8 | Orange | #6F4F25 |
| 9 | Brown | #433900 |
| 10 | Light Red | #9A6759 |
| 11 | Dark Gray | #444444 |
| 12 | Medium Gray | #6C6C6C |
| 13 | Light Green | #9AD284 |
| 14 | Light Blue | #6C5EB5 |
| 15 | Light Gray | #959595 |

### Color Cell System

**The defining constraint**: In standard character/bitmap modes, the screen is divided into 8×8 pixel "color cells." Each cell can only use 2 colors (foreground + background from the 16 available). This means color boundaries must align to 8×8 blocks.

**Multicolor mode**: Trades horizontal resolution for more colors. Pixels become 2× wide (effectively 160×200), but each 4×8 pixel cell gets up to 4 colors (background + 2 shared + 1 per cell).

### Sprite System

- **8 hardware sprites** (multiplexable per scanline)
- **Monochrome sprites**: 24×21 pixels, 1 color + transparent
- **Multicolor sprites**: 12×21 double-width pixels, 3 colors + transparent
  - 2 colors shared across ALL sprites
  - 1 individual color per sprite
- **Sprite multiplexing**: Reuse sprites on different scanlines to exceed 8 total (common technique)
- **Sprite-background priority**: Each sprite can be in front of or behind background

### Art Implications

- Color cell restriction creates "attribute clash" — visible color grid at 8×8 boundaries
- Artists learned to hide cell boundaries in natural feature edges (brick lines, window frames)
- Multicolor mode's double-wide pixels give everything a chunky, distinctive look
- The fixed palette's muted, slightly muddy tones define the C64 aesthetic

## SNES Extended Details

### Background Modes (Complete)

| Mode | BG Layers | Colors per Tile | Special |
|------|-----------|-----------------|---------|
| 0 | 4 | 4 each | Most layers, least color |
| 1 | 3 | 16, 16, 4 | Most common game mode |
| 2 | 2 | 16 each | Column scroll per tile |
| 3 | 2 | 256, 16 | Direct color available |
| 4 | 2 | 256, 4 | Column scroll per tile |
| 5 | 2 | 16, 4 | 512px hi-res mode |
| 6 | 1 | 16 | 512px + column scroll |
| 7 | 1 | 256 | **Rotation/scaling** |

### Color Math

SNES can blend two BG layers mathematically:
- **Add**: Layer colors added (creates glow/light effects)
- **Subtract**: Layer colors subtracted (creates shadow/dark effects)
- **Half**: Result divided by 2 (softer blending)

This enabled transparency, water effects, and shadow overlays without sprite tricks.

### Mode 7 Details

Single 1024×1024 pixel affine-transformed background layer:
- Rotation, scaling, skewing per scanline
- Creates pseudo-3D effects (F-Zero roads, Mario Kart tracks)
- 256 colors from a single palette
- No tile flipping — each orientation needs a separate tile

## Game Boy Advance

### Color System

- **15-bit RGB**: 32,768 possible colors
- **Background palettes**: 16 palettes × 16 colors = 256 BG colors
- **Sprite palettes**: 16 palettes × 16 colors = 256 sprite colors
- **Bitmap modes**: Full 240×160 framebuffer at 15-bit color

### Sprite System

- **Sizes**: 8×8 to 64×64 in various aspect ratios
- **Max sprites**: 128 OAM entries
- **Max per scanline**: 128 (but limited by pixel bandwidth)
- **Affine sprites**: Hardware rotation/scaling per sprite
- **Mosaic effect**: Pixelate sprites by hardware

### Background Modes

- **Mode 0**: 4 tiled BG layers (most common for RPGs)
- **Mode 1**: 2 tiled + 1 affine BG (parallax + rotation)
- **Mode 2**: 2 affine BG layers
- **Mode 3**: Single 240×160 16-bit bitmap
- **Mode 4**: Single 240×160 8-bit palettized bitmap (double-buffered)
- **Mode 5**: Single 160×128 16-bit bitmap (double-buffered, smaller)

### Art Implications

- GBA's generous palette (512 simultaneous colors) allowed near-SNES quality
- Smaller screen (240×160 vs SNES 256×224) meant tighter compositions
- No brightness control on original hardware — artists designed for unlit screens
- Affine sprites enabled rotation effects impossible on SNES
