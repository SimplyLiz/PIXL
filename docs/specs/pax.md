# PAX 2.0 — Pixel Art eXchange Format

**Version:** 2.0
**Status:** Authoritative specification
**Supersedes:** All prior PIXL/PAX planning documents

---

## 1. Design Principles

PAX is built on five constraints that must hold simultaneously:

1. **LLM-writable.** Any valid PAX file can be produced by a frontier LLM
   working at the tile level, not the pixel level. The format maps to how LLMs
   reason — symbolically, hierarchically, compositionally.

2. **Human-readable.** A developer can read a .pax file and understand what it
   contains without a tool. TOML + character grids satisfy this.

3. **Complete game asset.** A .pax file contains everything needed to render a
   game's visual assets — sprites, animations, tilesets, palette swaps, color
   cycling, collision shapes — without sidecar files.

4. **Source-of-truth, not intermediary.** PAX files go in version control. PNGs
   are build artifacts derived from .pax. This inverts the workflow where PNGs
   are committed and .aseprite files are optional.

5. **Renderable without decompression.** Every pixel value is derivable from
   the file without external lookup.

---

## 2. File Structure

A `.pax` file is UTF-8 text with TOML syntax. Multi-line literal strings
(`'''...'''`) embed pixel grid data without escape processing.

```
[pax]                         Header + version
[theme.<name>]                Semantic style layer
[palette.<name>]              Named indexed color tables
[palette_swap.<name>]         Palette substitution sets
[cycle.<name>]                Color cycling animation rules
[stamp.<name>]                Reusable pixel macro-blocks (2x2 to 8x8)
[tile.<name>]                 Tile definitions (8x8 to 64x64)
[[spriteset.<name>.sprite]]   Ordered sprite sequences
[wfc_rules]                   Semantic WFC constraints
[atlas]                       Export configuration
```

### 2.1 Header

```toml
[pax]
version = "2.0"
name    = "dungeon_hero"
author  = "claude"
created = "2026-03-22T12:00:00Z"
theme   = "dark_fantasy"
```

---

## 3. Theme

The theme is the semantic style layer. It bridges LLM reasoning (roles) and
pixel output (symbols). The LLM thinks "use the lit surface color here" and
never touches hex values while drawing.

```toml
[theme.dark_fantasy]
palette          = "dungeon"
scale            = 2              # default pixel upscale on render
canvas           = 16             # default tile canvas size (pixels)
max_palette_size = 16             # hard limit: GBA=16, NES=4, GB=4
light_source     = "top-left"     # global lighting direction (generation hint)

[theme.dark_fantasy.roles]
void    = "."    # transparent
bg      = "#"    # primary structure (walls, stone)
fg      = "+"    # lit surface
accent  = "o"    # interactive, glow
danger  = "r"    # damage, lava, blood
shadow  = "s"    # deep shadow
life    = "g"    # organic, moss, growth
hi      = "h"    # specular highlight
bone    = "w"    # bone white, light stone

[theme.dark_fantasy.constraints]
fg_brighter_than_bg         = true
shadow_darker_than_bg       = true
accent_hue_distinct_from_bg = true
max_colors_per_tile         = 16
```

**Constraint evaluation** uses declarative boolean checks (not an expression
language). Each named constraint maps to a known formula:

| Constraint                       | Formula                           |
|----------------------------------|-----------------------------------|
| `fg_brighter_than_bg`            | `lum(fg) > lum(bg)`              |
| `shadow_darker_than_bg`          | `lum(shadow) < lum(bg)`          |
| `highlight_brighter_than_fg`     | `lum(hi) > lum(fg)`              |
| `accent_hue_distinct_from_bg`    | `hue_distance(accent, bg) > 40`  |
| `max_colors_per_tile`            | `tile.unique_symbols <= value`    |

Constraint violations are **warnings** in V1, not errors. They inform the
artist but don't block rendering.

**`max_palette_size`** is a hard validation **error** — the parser rejects any
tile using more distinct symbols than this limit.

**`light_source`** is a generation **hint** — the MCP server injects a 4x4
reference shading quad into tile-creation prompts.

### Theme Inheritance

```toml
[theme.ice_cave]
extends = "dark_fantasy"
palette = "ice"               # override palette, inherit everything else
```

Child themes override only specified fields. Role mappings, constraints, scale,
and canvas are inherited from the parent. Circular `extends` chains are a parse
error.

---

## 4. Palette

```toml
[palette.dungeon]
"." = "#00000000"
"#" = "#2a1f3d"
"+" = "#4a3a6d"
"s" = "#1a0f2e"
"~" = "#1a3a5c"
"g" = "#2d5a27"
"o" = "#c8a035"
"r" = "#8b1a1a"
"h" = "#6a5a9d"
"w" = "#e8e0d4"
```

**Symbol constraints:**
- Single printable ASCII character (0x21-0x7E), no whitespace
- Maximum 94 symbols per palette
- Characters chosen mnemonically where possible (`~` for water, `g` for grass)

**TOML deserialization:** Serde produces `HashMap<String, String>`. A custom
deserializer must validate single-char keys and parse hex into `Rgba`:

```rust
pub struct Palette {
    pub name: String,
    #[serde(deserialize_with = "deserialize_symbols")]
    pub symbols: HashMap<char, Rgba>,
}
```

---

## 5. Palette Swaps

A palette swap substitutes one palette for another without changing pixel data.
Used for character color variants, day/night shifts, biome variants, damage
flash effects.

```toml
# Full swap — replace entire palette
[palette_swap.hero_red_team]
base   = "hero"
target = "hero_red"

# Partial swap — replace specific symbols, keep the rest
[palette_swap.frozen]
base    = "dungeon"
partial = true
map     = { "#" = "#2a3f6d", "+" = "#4a6a9d", "~" = "#e8f0ff" }
```

**Render-time application:**

```
pixel = grid[y][x]
color = if swap.partial:
          swap.map.get(pixel).unwrap_or(palette[pixel])
        else:
          target_palette[pixel]
```

**Usage in tiles:**

```toml
[tile.wall_solid]
palette       = "dungeon"
palette_swaps = ["frozen", "lava"]
```

**Shader-based runtime swaps:** For game engines, export a grayscale index
texture + palette LUT texture. The LUT has one row per variant (base + all
swaps). The game engine selects the active row via shader uniform — zero-cost
runtime palette swapping.

---

## 6. Color Cycling

Color cycling rotates a range of palette indices to produce animated effects
(water shimmer, lava flow, fire) without modifying pixel data.

```toml
[cycle.water_shimmer]
palette   = "dungeon"
symbols   = ["~", "h", "+", "o"]  # cycle through these symbols' colors
direction = "forward"              # "forward" | "backward" | "ping-pong"
fps       = 8

[cycle.torch_flicker]
palette   = "dungeon"
symbols   = ["o", "r"]
direction = "ping-pong"
fps       = 12
```

Cycling references symbols directly, not positional indices. This is immune
to palette key reordering (TOML spec does not guarantee key order) and lets
the LLM reason semantically ("cycle through water, highlight, lit stone, gold").

**How cycling works:** At frame `t`, the color for symbol `symbols[i]` is
replaced with the color of `symbols[(i + offset) % len]`:

```
offset = floor(t * fps) % len(symbols)
for each pixel with symbol in symbols:
  rotated_sym = symbols[(symbols.index(pixel_sym) + offset) % len]
  effective_color = palette[rotated_sym]
```

**Usage in tiles:**

```toml
[tile.water_surface]
palette = "dungeon"
cycles  = ["water_shimmer"]
```

**Note:** Color cycling uses symbol names, not positional indices. This makes
cycling immune to palette key reordering (TOML does not guarantee key order).

---

## 7. Stamps (Macro-blocks)

Stamps are fixed-size reusable pixel patterns (2x2 to 8x8). They are the
composition vocabulary — the BPE tokens of the format.

```toml
[stamp.brick_nw_4x4]
palette = "dungeon"
size    = "4x4"
grid    = '''
##++
#+++
++##
+##s
'''

[stamp.moss_patch_4x4]
palette = "dungeon"
size    = "4x4"
grid    = '''
.g..
gg+.
.g+.
..+.
'''
```

All stamps in a compose layout row must have identical heights.

---

## 8. Tile Definition

### 8.1 Three-Tier Encoding

| Grid Size | LLM Accuracy | Encoding  | What LLM Writes          |
|-----------|-------------|-----------|---------------------------|
| <= 16x16  | High (85-95%) | `grid`  | Raw character grid        |
| 17-32     | Moderate     | `rle`    | Run-length rows           |
| 33-64     | Low (<40%)   | `compose`| Named stamp placement     |
| > 64      | N/A          | tilemap  | Tile references only      |

With symmetry, effective grid size halves or quarters:
- `symmetry = "quad"` on 32x32 -> LLM writes 16x16 (high accuracy)

### 8.2 Common Tile Fields

**TOML ordering constraint:** The `grid`/`rle`/`layout` field MUST appear
before any subtable headers (like `[tile.X.semantic]`). TOML subtable headers
capture all subsequent keys until the next header — a `grid` after
`[tile.X.semantic]` ends up inside `semantic`, not the tile. Use inline table
syntax for `semantic` to avoid this: `semantic = { affordance = "...", ... }`.

```toml
[tile.wall_solid]
palette       = "dungeon"
size          = "16x16"
encoding      = "grid"           # "grid" | "rle" | "compose"
symmetry      = "none"           # "none" | "horizontal" | "vertical" | "quad"
palette_swaps = ["frozen"]
cycles        = []
template      = "wall_base"      # inherit grid from named tile (Section 8.7)
auto_rotate   = "none"           # "none" | "4way" | "flip" | "8way" (Section 8.8)

edge_class    = { n = "solid", e = "solid", s = "solid", w = "solid" }
tags          = ["wall", "interior"]
weight        = 1.0

[tile.wall_solid.semantic]
affordance = "obstacle"
collision  = "full"              # "full" | "none" | "half_top" | "slope_ne/nw/se/sw"
tags       = { light_blocks = true, biome = "dungeon" }

grid = '''
################
##++##++##++####
#+++++++++++++##
##++########++##
################
##++++++++####++
#++++##+++++++++
##++##++##++####
################
##++##++########
#+++++++++++++##
##++##++##++####
################
##++##++##++####
#+++++++++++++##
################
'''
```

### 8.3 Grid Encoding

Multi-line literal TOML string. Each character is a palette symbol.

**Parser algorithm:**
1. Split by `\n`, trim leading/trailing blank lines
2. Assert row count == declared height
3. For each row: assert char count == declared width
4. For each char: assert it exists in the referenced palette
5. Errors include line number, expected vs actual count, unknown symbols

**Symmetry expansion:**
- `horizontal`: grid = left half (W/2). Full = row + reverse(row)
- `vertical`: grid = top half (H/2). Full = grid + reverse(grid)
- `quad`: grid = top-left quadrant (W/2 x H/2). Expand H then V.

### 8.4 RLE Encoding

```toml
[tile.big_wall]
palette  = "dungeon"
size     = "32x32"
encoding = "rle"

# One RLE row per line. Row count MUST equal declared height.
# No silent row repetition — every row explicit.
rle = '''
32#
2# 12+ 6# 12+
2# 12+ 6# 12+
32#
...
'''
```

**Parser:** Split tokens by whitespace. Leading digits = count (default 1).
Remaining char = symbol. Sum of counts must equal row width.

### 8.5 Compose Encoding

```toml
[tile.castle_gate]
palette  = "dungeon"
size     = "32x32"
encoding = "compose"

layout = '''
@corner_nw  @wall_cap   @wall_cap   @corner_ne
@wall_side  @crack_v    @detail     @wall_side
@wall_side  _           _           @wall_side
@wall_side  _           _           @wall_side
@corner_sw  @floor_edg  @floor_edg  @corner_se
'''
```

**Grammar:**
```
layout_row ::= (stamp_ref | void_block) (' ' (stamp_ref | void_block))*
stamp_ref  ::= '@' identifier
void_block ::= '_'
```

`_` fills a void area. Its width is determined by the stamp in the same column
position from the first row that has a stamp in that column. All stamps in a
compose layout must use uniform cell widths within each column. If a row
contains only `_` blocks and no stamps, all `_` blocks inherit the row height
from the nearest row with stamps.

No inline pixel strings in V1 — use named stamps instead.

The compose resolver validates that row widths sum to `tile_width` and row
heights sum to `tile_height`, with actionable errors including row number and
expected vs actual pixel counts.

### 8.6 Edge System

**Edge classes** (LLM-specified, relaxed matching):
```toml
edge_class = { n = "solid", e = "solid", s = "floor", w = "mixed_stone" }
```

Two tiles can be adjacent if their touching edge classes match (exact string
comparison).

**Auto-classification** (tool-generated):
```
If all symbols identical     -> "solid_<sym>"
If all symbols == '.'        -> "open"
If edge == reverse(edge)     -> "sym_<hash4>"
Else                         -> "mixed_<hash8>"
```
Hash uses FNV-1a via `fnv` crate (504M downloads, fast, deterministic, no
crypto dependency). Note: `std::hash::DefaultHasher` uses SipHash, not FNV-1a.

**`pixl check --fix`** auto-generates edge classes from grid content, so the
LLM can omit them entirely.

### 8.7 Tile Templates

Inherit pixel data from another tile, override only palette. For biome
variants without grid duplication.

```toml
[tile.wall_stone]
palette = "dungeon"
size    = "16x16"
grid    = '''...'''

[tile.wall_ice]
template   = "wall_stone"       # inherit grid
palette    = "ice_cave"         # different palette
edge_class = { n = "ice_solid", e = "ice_solid", s = "ice_solid", w = "ice_solid" }
# grid MUST NOT be present on template tiles
# template chains forbidden (no template-of-template)
# edge_class SHOULD be declared explicitly — ice walls connect to ice floors,
# not stone floors. Inherited edge_class from base is a fallback, not default.
```

### 8.8 Tile Rotation

Auto-generate rotated/reflected variants for WFC.

```toml
[tile.wall_corner_ne]
palette    = "dungeon"
size       = "16x16"
auto_rotate = "4way"         # generates 0, 90, 180, 270 degree variants
# "none" | "4way" | "flip" | "8way" (4 rotations x 2 reflections)

edge_class = { n = "solid", e = "solid", s = "open", w = "open" }
grid = '''...'''
```

Generated variant names: `wall_corner_ne_90`, `wall_corner_ne_180`,
`wall_corner_ne_270`. Edge classes rotate accordingly (N->E, E->S, S->W, W->N).

**Constraints:**
- Rotation only valid for square tiles (width == height).
- Auto-generated variant names (`<source>_90`, `_180`, `_270`, `_flip`, etc.)
  are reserved. Defining a tile whose name matches a generated variant pattern
  is a validation error.
- Auto-rotated variants enter the WFC tile pool automatically with their
  rotated edge classes. They do not need to be listed in `variant_groups`
  (variant groups are for visually-different-but-functionally-equivalent tiles).

**Grid rotation (90 CW):**
```rust
fn rotate_cw(grid: &[Vec<char>], h: usize, w: usize) -> Vec<Vec<char>> {
    let mut out = vec![vec!['.'; h]; w];
    for y in 0..h {
        for x in 0..w {
            out[x][h - 1 - y] = grid[y][x];
        }
    }
    out
}
```

**WFC weight for variants:**
```toml
auto_rotate_weight = "source_only"  # default
# "source_only" — original gets full weight, variants get 0.1
# "equal"       — weight = source.weight / num_variants
```

`source_only` is default because a north-facing wall cap should appear
frequently on the north edge, not equally in all directions.

---

## 9. Sprites and Animation

### 9.1 Spritesets

A spriteset groups all animations for one game entity.

```toml
[spriteset.hero]
palette       = "hero"
size          = "16x32"
palette_swaps = ["hero_red", "hero_blue"]
```

### 9.2 Sprites and Frames

```toml
[[spriteset.hero.sprite]]
name = "idle"
fps  = 4
loop = true
frames = [
  { index = 1, encoding = "grid", grid = '''
....wwwwwwww....
...ww######ww...
...w#o####o#w...
....w######w....
...wwwwwwwwww...
..ww++wwww++ww..
...w+++++++w....
...w+++++++w....
....w+++++w.....
...w+..w..+w....
..w+...w...+w...
..w+...w...+w...
..ww...w...ww...
...ww.....ww....
....w.....w.....
................
''' },
  { index = 2, encoding = "delta", base = 1,
    changes = [
      { x = 4, y = 14, sym = "+" },
      { x = 11, y = 14, sym = "+" }
    ]
  },
  { index = 3, encoding = "linked", link_to = 1 },
  { index = 4, encoding = "delta", base = 1, duration_ms = 200,
    changes = [
      { x = 4, y = 14, sym = "w" },
      { x = 11, y = 14, sym = "w" }
    ]
  }
]

# Animation tags — named frame ranges
tags = [
  { name = "blink", from_frame = 3, to_frame = 4 }
]
```

### 9.3 Frame Encoding Types

**`grid`** — complete pixel grid. Required for frame 1 of every sprite.

**`delta`** — changed pixels only, referencing a `grid`-encoded base frame:
```toml
{ index = 2, encoding = "delta", base = 1, changes = [...] }
```
Delta chains forbidden — all deltas must reference a `grid` frame.

**`linked`** — shares pixel data with another frame (zero storage):
```toml
{ index = 3, encoding = "linked", link_to = 1 }
```

**Variable frame duration:**
```toml
{ index = 1, encoding = "grid", duration_ms = 150, grid = '''...''' }
```
If `duration_ms` absent, frame duration = `1000 / fps`. Per-frame duration
overrides sprite-level fps for that frame.

### 9.4 Frame Resolution

```
resolve_frame(sprite, index):
  frame = sprite.frames[index]
  match frame.encoding:
    Grid    -> parse_grid(frame.grid)
    Delta   -> resolve_frame(sprite, frame.base).apply(frame.changes)
    Linked  -> resolve_frame(sprite, frame.link_to)
```

**Validation:**
- Frame indices: contiguous, starting at 1
- Delta base < current index, must be Grid-encoded
- Linked targets: valid indices within same sprite
- All delta changes within sprite dimensions
- Tag ranges within frame count, non-overlapping

---

## 10. Tilemaps

```toml
[tilemap.dungeon_room]
width       = 12
height      = 8
tile_width  = 16
tile_height = 16

[tilemap.dungeon_room.layer.terrain]
z_order   = 0
blend     = "normal"       # "normal" | "multiply" | "screen" | "add"
collision = true
grid = '''
floor_stone floor_stone floor_stone floor_stone floor_stone
floor_stone floor_water floor_water floor_stone floor_stone
floor_stone floor_water floor_water floor_stone floor_stone
floor_stone floor_stone floor_stone floor_stone floor_stone
'''

[tilemap.dungeon_room.layer.walls]
z_order   = 1
blend     = "normal"
collision = true
grid = '''
wall_solid wall_solid . wall_solid wall_solid
wall_solid .          . .          wall_solid
wall_solid .          . .          wall_solid
wall_solid wall_solid . wall_solid wall_solid
'''
```

`"."` in a tilemap layer = empty cell (no tile rendered).

Layers render bottom-to-top by `z_order`. Blend modes:
- `normal` — standard alpha compositing (Porter-Duff over)
- `multiply` — darkening (shadow overlays)
- `screen` — lightening (glow overlays)
- `add` — additive (fire/magic effects)

### 10.1 WFC Constraint Painting

Pre-collapse specific cells before WFC generation runs. This is how you get
"authored dungeon with procedural fill" instead of pure randomness.

```toml
[tilemap.dungeon.constraints]

# Pin tiles to specific cells or ranges
[[tilemap.dungeon.constraints.pins]]
x = 0, y = 0, to_x = 19, to_y = 0
tile = "wall_solid"

[[tilemap.dungeon.constraints.pins]]
x = 10, y = 7
tile = "treasure_chest"

# Force rectangular zones to a tile type
[[tilemap.dungeon.constraints.zones]]
x1 = 7, y1 = 5, x2 = 12, y2 = 9
type = "floor"

# Require passable paths between points
[[tilemap.dungeon.constraints.paths]]
from = { x = 0, y = 7 }
to   = { x = 19, y = 7 }
```

Path validation uses BFS on the collapsed grid, checking `semantic.passable`.
Blocked paths trigger WFC restart with `seed + 1` (max 5 retries).

### 10.2 Layer Properties

```toml
[tilemap.castle.layer.background]
z_order        = 0
blend          = "normal"
layer_role     = "background"     # render behind player
collision      = false
cycles         = []               # layer-level color cycling

[tilemap.castle.layer.platforms]
z_order        = 1
blend          = "normal"
layer_role     = "platform"       # solid ground
collision      = true
collision_mode = "full"           # "full" | "top_only" (one-way platforms)

[tilemap.castle.layer.foreground]
z_order        = 2
blend          = "normal"
layer_role     = "foreground"     # renders in front of player
collision      = false

[tilemap.castle.layer.effects]
z_order        = 3
layer_role     = "effects"
collision      = false
cycles         = ["lightning_arc", "spectral_glow"]  # all tiles in this layer animate
```

**`layer_role`** values: `"background"`, `"platform"`, `"foreground"`,
`"effects"`. Tells game engine exporters how to configure each layer. Tiled,
Godot, and Unity all have this concept — PAX encodes it so exports are correct
by default.

**`collision_mode`**: `"full"` (default) or `"top_only"` for one-way platforms
(player passes through from below, lands on top).

**Layer-level `cycles`**: Applied to ALL tiles in the layer. Eliminates putting
`cycles = [...]` on every torch and lightning tile individually.

---

## 10.3 Multi-Tile Objects

Buildings, trees, statues — objects spanning multiple tiles with internal
topology, z-sorting, and collision masks.

```toml
[object.cottage]
size_tiles        = "3x4"         # width x height in tiles
base_tile         = "grass_plain" # what goes underneath
above_player_rows = [0, 1]        # rows rendered in front of player
below_player_rows = [2, 3]        # rows rendered behind player

tiles = '''
roof_l      roof_c      roof_r
wall_win_l  wall_door   wall_win_r
wall_base_l wall_base_c wall_base_r
shadow_l    shadow_c    shadow_r
'''

collision = '''
...
.X.
XXX
...
'''
```

**`above_player_rows` / `below_player_rows`**: Controls depth sorting. The roof
renders in front of the player; the base renders behind.

**`collision`**: Uses a fixed mini-palette independent of the tile palette:
`"."` = passable, `"X"` = blocked. These symbols have fixed collision meaning
regardless of what `"."` or `"X"` mean in the object's tile palette. Parsed by
the same grid parser with this implicit 2-symbol palette.

**WFC interaction:** Objects are placed **after** WFC terrain generation
(two-pass). WFC fills terrain first, then objects are placed via clearance
check on the collapsed grid. `[[objects]]` is post-WFC.

**Tilemap placement:**
```toml
[[tilemap.village.objects]]
object = "cottage"
x      = 5
y      = 3
```

---

## 10.4 Tile Run Groups

Horizontal/vertical sequences with distinct caps and repeating middle: fences,
bridges, carpets, platforms.

```toml
[tile_run.red_carpet]
orientation = "horizontal"        # "horizontal" | "vertical"
left        = "carpet_cap_l"     # start cap tile
middle      = "carpet_mid"       # repeating middle
right       = "carpet_cap_r"     # end cap tile
single      = "carpet_single"   # fallback for length=1

[tile_run.stone_platform]
orientation = "horizontal"
left        = "platform_l"
middle      = "platform_mid"
right       = "platform_r"
single      = "platform_single"
```

**Tilemap usage:** `run:red_carpet` instead of individual tile names. The
renderer auto-selects cap/middle based on run length and neighbor context.

**Edge class validation:** The validator auto-checks that `left.edge_class.e`
matches `middle.edge_class.w`, and `middle.edge_class.e` matches
`right.edge_class.w`. Validation error with specific mismatch details if caps
don't match the middle section.

---

## 10.5 Tall Tiles (Pseudo-3D Depth)

Tiles that visually extend into the row below their grid position, creating
the wall-face depth illusion standard in top-down RPGs.

```toml
[tile.dungeon_wall_top]
palette             = "dungeon"
size                = "16x16"
visual_height_extra = 8           # render 8 extra pixels BELOW grid position
```

**Rendering:** The renderer blits the tile normally at grid position `y`, then
blits the bottom `visual_height_extra` pixels again at `y + tile_height`,
rendered in front of whatever occupies that row. Purely a render-time
operation — the tile grid itself is still 16x16.

This is how RPG Maker, Godot's Y-sort, and most top-down engines achieve the
"wall face below the wall cap" illusion without requiring separate tiles.

---

## 11. WFC Rules

```toml
[wfc_rules]

# FORBIDS — hard constraints, applied during AC-3 propagation
forbids = [
  "affordance:obstacle forbids affordance:hazard adjacent",
  "affordance:hazard forbids affordance:walkable adjacent"
]

# REQUIRES — soft constraints, applied as weight bias at collapse time
require_boost = 3.0
requires = [
  "affordance:walkable requires affordance:obstacle adjacent_any",
  "affordance:interactive requires affordance:walkable adjacent_any"
]

# Variant groups — tiles interchangeable for WFC edge matching
[wfc_rules.variant_groups]
grass       = ["grass_plain", "grass_flowers", "grass_rocks"]
stone_floor = ["floor_stone", "floor_cracked", "floor_worn"]
```

**Critical architectural decision:** `forbids` prunes possibilities during
propagation (safe). `requires` adjusts weights at collapse time (safe).
Applying `requires` during propagation causes spurious contradictions because
early cells are in superposition.

**Variant groups:** Members share edge compatibility. A tile compatible with
any group member is compatible with all members. Individual `weight` values
control variety density within a group.

---

## 12. Atlas Configuration

```toml
[atlas]
format     = "texturepacker"
padding    = 1
scale      = 1
columns    = 8
include    = ["wall_*", "floor_*"]
output     = "dungeon_atlas.png"
map_output = "dungeon_atlas.json"
```

**TexturePacker JSON Hash** is the primary format (48+ game engines). 9-slice
tiles include `border` metadata.

**Note:** TexturePacker's JSON format does NOT include animation tags in its
`meta` section — that's an Aseprite JSON feature (`frameTags`). PAX exports
animation tags using Aseprite-compatible `frameTags` format in a separate
sidecar JSON, or directly in the Tiled TMJ export.

**Atlas size validation:** All tiles in an atlas must share dimensions. Mixed
sizes produce an error: "use --include to filter by size."

---

## 13. Autotiling

### 13.1 Blob 47-Tile System

8-bit neighbor bitmask: NW=1, N=2, NE=4, W=8, E=16, SW=32, S=64, SE=128.

Corner cleanup: corner bit only counted if both adjacent cardinal bits set.
After cleanup, 256 masks reduce to 47 unique visual cases.

The BITMASK_TO_47 lookup table is generated at build time via `build.rs`, not
hand-authored. Validated against canonical reference implementations.

### 13.2 Dual-Grid Alternative

Conceptualized by Oskar Stalberg. 5 tile *types* (edge, inner corner, outer
corner, filled, opposite corners) that produce 15 actual tiles via rotation —
or 6 drawn tiles if symmetric. Tiles placed at half-offsets using corner logic
instead of 8-neighbor edge logic.

Simpler for top-down terrain transitions (grass, sand, snow). Blob 47 is
correct for dungeon walls and cave systems where interior connectivity matters
visually.

Reference: [Boris the Brave - Classification of Tilesets](https://www.boristhebrave.com/2021/11/14/classification-of-tilesets/)

---

## 14. Display Algorithms

### 14.1 Tile Rendering

Nearest-neighbor upscaling only. No bilinear, no anti-aliasing.

```
render_tile(tile, scale, swap?, cycles?, frame):
  palette = apply_cycles(tile.palette, cycles, frame)
  for (y, row) in grid:
    for (x, sym) in row:
      color = resolve_color(sym, palette, swap)
      fill scale x scale block at (x*scale, y*scale) with color
```

### 14.2 Layer Compositing

Render layers bottom-to-top by z_order. Apply blend mode per layer. Empty
cells (`"."`) are transparent.

### 14.3 Animation Frame Selection

```
current_frame(sprite, elapsed_ms):
  if uniform timing: return (elapsed_ms / (1000/fps)) % frame_count
  if variable: accumulate duration_ms per frame, find current position
  if loop: wrap; else: clamp to last frame
```

### 14.4 Palette LUT Export

For shader-based runtime swaps: export grayscale index texture (pixel value =
palette index) + LUT texture (one row per palette variant).

---

## 15. 9-Slice Support

```toml
[tile.ui_panel]
palette    = "ui"
size       = "24x24"
nine_slice = { left = 8, right = 8, top = 8, bottom = 8 }
```

Corners are fixed size. Edges tile along one axis. Center tiles both axes.
Stretching is always tile-repeat (nearest-neighbor), never bilinear.

In TexturePacker JSON output, 9-slice tiles include `border` property.

---

## 16. Validation Rules

### Format-level
- Valid TOML, no numeric bare keys
- All palette/theme/stamp/template references resolve
- Template tiles have no `grid` field

### Palette
- Each key exactly one printable non-whitespace ASCII char
- Valid hex color values (#RRGGBB or #RRGGBBAA)
- No duplicate symbols
- Size <= `theme.max_palette_size` (hard error)

### Grid
- Row count == declared height
- Column count == declared width (every row)
- All symbols in referenced palette

### RLE
- Line count == declared height (no silent repetition)
- Run-length sum per line == declared width

### Symmetry
- Grid dimensions == tile dimensions / 2 (per axis)
- Tile dimensions must be even

### Compose
- All stamp references exist
- All stamps in a row have identical heights
- Sum of heights == tile_height, sum of widths == tile_width

### Animation
- Frame indices contiguous from 1
- Delta base < current index, base is Grid-encoded
- Linked targets valid, within same sprite
- Tag ranges within frame count, non-overlapping

### Tile Runs
Horizontal runs:
- left.edge_class.e == middle.edge_class.w
- middle.edge_class.e == middle.edge_class.w (self-repeating)
- middle.edge_class.e == right.edge_class.w

Vertical runs:
- top.edge_class.s == middle.edge_class.n
- middle.edge_class.s == middle.edge_class.n (self-repeating)
- middle.edge_class.s == bottom.edge_class.n

Validation error with specific mismatch if caps don't match middle

### Edges (`--check-edges`)
- Every tile has at least one compatible neighbor per direction
- Warning, not error, if isolated

### Atlas
- All tiles share dimensions (error with guidance if mixed)

---

## 17. LLM Generation Protocol

### Session Entry

Every MCP session begins with `pixl.session_start()` which returns: active
theme, palette symbols, canvas size, light source, available stamps,
`max_palette_size`, and a suggested workflow.

The LLM MUST examine palette symbols before writing any grid.

### Grid Writing Guidelines

- **8x8:** Write full grid directly.
- **16x16:** Use `symmetry = "quad"` for symmetric tiles. For asymmetric,
  write zone-by-zone (NW, NE, SW, SE as 8x8 quadrants).
- **32x32:** Always use compose mode. Never attempt raw 32x32.
- Never introduce symbols not in the session palette.
- Shadow role bottom-right of structure. Highlight role top-left of surfaces.

### Animation Guidelines

- Frame 1: always full grid (canonical rest pose)
- Subsequent frames: delta from frame 1 (small changes, <20 pixels)
- Use `linked` for repeated poses
- Walk cycles: mirror for opposite direction

### SELF-REFINE Loop

Based on Madaan et al., NeurIPS 2023:
1. Create tile -> examine 16x preview (PNG for tiles, GIF for sprites/cycling)
2. Refine via `pixl.refine_tile()` -> re-examine
3. Cap at 3 iterations (diminishing returns per research)

### MCP Tool Catalog (19 tools implemented)

**Discovery:**
- `pixl_session_start` -> theme, palette, stamps, tiles, light_source, workflow
- `pixl_get_palette(theme)` -> symbol table with roles and hex values
- `pixl_get_blueprint(model, width, height)` -> guide_text, landmarks, eye_size
- `pixl_list_tiles` -> tiles with edge classes, tags, template info
- `pixl_list_themes` -> themes with palette, scale, light source, roles
- `pixl_list_stamps` -> stamps with sizes

**Creation:**
- `pixl_create_tile(...)` -> validation + 16x preview PNG + edge_pixels +
  compatible_neighbors. Edge context shows actual border strings and which
  existing tiles can go in each direction.
- `pixl_load_source(source)` -> load a .pax string into session state

**Refinement (SELF-REFINE):**
- `pixl_check_edge_pair(tile_a, direction, tile_b)` -> compatible: bool, reason
- `pixl_render_tile(name, scale?)` -> base64 PNG at specified zoom
- `pixl_render_sprite_gif(spriteset, sprite, scale?)` -> animated base64 GIF.
  Resolves grid/delta/linked frames. Multimodal LLM examines animation quality.
- `pixl_delete_tile(name)` -> removes tile from session

**Style:**
- `pixl_learn_style(tiles?)` -> extract style latent from reference tiles.
  Returns 8-property fingerprint (light direction, run length, shadow ratio,
  palette breadth, pixel density, entropy, hue bias, luminance). Stored in
  session state for subsequent scoring.
- `pixl_check_style(name)` -> score a tile against the session latent (0-1)

**Validation & Generation:**
- `pixl_validate(check_edges?)` -> errors, warnings, stats
- `pixl_narrate_map(width, height, seed?, rules[])` -> rendered map PNG +
  tile name grid from spatial predicates. Rules: "border:wall_solid",
  "region:name:type:WxH:position", "path:x1,y1:x2,y2". Retries on
  contradiction (max 5).
- `pixl_generate_context(prompt, type?, size?)` -> enriched system_prompt +
  user_prompt for AI generation. Includes palette symbols, theme constraints,
  style latent, edge context. Studio sends this to Anthropic API directly.
- `pixl_pack_atlas(columns?, padding?, scale?)` -> base64 atlas PNG + JSON

**Export:**
- `pixl_get_file` -> full .pax TOML source

### HTTP API (20 endpoints, `pixl serve --port 3742`)

All MCP tools are also available as REST endpoints for PIXL Studio:

```
GET  /health                  POST /api/session
POST /api/palette             GET  /api/themes
GET  /api/stamps              GET  /api/tiles
POST /api/tile/create         POST /api/tile/render
POST /api/tile/delete         POST /api/tile/edge-check
POST /api/validate            POST /api/narrate
POST /api/style/learn         POST /api/style/check
POST /api/blueprint           POST /api/sprite/gif
GET  /api/file                POST /api/generate/context
POST /api/atlas/pack          POST /api/load
POST /api/tool                (generic: {tool, args})
```

**Edge context injection:** `create_tile` responses include `edge_pixels`
(actual border strings N/E/S/W) and `compatible_neighbors` (which tiles can
go in each direction). Concrete pixel targets, not just abstract class names.

**Animated previews:** `render_sprite_gif` returns base64 GIF for multimodal
LLM inspection of animation quality.

**Error state rendering:** Unknown symbols render as hot pink (#FF00FF). The
LLM can visually inspect mistakes in the preview. Invalid tiles are excluded
from WFC and atlas export.

**`pixl check --fix` behavior:** Fills missing edge classes from auto-
classification. Warns on mismatch (recognizes aliases: "solid" matches
"solid_#"). Never overwrites existing declarations.

---

## 18. Blueprint System

Blueprints are anatomy/layout reference data that live in `pixl-core` as
queryable data structures. They encode professional pixel art placement rules
(proportions, feature sizes, landmark positions) so that any consumer — MCP,
CLI, PIXL Studio, or a fine-tuned model — can query "where do eyes go on a
32x48 chibi character" and get exact pixel coordinates.

Blueprints are NOT part of the .pax file format. They are built-in reference
data shipped with the tool.

### 18.1 Anatomy Models

```rust
pub struct Blueprint {
    pub name: String,           // "humanoid_chibi", "humanoid_realistic"
    pub landmarks: Vec<Landmark>,
    pub size_rules: HashMap<(u32, u32), SizeRule>,
}

pub struct Landmark {
    pub name: String,           // "eye_left", "shoulder_right", "waist"
    pub x: f32,                 // fraction of canvas width  (0.0-1.0)
    pub y: f32,                 // fraction of canvas height (0.0-1.0)
}

pub struct SizeRule {
    pub canvas: (u32, u32),     // e.g. (16, 32)
    pub omit: Vec<String>,      // features too small to render at this size
    pub eye_size_px: u32,       // eye dimensions in pixels
    pub has_pupil: bool,
    pub has_highlight: bool,
}
```

### 18.2 Built-in Models

**humanoid_chibi** (6-head proportions, for game characters):
- Head = top 22% of canvas
- Eyes at 12% from top, spaced at 30% and 70% horizontal
- Shoulders at 28%, waist at 55%, knees at 78%
- At 16x16: no facial features (color region only)
- At 16x32: 2x1 eyes at row 4-5, no nose/mouth
- At 32x48: 3x3 eyes with 1px pupil, nose and mouth visible

**humanoid_realistic** (8-head proportions, for larger sprites 32x96+)

### 18.3 Querying Blueprints

```rust
impl Blueprint {
    /// Returns pixel-coordinate landmarks for a given canvas size
    pub fn resolve(&self, width: u32, height: u32) -> ResolvedBlueprint {
        let size_rule = self.size_rules.get(&(width, height))
            .or_else(|| self.nearest_size_rule(width, height));

        ResolvedBlueprint {
            landmarks: self.landmarks.iter()
                .filter(|l| !size_rule.omit.contains(&l.name))
                .map(|l| ResolvedLandmark {
                    name: l.name.clone(),
                    x: (l.x * width as f32).round() as u32,
                    y: (l.y * height as f32).round() as u32,
                })
                .collect(),
            eye_size: size_rule.eye_size_px,
            omitted: size_rule.omit.clone(),
        }
    }

    /// Render a text grid showing landmark positions for LLM consumption
    pub fn render_guide(&self, width: u32, height: u32) -> String { ... }
}
```

The `render_guide()` method produces a human/LLM-readable text map:

```
Canvas 32x48 (humanoid_chibi):
Row  0: head_top
Row  6: eye_left (col 11), eye_right (col 21) <- anchor here first
Row  8: nose (col 16)
Row  9: mouth (col 16)
Row 13: shoulder_left (col 6), shoulder_right (col 26)
Row 26: waist (col 16)
Row 37: knee_left (col 11), knee_right (col 21)
Row 48: feet

Eye size: 3x3 with 1px pupil
Draw eyes first. Everything else is measured from the eyes.
```

Any tool (MCP, CLI `pixl blueprint`, PIXL Studio) calls
`Blueprint::resolve()` and gets the same coordinates. The blueprint is the
single source of truth for anatomy placement, not duplicated across tool
prompt templates.

### 18.4 Eye Size Rules

"Build your sprites around the eyes" — this is the atomic constraint.

| Canvas    | Eye Size | Pupil | Highlight | Nose | Mouth |
|-----------|----------|-------|-----------|------|-------|
| 8x8       | N/A      | N/A   | N/A       | N/A  | N/A   |
| 16x16     | N/A      | N/A   | N/A       | N/A  | N/A   |
| 16x32     | 2x1      | No    | No        | No   | No    |
| 24x32     | 2x2      | 1x1   | No        | No   | No    |
| 32x48     | 3x3      | 1x1   | 1x1       | Yes  | Yes   |
| 32x64     | 4x4      | 2x2   | 1x1       | Yes  | Yes   |

---

## 19. Subsystem Status

### Implemented

| Subsystem | Status | CLI / API |
|-----------|--------|-----------|
| **Theme Library** | DONE | `pixl new <theme>` — 6 built-in themes (dark_fantasy, light_fantasy, sci_fi, nature, gameboy, nes) with curated stamps |
| **Diffusion Import** | DONE | `pixl import` — Lanczos downscale + perceptual palette quantize + Bayer dither |
| **Style Latent** | DONE | `pixl style` / `pixl_learn_style` — 8-property fingerprint, tile scoring, TOML-serializable |
| **Project Sessions** | DONE | `pixl project init/add-world/status/learn-style` — .pixlproject format with persistent style latent |
| **Narrate Pipeline** | DONE | `pixl narrate` / `pixl_narrate_map` — spatial predicates to WFC map |
| **Procedural Stamps** | DONE | `pixl generate-stamps` — 8 pattern types |
| **HTTP API** | DONE | `pixl serve` — 20 REST endpoints via axum |
| **Blueprint System** | DONE | `pixl blueprint` / `pixl_get_blueprint` — anatomy landmarks |

### Future (Not Yet Implemented)

**19.1 Skeletal Animation (V2):**
Body part sprites + RotSprite rotation + bone interpolation. Author 6-8 body
parts and 3-4 skeletal keyframe poses; system generates complete spritesheets.

**19.2 Fine-tuned PAX LoRA (V2+):**
Train LoRA adapter on GameTileNet corpus converted to PAX format. Local 7B
model generates PAX-native grids; frontier model validates via vision.

**19.3 WASM Playground:**
Compile `pixl-core` + `pixl-render` to wasm32. Browser-based .pax editor.

**19.4 Procedural Variation Engine:**
Auto-generate N tile variants conditioned on style latent. Crack placement,
moss density, color jitter.

---

## 20. Honest Capability Assessment

### What PAX enables

- Complete game art as version-controlled text
- LLM-authored pixel art within reliability zone via three-tier encoding
- Palette swaps and color cycling as first-class features
- Semantic WFC producing game-sensible maps
- SELF-REFINE vision loop for iterative quality improvement
- Blueprint-guided character sprites with correct anatomy placement

### What PAX cannot do in V1

- Skeletal animation (V2 — body parts + RotSprite + bone interpolation)
- Diffusion-to-PAX import (V1.1 — quantize reference images into palette)
- Cross-session project continuity (V1.2 — style latent + project files)
- 16x16 character faces (information-theoretic limit: 256 pixels is not enough)

### The remaining hard problem

WFC contradiction rate with semantic constraints on sparse tilesets (<15 tiles
+ strict rules + path requirements). The practical response: more tile variety,
fewer `forbids` rules, sparse constraint zones. WFC with strong constraints on
small tilesets is NP-hard in the general case.

---

*PAX 2.0 - End of specification*
