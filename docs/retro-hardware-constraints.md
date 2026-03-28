# Retro Gaming Hardware Constraints for Pixel Art Creation

A comprehensive technical reference covering the exact hardware specifications, artistic constraints, and pixel art techniques for major retro gaming platforms. Intended as an AI knowledge base for generating hardware-authentic pixel art.

---

## Table of Contents

1. [NES (Nintendo Entertainment System)](#1-nes-nintendo-entertainment-system)
2. [Game Boy (DMG)](#2-game-boy-dmg)
3. [SNES (Super Nintendo)](#3-snes-super-nintendo)
4. [GBA (Game Boy Advance)](#4-gba-game-boy-advance)
5. [Commodore 64](#5-commodore-64)
6. [Modern Retro Constraints](#6-modern-retro-constraints)
7. [Quick Reference Table](#7-quick-reference-table)

---

## 1. NES (Nintendo Entertainment System)

### 1.1 Core Specifications

| Property | Value |
|---|---|
| Resolution | 256x240 pixels (NTSC); 256x224 visible (top/bottom 8 lines typically hidden) |
| PPU chip | Ricoh 2C02 (NTSC) / 2C07 (PAL) |
| VRAM | 2 KB internal (CIRAM) for nametables |
| CHR memory | 8 KB ROM/RAM on cartridge for pattern tables (tile graphics) |
| OAM | 256 bytes (64 sprites x 4 bytes each) |
| Master palette | 52-54 usable colors from a 64-entry lookup (not RGB-defined; generated via NTSC signal) |
| PPU clock | 3 PPU cycles per 1 CPU cycle (NTSC) |

### 1.2 Color System

The NES does not use RGB color mixing. Its 64-entry palette is hardwired into the PPU and generates colors via NTSC composite signal encoding. Of the 64 entries, approximately 52-54 produce unique visible colors (some are duplicates of black, and column $xD produces "blacker than black" which can damage CRTs).

**Palette RAM layout (32 bytes at PPU $3F00-$3F1F):**

- **Background palettes (16 bytes):** 4 palettes of 4 colors each
  - $3F00: Universal background color (shared across all BG palettes as index 0)
  - $3F01-$3F03: BG palette 0, colors 1-3
  - $3F05-$3F07: BG palette 1, colors 1-3
  - $3F09-$3F0B: BG palette 2, colors 1-3
  - $3F0D-$3F0F: BG palette 3, colors 1-3

- **Sprite palettes (16 bytes):** 4 palettes of 4 colors each
  - Sprite palette index 0 is always transparent (mirrors $3F00 but never rendered)
  - $3F11-$3F13: Sprite palette 0, colors 1-3
  - $3F15-$3F17: Sprite palette 1, colors 1-3
  - $3F19-$3F1B: Sprite palette 2, colors 1-3
  - $3F1D-$3F1F: Sprite palette 3, colors 1-3

**Total simultaneous colors:** 25 maximum (1 shared background + 4x3 BG colors + 4x3 sprite colors). In practice, many games use fewer.

### 1.3 Background System

**Tiles (Pattern Tables):**
- Fixed 8x8 pixel tiles
- 2 bits per pixel (2bpp), stored as two bitplanes of 8 bytes each = 16 bytes per tile
- Two pattern tables of 256 tiles each (4 KB per table), one for BG and one for sprites (configurable)
- Total: 512 tile definitions available at once (mappers can bank-switch for more)

**Nametables (Tile Maps):**
- Each nametable: 1024 bytes (960 bytes tile indices + 64 bytes attribute table)
- Grid: 32 columns x 30 rows of 8x8 tiles = 256x240 pixels
- Four logical nametables arranged 2x2, but only 2 KB physical CIRAM (two physical nametables)
- Mirroring (horizontal or vertical) determined by cartridge hardware
- Each byte selects one of 256 tiles from the active pattern table

**Attribute Tables (Color Assignment):**
- 64 bytes appended to each nametable (at offset $3C0)
- Each byte controls palette assignment for a 32x32 pixel area (4x4 tiles)
- Each byte is subdivided into four 2-bit fields, each controlling a 16x16 pixel quadrant (2x2 tiles)
- This means palette can only change at 16x16 pixel granularity for backgrounds
- This is the single biggest constraint on NES background art

```
Each attribute byte:
  7654 3210
  |||| ||++- Palette for top-left     2x2 tile area (16x16 px)
  |||| ++--- Palette for top-right    2x2 tile area (16x16 px)
  ||++------ Palette for bottom-left  2x2 tile area (16x16 px)
  ++-------- Palette for bottom-right 2x2 tile area (16x16 px)
```

### 1.4 Sprite System

| Property | Value |
|---|---|
| Total sprites | 64 (stored in 256-byte OAM) |
| Sprite sizes | 8x8 or 8x16 pixels (global setting, all sprites same mode) |
| Per-scanline limit | 8 sprites; additional sprites on same scanline are dropped |
| Colors per sprite | 3 colors + transparent (from one of 4 sprite palettes) |
| Flipping | Horizontal and vertical flip per sprite |
| Priority | Behind or in front of background (per sprite) |

**OAM entry format (4 bytes per sprite):**
- Byte 0: Y position (top of sprite, offset by 1; $00 = row -1, invisible)
- Byte 1: Tile index number
- Byte 2: Attributes (palette select, priority, flip flags)
- Byte 3: X position (left edge)

**The 8-sprite scanline limit** is the most visible constraint. When exceeded, lower-priority sprites vanish. Games combat this by rotating sprite evaluation order each frame, producing characteristic "flickering" rather than permanent disappearance.

### 1.5 Pixel Art Techniques and Famous Examples

**Mega Man (1987-onward):**
- Mega Man's body is composed of ~10 individual 8x8 sprites, allowing different sprite tiles to use different palettes
- His face tile uses a different palette (skin color + white eyes) than his body (blue + dark blue)
- This multi-palette sprite compositing trick creates the illusion of 6+ colors on a single character
- Different weapon selections swap only the body palette, keeping face palette constant

**Castlevania (1986):**
- Environment art designed around 16x16 metatile units to align with attribute table boundaries
- Simple tile variations (e.g., stone blocks with slight pattern changes) create visual richness cheaply
- Whip attack animation uses "smear" frames (stretched motion blur pixels) for dynamic feel
- Crouch frame reused as jump frame to save CHR ROM space
- Art designed for CRT softening; pixel clusters that look harsh on modern displays blended naturally on period hardware

**Super Mario Bros. 3 (1988):**
- Uses MMC3 mapper for CHR bank switching, giving access to far more than 512 tiles
- Background/foreground layering: Mario walks "behind" white blocks using sprite priority bit
- World map and in-level graphics use separate CHR banks
- Attribute table edge artifacts visible on screen right during scrolling (16x16 palette grid doesn't align perfectly with 8-pixel scroll increments)
- Status bar at top rendered with sprites to avoid nametable scroll conflicts

**General NES Art Principles:**
- Silhouette-first design: at 8x8 to 16x32 pixel character sizes, readability depends on distinctive outlines
- Black or near-black outlines around characters are near-universal for contrast
- Backgrounds designed in 16x16 metatile units to respect attribute table boundaries
- Color palette planning is architectural: artists choose palettes before drawing, assigning each to 16x16 screen regions
- Tile reuse is critical: a well-designed NES tileset creates maximum visual variety from 256 unique 8x8 patterns
- Horizontal sprite flickering is a feature, not a bug: it ensures all gameplay-critical sprites remain partially visible

### 1.6 Key Constraints for Modern Artists Targeting NES Feel

1. Limit background tiles to 3 colors + 1 shared background per 16x16 area
2. Limit sprites to 3 colors + transparent; build large characters from multiple 8x8 sprites
3. Design backgrounds on a 16x16 grid for palette changes (the attribute table constraint)
4. Total on-screen palette: ~25 colors maximum
5. Think in 8x8 tile units; every background pixel belongs to a reusable tile
6. Embrace the grid: NES art has a distinctive "blocky" quality from the 16x16 palette regions
7. CRT-era art: slight softness and color bleed are period-accurate

---

## 2. Game Boy (DMG)

### 2.1 Core Specifications

| Property | Value |
|---|---|
| Resolution | 160x144 pixels |
| Refresh rate | 59.7 Hz |
| Colors | 4 shades of grey (displayed as green-tinted on DMG LCD) |
| PPU | Integrated in DMG-CPU SoC (Sharp LR35902) |
| VRAM | 8 KB (single bank) |
| OAM | 160 bytes (40 sprites x 4 bytes each), internal to PPU |
| Tile size | 8x8 pixels, 16 bytes per tile (2bpp) |

### 2.2 Color/Shade System

The DMG displays 4 shades, typically described as:
- Shade 0: Lightest (white/light green)
- Shade 1: Light grey
- Shade 2: Dark grey
- Shade 3: Darkest (black/dark green)

**Palette registers (8-bit each, remapping shade indices to display shades):**
- **BGP ($FF47):** Background & Window palette. Maps tile color indices 0-3 to display shades.
- **OBP0 ($FF48):** Object/Sprite palette 0. Index 0 is always transparent.
- **OBP1 ($FF49):** Object/Sprite palette 1. Index 0 is always transparent.

Each palette register encodes 4 shade mappings in 2-bit pairs:
```
Bits 7-6: Color for index 3
Bits 5-4: Color for index 2
Bits 3-2: Color for index 1
Bits 1-0: Color for index 0
```

This allows palette swapping without modifying tile data. A single tile drawn with different palette registers can appear as 4 different shade combinations.

### 2.3 Tile and VRAM System

**Tile storage:**
- Each tile: 8x8 pixels, 2 bits per pixel, stored as 16 bytes (two interleaved bitplanes)
- VRAM holds up to 384 tiles in three 128-tile blocks ($8000-$87FF, $8800-$8FFF, $9000-$97FF)
- Two addressing modes:
  - "$8000 method" (unsigned): tiles 0-255 at $8000-$8FFF (sprites always use this)
  - "$8800 method" (signed): tiles -128 to 127, base at $9000 (BG/Window can use this)
- Blocks 0 and 2 are non-overlapping; block 1 is shared between both addressing modes

**Background layer:**
- 256x256 pixel virtual map (32x32 tiles), of which 160x144 is visible
- Scrollable via SCX/SCY registers (pixel-level precision)
- Single tile map at $9800 or $9C00 (1024 bytes: 32x32 tile indices)
- Uses BGP palette exclusively

**Window layer:**
- Up to 160x144 pixels, positioned via WX/WY registers
- Fixed position overlay (cannot scroll independently like BG)
- Drawn on top of background; commonly used for HUD elements (life bars, score)
- Uses its own tile map area
- Shares BGP palette with background

### 2.4 Sprite System

| Property | Value |
|---|---|
| Total sprites | 40 (in OAM) |
| Per-scanline limit | 10 sprites |
| Sprite sizes | 8x8 or 8x16 pixels (global setting) |
| Colors per sprite | 3 shades + transparent (from OBP0 or OBP1) |
| Palettes | 2 sprite palettes (OBP0, OBP1) |
| Flipping | Horizontal and vertical flip per sprite |
| Priority | Behind BG (shade 1-3) or in front |

**OAM entry (4 bytes):**
- Byte 0: Y position (offset: actual Y = stored value - 16)
- Byte 1: X position (offset: actual X = stored value - 8)
- Byte 2: Tile index
- Byte 3: Attributes (priority, Y-flip, X-flip, palette select)

**OAM DMA:** CPU can bulk-transfer 160 bytes from main RAM to OAM via DMA register ($FF46), taking 160 cycles. CPU is locked out of all memory except HRAM during transfer.

### 2.5 Pixel Art Techniques

**Maximizing 4 Shades:**
- Treat the 4 shades as a full value range: pure highlight, light mid-tone, dark mid-tone, deep shadow
- Reserve shade 0 (lightest) for specular highlights and sky; shade 3 (darkest) for outlines and deep shadows
- Use shades 1 and 2 as your primary "drawing" tones for form and volume
- Strong contrast between adjacent shades is essential since there are only 4 levels

**Dithering:**
- Checkerboard dithering of shades 1 and 2 creates an effective "shade 1.5" at distance
- Ordered dithering patterns (horizontal lines, diagonal stripes) suggest textures: brick, wood, cloth
- At 160x144 resolution, dithering must be used sparingly; large dither fields read as noise
- Dithering works best in background areas; foreground characters should use solid fills for readability

**Tile Reuse Strategies:**
- The 160x144 visible screen is 20x18 tiles; a typical scene needs only 50-100 unique tiles
- Flip flags (H/V) on sprites effectively quadruple visual variety per tile
- Symmetrical designs are memory-efficient: half the character facing one way, flip for the other
- Environmental tiles designed as modular kits: corner, edge, fill patterns that combine into varied structures

**Famous Examples:**
- *The Legend of Zelda: Link's Awakening* (1993): Exceptional detail within 4 shades; heavy use of shade 2 for shadows that give depth to the overworld; tile reuse creates a cohesive but varied island
- *Metroid II: Return of Samus* (1991): Dark, atmospheric environments using shade 3 dominantly; minimal use of lightest shade creates oppressive feel; demonstrates how shade distribution controls mood
- *Kirby's Dream Land* (1992): Light, airy feel through dominant use of shades 0-1; shade 3 reserved for outlines only; shows opposite approach to Metroid II
- *Pokemon Red/Blue* (1996): Sprite design masterclass in 8x8 space; 151 distinct creature silhouettes readable at tiny sizes

### 2.6 Key Constraints for Modern Artists Targeting Game Boy Feel

1. Exactly 4 shades (no more, no less); they do not have to be green but traditionally are
2. 160x144 canvas, 8x8 tile grid
3. Background has 1 palette; sprites have 2 palettes of 3 visible shades each
4. Max 10 sprites per scanline; 40 total
5. Window layer overlays BG; useful for fixed UI
6. Tile-based: every background pixel is part of a reusable 8x8 tile
7. Dithering is authentic but must be deliberate at this low resolution
8. The green-tinted LCD is iconic but not mandatory; Super Game Boy mapped to full color palettes

---

## 3. SNES (Super Nintendo)

### 3.1 Core Specifications

| Property | Value |
|---|---|
| Resolution | 256x224 (standard); 512x224 (hi-res); 256x239 or 512x239 (overscan) |
| PPU | Two custom chips (PPU1 and PPU2) |
| VRAM | 64 KB (16-bit access, addresses $0000-$7FFF) |
| CGRAM | 512 bytes (256 colors x 15-bit RGB) |
| OAM | 544 bytes (512 + 32 bytes for size/X-MSB table) |
| Color depth | 15-bit (5 bits per channel: 32,768 possible colors) |
| Max on-screen colors | 256 (from CGRAM) |

### 3.2 Color System

**CGRAM (Color Generator RAM):**
- 256 entries of 15-bit color (5 bits red, 5 bits green, 5 bits blue)
- Format: `0bbbbbgg gggrrrrr` (little-endian)
- Organized as:
  - BG palettes: entries 0-127 (usage varies by mode)
  - Sprite palettes: entries 128-255 (8 palettes of 16 colors; index 0 in each is transparent)
- Color 0 of BG palette 0 is the universal "backdrop" color

**Palette organization depends on BG mode:**
- 2bpp layers: 8 palettes of 4 colors each
- 4bpp layers: 8 palettes of 16 colors each
- 8bpp layers: 1 palette of 256 colors (or "direct color" mode bypasses CGRAM entirely)

### 3.3 Background Modes

| Mode | BG1 | BG2 | BG3 | BG4 | Notes |
|---|---|---|---|---|---|
| 0 | 2bpp (4 color) | 2bpp (4 color) | 2bpp (4 color) | 2bpp (4 color) | 4 layers, each with 8 dedicated 4-color palettes |
| 1 | 4bpp (16 color) | 4bpp (16 color) | 2bpp (4 color) | -- | Most commonly used mode; BG3 has switchable priority |
| 2 | 4bpp (16 color) | 4bpp (16 color) | offset-per-tile | -- | BG3 provides per-tile scroll offsets for BG1/BG2 |
| 3 | 8bpp (256 color) | 4bpp (16 color) | -- | -- | Direct color mode available on BG1 |
| 4 | 8bpp (256 color) | 2bpp (4 color) | offset-per-tile | -- | Combines full-color BG with offset-per-tile |
| 5 | 4bpp (16 color) | 2bpp (4 color) | -- | -- | Hi-res mode (512 pixels wide, interlaced rendering) |
| 6 | 4bpp (16 color) | offset-per-tile | -- | -- | Hi-res with offset-per-tile |
| 7 | 8bpp (256 color) | -- | -- | -- | Rotation/scaling transforms; 128x128 tile map; 256 tiles; EXTBG adds a 128-color BG2 |

**Tile maps:**
- 32x32 tiles per screen block (2 KB each)
- Configurable to 32x32, 64x32, 32x64, or 64x64 tiles
- Each tilemap entry: 16 bits (tile number 0-1023, palette, priority, H/V flip)
- 1024 unique tile definitions available (vs NES's 256)
- Tiles can be 8x8 or 16x16 (per-layer setting)

### 3.4 Sprite System

| Property | Value |
|---|---|
| Total sprites | 128 |
| Per-scanline limit | 32 sprites or 34 tiles (8x8 units), whichever reached first |
| Color depth | Always 4bpp (16 colors per palette) |
| Palettes | 8 sprite palettes of 15 colors + transparent each |
| Tile access | Up to 512 sprite tiles (from a 16 KB VRAM region) |
| Sizes | Two sizes active at once, chosen from: 8x8, 16x16, 32x32, 64x64 |
| Priority | 4 levels (can layer between BG layers) |

**OAM format:**
- Main table: 512 bytes (128 entries x 4 bytes: X[7:0], Y, tile, attributes)
- Auxiliary table: 32 bytes (128 entries x 2 bits: X MSB + size-select)
- Attributes include: palette, priority, H/V flip, tile page

### 3.5 Color Math (Transparency/Blending)

The SNES supports hardware color blending between layers, a major advancement over the NES.

**Operations:**
- **Addition:** Adds main screen + sub screen color values per channel (lightens)
- **Subtraction:** Subtracts sub screen from main screen per channel (darkens)
- **Add + Half:** Averages main and sub screen (true 50% transparency)
- **Subtract + Half:** Subtracts then halves (deep darkening, rarely used)

**Configuration:**
- Any BG layer or sprites can be assigned to main screen or sub screen
- Fixed color register allows blending against a single solid color (useful for fade effects, fog)
- HDMA can update the fixed color per-scanline, creating gradient effects (e.g., Mode 7 horizon fading)
- Windowing can restrict where color math applies on screen

**Sprite color math constraints:**
- Sprites using palettes 0-3 (CGRAM 128-191) are exempt from color math
- Sprites cannot blend against other sprites, only against BG layers
- This is why HUD elements often use sprite palettes 0-3: they remain opaque during screen-wide transparency effects

### 3.6 Mode 7

- Single 256-color background layer with hardware rotation, scaling, and skewing
- Tile map: 128x128 tiles from a set of 256 unique 8x8 tiles
- Transformations defined by a 2x2 affine matrix, applied per-scanline for perspective effects (HDMA)
- Used for: F-Zero track, Super Mario Kart ground plane, FF6 airship, Pilotwings terrain
- EXTBG option splits the 8bpp data into two layers: 128-color BG1 + 128-color BG2 (bit 7 as priority)
- No per-tile flipping in Mode 7 (unlike other modes)
- Tile data and map share the same 64 KB VRAM space, competing for memory

### 3.7 Pixel Art Techniques

**Multi-layer parallax:**
- Mode 1's three BG layers enable foreground detail + main playfield + distant background
- Each layer scrolls independently for parallax depth (e.g., Donkey Kong Country's jungle layers)
- BG3 in Mode 1 often used for semi-transparent overlay effects (e.g., fog, rain)

**Color math for atmosphere:**
- Chrono Trigger: rain effects using additive blending on sprites
- Super Metroid: underwater tinting via subtractive color math on the sub screen
- Final Fantasy VI: Mode 7 combined with HDMA color gradient for the world map horizon

**16-color-per-tile workflow:**
- Mode 1 BG1/BG2 give 16 colors per tile from 8 selectable palettes
- Compared to NES's 4 colors per 16x16 region, this is a massive upgrade
- Artists could use different palettes per 8x8 tile (no attribute table grid constraint)
- Palette-swapped enemies became even more versatile: same tiles, different 16-color palette

**Sprite compositing:**
- 128 sprites with 15 colors each, 4 priority levels
- Large boss characters built from dozens of sprites, each potentially using a different palette
- Priority layering between BG layers creates depth (sprite behind BG1 but in front of BG2)

### 3.8 Key Constraints for Modern Artists Targeting SNES Feel

1. 256-color CGRAM total; typically 128 for BG, 128 for sprites
2. Tile-based: 8x8 or 16x16 tiles, up to 1024 unique tile definitions
3. Mode 1 is the canonical "SNES look": two 16-color layers + one 4-color overlay
4. Palette per 8x8 tile (not per 16x16 area like NES) -- much more flexible
5. 15-bit color (32K possible colors, but only 256 on screen at once)
6. Color math enables transparency, shadows, and lighting impossible on NES
7. Mode 7 rotation/scaling for pseudo-3D ground planes
8. Richer sprites: 15 colors + transparent, 128 objects, larger sizes up to 64x64

---

## 4. GBA (Game Boy Advance)

### 4.1 Core Specifications

| Property | Value |
|---|---|
| Resolution | 240x160 pixels |
| Color depth | 15-bit (32,768 colors) |
| VRAM | 96 KB (64 KB backgrounds + 32 KB sprites) |
| Palette RAM | 1 KB (256 BG colors + 256 sprite colors, 15-bit each) |
| OAM | 1 KB (128 sprites x 8 bytes each) |
| CPU | ARM7TDMI (32-bit, 16.78 MHz) |
| Refresh rate | ~59.7 Hz |

### 4.2 Video Modes

| Mode | Type | Layers | Description |
|---|---|---|---|
| 0 | Tile | 4 static BG layers | Most versatile tile mode; all layers are regular (no affine) |
| 1 | Tile | 3 layers (2 static + 1 affine) | Most commonly used; BG2 supports rotation/scaling |
| 2 | Tile | 2 affine layers | Both layers support rotation/scaling; no static layers |
| 3 | Bitmap | 1 layer | Full 240x160 at 16bpp (direct color); 76.8 KB; no page flip |
| 4 | Bitmap | 2 pages | 240x160 at 8bpp (palette-indexed); 38.4 KB each; page flipping |
| 5 | Bitmap | 2 pages | 160x128 at 16bpp; 40 KB each; page flipping; reduced resolution |

**Tile mode details:**
- Tiles: 8x8 pixels in 4bpp (16 colors from one of 16 palettes) or 8bpp (256 colors from single palette)
- Tile storage: 32 bytes (4bpp) or 64 bytes (8bpp) per tile
- Charblocks: 16 KB continuous regions; 4 for backgrounds, 2 for sprites
- Max tiles per charblock: 512 (4bpp) or 256 (8bpp)
- Tile maps: screenblocks of 32x32 tiles (2 KB each)
- BG dimensions: up to 512x512 pixels (regular) or 1024x1024 pixels (affine)

**Bitmap mode constraints:**
- No hardware scrolling
- Only one background layer
- Sprite tile data overlaps with bitmap framebuffer in VRAM
- ~90% of commercial GBA games used tile modes, not bitmap modes

### 4.3 Affine Transformations

Available on BG layers in Modes 1-2 and on up to 32 sprites:
- Rotation (any angle)
- Scaling (enlarge/shrink)
- Shearing
- Defined by a 2x2 matrix (PA, PB, PC, PD) per affine source
- 32 affine parameter sets shared among affine sprites
- Per-scanline affine updates via DMA create Mode 7-like effects

### 4.4 Sprite System

| Property | Value |
|---|---|
| Total sprites | 128 |
| Max size | 64x64 pixels (128x128 with affine double-size flag) |
| Shapes | Square (8x8 to 64x64) and rectangular (8x16, 16x8, 8x32, 32x8, 16x32, 32x16, 32x64, 64x32) |
| Color modes | 4bpp (16 colors from one of 16 palettes) or 8bpp (256 colors) |
| Affine sprites | 32 affine parameter sets; sprites can rotate/scale independently |
| Features | H/V flip (non-affine only), semi-transparency, window masking |
| Priority | 4 levels |

### 4.5 Special Effects

- **Mosaic:** Pixelation effect configurable per BG layer and sprites (H/V block size 1-16)
- **Alpha blending:** Two layers blended with configurable coefficients (EVA/EVB, 0-16 each in 1/16 steps)
- **Brightness fade:** Fade to white or black with configurable coefficient
- **Windowing:** Two rectangular windows + OBJ window; per-window layer enable and effect enable
- **Green swap:** Undocumented mode swapping green channels between adjacent pixels (vestige of planned stereoscopic 3D)

### 4.6 Pixel Art Techniques

**Bridging pixel art and pre-rendered 3D:**
- The GBA's 32,768-color space and 256 simultaneous colors enabled pre-rendered sprites (a la Donkey Kong Country)
- Games like Castlevania: Circle of the Moon used large, detailed sprites with gradient shading impossible on SNES
- The line between "pixel art" and "downscaled renders" blurred on GBA

**Affine sprite tricks:**
- Rotation/scaling on sprites enabled effects like character spin attacks, rotating power-ups
- Affine BG layers used for pseudo-3D racing games (Mario Kart: Super Circuit) and map screens

**Palette management:**
- 16 sub-palettes of 16 colors (4bpp mode) allows 256 colors total but organized for tile efficiency
- Common pattern: characters use 4bpp sprites with dedicated palettes; shared environment palettes for tiles
- 8bpp mode used sparingly for cinematic screens or full-color backgrounds

**Notable GBA art:**
- *Metroid Fusion / Zero Mission*: Detailed sprite animation with extensive palette work; environments mix pre-rendered elements with hand-pixeled tiles
- *Golden Sun*: Pre-rendered character sprites combined with hand-pixeled environments; affine effects for spell animations
- *Final Fantasy Tactics Advance*: Isometric tile art with careful palette sharing across terrain types
- *Mother 3*: Deliberately retro aesthetic despite GBA capabilities; charming SNES-style pixel art by choice

### 4.7 Key Constraints for Modern Artists Targeting GBA Feel

1. 240x160 canvas at 15-bit color (32K possible, 256 on-screen)
2. Tile-based in most games: 8x8 tiles, 16 or 256 colors per tile
3. Richer than SNES: more sprites, affine transforms, larger palette space
4. The GBA "look" includes: smooth gradients, more anti-aliasing, larger sprites
5. Alpha blending and fade effects are hardware-native
6. Many GBA games deliberately emulated SNES aesthetics on superior hardware
7. Bitmap modes existed but were rarely used in commercial games

---

## 5. Commodore 64

### 5.1 Core Specifications

| Property | Value |
|---|---|
| Resolution | 320x200 (hires) or 160x200 (multicolor) |
| Display area | 40x25 characters (8x8 pixels each) |
| VIC-II chip | MOS 6567 (NTSC) / 6569 (PAL) |
| Color palette | 16 fixed colors (not user-definable) |
| Address space | 16 KB visible to VIC-II at a time (bank-switchable) |
| Color RAM | 1 KB x 4 bits ($D800-$DBE7) |
| Raster lines | 262 (NTSC) / 312 (PAL) |

### 5.2 The 16 Fixed Colors

The C64's palette is hardware-defined and cannot be changed:

| Index | Color | Index | Color |
|---|---|---|---|
| 0 | Black | 8 | Orange |
| 1 | White | 9 | Brown |
| 2 | Red | 10 | Light Red |
| 3 | Cyan | 11 | Dark Grey |
| 4 | Purple | 12 | Medium Grey |
| 5 | Green | 13 | Light Green |
| 6 | Blue | 14 | Light Blue |
| 7 | Yellow | 15 | Light Grey |

### 5.3 Graphics Modes

**Standard Character Mode (Text Mode)**
- Resolution: 320x200 (40x25 characters, 8x8 each)
- Colors per character cell: 2 (foreground from Color RAM + shared background)
- 256 unique character definitions (8 bytes each = 2 KB)
- Foreground color individually assignable per character via Color RAM

**Multicolor Character Mode**
- Resolution: 160x200 (effectively; pixels are double-wide)
- Colors per 4x8 character cell: 4
  - Bit pattern 00: Background color (register $D021, shared globally)
  - Bit pattern 01: Color from register $D022 (shared globally)
  - Bit pattern 10: Color from register $D023 (shared globally)
  - Bit pattern 11: Color from Color RAM (individual per character, limited to colors 0-7)
- 3 shared colors + 1 individual per cell, individual restricted to first 8 palette entries
- Trade-off: double-wide pixels (half horizontal resolution) in exchange for more colors

**Standard Bitmap Mode (Hires)**
- Resolution: 320x200 pixels (1 bit per pixel)
- Colors per 8x8 cell: 2 (foreground + background, both individually selectable)
- 8 KB bitmap data + 1 KB screen RAM (encodes 2 colors per cell)
- Each 8x8 pixel cell: unique foreground and background pair, but only 2 colors within that cell

**Multicolor Bitmap Mode**
- Resolution: 160x200 pixels (double-wide pixels)
- Colors per 4x8 cell: 4
  - Bit pattern 00: Background color (shared globally)
  - Bit pattern 01: From high nybble of screen RAM (per cell)
  - Bit pattern 10: From low nybble of screen RAM (per cell)
  - Bit pattern 11: From Color RAM (per cell)
- The most colorful standard mode: 3 unique colors per cell + 1 shared background
- Memory: 8 KB bitmap + 1 KB screen RAM + 1 KB color RAM

**Extended Background Color Mode (ECM)**
- Resolution: 320x200 (standard character resolution)
- Each character selects 1 of 4 background colors (registers $D021-$D024)
- Trade-off: only 64 unique characters available (upper 2 bits of char index select BG color)
- Foreground color from Color RAM (per character)

### 5.4 Sprite System

| Property | Value |
|---|---|
| Hardware sprites | 8 |
| Sprite size | 24x21 pixels (standard) or 12x21 (multicolor, double-wide pixels) |
| Memory per sprite | 63 bytes (3 bytes per row x 21 rows) |
| Colors (standard) | 1 color + transparent |
| Colors (multicolor) | 3 colors + transparent (2 shared globally + 1 individual) |
| Expansion | 2x horizontal and/or 2x vertical stretching (per sprite) |
| Priority | Sprite 0 highest through Sprite 7 lowest |
| Collision detection | Hardware sprite-sprite and sprite-background collision registers |
| Positioning | Full screen range, including border area |

**Sprite data pointers:** Located at screen RAM + $03F8-$03FF (8 bytes, one per sprite). Each selects a 64-byte-aligned block within the VIC-II's 16 KB address space.

### 5.5 Advanced Techniques

**Sprite Multiplexing:**
- Only 8 hardware sprites, but after a sprite has been fully drawn on its scanlines, its registers can be reprogrammed to display a new sprite further down the screen
- Requires raster interrupt timing: detect when a sprite finishes, change Y-position and data pointer
- Commonly used in shoot-em-ups and platformers to display 16-24+ moving objects
- Trade-off: multiplexed sprites cannot overlap vertically

**FLI (Flexible Line Interpretation):**
- Exploits a VIC-II quirk: forces the chip to re-read character pointers every scanline
- Result: multicolor bitmap with different Color RAM values per scanline within each cell
- Increases effective colors per cell from 4 per 4x8 to 4 per 4x1 (different set each line)
- Cost: first 3 characters per row are corrupted (24 pixels lost on left side)
- Memory intensive and CPU-heavy (requires precisely timed raster interrupts every scanline)

**IFLI (Interlaced FLI):**
- Combines FLI with interlacing: alternates between two slightly offset screens each frame
- On CRT displays, persistence of vision blends them, creating apparent color mixes
- Effectively doubles color resolution but produces visible flicker on modern displays

**NUFLI:**
- Advanced demoscene format combining FLI, sprite overlays, and sprite crunching
- Achieves near-photographic color fidelity at 320x200
- Sprites overlaid on the image area provide additional color information per scanline
- "Sprite crunching" exploits a VIC-II bug to repeat sprite data across the full screen height

**Sprite Crunching/Stretching:**
- A VIC-II hardware bug allows sprites to "stretch" vertically beyond 21 lines
- By manipulating the Y-expansion register at precise raster timing, the VIC loses track of which sprite line to render
- Demoscene technique only; never used in commercial games due to extreme complexity

### 5.6 Pixel Art Techniques

**Working with the 16-color palette:**
- The palette has good luminance distribution: 5 distinct brightness levels across the 16 colors
- Color pairs for shading: Blue/Light Blue, Green/Light Green, Red/Light Red, Dark Grey/Medium Grey/Light Grey
- Brown + Orange provides warm tones; Purple + Light Red for skin
- Black outlines are near-universal for character definition

**Multicolor mode art:**
- The double-wide pixel means characters have a chunky, distinctive look
- Artists lean into the blockiness: bold shapes, clear silhouettes
- Dithering with double-wide pixels creates effective gradients (4x8 cells are surprisingly expressive)
- Strategic use of the 3 shared colors: typically background, shadow, and highlight; individual color for character identity

**Hires mode art:**
- Full 320x200 resolution but only 2 colors per 8x8 cell
- Used for detailed line art, text-heavy screens, and high-contrast illustrations
- Color cell boundaries must be carefully managed to avoid "color clash" (attribute clash)
- Skilled artists plan compositions to align color changes with cell boundaries

### 5.7 Key Constraints for Modern Artists Targeting C64 Feel

1. 16 fixed colors (not user-definable); learn the specific C64 palette
2. Choose your trade-off: 320x200 with 2 colors per cell, or 160x200 with 4 colors per cell
3. Multicolor mode has double-wide pixels (2:1 aspect ratio on pixels)
4. Color assignment is per 8x8 cell (hires) or per 4x8 cell (multicolor); "attribute clash" is a defining characteristic
5. Only 8 sprites of 24x21 pixels; multiplexing allows more but with vertical separation
6. The C64 palette has a specific look: muted, slightly warm, with characteristic light blue and brown
7. Border color is separately controlled and can differ from main screen

---

## 6. Modern Retro Constraints

### 6.1 Why Constraints Improve Pixel Art

Hardware constraints are not obstacles to pixel art quality; they are the structural foundation that makes pixel art what it is. Constraints force artists to:

- **Prioritize readability:** With limited colors, every shade must serve a purpose
- **Design efficient silhouettes:** Small sprite sizes demand instantly recognizable shapes
- **Plan architecturally:** Palette regions, tile reuse, and metatile grids require upfront design thinking
- **Develop distinctive styles:** The NES, Game Boy, and C64 each have immediately recognizable aesthetics specifically because of their unique constraint profiles

Modern pixel art without any constraints often drifts into "low-res digital painting" -- technically competent but lacking the distinctive character that hardware limitations enforce.

### 6.2 PICO-8 (Fantasy Console)

| Property | Value |
|---|---|
| Resolution | 128x128 pixels |
| Colors | 16 fixed (with 16 secret extended colors accessible via palette swap) |
| Sprites | 8x8 base; 256 sprite slots (128 from sprite sheet, 128 from map sheet) |
| Map | 128x32 tiles (or 128x64 sharing sprite sheet memory) |
| Sound | 4 channels |
| Code limit | 8192 tokens or 65536 characters |
| Cart size | 32 KB |

**The 16-color default palette:**

| Index | Name | Hex |
|---|---|---|
| 0 | Black | #000000 |
| 1 | Dark Blue | #1D2B53 |
| 2 | Dark Purple | #7E2553 |
| 3 | Dark Green | #008751 |
| 4 | Brown | #AB5236 |
| 5 | Dark Grey | #5F574F |
| 6 | Light Grey | #C2C3C7 |
| 7 | White | #FFF1E8 |
| 8 | Red | #FF004D |
| 9 | Orange | #FFA300 |
| 10 | Yellow | #FFEC27 |
| 11 | Green | #00E436 |
| 12 | Blue | #29ADFF |
| 13 | Indigo/Lavender | #83769C |
| 14 | Pink | #FF77A8 |
| 15 | Peach | #FFCCAA |

The palette is designed with curated hue, saturation, and value distribution. It has warm and cool variants, good skin tones (15/peach, 4/brown), and multiple grey levels. It is intentionally non-uniform (not a mathematically generated ramp) to feel hand-picked and characterful.

**Palette constraints in practice:**
- No dithering at 128x128: pixels are too large and too few for dithering to read as gradation
- Color ramps must be chosen from the fixed 16: e.g., 1 (dark blue) -> 12 (blue) -> 6 (light grey) -> 7 (white) for a sky gradient
- The limited palette forces strong color choices and bold contrast

### 6.3 Other Common Retro Palettes

**CPC (Amstrad CPC):**
- 27 colors in hardware (3 levels of R, G, B = 3x3x3)
- Mode 0: 16 colors at 160x200; Mode 1: 4 colors at 320x200; Mode 2: 2 colors at 640x200

**MSX:**
- 15 predefined colors + transparent
- 256x192 resolution, 8x1 pixel color attribute granularity
- Significant attribute clash (similar to ZX Spectrum but at 8x1 instead of 8x8)

**ZX Spectrum:**
- 256x192 resolution, 15 colors (8 base x 2 brightness levels, minus one duplicate)
- Color attribute: 2 colors per 8x8 cell (foreground + background, each with bright flag)
- "Attribute clash" (or "color clash") is its defining visual characteristic

**Sega Master System:**
- 256x192, 64 colors from palette of 64 (6-bit: 2 bits per channel)
- Two 16-color palettes (BG + sprites)
- 8x8 tiles, up to 448 unique tiles
- No attribute table restriction (palette selectable per tile)

**Sega Genesis / Mega Drive:**
- 320x224, 512-color master palette (9-bit: 3 bits per channel)
- 4 palettes of 16 colors each (64 simultaneous) for both BG and sprites
- Two scrollable BG layers + sprite layer
- Highlight/shadow mode doubles effective palette

### 6.4 How Modern Indies Choose Constraints

**Strict hardware emulation:**
- *Shovel Knight*: Targets NES-like constraints but relaxes specific limits (more sprites per scanline, relaxed palette rules, widescreen)
- Publicly documents which NES rules it follows and which it breaks
- Philosophy: capture the "feel" without punishing the player with actual hardware limitations

**Palette-driven aesthetics:**
- *Celeste*: Low-res pixel art (8x8 character base) with carefully limited palette per scene, but not bound to any specific hardware
- Uses modern color theory within retro resolution constraints
- Smooth sub-pixel animation that would be impossible on real retro hardware

**Fantasy console constraints:**
- PICO-8 games (Celeste classic, etc.) adopt the 128x128, 16-color constraint as a complete creative framework
- TIC-80: similar to PICO-8 but 240x136, 16 colors, more permissive
- The constraint set itself becomes the art style

**Resolution as identity:**
- Many modern indie games choose a "native" pixel resolution (e.g., 320x180, 384x216) and upscale
- The resolution choice defines character sizes: 16x16, 32x32, or 48x48 base sprites
- Higher pixel resolution allows more detail but less of the "chunky" retro feel

**Common modern retro palette strategies:**
- Use established palettes: PICO-8, Endesga 32, Resurrect 64, AAP-64
- Limit to 16-32 colors for "8-bit feel" or 48-64 for "16-bit feel"
- Design palette with clear value ramps and hue shifts (not just desaturated variations)
- Include dedicated skin tones, foliage greens, and sky blues for common game art needs

### 6.5 What Makes Retro Pixel Art "Feel Right"

The authenticity of retro pixel art comes from specific constraint interactions, not just low resolution:

1. **Tile-based construction:** Real retro art is modular. Backgrounds are assembled from reusable 8x8 or 16x16 tiles. This creates subtle repetition and grid alignment that is part of the aesthetic.

2. **Per-cell palette limits:** The NES attribute table, C64 color cells, and Spectrum attributes all force color decisions at a grid level coarser than individual pixels. This creates characteristic "color regions" in backgrounds.

3. **Limited sprite colors:** 3-4 colors per sprite forces bold, high-contrast character design with clear silhouettes.

4. **Sprite size limitations:** Characters built from multiple small sprites have subtly different alignment and priority, creating the layered look of multi-sprite composites.

5. **Scanline-aware design:** Real hardware renders line by line. Sprite limits per scanline, raster effects, and mid-frame palette changes all create artifacts that are part of the retro identity.

6. **CRT display characteristics:** Period hardware displayed on CRT televisions with inherent softening, slight bloom on bright colors, and scanline gaps. Pixel art designed for CRT looks intentionally different from art designed for sharp LCD display.

---

## 7. Quick Reference Table

| Platform | Resolution | Colors (palette) | Colors (on-screen) | Tile size | Max sprites | Sprite colors | BG palette granularity |
|---|---|---|---|---|---|---|---|
| NES | 256x240 | ~54 | 25 | 8x8 | 64 (8/scanline) | 3+transparent | 16x16 pixel regions |
| Game Boy | 160x144 | 4 shades | 4 shades | 8x8 | 40 (10/scanline) | 3+transparent | Whole screen (1 palette) |
| SNES | 256x224 | 32,768 | 256 | 8x8 or 16x16 | 128 (32/scanline) | 15+transparent | Per 8x8 tile |
| GBA | 240x160 | 32,768 | 512 (256+256) | 8x8 | 128 | 15 or 255+transparent | Per 8x8 tile |
| C64 (multi) | 160x200 | 16 fixed | 16 | 4x8 cell | 8 | 3+transparent | 4x8 pixel cells |
| C64 (hires) | 320x200 | 16 fixed | 16 | 8x8 cell | 8 | 1+transparent | 8x8 pixel cells |
| PICO-8 | 128x128 | 16 fixed | 16 | 8x8 | 128 (software) | 15+transparent | Per sprite (no BG palette grid) |

---

## Sources

- NESdev Wiki: PPU, PPU palettes, PPU nametables, PPU attribute tables, PPU OAM (https://www.nesdev.org/wiki/)
- SNESdev Wiki: Backgrounds, Color math, SNES PPU for NES developers (https://snes.nesdev.org/wiki/)
- Pan Docs / GBdev (https://gbdev.io/pandocs/)
- Copetti Architecture of Consoles: Game Boy, GBA (https://www.copetti.org/writings/consoles/)
- TONC: GBA programming reference (https://www.coranac.com/tonc/text/)
- VIC-II reference by Christian Bauer (https://www.zimmers.net/cbmpics/cbm/c64/vic-ii.txt)
- C64-Wiki: VIC, Graphics Modes (https://www.c64-wiki.com/wiki/)
- dustmop.io: NES Graphics series (https://www.dustmop.io/blog/2015/06/08/nes-graphics-part-2/)
- SLYNYRD Pixelblog: Castlevania Study (https://www.slynyrd.com/blog/2022/3/19/pixelblog-37-classic-castlevania-study)
- Mega Cat Studios: Creating NES Graphics, Super Nintendo Graphics Guide (https://megacatstudios.com/blogs/retro-development/)
- Lospec palette database (https://lospec.com/palette-list/)
