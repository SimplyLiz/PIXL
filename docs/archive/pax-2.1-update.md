# PAX 2.1 — Update Document

**Version:** 2.1-draft
**Based on:** PAX 2.0 authoritative specification
**Purpose:** Errata, enhancements, and scope definition for the complete PAX system

---

## Part I — Errata & Fixes to PAX 2.0

These are bugs in the 2.0 spec text that must be corrected before implementation.

### E-1. Color cycling — contradictory ordering note

Section 6 correctly switched from positional indices to symbol-based cycling:

```toml
symbols = ["~", "h", "+", "o"]
```

But a stale note at the bottom of the section still reads: *"Color cycling
references palette entries by position. Palette entry order in the TOML file
defines the slot indices (0-based). This ordering must be stable across edits."*

**Fix:** Delete that paragraph. Symbol-based cycling is the authoritative
mechanism. No positional indexing exists in PAX 2.1.

### E-2. Semantic tags use string booleans

Section 8.2 defines:

```toml
[tile.wall_solid.semantic]
tags = { light_blocks = "true", biome = "dungeon" }
```

`"true"` is a string, not a boolean. Consumers must string-compare instead of
type-check, which will produce bugs.

**Fix:** Use native TOML types. Restructure semantic tags as flat fields under
the semantic table:

```toml
[tile.wall_solid.semantic]
affordance    = "obstacle"
collision     = "full"
light_blocks  = true            # native boolean
biome         = "dungeon"       # string
```

This eliminates the inner `tags` table entirely. All semantic properties are
direct fields on `[tile.<name>.semantic]`.

### E-3. Compose void block `_` has undefined width

The compose grammar defines `_` as filling a "stamp-sized area" with the void
symbol, but never specifies which stamp's size to use. If a layout has columns
of different widths, `_` is ambiguous.

**Fix:** All cells in a compose layout are uniform size. The compose system
uses a regular grid where every cell is `stamp_size × stamp_size` pixels. The
stamp size is inferred from the first non-void stamp in the layout. A layout
mixing stamps of different widths is a validation error.

Add to Section 8.5:

> All stamps referenced in a single compose layout MUST have identical
> dimensions. The void block `_` fills one cell of that same size. Mixed-size
> stamps in a layout are a validation error with message: "compose layout
> requires uniform stamp size; found @stamp_a (4x4) and @stamp_b (8x8)."

### E-4. Collision grid on multi-tile objects — palette collision

Section 10.3 uses `"."` and `"#"` for collision grids, but `"#"` is a palette
symbol in the dungeon palette (stone wall), creating semantic overload.

**Fix:** Collision grids use a dedicated fixed vocabulary, not the tile palette:

```toml
collision = '''
...
.X.
XXX
...
'''
```

`"."` = passable, `"X"` = blocked. Always. These two symbols are parsed by a
dedicated collision grid parser that ignores the tile palette. This parser is
trivially simple (two symbols, no palette lookup) and cannot collide with any
tile palette definition.

### E-5. Auto-rotate name collisions unaddressed

If `wall_corner_ne` has `auto_rotate = "4way"`, the tool generates
`wall_corner_ne_90`, `wall_corner_ne_180`, `wall_corner_ne_270`. But if the
user also defines a tile named `wall_corner_ne_90`, there's a silent collision.

**Fix:** Add validation rule: tile names matching the pattern
`<existing_tile>_(90|180|270|h|v|90h|90v|180h|180v|270h|270v)` where
`<existing_tile>` has `auto_rotate` set are reserved. Defining them manually is
a validation error: "name 'wall_corner_ne_90' is reserved for auto-rotation of
'wall_corner_ne'."

### E-6. Template edge_class inheritance is unsafe

A stone wall template with `edge_class.e = "solid"` and an ice wall derived
from it via `template = "wall_stone"` will inherit `edge_class.e = "solid"`.
But the ice wall might need to connect to ice-specific tiles with
`edge_class.w = "solid_ice"`. Inheriting the wrong edge class causes WFC to
incorrectly tile ice walls next to stone floors.

**Fix:** Template tiles MUST declare their own `edge_class`. Edge classes are
NOT inherited from the base tile. The rationale: edge classes define
compatibility with *neighbors*, and biome variants have different neighbors.
Grid data is purely visual (safe to inherit); edge compatibility is structural
(unsafe to inherit).

```toml
[tile.wall_ice]
template   = "wall_stone"
palette    = "ice_cave"
edge_class = { n = "solid_ice", e = "solid_ice", s = "solid_ice", w = "solid_ice" }
# edge_class is REQUIRED on template tiles, not inherited
```

### E-7. Tile run validation missing vertical case

Section 16 validates horizontal tile runs but not vertical:

```
left.edge_class.e == middle.edge_class.w    # horizontal
middle.edge_class.e == right.edge_class.w   # horizontal
```

**Fix:** Add vertical run validation:

```
top.edge_class.s == middle.edge_class.n     # vertical
middle.edge_class.s == middle.edge_class.n  # vertical self-repeat
middle.edge_class.s == bottom.edge_class.n  # vertical
```

---

## Part II — New Features for 2.1

### F-1. Error-State Tile Rendering

**Problem:** When the LLM sends a malformed grid (wrong dimensions, unknown
symbols), discarding the tile means the LLM can't visually inspect its mistake
during the SELF-REFINE loop.

**Solution:** Store malformed tiles with `status: "invalid"` and render them
with error visualization:

- Unknown symbols render as hot pink `#FF00FF` pixels
- Excess rows are rendered with a red bottom border
- Short rows are padded with hot pink to declared width
- Excess columns are truncated with a red right border

The preview PNG returned from `pixl.create_tile()` always renders *something*,
even on failure. The validation errors are returned alongside the preview, not
instead of it.

```rust
pub enum TileStatus {
    Valid,
    Invalid { errors: Vec<ValidationError> },
}

// Renderer behavior:
fn resolve_color_with_fallback(sym: char, palette: &Palette) -> Rgba {
    palette.symbols.get(&sym)
        .copied()
        .unwrap_or(Rgba::from_hex("#FF00FF"))  // hot pink for unknown symbols
}
```

**MCP response for invalid tiles:**

```json
{
  "ok": false,
  "status": "invalid",
  "errors": [
    "row 5: unknown symbol 'q' (did you mean 'g'?)",
    "row 12: 17 chars, expected 16"
  ],
  "preview_b64": "data:image/png;base64,...",
  "refinement_hint": "Fix row 5 col 9 and row 12 width"
}
```

The LLM sees the preview with pink error pixels, reads the errors, and can
refine intelligently.

### F-2. Tile Deletion and Renaming in MCP

**Problem:** During a session, the LLM creates exploratory tiles that pollute
the WFC tile pool. Without deletion, the only recourse is to create replacements
and hope the old ones don't interfere.

**New MCP tools:**

```
pixl.delete_tile(name: string)
  → Removes tile from in-memory state. Fails if other tiles reference it
    (templates, tilemap layers, compose layouts). Returns list of dependents
    if blocked.

pixl.rename_tile(old_name: string, new_name: string)
  → Renames tile and updates all references (templates, tilemaps, WFC rules,
    variant groups, tile runs). Fails if new_name already exists.

pixl.delete_stamp(name: string)
  → Same semantics as delete_tile. Blocks if used in any compose layout.

pixl.delete_sprite(spriteset: string, name: string)
  → Removes a sprite from its spriteset.
```

**Cascading reference check:**

```rust
fn can_delete_tile(name: &str, state: &PaxState) -> Result<(), Vec<String>> {
    let mut dependents = Vec::new();

    // Check templates
    for tile in state.tiles.values() {
        if tile.template.as_deref() == Some(name) {
            dependents.push(format!("tile '{}' uses as template", tile.name));
        }
    }

    // Check tilemap layers
    for tilemap in state.tilemaps.values() {
        for layer in &tilemap.layers {
            if layer.grid.iter().any(|row| row.iter().any(|cell| cell == name)) {
                dependents.push(format!("tilemap '{}' layer '{}'", tilemap.name, layer.name));
            }
        }
    }

    // Check compose layouts, tile runs, variant groups, objects...
    // ...

    if dependents.is_empty() { Ok(()) } else { Err(dependents) }
}
```

### F-3. Progressive Resolution (Upscale Tool)

**Problem:** LLMs are 85-95% accurate at 8×8, but most game tiles are 16×16.
The quality gap between what the LLM can reliably produce and what the game
needs is exactly 2×.

**Solution:** A two-step workflow:

1. LLM creates tile at 8×8 (high accuracy zone)
2. Tool upscales to 16×16 via nearest-neighbor character grid doubling
3. LLM refines the 16×16 detail pass via `pixl.refine_tile()`

**New MCP tool:**

```
pixl.upscale_tile(name: string, factor: 2)
  → Doubles tile resolution. Each pixel becomes a 2×2 block of the same symbol.
    Size field updated from "8x8" to "16x16".
    Returns preview of the upscaled tile.
    Factor must be 2 (only 2× supported in V1).
```

**Algorithm:**

```rust
fn upscale_grid(grid: &[Vec<char>], factor: usize) -> Vec<Vec<char>> {
    let h = grid.len();
    let w = grid[0].len();
    let mut out = vec![vec!['.'; w * factor]; h * factor];
    for y in 0..h {
        for x in 0..w {
            let sym = grid[y][x];
            for dy in 0..factor {
                for dx in 0..factor {
                    out[y * factor + dy][x * factor + dx] = sym;
                }
            }
        }
    }
    out
}
```

After upscaling, the LLM sees a blocky 16×16 tile and refines it — adding edge
detail, texture variation, shading subtlety. This is much easier than creating
a 16×16 from scratch because the structure is already correct.

**Workflow example:**

```
LLM → pixl.create_tile("wall_draft", "8x8", grid='''
##++##++
#+++++++
##++##++
#+++++++
##++##++
#+++++++
##++##++
#+++++++
''')

LLM → pixl.upscale_tile("wall_draft", factor=2)
  ← Returns 16×16 preview (blocky but structurally correct)

LLM → pixl.refine_tile("wall_draft", 0, 0, 16, 16, grid='''
##++##++##++##++
##+++++++++++#+#
#+++++++++++++##
##++########++##
##++##++##++##++
##+++++++++++#+#
#+++++++++++++##
##++########++##
##++##++##++##++
##+++++++++++#+#
#+++++++++++++##
##++########++##
##++##++##++##++
##+++++++++++#+#
#+++++++++++++##
################
''')
```

### F-4. Procedural Stamp Generation

**Problem:** The compose system requires a library of stamps before an LLM can
build large tiles. Manually authoring every stamp is slow. Many common pixel
art textures follow algorithmic patterns.

**New MCP tool:**

```
pixl.generate_stamps(pattern: string, size: string, palette: string,
                     sym_a?: char, sym_b?: char)
  → Generates stamp(s) from a built-in pattern library.
    Returns: { stamps_created: [...], previews: [...] }
```

**Built-in patterns:**

| Pattern | Description | Stamps Generated |
|---------|-------------|-----------------|
| `"running_bond"` | Brick masonry, offset rows | 2 variants |
| `"stack_bond"` | Aligned brick pattern | 1 |
| `"herringbone"` | Diagonal zigzag | 2 variants |
| `"checkerboard"` | Alternating symbols | 1 |
| `"diagonal_ne"` | Diagonal stripes NE | 1 |
| `"diagonal_nw"` | Diagonal stripes NW | 1 |
| `"dither_2x2"` | Ordered dither (Bayer 2×2) | 1 |
| `"dither_4x4"` | Ordered dither (Bayer 4×4) | 1 |
| `"dots_sparse"` | Scattered single pixels | 2 variants |
| `"dots_dense"` | Denser scattered pixels | 2 variants |

**Implementation example (running bond):**

```rust
fn generate_running_bond(size: usize, a: char, b: char) -> Vec<Stamp> {
    // Variant 1: brick starts at left
    let mut grid1 = vec![vec![a; size]; size];
    for y in 0..size {
        let mortar_row = y % (size / 2) == (size / 2 - 1);
        if mortar_row {
            for x in 0..size { grid1[y][x] = b; }
        } else {
            let offset = if y < size / 2 { 0 } else { size / 2 };
            let mortar_col = (offset + size - 1) % size;
            grid1[y][mortar_col] = b;
        }
    }

    // Variant 2: offset by half
    let grid2 = shift_grid(&grid1, size / 2, 0);

    vec![
        Stamp { name: "running_bond_a", grid: grid1, .. },
        Stamp { name: "running_bond_b", grid: grid2, .. },
    ]
}
```

Total implementation: ~150 lines for all 10 patterns. High value-to-effort
ratio.

### F-5. Edge Context Injection

**Problem:** When the LLM creates a tile that must be edge-compatible with
existing tiles, telling it `edge_class west = "solid"` is abstract. Showing it
the actual pixel border of the neighbor is concrete and actionable.

**Enhancement to `pixl.session_start()` and `pixl.create_tile()`:**

The `session_start()` response includes an `edge_context` map:

```json
{
  "edge_context": {
    "solid": {
      "example_tile": "wall_solid",
      "north": "################",
      "east":  "################",
      "south": "################",
      "west":  "################"
    },
    "floor": {
      "example_tile": "floor_stone",
      "north": "gggg+g+ggg+g+ggg",
      "east":  "+g+g+g+g+g+g+g+g",
      "south": "gggg+g+ggg+g+ggg",
      "west":  "+g+g+g+g+g+g+g+g"
    }
  }
}
```

**New dedicated tool:**

```
pixl.get_edge_context(edge_class: string, direction: string)
  → Returns: {
      pixel_string: "################",   # actual border pixels
      example_tile: "wall_solid",
      compatible_tiles: ["wall_cracked", "wall_mossy", ...],
      preview_strip_b64: "..."            # 4px-tall rendered strip
    }
```

When `pixl.create_tile()` is called with an `edge_class`, the response
includes the matching edge pixel strings inline:

```json
{
  "ok": true,
  "preview_b64": "...",
  "edge_guidance": {
    "north": "Your first row should match: ################",
    "west": "Your first column should match: #+#+#+#+#+#+#+#+"
  }
}
```

### F-6. Animated Previews

**Problem:** Static PNG previews can't convey animation quality. An LLM
examining a single frame can't judge walk cycle smoothness, cycling timing,
or frame transition quality.

**Enhancement:** `pixl.add_sprite()` and `pixl.render_cycle_frame()` return
base64 GIF instead of PNG when the content is animated.

```rust
fn render_sprite_preview(sprite: &Sprite, scale: u32) -> Vec<u8> {
    let mut gif_encoder = gif::Encoder::new(/* ... */);

    for (i, frame) in sprite.frames.iter().enumerate() {
        let grid = resolve_frame(sprite, i);
        let img = render_grid(&grid, sprite.width, sprite.height, scale, &palette, None);
        let delay_cs = frame.duration_ms.unwrap_or(1000 / sprite.fps) / 10;
        gif_encoder.write_frame(&img, delay_cs);
    }

    gif_encoder.finish()
}
```

For color cycling tiles, render one full cycle (all rotation states) as frames:

```rust
fn render_cycle_preview(tile: &Tile, cycle: &Cycle, scale: u32) -> Vec<u8> {
    let n = cycle.symbols.len();
    let mut gif_encoder = gif::Encoder::new(/* ... */);

    for offset in 0..n {
        let img = render_tile(tile, scale, None, &[cycle], offset as u64);
        let delay_cs = (1000 / cycle.fps) / 10;
        gif_encoder.write_frame(&img, delay_cs);
    }

    gif_encoder.finish()
}
```

### F-7. Sectioned File Retrieval

**Problem:** A complete PAX file can exceed LLM context windows. `pixl.get_file()`
returning the entire source for a 200-tile game is impractical.

**Enhanced tool:**

```
pixl.get_file(section?: string)
  → If section is null: returns full .pax source
  → If section is specified: returns only that section

  Valid sections:
    "header"     → [pax] block only
    "theme"      → all [theme.*] blocks
    "palettes"   → all [palette.*] and [palette_swap.*] blocks
    "stamps"     → all [stamp.*] blocks
    "tiles"      → all [tile.*] blocks
    "sprites"    → all [[spriteset.*]] blocks
    "tilemaps"   → all [tilemap.*] blocks
    "wfc"        → [wfc_rules] block
    "atlas"      → [atlas] block

pixl.get_tile_source(name: string)
  → Returns the TOML source for a single tile, including its semantic block.

pixl.get_sprite_source(spriteset: string, sprite_name: string)
  → Returns the TOML source for a single sprite with all frames.
```

### F-8. `pixl check --fix` Behavior Specification

**Problem:** The spec mentions `pixl check --fix` for auto-generating edge
classes but doesn't define what happens when user-specified and auto-classified
edges disagree.

**Behavior specification:**

```
pixl check          → validate only, report errors and warnings
pixl check --fix    → validate + auto-repair where safe

Auto-repair actions (safe, applied automatically):
  - Fill missing edge_class fields from grid content
  - Normalize hex color casing (#2A1F3D → #2a1f3d)
  - Add trailing newline to file if absent

Warning-only actions (reported but NOT auto-applied):
  - User edge_class differs from auto-classified edge
    "tile 'wall_solid' edge_class.n is 'solid' but grid shows
     mixed content. Auto-classification: 'mixed_a3f2b1c4'"
  - Theme constraint violations (fg not brighter than bg, etc.)

Never auto-repaired:
  - Grid dimension mismatches (could destroy data)
  - Unknown symbols (human must decide intent)
  - RLE sum mismatches (could corrupt tile)
```

### F-9. Auto-Rotate and WFC Pool Interaction

**Problem:** How do auto-generated rotation variants interact with WFC?

**Specification:**

Auto-rotated variants enter the WFC tile pool automatically when the source
tile is included (via `include` globs or tag filters). They do NOT need to be
listed in `wfc_rules.variant_groups` — variant groups are for
visually-different-but-functionally-equivalent tiles (grass_plain vs
grass_flowers), not geometric transformations of the same tile.

```
WFC tile pool construction:
  1. Collect all tiles matching tileset filter
  2. For each tile with auto_rotate != "none":
     a. Generate rotation variants (grid + edge classes)
     b. Add variants to pool with rotated edge classes
     c. Apply auto_rotate_weight rule:
        - "source_only": source.weight, variants get 0.1
        - "equal": source.weight / num_variants each
  3. Expand variant_groups: any tile compatible with a group member
     is compatible with all members
  4. Build adjacency rules from edge classes
  5. Apply forbids rules (prune adjacency)
  6. WFC runs with the enriched pool
```

### F-10. Narrate Map (Deferred to V1.2)

The `pixl.narrate_map` tool was present in earlier MCP catalogs but dropped
from PAX 2.0. It is the most ambitious tool in the system — natural language
to WFC constraints to tilemap.

**Deferred to V1.2** (requires project files and cross-session state). The
design sketch:

```
pixl.narrate_map(prompt: string, width: int, height: int, seed?: int)

Internal pipeline:
  1. LLM extracts spatial predicates from prompt:
     "A dungeon room with a river through the middle and treasure in the NE corner"
     → [
       { predicate: "border", type: "wall", complete: true },
       { predicate: "river", orientation: "vertical", position: "center",
         width: 2, type: "water" },
       { predicate: "treasure", position: "ne_corner", type: "interactive" }
     ]

  2. Predicates → WFC constraint pins:
     - "border wall" → pin all edge cells to wall tiles
     - "river center vertical" → pin column W/2 ± 1 to water tiles
     - "treasure ne_corner" → pin cell (W-2, 1) to treasure_chest

  3. WFC generates with pinned constraints

  4. Return: rendered preview + constraint visualization + TOML source
```

This requires the LLM to act as both the prompt interpreter and the constraint
validator, making it a two-pass LLM operation. V1.2 project files provide the
session continuity needed for this workflow.

### F-11. Tilemap Tile Name Parsing

**Clarification** needed in Section 10:

Tile names in tilemap layer grids are whitespace-delimited tokens. Rules:

- Tile names MUST NOT contain whitespace
- Tile names MUST NOT be `"."` (reserved for empty cell)
- Tile names MUST NOT start with `"@"` (reserved for stamp references)
- Tile names MUST NOT start with `"_"` (reserved for void blocks)
- Tile names MUST match `[a-zA-Z][a-zA-Z0-9_]*` (identifier pattern)
- The parser splits each row by one or more whitespace characters
- Consecutive whitespace is collapsed (two spaces == one space)

### F-12. Object Placement and WFC Interaction

**Clarification** needed in Section 10.3:

Objects are placed in a two-pass system:

```
Pass 1: WFC terrain generation
  - Run WFC on the terrain layer only
  - Respect constraint painting (pins, zones, paths)
  - Produce collapsed terrain grid

Pass 2: Object placement (post-WFC)
  - For each [[objects]] entry:
    a. Check clearance: all tiles in the object footprint on the terrain
       grid must be compatible (passable, correct type, etc.)
    b. If clearance check fails: warn, skip object
    c. If clearance passes: overlay object tiles onto appropriate layers
       based on above_player_rows / below_player_rows
  - Object tiles override terrain tiles in their footprint

WFC does NOT know about objects. Objects are decorations placed on valid terrain.
```

---

## Part III — Complete Scope Summary

When all items from PAX 2.0 + this 2.1 update are implemented, the system
contains:

### Format Features

| Feature | Section | Status |
|---------|---------|--------|
| TOML-based text format | 2 | Core |
| Themes with roles + constraints | 3 | Core |
| Theme inheritance | 3 | Core |
| Palettes (char → RGBA) | 4 | Core |
| Full palette swaps | 5 | Core |
| Partial palette swaps | 5 | Core |
| Shader-ready palette LUT export | 5, 14.4 | Core |
| Color cycling (symbol-based) | 6 | Core |
| Stamps (2×2 to 8×8 macro-blocks) | 7 | Core |
| Grid encoding (≤16×16) | 8.3 | Core |
| RLE encoding (17-32) | 8.4 | Core |
| Compose encoding (33-64) | 8.5 | Core |
| Symmetry (horizontal/vertical/quad) | 8.3 | Core |
| Edge class system + FNV-1a auto-classify | 8.6 | Core |
| Tile templates (biome variants) | 8.7 | Core |
| Tile auto-rotation (4way/flip/8way) | 8.8 | Core |
| Auto-rotate weight modes | 8.8 | Core |
| Semantic affordances + collision shapes | 8.2 | Core |
| Spritesets (entity animation groups) | 9.1 | Core |
| Frame types: grid, delta, linked | 9.3 | Core |
| Variable frame duration | 9.3 | Core |
| Animation tags (named frame ranges) | 9.2 | Core |
| Tilemaps with layered rendering | 10 | Core |
| Layer blend modes + z-order | 10.2 | Core |
| Layer roles (background/platform/foreground/effects) | 10.2 | Core |
| One-way platform collision mode | 10.2 | Core |
| Layer-level color cycling | 10.2 | Core |
| WFC constraint painting (pins/zones/paths) | 10.1 | Core |
| Multi-tile objects with depth sorting | 10.3 | Core |
| Tile run groups (cap/middle/cap) | 10.4 | Core |
| Tall tiles (pseudo-3D depth) | 10.5 | Core |
| WFC forbids (hard) + requires (soft) | 11 | Core |
| WFC variant groups | 11 | Core |
| Atlas with TexturePacker JSON | 12 | Core |
| Blob 47-tile autotiling | 13.1 | Core |
| Dual-grid autotiling | 13.2 | Core |
| 9-slice support | 15 | Core |
| Blueprint anatomy system | 18 | Core |

### Tool Features (CLI + MCP)

| Feature | Source | Status |
|---------|--------|--------|
| `pixl validate` + `--check-edges` + `--fix` | 16, F-8 | Core |
| `pixl render` (tile/sprite → PNG/GIF) | 14 | Core |
| `pixl atlas` (pack + JSON) | 12 | Core |
| `pixl wfc` (generate tilemap) | 11 | Core |
| `pixl autotile` (47-blob or dual-grid) | 13 | Core |
| `pixl export` (Tiled TMJ, Godot .tres) | 12 | Core |
| `pixl mcp` (MCP server, stdio) | 17 | Core |
| `pixl blueprint` (query anatomy models) | 18 | Core |
| Error-state rendering (hot pink) | F-1 | 2.1 |
| Tile/stamp deletion + renaming | F-2 | 2.1 |
| Progressive resolution (upscale 2×) | F-3 | 2.1 |
| Procedural stamp generation | F-4 | 2.1 |
| Edge context injection | F-5 | 2.1 |
| Animated GIF previews | F-6 | 2.1 |
| Sectioned file retrieval | F-7 | 2.1 |
| `pixl check --fix` behavior spec | F-8 | 2.1 |
| Auto-rotate + WFC pool rules | F-9 | 2.1 |
| Narrate map (NL → WFC) | F-10 | V1.2 |
| Diffusion import bridge | 19.2 | V1.1 |
| Project files + sessions | 19.3 | V1.2 |
| Skeletal animation | 19.1 | V2 |
| Fine-tuned PAX LoRA | 19.4 | V2+ |

### Validation Rules (Complete)

| Rule | Category | Severity |
|------|----------|----------|
| Valid TOML, no numeric bare keys | Format | Error |
| All references resolve (palette, theme, stamp, template) | Format | Error |
| Template tiles have no grid field | Format | Error |
| Template tiles MUST have edge_class | Format | Error |
| No template-of-template chains | Format | Error |
| Auto-rotate variant names are reserved | Format | Error |
| Palette keys are single printable ASCII chars | Palette | Error |
| Valid hex color values | Palette | Error |
| No duplicate palette symbols | Palette | Error |
| Palette size ≤ max_palette_size | Palette | Error |
| Grid row count == declared height | Grid | Error |
| Grid column count == declared width (every row) | Grid | Error |
| All grid symbols in referenced palette | Grid | Error |
| RLE line count == declared height | RLE | Error |
| RLE run-length sum == declared width (per line) | RLE | Error |
| Symmetry grid dimensions == tile dimensions / 2 | Symmetry | Error |
| Tile dimensions even when symmetry != none | Symmetry | Error |
| All compose stamp refs exist | Compose | Error |
| All compose stamps have uniform dimensions | Compose | Error |
| Compose sum of widths == tile_width per row | Compose | Error |
| Compose sum of heights == tile_height | Compose | Error |
| Frame indices contiguous from 1 | Animation | Error |
| Delta base < current index | Animation | Error |
| Delta base frame is Grid-encoded | Animation | Error |
| Linked targets valid within same sprite | Animation | Error |
| Delta changes within sprite dimensions | Animation | Error |
| Tag ranges within frame count, non-overlapping | Animation | Error |
| Tile run edge classes match (horizontal) | Tile Run | Error |
| Tile run edge classes match (vertical) | Tile Run | Error |
| Object collision grid uses only `.` and `X` | Object | Error |
| Tile names match identifier pattern | Tilemap | Error |
| Atlas tiles share dimensions | Atlas | Error |
| Auto-rotate requires square tiles | Rotation | Error |
| Theme constraint violations | Theme | Warning |
| Tile has no compatible edge neighbor | Edge | Warning |
| `--fix` edge class mismatch with auto-classified | Edge | Warning |

### Algorithms

| Algorithm | Section | Complexity |
|-----------|---------|------------|
| Grid parser (2D char array) | 8.3 | O(W×H) |
| RLE parser/encoder | 8.4 | O(W) per row |
| Symmetry expansion (H/V/quad) | 8.3 | O(W×H) |
| Grid rotation (90° CW) | 8.8 | O(W×H) |
| Grid horizontal/vertical reflection | 8.8 | O(W×H) |
| Stamp composition resolver | 8.5 | O(tile_area) |
| Delta frame resolver | 9.4 | O(changes) |
| FNV-1a edge auto-classification | 8.6 | O(edge_length) |
| Tile template resolution | 8.7 | O(1) lookup |
| Upscale grid (nearest-neighbor 2×) | F-3 | O(W×H) |
| Tile rendering (grid → RGBA image) | 14.1 | O(W×H×scale²) |
| Layer compositing (Porter-Duff over) | 14.2 | O(pixels) |
| Blend modes (multiply/screen/add) | 14.2 | O(pixels) |
| Color cycling rotation | 6 | O(cycle_length) |
| Palette swap application | 5 | O(1) per pixel |
| Palette LUT texture generation | 14.4 | O(palette_size × num_swaps) |
| Animation frame selection | 14.3 | O(frames) worst case |
| WFC: observe (min-entropy) | 11 | O(log n) with heap |
| WFC: collapse (weighted random) | 11 | O(tiles) |
| WFC: propagate (AC-3) | 11 | O(cells × tiles) |
| WFC: backtrack (snapshot stack) | 11 | O(cells) per snapshot |
| WFC: forbids rule enforcement | 11 | O(rules × tiles) at init |
| WFC: requires weight bias | 11 | O(rules) at collapse |
| WFC: path validation (BFS) | 10.1 | O(W×H) |
| WFC: constraint pin pre-collapse | 10.1 | O(pins) |
| Autotile bitmask (blob 47) | 13.1 | O(1) per cell |
| Autotile corner cleanup | 13.1 | O(1) per cell |
| BITMASK_TO_47 table generation | 13.1 | O(256) at build time |
| Dual-grid autotile | 13.2 | O(1) per cell |
| Atlas grid packing | 12 | O(tiles) |
| TexturePacker JSON generation | 12 | O(tiles) |
| 9-slice rendering | 15 | O(output_area) |
| Blueprint resolution | 18.3 | O(landmarks) |
| Procedural stamp generation | F-4 | O(stamp_area) per pattern |
| Tile run edge validation | 10.4 | O(1) per run |
| Object clearance check | F-12 | O(footprint_area) |
| GIF encoding (animated preview) | F-6 | O(frames × pixels) |
| Error-state rendering | F-1 | O(W×H) |

### Dependency Map (Rust)

```
pixl-core
  ├── types.rs           PaxFile, Palette, Theme, Tile, Sprite, Spriteset, ...
  ├── parser.rs          TOML → PaxFile (custom char deserializer)
  ├── grid.rs            grid string → Vec<Vec<char>>
  ├── rle.rs             RLE encode/decode
  ├── compose.rs         stamp composition resolver
  ├── symmetry.rs        quad/h/v expansion
  ├── rotation.rs        90/180/270/flip grid transforms + edge class rotation
  ├── template.rs        tile template inheritance resolver
  ├── edges.rs           FNV-1a edge classification
  ├── theme.rs           theme resolver, inheritance, role mapping, constraints
  ├── validate.rs        all validation rules (complete table above)
  ├── cycle.rs           symbol-based palette rotation
  ├── blueprint.rs       anatomy models, landmark resolution, render_guide()
  ├── stamps_proc.rs     procedural stamp generation (10 patterns)
  ├── tile_run.rs        cap/middle/cap resolution + edge validation
  └── object.rs          multi-tile object resolution + clearance check

pixl-render (depends: pixl-core, image, gif)
  ├── renderer.rs        tile/sprite grid → ImageBuffer (nearest-neighbor)
  ├── error_render.rs    invalid tile rendering (hot pink unknown symbols)
  ├── composite.rs       layer blending (normal, multiply, screen, add)
  ├── palette_lut.rs     grayscale sprite + LUT texture generation
  ├── atlas.rs           TexturePacker JSON Hash + atlas PNG packing
  ├── nine_slice.rs      9-slice tile rendering
  ├── tall_tile.rs       visual_height_extra rendering
  ├── animation.rs       frame selection, variable duration
  ├── upscale.rs         progressive resolution (2× character grid doubling)
  ├── preview.rs         16× zoom with grid overlay for SELF-REFINE
  └── gif.rs             animated GIF export (sprites + cycling)

pixl-wfc (depends: pixl-core)
  ├── adjacency.rs       edge class → adjacency rules (with auto-rotate expansion)
  ├── wfc.rs             core WFC: observe / collapse / propagate
  ├── semantic.rs         FORBIDS (propagation) / REQUIRES (weight bias)
  ├── backtrack.rs       snapshot stack, contradiction recovery
  ├── constraints.rs     pin/zone/path pre-collapse + BFS path validation
  ├── variant_groups.rs  group-level edge compatibility expansion
  ├── autotile.rs        47-tile bitmask (build.rs generated table)
  └── dual_grid.rs       5-tile dual-grid autotiling

pixl-export (depends: pixl-core, pixl-render)
  ├── texturepacker.rs   JSON Hash format (primary)
  ├── tiled.rs           TMJ format + frameTags sidecar
  ├── godot.rs           .tres TileSet resource
  └── aseprite_json.rs   Aseprite-compatible JSON with frameTags

pixl-mcp (depends: pixl-core, pixl-render, pixl-wfc)
  ├── server.rs          rmcp stdio transport
  ├── state.rs           in-memory PaxFile + status tracking + undo stack
  ├── tools.rs           MCP tool definitions (complete catalog)
  ├── handlers.rs        tool request handlers
  ├── edge_context.rs    edge pixel string injection
  ├── session.rs         session_start, get_blueprint, get_edge_context
  └── crud.rs            create / delete / rename / refine handlers

pixl-cli (depends: all above)
  └── main.rs            clap CLI: validate, render, atlas, wfc, check, blueprint, mcp
```

**Crate dependencies:**

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
image = { version = "0.25", features = ["png"] }
gif = "0.13"
rand = "0.9"
fnv = "1.0"
rmcp = { version = "1", features = ["server", "transport-io"] }
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
glob = "0.3"
```

No `sha2`, no `base64` (use `data-encoding` or inline), no `fixedbitset`
(use `Vec<bool>` for WFC cell state — simpler, fast enough for <10k cells).

### Implementation Priority

**Phase 1 — MVP (Weeks 1-2):** Parse + validate + render

- TOML parser with custom char deserializer
- Grid, RLE, symmetry parsers
- Palette resolution + tile rendering (nearest-neighbor)
- PNG export
- `pixl validate` (format + palette + grid rules)
- `pixl render` (single tile → PNG)

**Phase 2 — Composition (Week 2-3):** Stamps + compose + templates

- Stamp parser
- Compose resolver
- Template resolver
- Tile rotation (4way/flip/8way)
- Palette swaps (full + partial)
- `pixl atlas` (grid packing + TexturePacker JSON)

**Phase 3 — Animation (Week 3):** Sprites + cycling

- Spriteset/sprite/frame parsing
- Delta + linked frame resolution
- Variable frame duration
- Animation frame selection
- Color cycling (symbol-based)
- GIF export

**Phase 4 — WFC (Week 3-4):** Tilemap generation

- Edge auto-classification (FNV-1a)
- Adjacency rule builder (with auto-rotate expansion)
- Core WFC (observe/collapse/propagate with min-entropy heap)
- Forbids (propagation) + requires (weight bias)
- Variant groups
- Backtracking (snapshot stack, max 100 backtracks, 10 restarts)
- Constraint painting (pins/zones)
- Path validation (BFS)
- `pixl wfc` command

**Phase 5 — MCP Server (Week 4-5):** LLM integration

- rmcp stdio server setup
- Session state management
- All creation tools (create_tile, create_stamp, compose_tile, etc.)
- Error-state rendering (hot pink)
- Edge context injection
- SELF-REFINE loop (refine_tile with refinement counter)
- Animated GIF previews for sprites/cycling
- Blueprint queries
- Tile deletion/renaming
- Progressive resolution (upscale)
- Procedural stamp generation
- `pixl mcp` command

**Phase 6 — Polish (Week 5-6):** Exports + advanced features

- Tiled TMJ export
- Godot .tres export
- Blob 47-tile autotiling (build.rs table generation)
- Dual-grid autotiling
- 9-slice rendering
- Multi-tile objects + clearance check
- Tile runs + edge validation
- Tall tile rendering
- Layer blend modes (multiply/screen/add)
- Palette LUT export (grayscale + LUT texture)
- Sectioned file retrieval
- `pixl check --fix`
- `pixl export` command

### Deferred (Post V1)

| Feature | Version | Dependency |
|---------|---------|------------|
| Diffusion import bridge | V1.1 | `image` crate palette quantization |
| Narrate map (NL → WFC) | V1.2 | Project files, two-pass LLM pipeline |
| Project files + sessions | V1.2 | `.pixlproject` format design |
| Skeletal animation | V2 | RotSprite algorithm, bone interpolation |
| Fine-tuned PAX LoRA | V2+ | GameTileNet corpus conversion |

---

## Part IV — Open Questions

These are design decisions that should be resolved before Phase 4 begins, but
don't block Phases 1-3.

**Q1. Rust or Go?**
The spec is written in Rust (serde, rmcp, image crate). CKB is Go. The Aseprite
MCP that ships is Go. Decision needed. Recommendation: Rust for the core
library (serde custom deserializers, zero-cost abstractions in the renderer,
strong type system for validation); but this means a new language in the
TasteHub stack. Go is faster to ship but the palette deserializer and WFC
implementation are more verbose. Pick based on long-term maintenance, not
initial velocity.

**Q2. How large should the stamp library ship?**
Procedural generation (F-4) creates ~20 stamps from 10 patterns. Should there
be a larger curated library of hand-authored stamps bundled with the tool (50?
100?)? Or should the default workflow rely on LLM-authored stamps per project?

**Q3. BITMASK_TO_47 validation reference.**
The build.rs generator needs a canonical reference to validate against. The
cr31.com blob reference is the standard, but it's a web page, not a
machine-readable format. Should we extract and commit a reference test fixture
(256-entry expected output), or validate against a second independent
implementation?

**Q4. WFC maximum practical size.**
WFC on a 100×100 tilemap with 50 tile types and semantic constraints could take
seconds. What's the timeout? Should the MCP server stream partial results? Or
cap map size at 32×32 for interactive use and allow larger sizes only via CLI?

**Q5. Multi-file PAX projects.**
A game has multiple .pax files (dungeon.pax, overworld.pax, characters.pax).
Can they share palettes? Cross-reference tiles? V1 treats each file as
self-contained. V1.2 project files introduce cross-file references. Is this
the right boundary?

---

*PAX 2.1 Update Document — End*
