# PAX-L — LLM-Optimized Compact Representation

**Version:** 0.1 draft
**Status:** Proposal
**Companion to:** PAX 2.1 (`pax.md`), PAX Backdrop (`backdrop.md`)

---

## 1. Purpose

PAX-L is a **runtime wire format** — a lossless, token-efficient representation
of PAX data optimized for LLM context windows. It is not a file format. No
`.paxl` files exist on disk.

```
                  ┌─────────┐
   .pax (TOML)    │  Engine  │    PAX-L (compact)
   on disk ──────►│  pixl    │──────► LLM context
   human-read     │  core    │◄────── LLM output
   VCS-friendly   │         │    PAX-L (compact)
                  └────┬────┘
                       │
                       ▼
                  Studio displays
                  human-readable .pax
```

**Design constraint:** Every valid PAX-L representation round-trips to an
identical .pax file. `pax_to_paxl(paxl_to_pax(input)) == input` must hold.

### 1.1 When PAX-L is used

| Flow | Format | Why |
|------|--------|-----|
| MCP tool responses → LLM | PAX-L | Minimize context tokens |
| LLM output → MCP tool input | PAX-L | LLM writes what it reads |
| `pixl compact FILE` CLI | PAX-L to stdout | Inspection, debugging |
| Studio display | PAX (TOML) | Human readability |
| Version control | PAX (TOML) | Diff-friendly |
| `pixl expand` CLI | PAX-L stdin → .pax | Convert back |

### 1.2 Design goals (ordered)

1. **Lossless** — zero information loss vs .pax
2. **Token-minimal** — ~40-70% fewer tokens than equivalent TOML (measured)
3. **LLM-comprehensible** — structural patterns explicit, not implicit
4. **LLM-writable** — compositional units, not pixel-level decisions
5. **Streaming** — parseable top-to-bottom, no forward references required

---

## 2. Syntax Overview

PAX-L uses line-oriented directives. Each line starts with a sigil that
declares its type. No quoting, no braces, no nested tables. Whitespace is
the universal delimiter.

```
@pax dungeon_tileset 2.0 L1
@theme dark_fantasy dungeon s2 c16 p16 tl
@roles .void #bg +fg oaccent rdanger sshadow glife hhi wbone
@style dark_fantasy light=tl run=3.2 shadow=0.35 breadth=8 density=0.62 entropy=2.1 hue=270 lum=0.38

@pal dungeon
  . #00000000  s #12091f  # #2a1f3d  + #5a4878
  h #8070a8    w #d8d0e8  ~ #1a3a5c  g #2d5a27
  G #4a8a3a    o #c8a035  r #8b1a1a

// auto-discovered stamps from tile grids
@stamp brick_a 4x4
  h+++|++++|++++|s+++

@tile wall_solid 16x16[16] e=solid w0.4 obstacle
  #h++++s#h++++s##
  #++++++#++++++##
  #++++++#+++++s##
  #s++++s#s++++###
  ################
  ##h++++s#h+++++s
  ##++++++#++++++#
  ##++++++#++++++#
  ##s++++s#s++++s#
  ################
  =1
  =2
  =3
  =4
  ################
  ##h++++s#h+++++s
```

### 2.1 Principles

- **One palette assumed.** If a .pax file has one palette (the common case),
  PAX-L omits `pal=` from every tile. Multiple palettes use explicit `pal=X`.
- **Defaults are silent.** `symmetry=none`, `auto_rotate=none`, `weight=1.0`,
  `collision=none` are never emitted.
- **Row references.** `=N` means "identical to row N" (1-indexed).
- **Row count markers.** `16x16[16]` declares "16 grid rows follow." The
  parser validates the count. The LLM knows exactly when it's done writing
  rows — no ambiguity about grid block boundaries. Row references (`=N`)
  count toward the total. Compose layout rows use the same marker:
  `32x32[4] compose` means 4 rows of stamp references. Inspired by TOON's
  length markers which improve LLM structural accuracy.
- **Edge shorthand.** `e=####` means all 4 edges are the same class. Full
  form: `e=solid/floor/solid/floor` (N/E/S/W).
- **Comments use `//`**, not `#`. The `#` character is a palette symbol in
  most PAX themes (bg/walls). Using `#` for comments would create ambiguity
  with grid data. `//` is unambiguous — no palette symbol uses `/`.
- **Grid blocks terminate on `@`** — the next directive. No blank-line
  counting. Single blank lines within grid data are ignored for visual
  grouping but do NOT terminate the block. This is robust against LLM
  whitespace inconsistency.
- **Version stamp.** The header includes a compact representation version:
  `@pax name 2.0 L1`. `L1` is the PAX-L wire format version, independent
  of the PAX spec version. Parsers must reject unknown `L` versions.

---

## 3. Directives Reference

### 3.1 Header

```
@pax <name> <version> <L_version> [author=<author>] [profile=<color_profile>]
```

`<L_version>` is the PAX-L wire format version (`L1`). Defaults:
`author=claude`, `profile=srgb`. Omitted when default.

```
@pax dungeon_tileset 2.0 L1
```

### 3.2 Theme

```
@theme <name> <palette> s<scale> c<canvas> p<max_palette_size> <light_source_abbrev>
```

Light source abbreviations: `tl`=top-left, `tr`=top-right, `bl`=bottom-left,
`br`=bottom-right, `t`=top, `l`=left.

```
@theme dark_fantasy dungeon s2 c16 p16 tl
```

Theme inheritance:

```
@theme ice_cave :dark_fantasy pal=ice
```

The `:parent` syntax means "extends parent, override listed fields only."

### 3.3 Roles

```
@roles <sym><role> <sym><role> ...
```

No `=` sign — the symbol IS the first character, the role is the rest.

```
@roles .void #bg +fg oaccent rdanger sshadow glife hhi wbone
```

### 3.4 Constraints

```
@constraints fg>bg shadow<bg accent!=bg max_colors=16
```

Shorthand operators: `>` = brighter than, `<` = darker than, `!=` = hue
distinct from. Omit entirely when using standard dark_fantasy defaults.

### 3.5 Style Latent

```
@style <name> light=<dir> run=<f> shadow=<f> breadth=<n> density=<f> entropy=<f> hue=<n> lum=<f>
```

The style latent is the ambient aesthetic constraint for the file. Emitted
once, right after `@theme` and `@constraints`. All 8 properties come from
`pixl_learn_style` output.

```
@style dark_fantasy light=tl run=3.2 shadow=0.35 breadth=8 density=0.62 entropy=2.1 hue=270 lum=0.38
```

**Purpose:** The LLM sees this at the top of every PAX-L context and
internalizes the visual style. New tiles that deviate are flagged by
`pixl_check_style`. This is NOT a generation directive — it's a passive
constraint that the MCP server includes automatically when the session has
a learned style.

**MCP behavior:** When a style latent exists in the session, the MCP server:
1. Includes `@style` in every `pixl_get_file(format="paxl")` response
2. Auto-applies style checking to `pixl_create_tile` responses
3. Includes style deviation warnings in `pixl_critique_tile` output

If no style has been learned, the directive is simply absent.

| Property | Meaning | Range |
|---|---|---|
| `light` | Dominant light direction | tl/tr/bl/br/t/l |
| `run` | Average run-length of same-symbol sequences | 1.0-16.0 |
| `shadow` | Ratio of shadow-role pixels to total | 0.0-1.0 |
| `breadth` | Number of distinct symbols used per tile (avg) | 2-16 |
| `density` | Fraction of non-void pixels | 0.0-1.0 |
| `entropy` | Shannon entropy of symbol distribution | 0.0-4.0 |
| `hue` | Dominant hue angle (OKLab) | 0-360 |
| `lum` | Mean luminance (OKLab L) | 0.0-1.0 |

### 3.6 Palette

```
@pal <name>
  <sym> <hex>  <sym> <hex>  ...
```

Symbols and hex values in pairs, whitespace-separated. Multiple pairs per
line allowed for density. Indent indicates continuation.

```
@pal dungeon
  . #00000000  s #12091f  # #2a1f3d  + #5a4878
  h #8070a8    w #d8d0e8  ~ #1a3a5c  g #2d5a27
  G #4a8a3a    o #c8a035  r #8b1a1a
```

### 3.7 Extended Palette

```
@pal_ext <name> :<base_palette>
  <sym> <hex>  <sym> <hex>  ...
```

```
@pal_ext night :night
  2a #102040ff  2b #182848ff  2c #203050ff
```

### 3.8 Palette Swap

```
@swap <name> <base> [partial] <sym>=<hex> ...
```

```
@swap frozen dungeon partial #=#2a3f6d +=#5a7aad h=#90b0d8 ~=#e8f0ff
```

Full swaps (target palette):

```
@swap hero_red hero target=hero_red
```

### 3.9 Color Cycle

```
@cycle <name> <palette> <sym,sym,...> <direction> <fps>fps
```

```
@cycle water_shimmer dungeon ~,h ping-pong 6fps
@cycle torch_flicker dungeon o,r ping-pong 12fps
```

### 3.10 Stamps

```
@stamp <name> <WxH>
  <row>|<row>|...
```

Rows separated by `|` on a single line. This keeps stamps ultra-compact
since they're small (2x2 to 8x8).

```
@stamp brick_nw 4x4
  h+++|++++|s##h|####

@stamp brick_ne 4x4
  +++h|++++|h##s|####
```

For stamps larger than ~6 wide, multi-line is allowed:

```
@stamp wide_arch 8x4
  .h++++h.
  h++++++h
  +++++++s
  s######s
```

### 3.11 Tiles

```
@tile <name> <WxH>[<row_count>] [pal=<palette>] [e=<edges>] [ce=<corners>]
     [w<weight>] [<affordance>] [col=<collision>] [sym=<symmetry>]
     [rot=<auto_rotate>] [tags: <tag>,<tag>] [swaps: <swap>,<swap>]
     [cycles: <cycle>,<cycle>] [layer=<target_layer>] [sem: <key>=<val> ...]
  <grid_data>
```

**Row count marker `[N]`:** The number in brackets declares how many data
rows follow. For grid encoding, `N` = tile height (after symmetry, if any).
For compose encoding, `N` = number of layout rows. For `@fill`, `N` =
pattern height. For `@delta`, `N` = number of patch lines.

The parser rejects tiles where the actual row count doesn't match `[N]`.
This eliminates grid block termination ambiguity entirely — the LLM writes
exactly `N` rows because the format told it to.

```
@tile wall_solid 16x16[16] e=solid obstacle     // 16 grid rows expected
@tile castle 32x32[4] compose                   // 4 compose layout rows
@tile water 16x16[4] e=water @fill 4x4          // 4 pattern rows
@tile floor_moss 16x16[5] @delta floor_stone    // 5 patch lines
```

**Edge shorthand (`e=`):**

| Form | Meaning |
|------|---------|
| `e=solid` | All 4 edges = "solid" |
| `e=####` | All 4 edges = same auto-class (all `#` border) |
| `e=solid/floor/solid/floor` | N/E/S/W explicit |

**Corner shorthand (`ce=`):**

```
ce=grass/dirt/dirt/grass    # NE/SE/SW/NW
```

**Affordance** is a bare keyword: `obstacle`, `walkable`, `hazard`, `portal`,
`interactive`. No `affordance=` prefix needed — it's unambiguous.

**Collision** uses `col=`: `col=full`, `col=none`, `col=half_top`,
`col=slope_ne`, `col=polygon:0,0:16,0:16,16:0,16`.

**Metadata that equals the default is never emitted:**
- `pal=` omitted when file has one palette
- `sym=none` omitted (symmetry defaults to none)
- `rot=none` omitted
- `w1.0` omitted (weight defaults to 1.0)
- `col=none` omitted for walkable tiles
- `col=full` omitted for obstacle tiles (inferred from affordance)

```
@tile wall_solid 16x16[16] e=solid w0.4 obstacle
  sem: light_blocks biome=dungeon
  #h++++s#h++++s##
  #++++++#++++++##
  #++++++#+++++s##
  #s++++s#s++++###
  ################
  ##h++++s#h+++++s
  ##++++++#++++++#
  ##++++++#++++++#
  ##s++++s#s++++s#
  ################
  =1
  =2
  =3
  =4
  ################
  ##h++++s#h+++++s
```

#### 3.11.1 Row References

`=N` on a grid line means "this row is identical to row N" (1-indexed from
the first grid row of this tile). The parser expands references before any
further processing.

```
  h++++++s#h+++++s     ← row 1
  ++++++++#++++++#     ← row 2
  +++++++s#+++++++     ← row 3
  ################     ← row 4
  =1                   ← row 5 = copy of row 1
  =2                   ← row 6 = copy of row 2
```

**Chain references forbidden:** `=N` must point to a literal row, not another
reference. This keeps parsing single-pass.

**Row references in RLE:** Also valid. `=3` in an RLE block copies the
expanded pixel content of row 3.

#### 3.11.2 Pattern Fill

For tiles that are pure repeating texture:

```
@tile water_surface 16x16[4] e=water hazard cycles:water_shimmer
  @fill 4x4
  ~~h~
  ~~~~
  ~h~~
  ~~~~
```

`@fill WxH` declares a pattern block that tiles to fill the declared tile
size. Tile dimensions must be exact multiples of pattern dimensions.

The LLM writes 16 pixels instead of 256. The intent ("repeating 4x4 water
texture") is explicit.

#### 3.11.3 Compose Encoding (any size)

PAX 2.0 restricted compose to 33-64px. PAX 2.1 and PAX-L allow compose at **any tile
size**, including 8×8 and 16×16:

```
@tile wall_solid 16x16[4] e=solid w0.4 obstacle compose
  @brick_nw @brick_ne @brick_nw @brick_ne
  @mortar_16x2
  @brick_ne @brick_nw @brick_ne @brick_nw
  @mortar_16x2
  @brick_nw @brick_ne @brick_nw @brick_ne
  @mortar_16x2
  @brick_ne @brick_nw @brick_ne @brick_nw
  @mortar_16x2
```

The LLM now makes 16 placement decisions instead of 256 pixel decisions.
Stamps become a reusable vocabulary — BPE tokens for pixel art.

#### 3.11.4 Delta Tiles

Grid-level inheritance. Like `template` but with pixel patches:

```
@tile floor_moss 16x16[5] e=floor walkable
  @delta floor_stone
  +4,1 g  +5,1 G  +4,2 G  +5,2 G  +3,2 g
  +4,9 g  +5,9 G  +6,9 G  +5,10 G  +4,10 g
```

`@delta <base_tile>` inherits the full grid, then applies `+x,y sym` patches.
Much more efficient than repeating the entire grid when tiles are 90% similar
(e.g., floor variants).

**Rules:**
- Base tile must be defined before the delta tile
- Patches are `+<x>,<y> <symbol>` — coordinates are 0-indexed
- All patches must be within tile dimensions
- Palette must match base tile (or override with `pal=`)
- Delta chains forbidden (no delta-of-delta)

**Converter rule:** Delta encoding is only emitted when the patch count is
below the break-even threshold (~10-12 patches for 16×16 tiles). Above
that, the converter emits a full grid with row references instead. The
converter always computes both representations and picks the one with fewer
tokens. See Section 6.1 for measured analysis.

**Coordinate resolution:** Delta patches use pixel coordinates on the
**fully expanded** grid of the base tile:
- If the base tile uses compose encoding → coordinates apply to the
  resolved pixel grid after stamp blitting
- If the base tile has `symmetry` → coordinates apply to the full grid
  after symmetry expansion
- If the base tile is itself a template → coordinates apply to the
  inherited grid after palette swap (pixel positions, not colors)

This means `+4,3 g` always means "pixel at column 4, row 3 of the final
rendered grid." No ambiguity about which stage the coordinates reference.

#### 3.11.5 Symmetry in Grid

When `sym=h` (horizontal symmetry), only the left half is written:

```
@tile pillar 16x16[8] e=solid obstacle sym=h
  ##h+++++
  #+++++++
  #+++++++
  #s++++++
  ########
  ...
```

The engine mirrors the left half to produce the full 16-wide tile.

### 3.12 Template Tiles

```
@tile wall_ice :wall_solid pal=dungeon swaps:frozen e=ice/ice/ice/ice
```

`:base_tile` syntax = template. No grid data. The `e=` MUST be declared
(edge classes are not inherited — PAX 2.0 errata E-6).

### 3.13 Auto-Rotate

```
@tile wall_corner_ne 16x16[16] e=solid/solid/floor/floor rot=4way obstacle
  ...grid...
```

`rot=4way` / `rot=flip` / `rot=8way`. Generated variants are implicit —
never emitted in PAX-L output (they don't need to be in LLM context since
the engine generates them).

### 3.14 Sprites

```
@spriteset hero 16x32 pal=hero swaps:hero_red,hero_blue

@sprite hero.idle fps=4 loop
  @frame 1
    ....wwwwwwww....
    ...ww######ww...
    ...w#o####o#w...
    ....w######w....
    ...

  @frame 2 delta=1
    +4,14 +  +11,14 +

  @frame 3 link=1

  @frame 4 delta=1 ms=200
    +4,14 w  +11,14 w

  @tags blink 3-4
```

Frame data is indented under `@frame`. Delta frames use the same `+x,y sym`
patch syntax as delta tiles. Linked frames are a single line.

### 3.15 Composites

```
@composite knight 32x32 tile=16x16
  knight_head_l  knight_head_l!h
  knight_torso_l knight_torso_r

@variant knight.attack
  0,0=knight_head_yell_l  1,0=knight_attack_l

@offset knight
  0,0=[0,-2]  1,1=[3,0]

@anim knight.walk fps=8 loop
  @f 1
  @f 2 swap:1,0=knight_walk2_l,1,1=knight_walk2_r offset:0,0=[0,-1]
  @f 3 swap:1,0=knight_walk3_l,1,1=knight_walk3_r

@anim knight.walk_left source=walk mirror=h
```

### 3.16 Tilemaps

```
@tilemap dungeon_room 12x8 tile=16x16

@layer terrain z=0 collision
  floor_stone floor_stone floor_stone floor_stone floor_stone
  floor_stone floor_water floor_water floor_stone floor_stone
  floor_stone floor_water floor_water floor_stone floor_stone
  floor_stone floor_stone floor_stone floor_stone floor_stone

@layer walls z=1 collision
  wall_solid wall_solid . wall_solid wall_solid
  wall_solid .          . .          wall_solid
  wall_solid .          . .          wall_solid
  wall_solid wall_solid . wall_solid wall_solid
```

### 3.17 Objects

```
@object cottage 3x4 base=grass_plain above=0,1 below=2,3
  roof_l      roof_c      roof_r
  wall_win_l  wall_door   wall_win_r
  wall_base_l wall_base_c wall_base_r
  shadow_l    shadow_c    shadow_r
  @col ...|.X.|XXX|...
```

Collision mask uses `|` row separator for compactness.

### 3.18 Tile Runs

```
@run red_carpet horizontal
  left=carpet_cap_l mid=carpet_mid right=carpet_cap_r single=carpet_single
```

### 3.19 WFC Rules

```
@wfc
  forbid obstacle~hazard adjacent
  forbid hazard~walkable adjacent
  require walkable~obstacle adjacent_any boost=3.0
  require interactive~walkable adjacent_any boost=3.0
  group stone_floor: floor_stone floor_cracked floor_worn
```

`~` separates the two affordance/tag sides. Much more compact than the
TOML array-of-strings approach.

### 3.20 Atlas

```
@atlas texturepacker pad=1 s2 cols=8
  include wall_* floor_* water_* door_*
  out dungeon_atlas.png map dungeon_atlas.json
```

### 3.21 Backdrop Tiles

```
@bgtile water_a 16x16 pal=night ext=night rle
  4~ 1:2a 3~ 1:2b 3~ 1:2a 2~
  8~ 2:2b 4~ 1:2a 1~
  ...

@bgtile torch_animated 16x16
  ...grid...
  @anim torch_1:120ms torch_2:120ms torch_1:80ms
```

### 3.22 Backdrop Scenes

```
@backdrop moonlit_waterfall 160x240 tile=16x16 pal=night ext=night
  sky_a     sky_b     moon_a    moon_b    sky_c
  cliff_a   cliff_b   fall_a    cliff_c   cliff_d
  water_a   water_b   water_c!h water_d   water_e!v

@zone water_surface cycle=water_shimmer
  rect 16,144 128x96

@zone moon_reflection wave=moonlight_pulse phase=4
  rect 32,160 32x64

@zone torchlight flicker=fire_glow
  rect 0,80 16x32

@zone waterfall_flow scroll_down speed=1.0
  rect 64,32 32x112
```

Multi-layer backdrops:

```
@backdrop forest_scene 256x160 tile=16x16 pal=nature

@blayer far_sky scroll=0.2
  sky_a sky_a sky_a sky_a sky_b sky_b sky_a
  ...

@blayer mid_trees scroll=0.5 opacity=0.8
  . tree_a . tree_b!h . tree_a!h .
  ...

@blayer foreground scroll=1.0
  ground_a ground_b rock_a ground_c ground_d rock_a!hv ground_e
  ...
```

### 3.23 Animation Clocks

```
@clock water fps=6 frames=4 loop
```

### 3.24 Nine-Slice

```
@tile ui_panel 24x24 pal=ui 9slice=8,8,8,8
  ...grid...
```

`9slice=left,right,top,bottom` — inline on the tile directive.

### 3.25 Tall Tiles

```
@tile dungeon_wall_top 16x16[16] e=solid obstacle vextra=8
  ...grid...
```

`vextra=N` = `visual_height_extra`.

---

## 4. Auto-Stamp Extraction (BPE for Pixel Art)

The most powerful compression in PAX-L is automatic: the converter scans all
tile grids and discovers repeating spatial patterns, turning them into stamps
the LLM can reference by name.

**Research basis:** This is 2D Byte Pair Encoding. Elsner et al. ("Multi-
dimensional Byte Pair Encoding: Shortened Sequences for Improved Visual Data
Generation," ICCV 2025) formalize extending BPE from 1D text to 2D image
grids. Their "Token Shape Encoding" — where a merged token covers a 2×2 or
4×4 region rather than a single anchor — maps directly to PAX stamps. Their
priority-guided merge ranking (frequency × spatial consistency) informs our
ranking in step 4 below. Their lossy variant (merging "similar enough"
blocks within a perceptual threshold) is a candidate for future PAX-L
optimization where near-identical stamps could be unified.

### 4.1 Algorithm

```
extract_stamps(tiles: &[Tile]) -> Vec<AutoStamp>:
    1. For each tile with a grid encoding:
       - Extract all 4×4 sub-blocks at stride 4 (non-overlapping)
       - Extract all 2×2 sub-blocks at stride 2 (for 8×8 tiles)

    2. Build frequency table: HashMap<BlockContent, usize>
       - BlockContent = flattened char array (e.g., "h+++|++++|s##h|####")
       - Count occurrences across ALL tiles

    3. Filter: keep blocks appearing >= 3 times

    4. Rank by (frequency × block_area) — this is the BPE merge priority
       Larger, more frequent blocks save more tokens
       Note: blocks can be RECTANGULAR, not just square. A mortar row
       might be 16×2 instead of four 4×2 blocks. Variable-size stamps
       (following Elsner et al.'s multi-shape token vocabulary) are
       allowed and often more efficient.

    5. Name generation:
       - If block matches a role pattern → semantic name
         (all '#' → "solid_4x4", all '+' → "flat_4x4")
       - If block matches an existing named stamp → use that name
       - Otherwise → "auto_<hash4>" with a comment showing content

    6. For each tile: attempt to decompose into discovered stamps
       - Greedy left-to-right, top-to-bottom placement
       - If full decomposition succeeds → emit as compose
       - If partial → keep raw grid but with row references

    7. Emit discovered stamps as @stamp directives at file top
```

### 4.2 Example

Input PAX (raw grids):

```toml
[tile.wall_solid]
grid = '''
#h++++s#h++++s##
#++++++#++++++##
#++++++#+++++s##
#s++++s#s++++###
################
##h++++s#h+++++s
##++++++#++++++#
##++++++#++++++#
##s++++s#s++++s#
################
#h++++s#h++++s##
...
'''

[tile.wall_floor_n]
grid = '''
#h++++s#h++++s##
#++++++#++++++##
...same brick pattern top half...
###hsshss#######
h++++++s#h+++++s
...floor pattern bottom...
'''
```

Output PAX-L (after auto-stamp extraction):

```
@stamp brick_a 4x4
  h+++|++++|++++|s+++

@stamp brick_b 4x4
  +++s|++++|+++s|+++#

@stamp mortar 4x4
  ####|####|####|####

@stamp flat 4x4
  ++++|++++|++++|++++

@tile wall_solid 16x16[4] e=solid w0.4 obstacle compose
  @brick_a @brick_b @brick_a @brick_b
  @mortar  @mortar  @mortar  @mortar
  @brick_b @brick_a @brick_b @brick_a
  @mortar  @mortar  @mortar  @mortar

@tile wall_floor_n 16x16[5] e=solid/solid/floor/solid rot=4way obstacle compose
  @brick_a @brick_b @brick_a @brick_b
  @mortar  @mortar  @mortar  @mortar
  @brick_b @brick_a @brick_b @brick_a
  @transition_row_16x2
  @flat    @flat    @flat    @flat
```

The LLM sees that `wall_solid` and `wall_floor_n` share the same brick
pattern in their top half. Structural understanding, not just pixel matching.

### 4.3 Naming heuristics

| Block pattern | Generated name |
|---|---|
| All one symbol `#` | `solid_4x4` |
| All one symbol `+` | `flat_4x4` |
| All one symbol `.` | `void_4x4` |
| Matches existing stamp exactly | Use existing stamp name |
| Has clear light direction (h top-left, s bottom-right) | `lit_4x4`, `shadow_4x4` |
| Brick-like (h/+ with # mortar) | `brick_<pos>_4x4` |
| Otherwise | `p_<hash4>` (p for pattern) |

When the LLM creates new tiles using compose, it can reference these
discovered stamps by name. The converter validates that the stamp content
matches when converting PAX-L back to .pax.

### 4.4 Opt-out

Some tiles should NOT be decomposed (e.g., character sprites where the
spatial structure isn't block-aligned). The converter handles this
**server-side** — the LLM never needs to decide.

Auto-detection rules (applied by the converter, not the LLM):
- Tiles referenced by `[[spriteset.*.sprite]]` → skip decomposition
- Tiles referenced by `[composite.*]` → skip decomposition
- Tiles with `tags` containing "character", "npc", "item" → skip
- Tiles where decomposition produces more tokens than raw grid → skip

The `noauto` flag exists as an explicit override for edge cases:

```
@tile hero_face 16x16[16] e=floor walkable noauto
  ...raw grid, no stamp decomposition attempted...
```

`noauto` is set by the converter, not by the LLM. When the LLM creates
new tiles, it writes raw grid or compose — the server decides whether to
decompose on the next `compact` pass.

---

## 5. Conversion Pipeline

### 5.0 Honesty about file format status

PAX-L is described as a "wire format" — no blessed file extension, no
`.paxl` files in version control. But the moment it flows through CLI pipes
and MCP responses, it IS a format in practice. Prompt templates that
include PAX-L output will be versioned and cached. The `L1` version stamp
in the header exists for this reason: future parser changes must not
silently break cached PAX-L content. Treat `L1` as a stability contract.

### 5.1 PAX → PAX-L (`compact`)

```
pixl compact dungeon.pax              # stdout
pixl compact dungeon.pax --stamps     # with auto-stamp extraction (default)
pixl compact dungeon.pax --no-stamps  # skip extraction, syntax only
```

Steps:
1. Parse .pax via normal TOML parser → `PaxFile`
2. Run auto-stamp extraction (if `--stamps`, default on)
3. Detect single-palette files → omit `pal=` on tiles
4. Detect default values → omit them
5. Scan tile grids for duplicate rows → emit `=N` references
6. Scan tile grids for repeating patterns → emit `@fill`
7. Attempt compose decomposition using discovered stamps
8. Emit PAX-L text

### 5.2 PAX-L → PAX (`expand`)

```
pixl expand < compact.paxl > dungeon.pax
pixl expand --from-stdin                   # piped from LLM output
```

Steps:
1. Parse PAX-L directives line-by-line
2. Expand `=N` row references to full rows
3. Expand `@fill` patterns to full grids
4. Resolve compose `@stamp` references to inline grids
5. Expand delta tile patches onto base grids
6. Re-insert default values (symmetry, weight, etc.)
7. Emit valid TOML .pax

### 5.3 MCP Integration

The MCP server handles conversion transparently:

```
pixl_session_start() →
  Returns PAX-L representation of current file

pixl_create_tile(paxl_data) →
  Accepts PAX-L tile definition
  Internally expands to PAX grid for validation + rendering
  Stores as .pax TOML

pixl_get_file(format="paxl") →
  Returns entire file as PAX-L (default for LLM context)

pixl_get_file(format="pax") →
  Returns raw .pax TOML (for human inspection)
```

### 5.4 Diff Mode

In multi-turn MCP sessions, the LLM already has the previous file state in
context. Sending the full PAX-L again after creating one tile wastes tokens.
Diff mode returns only changed/new directives:

```
pixl_create_tile(paxl_data) →
  response includes:
    tile: <PAX-L for the new tile only>
    new_stamps: <any auto-discovered stamps from this tile>
    removed: []
    file_token_count: 1612

pixl_delete_tile(name) →
  response includes:
    removed: ["floor_wet"]
    orphaned_stamps: ["moss_drip_4x4"]  // stamp only used by deleted tile
    file_token_count: 1528
```

The LLM can reconstruct current state from its context + the diff. Full
file re-send (`pixl_get_file`) is available as a reset if context drifts.

### 5.5 Error Token Budget

Error messages consume context tokens. MCP responses enforce:
- Max **200 tokens** per individual error message
- Max **5 errors** before truncation with `... and N more errors`
- Errors are ordered by severity (ERROR before WARNING)
- Each error includes line number, directive name, and a one-line fix hint

This prevents a catastrophically invalid file from blowing out the context
window. The LLM fixes the first 5 errors, resubmits, and gets the next
batch if any remain.

### 5.6 Structured Critique Integration

When PAX-L is used in the SELF-REFINE loop, critique responses use
machine-readable structured data alongside natural language:

```
@critique wall_broken
  outline_coverage: 0.62  target: 0.80
  centering: 0.91  target: 0.85  OK
  contrast: 0.44  target: 0.50
  fix_rows: 0,15  fix: "darken border to # on row 0 cols 0-15 and row 15"
```

This avoids the telephone-game effect where natural language critique gets
reinterpreted by the LLM on each refinement iteration. The structured
fields map directly to the structural validators in `pixl-core/src/
structural.rs`.

---

## 6. Token Budget Analysis

All measurements use `cl100k_base` tokenizer (OpenAI/Claude-family). Actual
Claude tokenization may differ slightly but the ratios are representative.

### 6.1 Per-tile comparison (measured)

**`wall_solid` (16×16, repeating brick pattern):**

| Representation | Tokens | Savings |
|---|---|---|
| PAX TOML (current) | 213 | baseline |
| PAX-L grid + row refs | 135 | **36.6%** |
| PAX-L compose (auto-stamps) | 140 | **34.3%** |

Compose is slightly worse than grid+refs for a single tile because the stamp
definitions cost tokens upfront. However, stamps amortize across all tiles
that share them — in a file with 40+ tiles reusing the same brick/mortar
vocabulary, compose pulls ahead significantly.

**`water_surface` (16×16, pure repeating 4×4 pattern):**

| Representation | Tokens | Savings |
|---|---|---|
| PAX TOML | 195 | baseline |
| PAX-L @fill | 61 | **68.7%** |

The strongest single-tile compression. The LLM writes 16 pixels instead of
256 and the repetition is explicit.

**`floor_moss` (16×16, variant of floor_stone with ~25 moss patches):**

| Representation | Tokens | Savings |
|---|---|---|
| PAX TOML | 230 | baseline |
| PAX-L @delta (25 patches) | 251 | **-9.1% (WORSE)** |

**Delta encoding is a trap for moderate diffs.** The `+x,y sym` coordinate
patches are token-expensive. With 25 patches, the coordinates alone exceed
the cost of a full grid. The converter MUST compare both representations
and emit whichever is cheaper.

**Delta break-even point:** ~10-12 patches on a 16×16 tile. Below that,
delta saves tokens. Above that, full grid + row refs is cheaper. The
converter computes both and picks the winner automatically.

### 6.2 Whole-file comparison (measured: dungeon.pax, 12 tiles)

| Format | Tokens | Savings |
|---|---|---|
| PAX TOML (current .pax) | 2602 | baseline |
| PAX-L (with auto-stamps + delta where profitable) | 1528 | **41.3%** |

The 41% savings comes from three sources (approximate breakdown):
- Metadata compaction (TOML ceremony elimination): ~55% of savings
- Row references + @fill on repetitive tiles: ~30% of savings
- Auto-stamp compose on shared patterns: ~15% of savings

### 6.3 Scaling projections

The 41% on 12 tiles is the **floor**. Savings improve with file size because:

1. **Stamp amortization.** 5 stamp definitions shared by 40 tiles cost 5×12
   = 60 tokens overhead but save ~30 tokens per tile = 1200 tokens. Net:
   +1140 tokens saved.
2. **More delta candidates.** Larger tilesets have more variant families
   (floor_stone → floor_moss → floor_cracked → floor_wet) where small
   deltas (<10 patches) pay off.
3. **Metadata savings scale linearly.** Every tile saves ~80 tokens of TOML
   boilerplate regardless of file size.

| File scale | Est. PAX tokens | Est. PAX-L | Projected savings |
|---|---|---|---|
| 12 tiles (measured) | 2,602 | 1,528 | 41% |
| 30 tiles (small game) | ~6,500 | ~3,250 | ~50% |
| 60 tiles + backdrop | ~13,000 | ~5,900 | ~55% |
| 100+ tiles (production) | ~22,000 | ~8,800 | ~60% |

These projections assume typical tile reuse patterns. Worst case (all tiles
completely unique, no repeating rows, no shared stamps) = ~35% savings from
metadata compaction alone.

---

## 7. Parser Specification

### 7.1 Lexical rules

- Lines starting with `//` are comments (NOT `#` — that's a palette symbol)
- Empty lines are ignored everywhere, including inside grid blocks
- Indented lines (2+ spaces) are continuation of the previous directive
- Grid data starts after a tile/stamp/sprite header and continues until
  the next `@` directive (or EOF). Blank lines within grid data are
  ignored, not treated as terminators — this is intentionally robust
  against LLM whitespace inconsistency
- All text is UTF-8

### 7.2 Directive dispatch

```
@pax          → parse_header
@theme        → parse_theme
@roles        → parse_roles
@constraints  → parse_constraints
@pal          → parse_palette
@pal_ext      → parse_palette_ext
@swap         → parse_swap
@cycle        → parse_cycle
@stamp        → parse_stamp
@tile         → parse_tile (grid, compose, fill, delta, or template)
@spriteset    → parse_spriteset
@sprite       → parse_sprite
@frame        → parse_frame (grid or delta)
@composite    → parse_composite
@variant      → parse_variant
@offset       → parse_offset
@anim         → parse_anim
@tilemap      → parse_tilemap
@layer        → parse_layer
@object       → parse_object
@run          → parse_tile_run
@wfc          → parse_wfc_rules
@atlas        → parse_atlas
@bgtile       → parse_backdrop_tile
@backdrop     → parse_backdrop
@blayer       → parse_backdrop_layer
@zone         → parse_zone
@clock        → parse_anim_clock
```

### 7.3 Grid block termination

Grid data (indented lines of palette symbols or row references) terminates
when the parser encounters:
- A line starting with `@` (new directive)
- A non-indented non-empty line that is not grid data
- End of file

Blank lines within grid data are **always ignored** — they never terminate
the block. This is a deliberate robustness decision: LLMs are inconsistent
with whitespace output, and a stray blank line should not silently truncate
a tile grid. The parser validates row count against declared tile height
after collecting all grid lines, catching any actual truncation.

### 7.4 Strict and Lenient Modes

PAX-L has two parsing modes:

**Strict mode** (used when the server EMITS PAX-L):
- Row count must match `[N]` marker exactly
- All palette symbols must resolve
- `=N` references must target literal rows (no chains)
- All stamps in compose must exist
- Rejects any structural error with line number and context

**Lenient mode** (used when the server ACCEPTS LLM output):
- Extra/missing grid rows → warning, auto-truncate or pad with void
- Unknown palette symbols → warning, render as hot pink (#FF00FF)
- `=N` pointing at another reference → chase the chain (one level max)
- Row count mismatch with `[N]` → warning if within ±2, error beyond
- Missing `[N]` marker → accept, infer count from grid data

The MCP server always emits strict PAX-L and accepts lenient PAX-L. This
asymmetry is deliberate: the server is deterministic, the LLM makes
mistakes. Lenient mode catches 80% of common LLM errors (off-by-one row
counts, stray whitespace) without rejecting the whole tile.

### 7.5 Conformance Test Suite

PAX-L requires a language-agnostic test suite at `tests/paxl/`:

```
tests/paxl/
  roundtrip/          PAX → PAX-L → PAX, verify identical
    dungeon.pax       input
    dungeon.paxl      expected PAX-L output
    dungeon.rt.pax    expected round-tripped PAX
  invalid/            malformed PAX-L with expected errors
    missing_rows.paxl         error: "expected 16 rows, got 14"
    unknown_symbol.paxl       error: "unknown symbol 'Q' at row 3"
    delta_chain.paxl          error: "delta chains forbidden"
  edge_cases/
    single_tile.paxl          minimal valid file
    max_palette.paxl          94-symbol palette
    all_encodings.paxl        grid + rle + compose + fill + delta
    row_ref_stress.paxl       every row is =1
    empty_stamps.paxl         file with no auto-discovered stamps
```

Any implementation of a PAX-L parser MUST pass all roundtrip and invalid
test cases. Edge cases are advisory. This is inspired by TOON's conformance
approach — the test fixtures are the real spec.

---

## 8. Compatibility

### 8.1 PAX-L output from older .pax files

Any valid PAX 2.1 (or 2.0) file can be converted to PAX-L. Features not present in
the source file simply don't emit directives. A .pax file with no stamps
still works — auto-stamp extraction may discover patterns or may not.

### 8.2 PAX-L features that map to PAX 2.1

| PAX-L feature | PAX 2.1 equivalent |
|---|---|
| `=N` row references | Stored as `=N` in .pax grid/RLE (PAX 2.1 native) |
| `@fill` pattern | Stored as `fill` encoding in .pax (PAX 2.1 native) |
| `@delta` tiles | Stored as `delta` + `patches` in .pax (PAX 2.1 native) |
| Compose at 16×16 | Stored as compose encoding (PAX 2.1 allows any size) |
| Omitted defaults | Re-inserted with default values |
| Compact metadata | Expanded to full TOML fields |
| Auto-stamps | Stored as `[stamp.*]` sections in .pax |

### 8.3 What PAX-L does NOT change

- The rendered output is pixel-identical
- Edge classes, WFC rules, collision shapes — all preserved exactly
- Animation timing, cycling, palette swaps — all preserved exactly
- The .pax file on disk is the source of truth, unchanged

---

## 9. Future Extensions

### 9.1 Context-aware stamp suggestion

When the LLM creates a new tile via `pixl_create_tile`, the MCP response
includes a `suggested_stamps` list — stamps from the current file that the
LLM could use in a compose layout. This turns auto-discovered stamps into
a live vocabulary.

### 9.2 Cross-tile delta chains

Currently delta tiles reference one base. Future: reference a "most similar"
tile automatically, computed by grid hamming distance. The converter picks
the base that minimizes patch count.

### 9.3 Semantic stamp naming via vision

Use the rendered stamp PNG + vision model to generate human-readable stamp
names: `brick_lit_tl`, `moss_patch_sparse`, `water_deep_ripple`. Better
than hash-based names for LLM comprehension.

### 9.4 Lossy stamp merging (perceptual BPE)

Inspired by the lossy variant in Elsner et al. (ICCV 2025): two stamps
that differ by only 1-2 pixels could be unified if the perceptual distance
(OKLab ΔE) between the differing pixels is below a threshold. This reduces
stamp vocabulary size at the cost of minor pixel changes. Useful for
compression-first scenarios (e.g., fitting a very large tileset into a
small context window). Would require a `--lossy` flag on `pixl compact`
and a perceptual distance threshold parameter.

### 9.5 Sub-complete tileset validation

Reference: Nie et al., "N-WFC" (IEEE Transactions on Games, 2024; Best
Paper, IEEE Conference on Games 2023). If a tileset is "sub-complete" —
every edge class has at least one compatible tile for every other edge
class that can appear on an adjacent boundary — then WFC propagation is
mathematically guaranteed contradiction-free. No backtracking needed.

`pixl check --subcomplete` would verify this property and report which
edge class combinations are missing. Combined with PAX-L's structural
explicitness, this turns the "remaining hard problem" (Section 20 of PAX
2.0) into a solvable design constraint rather than a runtime retry loop.

### 9.6 Streaming PAX-L for progressive loading

For very large files (100+ tiles), stream PAX-L directives in dependency
order so the LLM can start working before the full file is loaded. The
streaming order: header → theme → palette → stamps → tiles (most
referenced first) → sprites → tilemaps. The LLM begins composing with
available stamps while remaining tiles still arrive.

---

*PAX-L 0.1 — End of specification*
