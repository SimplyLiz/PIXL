---
title: "Popular Pixel Art Palettes: PICO-8, DB16, DB32, Endesga"
source: "https://lospec.com/palette-list/pico-8"
topic: "palettes"
fetched: "2026-03-23"
---

# Popular Pixel Art Palettes

## PICO-8 (16 colors)

The fantasy console's fixed palette. Every pixel is a 4-bit value — only 16 colors possible per frame.

### Default Palette

| # | Name | Hex | Use |
|---|------|-----|-----|
| 0 | Black | #000000 | Background, outlines |
| 1 | Dark Blue | #1D2B53 | Night sky, deep water |
| 2 | Dark Purple | #7E2553 | Shadows, dark magic |
| 3 | Dark Green | #008751 | Foliage, grass shadow |
| 4 | Brown | #AB5236 | Wood, earth, skin dark |
| 5 | Dark Gray | #5F574F | Stone, metal shadow |
| 6 | Light Gray | #C2C3C7 | Metal, fog, UI |
| 7 | White | #FFF1E8 | Highlights, text (warm white) |
| 8 | Red | #FF004D | Danger, blood, fire |
| 9 | Orange | #FFA300 | Fire, warm light, fruit |
| 10 | Yellow | #FFEC27 | Gold, bright light, energy |
| 11 | Green | #00E436 | Grass, slime, health |
| 12 | Blue | #29ADFF | Sky, water, ice |
| 13 | Lavender | #83769C | Indigo, twilight, mystic |
| 14 | Pink | #FF77A8 | Flowers, soft glow |
| 15 | Peach | #FFCCAA | Skin, sand, warm neutral |

### Secret Palette (16 additional)

PICO-8 has 16 hidden colors (128–143) that can be swapped in via `pal()`. Only 16 total on screen at once. The secret palette fills gaps: darker reds, more greens, blue-grays, and flesh tones.

### Design Properties

- Warm white (#FFF1E8) instead of pure white — everything feels warm
- High-saturation primaries (8–12) for foreground pop
- Low-saturation darks (1–5) for backgrounds
- Good contrast between adjacent indices for UI readability
- The palette deliberately avoids true gray — light gray and dark gray both carry subtle warmth

## DawnBringer 16 (DB16)

Created by DawnBringer (Richard Fhager). A carefully balanced 16-color palette.

### Hex Values

```
#140C1C  #442434  #30346D  #4E4A4F
#854C30  #346524  #D04648  #757161
#597DCE  #D27D2C  #8595A1  #6DAA2C
#D2AA99  #6DC2CA  #DAD45E  #DEEED6
```

### Properties
- Darker overall than PICO-8 — more "gritty" aesthetic
- Better gray/neutral range for stone and metal
- Strong warm-cool balance
- Popular for game jams and retro-styled games

## DawnBringer 32 (DB32)

Extended version with 32 colors. Doubles the ramp depth — smoother gradients possible.

### Properties
- Two-step ramps for every major hue
- Dedicated skin tones (3 values)
- Better vegetation range (4 greens)
- Sufficient for detailed character sprites at 32×32

## Endesga 32

Created by Endesga (Humberto Giambasti). Modern 32-color palette designed for versatility.

### Properties
- Wider hue range than DB32
- Better saturation control — vivid accents + muted naturals
- Strong hue-shifting in ramps (shadows go cool, highlights go warm)
- Popular in modern indie games
- Excellent for both environments and characters

## Choosing a Palette

### By Color Count

- **4 colors**: Game Boy homage, extreme constraint. Forces creativity.
- **8 colors**: Tight but workable. One ramp per major hue.
- **16 colors**: Sweet spot for small games. PICO-8/DB16.
- **32 colors**: Rich enough for detailed work. DB32/Endesga 32.
- **64 colors**: Luxurious. Rarely needed — indicates possible over-engineering.

### By Mood

- **PICO-8**: Warm, friendly, cartoonish
- **DB16**: Moody, earthy, retro
- **DB32**: Balanced, natural, versatile
- **Endesga 32**: Vibrant, modern indie feel

### Practical Tips

- Start with an existing palette rather than creating one from scratch
- Palette swapping is trivial in pixel art — commit to a palette early, change later if needed
- Test your palette on a simple scene (sky + ground + character) before committing
- Lospec.com hosts 1000+ curated pixel art palettes with sorting by color count
