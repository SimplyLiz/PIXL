# PAX Backdrop Format Extension

Specification for large animated backgrounds stored as tile-decomposed scenes
in the PAX format.

**Status:** v0.1 draft
**Depends on:** PAX base format (see `docs/specs/pax.md`)

---

## Table of Contents

1. [Overview](#1-overview)
2. [Extended Palette (`palette_ext`)](#2-extended-palette)
3. [Backdrop Tile (`backdrop_tile`)](#3-backdrop-tile)
4. [Backdrop Scene (`backdrop`)](#4-backdrop-scene)
5. [Animation Zones (`backdrop.*.zone`)](#5-animation-zones)
6. [Extended RLE Encoding](#6-extended-rle-encoding)
7. [Size Budget Analysis](#7-size-budget-analysis)
8. [Validation Rules](#8-validation-rules)

---

## 1. Overview

The base PAX format handles sprites and tiles up to 16 colors using
single-character palette symbols. Backdrops extend this with:

- **Extended palettes** supporting 17-48 colors via multi-character symbols.
- **Backdrop tiles** -- lightweight tiles without edge constraints or WFC
  metadata.
- **Backdrop scenes** -- tilemap-based compositions with procedural animation
  zones.

All backdrop sections are optional. A PAX file may contain any combination of
base-format and backdrop-format sections.

---

## 2. Extended Palette (`palette_ext`)

### 2.1 Purpose

Standard `[palette.*]` sections use single-character symbols (`a`-`z`, `0`-`9`,
etc.) and are limited to 16 entries. Extended palettes add symbols for colors
17-48, using multi-character identifiers.

### 2.2 Section Format

```toml
[palette_ext.night]
base = "night"
"2a" = "#102040ff"
"2b" = "#182848ff"
"2c" = "#203050ff"
"2d" = "#284060ff"
```

### 2.3 Fields

| Field    | Type   | Required | Description                                      |
|----------|--------|----------|--------------------------------------------------|
| `base`   | string | yes      | Name of the `[palette.*]` section this extends    |
| `"XY"`   | string | yes (1+) | Hex color in `#RRGGBBaa` format, keyed by symbol  |

### 2.4 Symbol Rules

- Multi-character symbols are exactly **2 characters** long.
- Valid characters: `[0-9a-z]`.
- The first character must be a digit `2`-`4` (reserving `0`-`1` for future
  base-palette expansion).
- Symbols must not collide with any single-character symbol in the base palette.
- Maximum total palette size (base + ext): **48 colors**.

### 2.5 Usage Scope

Extended palette symbols are valid **only** in RLE-encoded pixel data, using
the colon separator syntax described in section 6. They cannot appear in
grid-encoded pixel data.

---

## 3. Backdrop Tile (`backdrop_tile`)

### 3.1 Purpose

Backdrop tiles are pixel-data tiles used exclusively in backdrop scene
composition. Unlike standard `[tile.*]` sections, they carry no edge
descriptors, adjacency rules, or WFC metadata. They exist solely to store
pixel content for tilemap reference.

### 3.2 Section Format

```toml
[backdrop_tile.water_a]
palette = "night"
palette_ext = "night"
size = "16x16"
rle = '''
4~ 1:2a 3~ 1:2b 3~ 1:2a 2~
8~ 2:2b 4~ 1:2a 1~
...
'''
```

```toml
[backdrop_tile.sky_solid]
palette = "night"
size = "8x8"
grid = '''
........
........
........
........
........
........
........
........
'''
```

### 3.3 Fields

| Field          | Type   | Required | Description                                  |
|----------------|--------|----------|----------------------------------------------|
| `palette`      | string | yes      | Name of the base `[palette.*]`               |
| `palette_ext`  | string | no       | Name of the `[palette_ext.*]` (required if tile uses extended symbols) |
| `size`         | string | yes      | Tile dimensions as `WxH` (e.g. `16x16`)     |
| `grid`         | string | either   | Grid-encoded pixel data (single-char symbols only) |
| `rle`          | string | either   | RLE-encoded pixel data (supports extended symbols) |
| `template`     | string | no       | Inherit pixel data from another `backdrop_tile`    |
| `animation`    | array  | no       | Frame-based animation: sequence of `{tile, duration_ms}` |

Exactly one of `grid`, `rle`, or `template` must be present.

### 3.4 Per-Tile Frame Animation

Backdrop tiles can define frame-based animation by referencing other backdrop
tiles as frames:

```toml
[backdrop_tile.torch_1]
palette = "dungeon"
size = "16x16"
grid = '''...'''

[backdrop_tile.torch_2]
palette = "dungeon"
size = "16x16"
grid = '''...'''

[backdrop_tile.torch_animated]
palette = "dungeon"
size = "16x16"
grid = '''...'''     # base frame (also frame 0)
animation = [
  { tile = "torch_1", duration_ms = 120 },
  { tile = "torch_2", duration_ms = 120 },
  { tile = "torch_1", duration_ms = 80 },
]
```

The engine cycles through the frames at runtime. All referenced tiles must
have the same dimensions.

### 3.5 Tile Size Constraints

- Width and height must each be a power of 2: 8, 16, or 32.
- Width and height may differ (e.g. `16x8`), but square tiles are recommended.

---

## 4. Backdrop Scene (`backdrop`)

### 4.1 Purpose

A backdrop scene composes backdrop tiles into a large image via one or more
layers with parallax scrolling, blend modes, and animation zones.

### 4.2 Single-Layer Format (Simple)

```toml
[backdrop.moonlit_waterfall]
palette = "night"
palette_ext = "night"
size = "160x240"
tile_size = "16x16"
tilemap = '''
sky_a     sky_b     moon_a    moon_b    sky_c
cliff_a   cliff_b   fall_a    cliff_c   cliff_d
water_a   water_b   water_c!h water_d   water_e!v
'''
```

### 4.3 Multi-Layer Format (Parallax)

```toml
[backdrop.forest_scene]
palette = "nature"
size = "256x160"
tile_size = "16x16"

[[backdrop.forest_scene.layer]]
name = "far_sky"
scroll_factor = 0.2
tilemap = '''
sky_a  sky_a  sky_a  sky_a  sky_b  sky_b  sky_a
...
'''

[[backdrop.forest_scene.layer]]
name = "mid_trees"
scroll_factor = 0.5
opacity = 0.8
blend = "normal"
tilemap = '''
.      tree_a  .       tree_b!h  .      tree_a!h  .
...
'''

[[backdrop.forest_scene.layer]]
name = "foreground"
scroll_factor = 1.0
tilemap = '''
ground_a  ground_b  rock_a  ground_c  ground_d  rock_a!hv  ground_e
...
'''
```

When `layer` is present, `tilemap` is ignored. Layers render back-to-front
(first layer = farthest, last = nearest).

### 4.4 Scene Fields

| Field          | Type   | Required | Description                                           |
|----------------|--------|----------|-------------------------------------------------------|
| `palette`      | string | yes      | Base palette name                                     |
| `palette_ext`  | string | no       | Extended palette name                                 |
| `size`         | string | yes      | Total scene dimensions as `WxH` in pixels             |
| `tile_size`    | string | yes      | Tile dimensions as `WxH` (must match referenced tiles)|
| `tilemap`      | string | see note | Single-layer tilemap (ignored if `layer` present)     |
| `layer`        | array  | see note | Multi-layer definitions (see 4.5)                     |

Either `tilemap` or `layer` must be present.

### 4.5 Layer Fields

| Field           | Type   | Default    | Description                                    |
|-----------------|--------|------------|------------------------------------------------|
| `name`          | string | required   | Layer identifier (referenced by zones)         |
| `tilemap`       | string | required   | Whitespace-separated grid of tile references   |
| `scroll_factor` | float  | `1.0`      | Parallax: 0.0 = fixed (far), 1.0 = near       |
| `opacity`       | float  | `1.0`      | Layer opacity (0.0 = transparent)              |
| `blend`         | string | `"normal"` | Blend mode (see 4.7). Aliases: `"add"`=`"additive"`, `"mul"`=`"multiply"` |
| `offset_x`      | int    | `0`        | Horizontal pixel offset                        |
| `offset_y`      | int    | `0`        | Vertical pixel offset                          |
| `fade`          | table  | —          | Layer-level fade effect (see 4.5.1)            |
| `scroll_lock`   | table  | —          | Viewport-pinned region that ignores scroll (see 4.5.2) |

#### 4.5.1 Layer Fade (GBA BLDY)

A layer can fade toward black or white, emulating the GBA BLDY register for
atmospheric effects (distant fog, fade-in transitions, darkness).

```toml
[[backdrop.cave.layer]]
name = "far_bg"
scroll_factor = 0.1
fade = { target = "black", amount = 0.4 }
tilemap = '''...'''
```

| Field    | Type   | Required | Description                                     |
|----------|--------|----------|-------------------------------------------------|
| `target` | string | yes      | `"black"` or `"white"`                          |
| `amount` | float  | no       | Fade intensity: 0.0 (none) to 1.0 (fully faded). Default 0.0. |

**Rendering:** Each pixel's RGB is interpolated toward the target color:
- Fade to black: `RGB_out = RGB * (1 - amount)`
- Fade to white: `RGB_out = RGB + (255 - RGB) * amount`

#### 4.5.2 Scroll Lock (Genesis Window Plane)

A rectangular region of the layer that stays fixed to the viewport, ignoring
the layer's parallax `scroll_factor`. Useful for HUD elements or status bars
rendered as part of the backdrop.

```toml
scroll_lock = { x = 0, y = 0, w = 160, h = 16 }
```

### 4.6 Tile Flip Flags

Tilemap entries can append `!` followed by flip flags to the tile name:

| Suffix | Effect                              | Equivalent rotation |
|--------|-------------------------------------|---------------------|
| `!h`   | Flip horizontally (mirror X)        | —                   |
| `!v`   | Flip vertically (mirror Y)          | 180° + `!h`         |
| `!hv`  | Flip both axes                      | 180° rotation       |
| `!d`   | Diagonal flip (transpose X↔Y)       | —                   |
| `!dh`  | Diagonal + horizontal               | 90° CW rotation     |
| `!dv`  | Diagonal + vertical                 | 90° CCW rotation    |

```
water_a      # no flip
water_a!h    # horizontal mirror
water_a!v    # vertical mirror
water_a!hv   # 180° rotation
water_a!dh   # 90° clockwise
```

This reduces the number of unique `backdrop_tile` definitions needed. A single
corner tile can serve all four corners via flip combinations.

#### Tile Brightness Modifiers (Genesis VDP)

Tilemap entries can append a colon-separated modifier for per-tile brightness
adjustment, inspired by the Genesis VDP shadow/highlight mode:

| Suffix         | Effect                          | Formula                    |
|----------------|---------------------------------|----------------------------|
| `:shadow` / `:s`   | Darken tile                 | `RGB_out = RGB / 2`       |
| `:highlight` / `:hi` | Brighten tile             | `RGB_out = RGB / 2 + 128` |

Modifiers combine with flip flags. The full tile reference syntax is:

```
tile_name[!flip_flags][:modifier]
```

Examples:
```
water_a              # no flip, no modifier
water_a!h            # horizontal flip
water_a!h:shadow     # flip + darken
floor:highlight      # brighten, no flip
wall!hv:s            # flip both + darken (short form)
```

### 4.7 Blend Modes

| Mode         | Formula                    | Use Case                    |
|--------------|----------------------------|-----------------------------|
| `normal`     | Standard alpha-over        | Default, opaque layers      |
| `additive`   | `dst + src * alpha`        | Glow, light shafts, magic   |
| `multiply`   | `dst * src`                | Shadows, darkening overlays |
| `screen`     | `1 - (1-dst)(1-src)`       | Fog, brightening overlays   |

### 4.8 Tilemap Rules

- Tile names are whitespace-separated. Flip suffixes are part of the name token.
- Rows are separated by newlines.
- Columns × `tile_size` width must equal `size` width.
- Rows × `tile_size` height must equal `size` height.
- Use `.` for empty/transparent cells.
- Every base tile name (before `!` suffix) must reference a `[backdrop_tile.*]`.

---

## 5. Animation Zones (`backdrop.*.zone`)

### 5.1 Purpose

Zones define rectangular sub-regions of a backdrop that animate procedurally.
Each zone specifies a behavior type and the parameters that drive it.

### 5.2 Section Format

Zones are TOML arrays-of-tables:

```toml
[[backdrop.moonlit_waterfall.zone]]
name = "water_surface"
rect = { x = 16, y = 144, w = 128, h = 96 }
behavior = "cycle"
cycle = "water_shimmer"

[[backdrop.moonlit_waterfall.zone]]
name = "moon_reflection"
rect = { x = 32, y = 160, w = 32, h = 64 }
behavior = "wave"
cycle = "moonlight_pulse"
phase_rows = 4

[[backdrop.moonlit_waterfall.zone]]
name = "torchlight"
rect = { x = 0, y = 80, w = 16, h = 32 }
behavior = "flicker"
cycle = "fire_glow"

[[backdrop.moonlit_waterfall.zone]]
name = "waterfall_flow"
rect = { x = 64, y = 32, w = 32, h = 112 }
behavior = "scroll_down"
```

### 5.3 Common Fields

| Field      | Type   | Required | Description                                    |
|------------|--------|----------|------------------------------------------------|
| `name`     | string | yes      | Unique identifier within the backdrop          |
| `rect`     | table  | yes      | Pixel rectangle: `{ x, y, w, h }` integers    |
| `behavior` | string | yes      | One of: `cycle`, `wave`, `flicker`, `scroll_down`, `hscroll_sine`, `color_gradient`, `mosaic`, `window`, `vscroll_sine`, `palette_ramp` |
| `layer`    | string | no       | Target layer name (default: applies to all layers) |

### 5.4 Behavior Types

#### 5.4.1 `cycle`

Rotates symbol colors according to a referenced `[cycle.*]` section. All pixels
in the zone that use colors in the cycle's symbol list advance to the next color
each frame.

| Field   | Type   | Required | Description                   |
|---------|--------|----------|-------------------------------|
| `cycle` | string | yes      | Name of the `[cycle.*]` entry |

#### 5.4.2 `wave`

Identical to `cycle`, but each row (or group of rows) is offset by one phase
step, producing a ripple effect. Useful for moonlight reflections on water.

| Field        | Type    | Required | Description                                  |
|--------------|---------|----------|----------------------------------------------|
| `cycle`      | string  | yes      | Name of the `[cycle.*]` entry                |
| `phase_rows` | integer | yes      | Number of pixel rows per phase step          |
| `wave_dx`    | integer | no       | Horizontal pixel offset per phase step (default 1) |

A `phase_rows` of 4 means rows 0-3 are at phase 0, rows 4-7 at phase 1, etc.
`wave_dx` adds a horizontal scroll offset per phase step, creating diagonal
ripple patterns instead of purely vertical ones.

#### 5.4.3 `flicker`

Each frame, a random subset of pixels that participate in the cycle are marked
active; the rest hold their base color. Creates a shimmering or fire-like
effect.

| Field   | Type   | Required | Description                   |
|---------|--------|----------|-------------------------------|
| `cycle` | string | yes      | Name of the `[cycle.*]` entry |

#### 5.4.4 `scroll_down`

Shifts all pixel data in the zone downward by one pixel per frame. Pixels that
exit the bottom edge wrap to the top. No cycle reference is needed -- the
animation operates on raw pixel data.

| Field   | Type  | Required | Description                          |
|---------|-------|----------|--------------------------------------|
| `speed` | float | no       | Pixels per frame (default 1.0)       |
| `wrap`  | bool  | no       | Wrap pixels at edge (default true)   |

#### 5.4.5 `hscroll_sine` (SNES HDMA-style)

Applies a per-scanline horizontal offset following a sine wave. Emulates the
SNES HDMA horizontal scroll effect used for heat haze, water distortion, and
wavy backgrounds.

```toml
[[backdrop.desert.zone]]
name = "heat_haze"
rect = { x = 0, y = 80, w = 256, h = 80 }
behavior = "hscroll_sine"
amplitude = 3
period = 16
speed = 0.5
```

| Field       | Type    | Required | Description                                    |
|-------------|---------|----------|------------------------------------------------|
| `amplitude` | integer | no       | Horizontal sine wave amplitude in pixels (default 2) |
| `period`    | integer | no       | Sine wave period in scanlines (default 16)     |
| `speed`     | float   | no       | Animation speed multiplier (default 1.0)       |

**Rendering:** For each scanline `y` at time `t`:
```
x_offset = amplitude * sin(2π * y / period + t * speed)
```

#### 5.4.6 `color_gradient` (Raster Gradient)

Applies a per-pixel color tint that interpolates between two colors across the
zone. Used for sky gradients, underwater depth tinting, and atmospheric
perspective.

```toml
[[backdrop.sky.zone]]
name = "sky_gradient"
rect = { x = 0, y = 0, w = 256, h = 128 }
behavior = "color_gradient"
from = "#4060a0"
to = "#c08040"
direction = "vertical"
```

| Field       | Type   | Required | Description                                     |
|-------------|--------|----------|-------------------------------------------------|
| `from`      | string | yes      | Gradient start color (hex `#RRGGBB`)            |
| `to`        | string | yes      | Gradient end color (hex `#RRGGBB`)              |
| `direction` | string | no       | `"vertical"` (default) or `"horizontal"`        |
| `symbol`    | string | no       | Ramp only this palette symbol (Konami raster style) |

**Rendering:** The gradient is multiplied (tinted) onto existing pixel colors.
When `symbol` is specified, only pixels using that palette symbol are affected.

#### 5.4.7 `mosaic` (GBA-style Pixelation)

Reduces effective resolution within the zone by snapping pixels to larger
blocks. Emulates the GBA mosaic register with independent X/Y block sizes.
Useful for transition effects, damage flashes, or stylistic pixelation.

```toml
[[backdrop.battle.zone]]
name = "mosaic_effect"
rect = { x = 0, y = 0, w = 160, h = 144 }
behavior = "mosaic"
size_x = 4
size_y = 4
```

| Field    | Type    | Required | Description                                 |
|----------|---------|----------|---------------------------------------------|
| `size_x` | integer | no      | Mosaic block width in pixels (default 2)    |
| `size_y` | integer | no      | Mosaic block height in pixels (default 2)   |

**Rendering:** Each block of `size_x × size_y` pixels displays the color of
the top-left pixel in that block.

#### 5.4.8 `window` (GBA WIN0/WIN1-style)

Defines a rectangular rendering window that overrides blend mode and opacity
for specific layers inside the zone. Emulates GBA windowing registers for
spotlight effects, HUD cutouts, and layer masking.

```toml
[[backdrop.dungeon.zone]]
name = "spotlight"
rect = { x = 64, y = 48, w = 48, h = 48 }
behavior = "window"
layers_visible = ["foreground", "mid_trees"]
blend_override = "additive"
opacity_override = 0.8
```

| Field              | Type     | Required | Description                                |
|--------------------|----------|----------|--------------------------------------------|
| `layers_visible`   | [string] | no       | Which layers are visible inside this window (default: all) |
| `blend_override`   | string   | no       | Override blend mode inside window           |
| `opacity_override` | float    | no       | Override opacity inside window              |

Layers not listed in `layers_visible` are hidden within the window rectangle.
Outside the window, normal rendering applies.

#### 5.4.9 `vscroll_sine` (Genesis VSRAM-style)

Per-column vertical scroll with sine offset — the vertical counterpart to
`hscroll_sine`. Emulates the Genesis VDP's VSRAM per-cell vertical scroll
tables, used for vertical waterfall columns and independent column distortion.

```toml
[[backdrop.waterfall.zone]]
name = "waterfall_columns"
rect = { x = 64, y = 32, w = 48, h = 128 }
behavior = "vscroll_sine"
amplitude = 2
period = 16
speed = 1.5
```

| Field       | Type    | Required | Description                                    |
|-------------|---------|----------|------------------------------------------------|
| `amplitude` | integer | no       | Vertical sine wave amplitude in pixels (default 2) |
| `period`    | integer | no       | Sine wave period in columns (default 16)       |
| `speed`     | float   | no       | Animation speed multiplier (default 1.5)       |

**Rendering:** For each column `x` at time `t`:
```
y_offset = amplitude * sin(2π * x / period + t * speed)
```

#### 5.4.10 `palette_ramp` (Konami Raster-style)

Per-scanline palette entry replacement — interpolates one palette symbol's
color from `from` to `to` across the zone height. The classic "Konami sky"
technique where palette entries change per scanline for sky gradients,
underwater color shifts, and atmospheric depth.

```toml
[[backdrop.sky.zone]]
name = "sky_ramp"
rect = { x = 0, y = 0, w = 160, h = 60 }
behavior = "palette_ramp"
symbol = "s"
from = "#0a1525ff"
to = "#2a4565ff"
```

| Field    | Type   | Required | Description                                     |
|----------|--------|----------|-------------------------------------------------|
| `symbol` | string | yes      | Which palette symbol to replace per scanline    |
| `from`   | string | yes      | Color at the top of the zone (hex)              |
| `to`     | string | yes      | Color at the bottom of the zone (hex)           |

**Rendering:** Only pixels using the specified palette symbol are affected.
Each scanline within the zone gets a linearly interpolated color between
`from` and `to`. Unlike `color_gradient` (which tints all pixels), this
replaces a specific symbol's color — matching how hardware palette writes work.

### 5.5 Global Animation Clock (`anim_clock`)

Neo Geo-style global animation clocks allow multiple tiles to share a
synchronized animation timer with zero per-tile state.

```toml
[anim_clock.water]
fps = 6
frames = 4
mode = "loop"

[backdrop_tile.water_a]
anim_clock = "water"
```

Tiles referencing an `anim_clock` cycle through companion tiles named
`{name}_0` through `{name}_{frames-1}`. The base tile serves as frame 0.
All tiles sharing a clock stay perfectly synchronized — no per-tile animation
timeline needed.

| Field    | Type    | Default  | Description                     |
|----------|---------|----------|---------------------------------|
| `fps`    | integer | `6`      | Frames per second               |
| `frames` | integer | `4`      | Number of animation frames      |
| `mode`   | string  | `"loop"` | `"loop"` or `"ping-pong"`       |

### 5.6 Zone Overlap

When zones overlap, the **last zone in document order** takes priority for
contested pixels. Authors can use this to layer effects (e.g., a broad `cycle`
for water shimmer with a narrower `wave` on top for a reflection streak).

---

## 6. Extended RLE Encoding

### 6.1 Base RLE Recap

Standard PAX RLE encodes runs as `<count><symbol>`, where `<count>` is a
decimal integer and `<symbol>` is a single character:

```
4~ 3a 2.
```

This means: 4x `~`, 3x `a`, 2x `.`.

### 6.2 Colon Separator for Extended Symbols

Multi-character symbols cannot be concatenated directly to a run count without
ambiguity. The colon separator resolves this:

```
<count>:<symbol>
```

Examples:

```
1:2a        # 1x symbol "2a"
5:2f        # 5x symbol "2f"
4~ 1:2a 3~  # 4x "~", 1x "2a", 3x "~"
```

### 6.3 Mixing Base and Extended Symbols

A single RLE line may freely mix base-format runs and colon-separated extended
runs:

```
4~ 1:2a 3~ 1:2b 3~ 1:2a 2~
```

Tokens are separated by whitespace. The parser distinguishes the two forms:

- No colon: `<count><symbol>` -- single-character symbol (base palette).
- With colon: `<count>:<symbol>` -- multi-character symbol (extended palette).

### 6.4 Grid Encoding Restriction

Extended symbols (multi-character) are **not valid** in grid-encoded pixel data.
Grid encoding requires exactly one character per pixel. Tiles or sprites that
need extended palette colors must use RLE encoding.

---

## 7. Size Budget Analysis

Backdrops are larger than standard PAX sprites, so understanding their storage
cost matters.

### 7.1 Tilemap Deduplication

A 160x240 backdrop at 16x16 tile size decomposes into a 10x15 grid = 150 tile
slots. In practice, many slots reuse the same tile (sky, water, ground
repeats). A typical scene may have 40-80 unique tiles.

### 7.2 Per-Tile Cost

A 16x16 tile in RLE encoding averages 80-200 bytes depending on content
complexity. Grid encoding is fixed at 256 bytes (16 * 16 + 15 newlines).

### 7.3 Estimated Total

| Component       | Tiles | Avg Bytes | Subtotal |
|-----------------|-------|-----------|----------|
| Unique tiles    | 60    | 150       | ~9 KB    |
| Tilemap         | 1     | ~600      | ~0.6 KB  |
| Palette + ext   | 1     | ~400      | ~0.4 KB  |
| Zone defs       | 4     | ~120      | ~0.5 KB  |
| **Total**       |       |           | **~10.5 KB** |

For comparison, the equivalent raw PNG at 160x240 is typically 15-30 KB. The
PAX representation is competitive in size while being fully editable and
animation-ready.

### 7.4 Scaling

At 320x480 (4x area), tile deduplication becomes even more effective. Expect
80-120 unique tiles with a total PAX size of 15-25 KB.

---

## 8. Validation Rules

Parsers and tools must enforce the following rules. Violations are errors unless
noted as warnings.

### 8.1 Extended Palette

1. `base` must reference an existing `[palette.*]` section.
2. Every symbol must be exactly 2 characters matching `[0-9a-z]{2}`.
3. First character of each symbol must be in the range `2`-`4`.
4. No symbol may collide with a single-character symbol in the base palette.
5. Combined base + extended palette must not exceed 48 colors.
6. Color values must be valid `#RRGGBBaa` hex strings.

### 8.2 Backdrop Tile

7. Exactly one of `grid` or `rle` must be present.
8. `palette` must reference an existing `[palette.*]` section.
9. If `palette_ext` is present, it must reference an existing `[palette_ext.*]`
   section.
10. `size` must be in `WxH` format where W and H are each 8, 16, or 32.
11. Pixel data must decode to exactly W * H pixels.
12. Every symbol in the pixel data must exist in the referenced palette (or
    palette_ext if present).
13. Grid-encoded tiles must not contain multi-character symbols.

### 8.3 Backdrop Scene

14. `size` must be in `WxH` format with positive integer dimensions.
15. `tile_size` must match the `size` of every referenced backdrop tile.
16. Tilemap column count * tile width must equal scene width.
17. Tilemap row count * tile height must equal scene height.
18. Every name in the tilemap must reference an existing `[backdrop_tile.*]`
    section.
19. `palette` and `palette_ext` (if present) must reference existing sections.

### 8.4 Animation Zones

20. `name` must be unique among all zones in the same backdrop.
21. `rect` must define a rectangle fully contained within the backdrop `size`.
22. `rect` values (`x`, `y`, `w`, `h`) must be non-negative integers; `w` and
    `h` must be greater than zero.
23. `behavior` must be one of: `cycle`, `wave`, `flicker`, `scroll_down`,
    `hscroll_sine`, `color_gradient`, `mosaic`, `window`, `vscroll_sine`,
    `palette_ramp`.
24. For `cycle`, `wave`, and `flicker`: `cycle` field must reference an existing
    `[cycle.*]` section.
25. For `wave`: `phase_rows` must be a positive integer less than or equal to
    the zone height.
26. **Warning** (non-fatal): zone `rect` not aligned to `tile_size` boundaries.
27. For `color_gradient`: `from` and `to` must be valid `#RRGGBB` hex strings.
28. For `color_gradient`: `direction` must be `"vertical"` or `"horizontal"`.
29. For `mosaic`: `size_x` and `size_y` must be positive integers.
30. For `window`: `layers_visible` entries must reference existing layer names.
31. For `hscroll_sine`: `amplitude` must be non-negative; `period` must be a
    positive integer.
32. For `vscroll_sine`: `amplitude` must be non-negative; `period` must be a
    positive integer.
33. For `palette_ramp`: `symbol` must be a valid palette symbol (1-2 chars);
    `from` and `to` must be valid hex color strings.
34. For `anim_clock` on a backdrop tile: the referenced clock must exist in
    `[anim_clock.*]`, and companion tiles `{name}_1`..`{name}_{frames-1}`
    must exist as `[backdrop_tile.*]` entries.

### 8.5 Extended RLE

27. Colon-separated tokens must match the pattern `<digits>:<symbol>` where
    `<symbol>` is a valid 2-character extended palette symbol.
28. Run count must be a positive integer (no zero-length runs).
29. Each RLE row must decode to exactly the tile width in pixels.
