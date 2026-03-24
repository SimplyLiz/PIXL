# PIXL Implementation Plan

**Product:** PIXL — Pixel Intelligence eXchange Layer
**Format:** PAX — Pixel Art eXchange Format (`.pax` files)
**Language:** Rust
**Date:** 2026-03-22
**Team:** Lisa + Claude

---

## Table of Contents

1. [Product Architecture](#1-product-architecture)
2. [Research Foundations](#2-research-foundations)
3. [The PAX 2.0 Format Specification](#3-the-pax-20-format-specification)
4. [Rust Crate Architecture](#4-rust-crate-architecture)
5. [Core Algorithms](#5-core-algorithms)
6. [MCP Integration — The AI Layer](#6-mcp-integration--the-ai-layer)
7. [Implementation Phases](#7-implementation-phases)
8. [Tech Stack & Dependencies](#8-tech-stack--dependencies)
9. [Beyond MVP — Future Roadmap](#9-beyond-mvp--future-roadmap)
10. [Success Metrics](#10-success-metrics)
11. [Open Decisions](#11-open-decisions)

---

## 1. Product Architecture

```
PIXL (Product)
├── PAX Format (.pax files)          ← TOML-based, LLM-native pixel art source format
├── pixl CLI (Rust binary)           ← render, validate, wfc, pack, export
├── pixl MCP Server (Rust, stdio)    ← AI integration layer with SELF-REFINE vision loop
├── pixl HTTP API (Rust, optional)   ← web integrations
├── GameTileNet Stamp Corpus         ← 2,142 labeled tiles from OpenGameArt.org (CC)
└── PIXL Studio (Flutter, future)    ← desktop editor with live preview
```

### Monorepo Layout

```
PIXL/
├── tool/                  # Rust workspace — the engine
│   ├── Cargo.toml         # workspace manifest
│   ├── crates/
│   │   ├── pixl-core/     # format types, parser, validator
│   │   ├── pixl-render/   # pixel renderer, atlas, GIF
│   │   ├── pixl-wfc/      # Wave Function Collapse + semantic constraints
│   │   ├── pixl-mcp/      # MCP server (rmcp)
│   │   ├── pixl-export/   # TexturePacker, Tiled, Godot, Unity, GBStudio
│   │   └── pixl-cli/      # CLI binary
│   └── examples/          # .pax example files
├── studio/                # Flutter desktop app (future)
├── corpus/                # GameTileNet stamp corpus converted to .pax
├── docs/
│   ├── concept/           # original specs (PIXL_SPEC.md, pax-plan.md)
│   ├── plans/             # this file
│   └── research/          # paper references, notes
└── .gitignore
```

### The SELF-REFINE Core Loop

Based on Madaan et al., NeurIPS 2023 — formally proven iterative self-improvement:

```
GENERATE → SELF-CRITIQUE → REFINE (max 3 iterations, then accept best)

Pass 1: LLM writes PAX source
  → pixl validates (format + edges + semantics)
  → pixl renders PNG at 16× zoom
  → LLM receives PNG in MCP response

Pass 2: LLM examines rendered output visually
  → identifies issues ("left edge has a gap at row 8")
  → uses pixl.refine_tile() to patch specific regions
  → pixl re-renders, LLM re-examines

Pass 3: Final polish (if needed)
  → diminishing returns after pass 3 per SELF-REFINE research
  → tool tracks iteration count in metadata

→ pixl packs atlas + exports to game engine format
```

The research shows 10–20% quality improvement per iteration, stronger models
benefit more, and gains plateau after 3 passes. The MCP tools enforce this by
tracking refinement count and surfacing it to the LLM.

---

## 2. Research Foundations

### 2.1 SELF-REFINE — The Visual Correction Loop

**Paper:** Madaan et al., "Self-Refine: Iterative Refinement with Self-Feedback",
NeurIPS 2023.

**Key findings:**
- LLMs can generate → self-critique → refine without additional training
- 10–20% quality improvement per iteration
- Stronger models (GPT-4 class) benefit more than weaker ones
- 3 passes capture most gains; beyond that, marginal returns

**How PIXL uses it:** Every MCP tool that creates or modifies a tile returns a
rendered PNG at 16× zoom. The tool description explicitly instructs the LLM to
examine the output, critique it, and refine. The MCP state tracks iteration
count per tile. After 3 refinement passes, the tool suggests accepting the
result. This is not just a design choice — it's a research-validated architecture.

### 2.2 GameTileNet — The Stamp Corpus

**Paper:** Chen & Jhala, "GameTileNet", AAAI AIIDE 2025.

**What it provides:**
- 2,142 labeled game objects from 67 tilesets (OpenGameArt.org, CC licensed)
- Object names, semantic tags, connectivity metadata
- Affordance taxonomy: walkable, obstacle, hazard, collectible, character, decoration
- Hierarchical metadata with 361 normalized tags
- 92.2% classification accuracy with ResNet18

**How PIXL uses it:**
- Pre-built stamp corpus — PAX ships with real art from day one
- Affordance taxonomy adopted as the semantic tag vocabulary for WFC rules
- No need to design our own tag vocabulary — we use theirs
- The `corpus/` directory contains GameTileNet assets converted to `.pax` stamps
- Combined with CC0 OpenGameArt HuggingFace mirror for additional assets

### 2.3 Narrative-to-Scene Pipeline

**Paper:** arXiv 2025 — lightweight pipeline transforming narrative prompts into
2D tile-based game scenes using Object-Relation-Object triples and
affordance-aware semantic embeddings.

**The gap it identifies:** spatial conflict resolution. Objects get placed in
contradictory locations because the pipeline lacks constraint propagation.

**How PIXL uses it:** PAX's semantic WFC is the missing constraint solver.
The pipeline becomes:

```
"A dark forest dungeon with a boss chamber in the southeast corner
 and three loot rooms" →
   LLM extracts spatial predicates (Object-Relation-Object triples) →
   predicates become WFC semantic constraints →
   WFC assembles the map with both edge AND semantic correctness →
   rendered PNG in 30 seconds
```

This is the killer demo. It's comprehensible to non-technical audiences.

### 2.4 Aseprite MCP Experiment

**Source:** ljvmiranda921, "Scaling Celeste Mountain I", July 2025.

**Conclusion:** "Drawing pixel art might not be the best use-case for a
tool-calling LLM." Suggested better use-cases:
- Exporting drawings into Godot-compatible spritesheets
- Recoloring artwork with different palettes
- Correcting pixel dimensions for isometric art

**How PIXL uses it:** PAX does all three suggested use-cases *and* solves
the generation problem correctly — by working at tile-level abstraction
(semantic symbols, named stamps) instead of per-pixel tool calls. We're
filling the exact gap this experiment documented.

### 2.5 LLM Spatial Reasoning Limits

**Sources:** ASCIIBench (NeurIPS 2025 Workshop), "Stuck in the Matrix" (Oct 2025),
Martorell 2025.

**Key findings:**
- LLMs fail at 2D grid reasoning when dimensions exceed ~12×12
- Performance degrades 42–84% as grid complexity increases
- Coordinate-based representations outperform grid-based
- Symbolic reasoning works; pixel-level reasoning doesn't

**How PIXL uses it:** Three-tier encoding (grid / RLE / compose) directly maps
to these accuracy thresholds. Symmetry declarations halve or quarter the
required grid size. Stamp composition eliminates spatial reasoning entirely
for large tiles.

### 2.6 Game Engine Compatibility — The Definitive Answer

**TexturePacker JSON Hash** is the de facto sprite atlas standard:
- Supports 48+ game engines out of the box
- Unity, Godot, Phaser, libGDX, Bevy, Defold, Cocos2d, GDevelop all read it
- ~80 lines of Rust to generate

**Tiled TMJ (JSON)** covers tilemap assembly:
- Tiled ships with Godot 4 export plugin (.tscn)
- SuperTiled2Unity imports TMJ directly
- Virtually every 2D engine with tilemap support has a Tiled import path

**GBStudio** needs special handling:
- 160×144 PNG grid layout for Game Boy style games
- Cult following among indie devs, nobody automates asset generation for it
- Worth implementing as a separate export target

**Strategy:** PAX is a Tiled-compatible tool. Two export functions cover the
entire game engine ecosystem. Don't fight it — plug into the most widely
supported intermediate formats.

---

## 3. The PAX 2.0 Format Specification

### 3.1 File Structure

A `.pax` file is UTF-8 text using TOML syntax. Multi-line literal strings (`'''...'''`)
embed pixel grid data without escape processing.

```
┌──────────────────────────────────────────┐
│  [pax]                                   │  Header + version
│  [theme.<name>]                          │  Semantic style layer (NEW)
│  [palette.<name>]                        │  Named color palettes
│  [stamp.<name>]                          │  Reusable macro-blocks (2×2 to 8×8)
│  [tile.<name>]                           │  Tile definitions (8×8 to 64×64)
│  [sprite.<name>]                         │  Multi-frame animated sprites
│  [tilemap.<name>]                        │  Composed tilemaps
│  [wfc_rules]                             │  Semantic constraints for WFC (NEW)
│  [atlas]                                 │  Atlas export configuration
└──────────────────────────────────────────┘
```

### 3.2 Header

```toml
[pax]
version = "0.1"
name = "dungeon_tileset"
author = "claude"
created = "2026-03-22T12:00:00Z"
```

### 3.3 Theme — The Semantic Style Layer

Themes are the bridge between LLM reasoning and pixel output. The LLM thinks
in roles ("use the background color here"), not symbols or hex values.

```toml
[theme.dark_fantasy]
palette = "dungeon"
scale = 2                    # pixel upscale factor on render
canvas = 16                  # default tile size
max_palette_size = 16        # hard limit — GBA uses 16 colors per palette
light_source = "top-left"    # universal light direction for coherent shading

# Semantic color roles — the LLM reasons in these
[theme.dark_fantasy.roles]
void    = "."                # transparent / empty
bg      = "#"                # primary structure (walls, stone)
fg      = "+"                # lit surface / foreground
accent  = "o"                # interactive elements, glow
danger  = "r"                # damage, blood, lava
neutral = "s"                # shadow, depth
life    = "g"                # organic, moss, growth

# Validation rules — tool enforces these, LLM doesn't need to think about them
[theme.dark_fantasy.rules]
bg_contrast = "luminance(fg) > luminance(bg) + 30"
accent_distinct = "hue_distance(accent, bg) > 60"
```

**`max_palette_size`** — Hard validation error if a tile uses more symbols than
this limit. GBA: 16. NES: 4 per tile. Game Boy: 4. This is a constraint, not a
suggestion — the LLM gets immediate corrective feedback when it exceeds the
palette budget.

**`light_source`** — Declares the global lighting direction. In V1, this is a
**generation hint**, not a hard validation rule: the MCP tool injects a 4×4
reference shading quad into generation prompts so the LLM sees the expected
shadow placement. Proper shadow-position validation (checking that shadow-role
pixels appear on the correct side of solid forms) is deferred to V1.1 — the
heuristic is too fragile for V1 and would produce false positives on decorative
tiles. The hint approach is simpler and more reliable: the LLM sees the
reference, follows it, and the SELF-REFINE vision loop catches deviations.

Theme inheritance (V1 — required for multi-world games like Zelda):
```toml
[theme.light_fantasy]
extends = "dark_fantasy"
palette = "cathedral"        # override just the palette
```

### 3.4 Palette

```toml
[palette.dungeon]
# symbol = "#RRGGBB" or "#RRGGBBAA"
"." = "#00000000"     # transparent
"#" = "#2a1f3d"       # stone dark
"+" = "#4a3a6d"       # stone lit
"s" = "#1a0f2e"       # shadow
"~" = "#1a3a5c"       # water
"g" = "#2d5a27"       # moss
"o" = "#c8a035"       # gold / torch glow
"r" = "#8b1a1a"       # blood / danger
"w" = "#e8e0d4"       # bone white
```

Design constraints:
- Single printable ASCII character per symbol
- Max 94 symbols (printable ASCII range 0x21–0x7E plus space)
- `"."` is conventionally transparent but not enforced
- Characters chosen for visual mnemonics where possible

### 3.5 Stamps (Macro-blocks)

Reusable 2×2 to 8×8 patterns. The composition building blocks.

```toml
[stamp.brick_2x2]
palette = "dungeon"
size = "4x4"
grid = '''
##++
#+++
++##
+##.
'''

[stamp.corner_nw]
palette = "dungeon"
size = "4x4"
grid = '''
####
#+..
#+..
##..
'''
```

### 3.6 Tile Definition

Tiles support three encoding modes, chosen based on size and LLM capability:

| Size    | LLM Accuracy | Encoding     | LLM Task                          |
|---------|-------------|--------------|-----------------------------------|
| ≤ 16×16 | High        | `grid`       | Write raw character grid          |
| 17–32   | Moderate    | `rle`        | Write run-length sequences        |
| 33–64   | Low         | `compose`    | Arrange named stamps              |
| > 64    | N/A         | tilemap      | Multi-tile composition            |

#### Grid Mode (≤ 16×16)

```toml
[tile.wall_solid]
palette = "dungeon"
size = "16x16"
symmetry = "none"            # none | horizontal | vertical | quad
edge_class = { n = "solid", e = "solid", s = "solid", w = "solid" }
tags = ["wall", "interior"]
weight = 1.0                 # WFC frequency hint

[tile.wall_solid.semantic]
type = "wall"
passable = false
light_blocks = true

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

#### Grid Mode with Symmetry

```toml
[tile.gem]
palette = "dungeon"
size = "16x16"
symmetry = "quad"            # write top-left 8×8, tool mirrors the rest
edge_class = { n = "open", e = "open", s = "open", w = "open" }

grid = '''
........
...oo...
..oooo..
.ooh....
oo......
o.......
........
........
'''
# Tool expands to full 16×16 by mirroring horizontally then vertically
```

Symmetry modes:
- `none` — grid is the full tile (default)
- `horizontal` — grid is left half, mirrored to right
- `vertical` — grid is top half, mirrored to bottom
- `quad` — grid is top-left quadrant, mirrored both axes

A 32×32 tile with `symmetry = "quad"` requires only an 16×16 grid from the LLM —
well within reliable accuracy. This is the highest-leverage feature for scaling
tile size without losing quality.

#### RLE Mode (17–32)

```toml
[tile.big_wall]
palette = "dungeon"
size = "32x32"
encoding = "rle"
edge_class = { n = "solid", e = "solid", s = "solid", w = "solid" }

# RLE format: <count><symbol> pairs separated by spaces, one row per line
# V1: every row must be explicit — no silent repetition of omitted rows.
# This prevents silent error propagation when the LLM makes a mistake on one row.
# V1.1: add optional *N suffix for explicit row repetition (e.g., "32# *4")
rle = '''
32#
2# 12+ 6# 12+
2# 12+ 6# 12+
32#
8# 16+ 8#
8# 16+ 8#
32#
'''
# Validator error if row count != declared height
```

#### Compose Mode (33–64)

```toml
[tile.castle_gate]
palette = "dungeon"
size = "32x32"
encoding = "compose"
edge_class = { n = "solid", e = "solid", s = "open", w = "solid" }

# Grid of stamp references — each stamp fills its declared size
# '@' prefix = stamp reference, bare chars = inline pixels
layout = '''
@corner_nw @wall_cap  @wall_cap  @corner_ne
@wall_side @crack_v   @detail    @wall_side
@wall_side @arch_top  @arch_top  @wall_side
@wall_side ....       ....       @wall_side
@wall_side ....       ....       @wall_side
@corner_sw @floor_edg @floor_edg @corner_se
'''
```

### 3.7 Edge System

Two levels of edge matching:

**Edge class (relaxed):** Short string labels for WFC. Tiles match if their
touching edge classes are equal.

```toml
edge_class = { n = "solid", e = "solid", s = "floor", w = "mixed_a3f2" }
```

Common classes:
- `"solid"` — uniform single symbol (walls)
- `"open"` — all transparent (doorways, sky)
- `"floor"` — ground surface
- Custom names for specific transitions: `"grass_top"`, `"water_edge"`

**Full edge (strict):** The actual symbol string from the grid border. Extracted
automatically by the tool and used for strict validation.

```toml
# Auto-extracted, not manually written:
edges = { n = "################", e = "################", s = "#+++++++++++++#", w = "################" }
```

### 3.8 Sprites & Animation

```toml
[sprite.hero_idle]
palette = "dungeon"
size = "16x16"
fps = 4
loop = true

# Frames use [[...]] array-of-tables syntax (TOML requirement:
# bare numeric keys like [frame.1] are illegal)
[[sprite.hero_idle.frame]]
index = 1
grid = '''
....wwww........
...wwrrww.......
...wrooow.......
....wwww........
..wwwwwwww......
.w++wwww++w.....
..w++++++w......
...w++++w.......
....w++w........
...w+..+w.......
..w+....+w......
..w......w......
..ww....ww......
...ww..ww.......
....w..w........
................
'''

[[sprite.hero_idle.frame]]
index = 2
encoding = "delta"
base = 1
# Delta: only changed pixels, as [x, y, "symbol"] triples
changes = [
    [4, 8, "+"],
    [5, 8, "w"],
    [6, 9, "+"],
]
```

**V1 limitation: all frames use uniform `1/fps` duration.** Professional sprite
animation uses variable frame timing (idle holds rest for 8 frames, blink for 2).
V1.1 adds per-frame `duration_ms` override:
```toml
[[sprite.hero_idle.frame]]
index = 1
duration_ms = 200    # overrides fps for this frame
```

**V1 limitation: no palette swaps.** Same tile shape with different palette
(dark dungeon wall / blue ice wall / red lava wall) requires duplicating the
grid in separate tile definitions. V1.1 adds `template` field:
```toml
[tile.wall_ice]
palette = "ice_cave"
template = "wall_dungeon"   # reuse grid from this tile
```

### 3.9 Semantic Tile Properties (GameTileNet Affordance Taxonomy)

Every tile declares semantic properties using the GameTileNet affordance
taxonomy. This vocabulary is not invented — it's adopted from a labeled
dataset of 2,142 game objects with 361 normalized tags.

**Core affordance types:**

| Affordance    | Description                              | Examples                    |
|--------------|------------------------------------------|-----------------------------|
| `walkable`   | Player can traverse                      | floor, path, bridge         |
| `obstacle`   | Blocks movement                          | wall, rock, tree            |
| `hazard`     | Damages player on contact                | lava, spikes, acid          |
| `collectible`| Player can pick up                       | coin, gem, key              |
| `character`  | NPC or player entity                     | hero, merchant, enemy       |
| `decoration` | Visual only, no gameplay effect          | torch, banner, crack        |
| `transition` | Connects different tile types            | wall-floor edge, shoreline  |
| `interactive`| Player can activate                      | door, lever, chest          |

```toml
[tile.floor_moss.semantic]
affordance = "walkable"
collision = "none"
tags = { moisture = "high", light_level = "dim", biome = "dungeon" }

[tile.floor_water.semantic]
affordance = "hazard"
collision = "full"         # blocks movement even though it's a floor tile
tags = { moisture = "max", depth = "shallow", biome = "dungeon" }

[tile.wall_torch.semantic]
affordance = "obstacle"
collision = "full"
tags = { light_source = true, light_radius = "3", biome = "dungeon" }

[tile.chest_closed.semantic]
affordance = "interactive"
collision = "full"
tags = { loot_tier = "common", requires_key = false }
```

**`collision`** — Required for game engine export. Values:
- `"full"` — bounding box covers the whole tile (walls, obstacles) — 95% of cases
- `"none"` — no collision (floors, decorations)
- `"custom"` — V1.1: custom polygon shape (not yet supported)

Without collision data, Tiled TMJ and Godot `.tres` exports produce a pretty
picture, not a playable tileset. Every game engine expects collision geometry.

The `affordance` field uses GameTileNet's taxonomy directly. The `tags` map
is freeform key-value for domain-specific properties. WFC semantic rules
can reference both.

### 3.10 WFC Semantic Rules (Affordance-Aware)

```toml
[wfc_rules]
# Rules are predicates checked during WFC constraint propagation.
# They use GameTileNet affordance types and freeform tags.
# Format: "description" = "predicate expression"

# Hazards cannot be directly adjacent to walkable without a transition
"hazard_needs_transition" = "affordance:hazard requires affordance:transition adjacent"

# Obstacles need at least one walkable or transition neighbor (no solid blocks)
"obstacle_needs_space" = "affordance:obstacle requires affordance:walkable|transition adjacent_any"

# Water tiles need moisture:high neighbors (no water next to dry stone)
"water_needs_moisture" = "affordance:hazard AND moisture:max requires moisture:high|max adjacent"

# Walkable tiles must be reachable (connected via other walkable)
"walkable_connected" = "affordance:walkable requires affordance:walkable|transition adjacent_any"

# Decorations can only be placed on obstacle tiles (torches on walls)
"decoration_on_obstacle" = "affordance:decoration requires affordance:obstacle adjacent_any"

# Interactive tiles must be accessible from walkable space
"interactive_accessible" = "affordance:interactive requires affordance:walkable adjacent_any"
```

**How rules are applied in WFC (see Section 5.8):**
- `forbids` rules → **hard constraint** during AC-3 propagation (prunes impossible tiles)
- `requires` rules → **soft weight bias** at collapse-time (boosts probability, doesn't prune)

This split prevents spurious contradictions from order-dependent "requires"
checks during early propagation. The affordance-based vocabulary means maps are
game-sensible, not just geometrically valid — water doesn't spawn next to dry
stone, torches only appear on walls, chests are always reachable.

### 3.11 Tilemap

```toml
[tilemap.dungeon_room]
size = "8x6"              # in tiles
tile_size = "16x16"       # pixel size per tile
layers = ["floor", "walls"]

[tilemap.dungeon_room.layer.floor]
z_order = 0                # render order: 0=background, 1=mid, 2=foreground
grid = '''
floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone
floor_stone floor_stone floor_moss  floor_water floor_water floor_moss  floor_stone floor_stone
floor_stone floor_moss  floor_moss  floor_water floor_water floor_moss  floor_moss  floor_stone
floor_stone floor_stone floor_moss  floor_water floor_water floor_moss  floor_stone floor_stone
floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone
floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone
'''
```

### 3.12 Atlas Export

```toml
[atlas]
format = "png"
padding = 1
columns = 8
include = ["wall_*", "floor_*"]
output = "dungeon_atlas.png"
# Primary: TexturePacker JSON Hash — de facto standard, 48+ engines
map_output = "dungeon_atlas.json"
# Secondary: Tiled TMJ for tilemap data
tiled_output = "dungeon.tmj"
```

**TexturePacker JSON Hash format** (generated by `pixl atlas`):
```json
{
  "frames": {
    "wall_solid": {
      "frame": {"x": 1, "y": 1, "w": 16, "h": 16},
      "rotated": false,
      "trimmed": false,
      "spriteSourceSize": {"x": 0, "y": 0, "w": 16, "h": 16},
      "sourceSize": {"w": 16, "h": 16},
      "pivot": {"x": 0.5, "y": 0.5}
    }
  },
  "meta": {
    "app": "pixl",
    "version": "0.1.0",
    "image": "dungeon_atlas.png",
    "format": "RGBA8888",
    "size": {"w": 256, "h": 128},
    "scale": "1"
  }
}
```

This single format gives you Unity, Godot, Phaser, libGDX, Bevy, Defold,
Cocos2d, and GDevelop support out of the box.

---

## 4. Rust Crate Architecture

### 4.1 Workspace Layout

```
PIXL/                                     # monorepo root
├── tool/                                 # Rust workspace
│   ├── Cargo.toml                        # workspace manifest
│   ├── crates/
│   │   ├── pixl-core/                    # format types, parser, validator
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── types.rs              # PaxFile, Palette, Theme, Tile, Sprite, etc.
│   │   │       ├── parser.rs             # TOML → PaxFile deserialization
│   │   │       ├── grid.rs              # grid string → Vec<Vec<char>> parser
│   │   │       ├── rle.rs                # RLE encoder/decoder
│   │   │       ├── compose.rs            # stamp composition resolver
│   │   │       ├── symmetry.rs           # symmetry expansion (quad/h/v)
│   │   │       ├── edges.rs              # edge extraction + classification
│   │   │       ├── validate.rs           # format + edge + semantic validation
│   │   │       └── theme.rs              # theme resolver, role → symbol mapping
│   │   │
│   │   ├── pixl-render/                  # pixel grid → image output
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── renderer.rs           # Tile → ImageBuffer<Rgba<u8>>
│   │   │       ├── atlas.rs              # TexturePacker JSON Hash + atlas PNG
│   │   │       ├── gif.rs                # animated GIF export
│   │   │       ├── preview.rs            # 16× zoom preview for SELF-REFINE loop
│   │   │       └── spritesheet.rs        # spritesheet layout
│   │   │
│   │   ├── pixl-wfc/                     # Wave Function Collapse engine
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── wfc.rs                # core WFC algorithm
│   │   │       ├── adjacency.rs          # edge-class → adjacency rule builder
│   │   │       ├── semantic.rs           # affordance-aware constraint filter
│   │   │       ├── backtrack.rs          # backtracking extension
│   │   │       └── autotile.rs           # 47-tile bitmask autotile computation
│   │   │
│   │   ├── pixl-mcp/                     # MCP server
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── server.rs             # rmcp server setup (stdio + HTTP/SSE)
│   │   │       ├── tools.rs              # MCP tool definitions
│   │   │       ├── handlers.rs           # tool request handlers
│   │   │       └── state.rs              # in-memory PaxFile state + refinement tracking
│   │   │
│   │   ├── pixl-export/                  # game engine export
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── texturepacker.rs      # TexturePacker JSON Hash (48+ engines)
│   │   │       ├── tiled.rs              # Tiled TMJ JSON export
│   │   │       ├── godot.rs              # Godot .tres tileset export
│   │   │       ├── unity.rs              # Unity tilemap metadata
│   │   │       └── gbstudio.rs           # GBStudio 160×144 PNG grid layout
│   │   │
│   │   └── pixl-cli/                     # CLI binary
│   │       └── src/
│   │           └── main.rs               # clap-based CLI, delegates to crates above
│   │
│   ├── examples/                         # example .pax files
│   │   ├── dungeon.pax
│   │   ├── platformer.pax
│   │   └── gameboy.pax
│   │
│   └── tests/                            # integration tests
│       ├── parse_roundtrip.rs
│       ├── render_snapshot.rs
│       ├── wfc_deterministic.rs
│       └── mcp_workflow.rs
│
├── studio/                               # PIXL Studio — Flutter desktop app (future)
│
├── corpus/                               # GameTileNet stamp corpus (.pax format)
│
├── docs/
│   ├── concept/                          # original specs
│   ├── plans/                            # implementation plans
│   └── research/                         # paper references
│
└── .gitignore
```

### 4.2 Crate Dependency Graph

```
pixl-cli
  ├── pixl-core
  ├── pixl-render   → pixl-core
  ├── pixl-wfc      → pixl-core
  ├── pixl-mcp      → pixl-core, pixl-render, pixl-wfc
  └── pixl-export   → pixl-core, pixl-render
```

`pixl-core` has zero image dependencies — it's pure data types + parsing + validation.
This keeps it WASM-compilable and testable without rendering infrastructure.

### 4.3 Core Types (`pixl-core/src/types.rs`)

```rust
use std::collections::HashMap;

pub struct PaxFile {
    pub header: Header,
    pub themes: HashMap<String, Theme>,
    pub palettes: HashMap<String, Palette>,
    pub stamps: HashMap<String, Stamp>,
    pub tiles: HashMap<String, Tile>,
    pub sprites: HashMap<String, Sprite>,
    pub tilemaps: HashMap<String, Tilemap>,
    pub wfc_rules: Vec<SemanticRule>,
    pub atlas: Option<AtlasConfig>,
}

pub struct Header {
    pub version: String,
    pub name: String,
    pub author: String,
    pub created: Option<String>,
}

pub struct Theme {
    pub name: String,
    pub palette: String,
    pub scale: u32,
    pub canvas: u32,
    pub max_palette_size: Option<u32>,      // hard limit: GBA=16, NES=4, GB=4
    pub light_source: Option<String>,       // "top-left", "top", etc. (V1: hint only)
    pub roles: HashMap<String, char>,       // role_name → symbol
    pub rules: HashMap<String, String>,     // rule_name → expression
    pub extends: Option<String>,
}

pub struct Palette {
    pub name: String,
    // TOML deserializes quoted keys as String, not char.
    // Requires custom deserializer: validate single-char keys, parse hex → Rgba.
    #[serde(deserialize_with = "deserialize_symbols")]
    pub symbols: HashMap<char, Rgba>,
}

// Custom deserializer: HashMap<String, String> → HashMap<char, Rgba>
// Validates: each key is exactly 1 char, each value is #RRGGBB or #RRGGBBAA
fn deserialize_symbols<'de, D>(deserializer: D) -> Result<HashMap<char, Rgba>, D::Error>
where D: serde::Deserializer<'de> { /* ... */ }

pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

pub struct Stamp {
    pub name: String,
    pub palette: String,
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Vec<char>>,
}

pub enum Encoding {
    Grid,
    Rle,
    Compose,
}

pub enum Symmetry {
    None,
    Horizontal,
    Vertical,
    Quad,
}

pub struct EdgeClass {
    pub n: String,
    pub e: String,
    pub s: String,
    pub w: String,
}

pub enum Collision {
    Full,       // bounding box covers whole tile — 95% of cases
    None,       // no collision
    Custom,     // V1.1: custom polygon (not yet supported)
}

pub struct SemanticProperties {
    pub affordance: String,                 // GameTileNet: "walkable", "obstacle", "hazard", etc.
    pub collision: Collision,               // required for game engine export
    pub tags: HashMap<String, String>,      // arbitrary k/v for custom WFC rules
}

pub struct Tile {
    pub name: String,
    pub palette: String,
    pub width: u32,
    pub height: u32,
    pub encoding: Encoding,
    pub symmetry: Symmetry,
    pub edge_class: EdgeClass,
    pub semantic: Option<SemanticProperties>,
    pub tags: Vec<String>,
    pub weight: f64,
    pub grid: Vec<Vec<char>>,              // always resolved after parse
    // Raw source for round-tripping
    pub raw_grid: Option<String>,
    pub raw_rle: Option<String>,
    pub raw_compose: Option<String>,
}

pub struct Sprite {
    pub name: String,
    pub palette: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub loop_mode: bool,
    pub frames: Vec<Frame>,
}

pub struct Frame {
    pub index: u32,
    pub encoding: FrameEncoding,
    pub grid: Vec<Vec<char>>,              // resolved full frame
}

pub enum FrameEncoding {
    Full { raw_grid: String },
    Delta { base: u32, changes: Vec<DeltaChange> },
}

pub struct DeltaChange {
    pub x: u32,
    pub y: u32,
    pub sym: char,
}

pub struct Tilemap {
    pub name: String,
    pub width: u32,                        // in tiles
    pub height: u32,
    pub tile_width: u32,                   // pixels per tile
    pub tile_height: u32,
    pub layers: Vec<TilemapLayer>,
}

pub struct TilemapLayer {
    pub name: String,
    pub z_order: i32,                      // render order: 0=bg, 1=mid, 2=fg
    pub grid: Vec<Vec<String>>,            // tile names
}

pub struct AtlasConfig {
    pub format: String,
    pub padding: u32,
    pub columns: u32,
    pub include: Vec<String>,
    pub output: String,
    pub map_output: Option<String>,
}

pub struct SemanticRule {
    pub description: String,
    pub predicate: String,
}
```

---

## 5. Core Algorithms

### 5.1 Grid Parser

```
Input:  multi-line string from TOML '''...''' block
Output: Vec<Vec<char>>

Algorithm:
  1. Trim leading/trailing blank lines
  2. Split by newline
  3. For each row: collect chars, validate length == declared width
  4. Validate row count == declared height
  5. Validate all chars exist in referenced palette
```

### 5.2 RLE Parser

```
Input:  "8# 4+ 12# 8."
Output: ['#','#','#','#','#','#','#','#','+','+','+','+','#',...]

Algorithm:
  For each token (split by whitespace):
    Parse leading digits → count (default 1 if no digits)
    Remaining char(s) → symbol (must be single char)
    Emit symbol × count
  Validate: sum of counts == row width
```

### 5.3 Symmetry Expansion

```
Input:  partial grid + symmetry mode
Output: full grid

Horizontal (grid = left half, width = tile_width / 2):
  For each row:
    full_row = row + reverse(row)

Vertical (grid = top half, height = tile_height / 2):
  full_grid = grid + reverse(grid)

Quad (grid = top-left quadrant):
  1. Expand horizontal: each row → row + reverse(row)
  2. Expand vertical: grid → grid + reverse(grid)
```

### 5.4 Stamp Composition

```
Input:  layout string + stamp registry + target tile size
Output: resolved pixel grid

Algorithm:
  pixel_grid = new Grid(tile_width, tile_height)
  cursor_y = 0

  For each layout row:
    cursor_x = 0
    max_height = 0
    For each token:
      If token starts with '@':
        stamp = registry[token.trim_start('@')]
        Blit stamp.grid onto pixel_grid at (cursor_x, cursor_y)
        cursor_x += stamp.width
        max_height = max(max_height, stamp.height)
      Else:
        // Inline pixels
        For each char in token:
          pixel_grid[cursor_y][cursor_x] = char
          cursor_x += 1
        max_height = max(max_height, 1)
    cursor_y += max_height
```

### 5.5 Edge Extraction & Classification

```
Given tile.grid[h][w]:
  edge_n = grid[0][0..w]           // first row
  edge_s = grid[h-1][0..w]         // last row
  edge_w = grid[0..h].map(|r| r[0])     // first column
  edge_e = grid[0..h].map(|r| r[w-1])   // last column

Auto-classification:
  If all symbols identical      → "solid" (or "solid_<sym>" for specificity)
  If all symbols == '.'         → "open"
  If edge == reverse(edge)      → "sym_<hash4>"
  Else                          → "mixed_<hash8>"

  where hash = first N chars of std::hash::DefaultHasher output (hex-encoded)
  // No crypto dep needed — these are 16-char strings, not security-sensitive
```

### 5.6 Wave Function Collapse

```
Data structures:
  Cell:            BitSet of possible tile indices
  Grid:            2D array of Cells, size W × H
  AdjacencyRules:  HashMap<(tile_idx, Direction), BitSet<tile_idx>>
  Weights:         Vec<f64> indexed by tile_idx
  PropStack:       Vec<(x, y)>

Algorithm WFC(width, height, tiles, rules, semantic_rules, seed):
  rng = Rng::seed(seed)

  1. INITIALIZE
     For each cell: cell.possible = BitSet::all(num_tiles)

  2. MAIN LOOP
     While any cell has popcount > 1:

       a. OBSERVE — pick lowest entropy cell
          // Clamp weights to avoid NaN: 0.0 × log(0.0) = 0.0 × -inf = NaN
          w_i = weight[tile_i].max(1e-10)
          entropy(cell) = -Σ (p_i × log(p_i))
            where p_i = w_i / Σ w[possible]
          chosen = argmin(entropy), tie-break by rng

       b. COLLAPSE — pick one tile for chosen cell
          // Apply "requires" semantic rules as soft weight bias (not hard prune)
          adjusted_weights = weights.clone()
          For each t in chosen.possible:
            adjusted_weights[t] *= semantic_weight_bias(tiles[t], neighbors, semantic_rules)
          tile = weighted_random(chosen.possible, adjusted_weights, rng)
          chosen.possible = BitSet::just(tile)
          push chosen → PropStack

       c. PROPAGATE (AC-3)
          While stack not empty:
            (cx, cy) = stack.pop()
            For each direction d in [N, E, S, W]:
              (nx, ny) = neighbor(cx, cy, d)
              If out of bounds: continue

              // Compute allowed set for neighbor (edge-class matching)
              allowed = BitSet::empty()
              For each t in cell(cx,cy).possible:
                allowed |= rules[(t, d)]

              // Apply "forbids" semantic rules ONLY (hard prune, safe)
              For each t in cell(nx,ny).possible ∩ allowed:
                If !semantic_forbids(tiles[t], neighbors, semantic_rules):
                  allowed.remove(t)

              // Prune neighbor
              before = cell(nx,ny).possible.count()
              cell(nx,ny).possible &= allowed
              after = cell(nx,ny).possible.count()

              If after == 0: return Err(Contradiction)
              If after < before: push (nx,ny) → stack

  3. RESULT
     Return grid where each cell has exactly one tile

Adjacency rule construction:
  For each pair (A, B) in tiles:
    If A.edge_class.e == B.edge_class.w:
      rules[(A, East)] |= {B}
      rules[(B, West)] |= {A}
    If A.edge_class.s == B.edge_class.n:
      rules[(A, South)] |= {B}
      rules[(B, North)] |= {A}
```

### 5.7 Backtracking Extension

```
When Contradiction detected:
  1. Maintain snapshot stack: Vec<GridSnapshot>
  2. Before each collapse, push snapshot
  3. On contradiction: pop snapshot, remove the tile that caused it
  4. If cell becomes empty → pop deeper
  5. Re-collapse, re-propagate
  Max backtracks before full restart: 100
  Max restarts: 10
  If all restarts fail → return Err with diagnostic
```

### 5.8 Semantic Constraint Filter

**Critical design decision:** "forbids" rules are hard constraints applied
during AC-3 propagation. "requires" rules are soft weight biases applied at
collapse-time only. This distinction prevents spurious contradictions.

The problem: during early propagation, most cells are still open. A "requires
adjacent" check looks at current possibilities — which early in WFC are all
tiles. Later, as the grid fills, the rule might prune a valid tile right before
its required neighbor would have been placed. The constraint is order-dependent
in ways that produce unnecessary contradictions.

```
// HARD CONSTRAINT — applied during propagation (AC-3)
semantic_forbids(candidate_tile, neighbor_tiles, rules) → bool:
  For each rule in rules:
    match rule.predicate:
      "affordance:X forbids affordance:Y adjacent"
        → If candidate has affordance X and ANY neighbor possibility
          has affordance Y → return false (prune candidate)
  Return true

// SOFT BIAS — applied at collapse-time only (weight adjustment)
semantic_weight_bias(candidate_tile, neighbor_tiles, rules) → f64:
  bias = 1.0
  For each rule in rules:
    match rule.predicate:
      "affordance:X requires Y:Z adjacent"
        → If candidate has affordance X and at least one neighbor
          has property Y == Z → bias *= 2.0 (boost)
        → If no neighbor has it → bias *= 0.1 (suppress, don't prune)
      "affordance:X requires affordance:Y adjacent_any"
        → Same logic with affordance check
  Return bias
```

The "forbids" check injects into the propagation loop (safe — it only removes
impossible tiles). The "requires" check adjusts weights at collapse (safe — it
biases toward correct placement without hard-pruning valid states). This is
simpler to implement and produces far fewer contradictions.

### 5.9 Autotile Bitmask (47-tile blob)

```
compute_bitmask(tilemap, x, y) → u8:
  this_type = tilemap[y][x].semantic.type
  mask = 0u8

  // 8-neighbor weights: NW=1 N=2 NE=4 W=8 E=16 SW=32 S=64 SE=128
  For each (dx, dy, bit) in neighbors:
    If in_bounds(x+dx, y+dy) && tilemap[y+dy][x+dx].type == this_type:
      mask |= bit

  // Corner cleanup: corners only count if both adjacent edges present
  If (mask & NW) && !(mask & N && mask & W): mask &= !NW
  If (mask & NE) && !(mask & N && mask & E): mask &= !NE
  If (mask & SW) && !(mask & S && mask & W): mask &= !SW
  If (mask & SE) && !(mask & S && mask & E): mask &= !SE

  return BITMASK_TO_47[mask]    // 256 → 47 lookup table
```

### 5.10 Delta Frame Resolution

```
resolve_delta(base_frame, delta) → Frame:
  result = base_frame.grid.clone()
  For each change in delta.changes:
    result[change.y][change.x] = change.sym
  Return result
```

### 5.11 Atlas Packing

Grid-based packing. **V1 validation rule: all tiles in an atlas must share the
same dimensions.** A PAX file CAN define mixed-size tiles (8×8 floors + 32×32
bosses), but `pixl atlas` rejects mixed sizes with an actionable error rather
than producing a corrupted atlas. For mixed sizes, export separate atlases per
size tier. Proper bin-packing (via `rectangle-pack` crate) is a V1.1 feature.

```
pack_atlas(tiles, columns, padding) → Result<(Image, Metadata), Error>:
  // Validate uniform size
  sizes = tiles.map(|t| (t.width, t.height)).collect::<HashSet>()
  If sizes.len() > 1: return Err("Mixed tile sizes in atlas. Sizes found: {sizes}.
    Use --include to filter by size, or export separate atlases.")

  tile_w = tiles[0].width + padding
  tile_h = tiles[0].height + padding
  rows = ceil(tiles.len() / columns)

  atlas = Image::new(columns * tile_w + padding, rows * tile_h + padding)
  metadata = HashMap::new()

  For (i, tile) in tiles.iter().enumerate():
    col = i % columns
    row = i / columns
    x = padding + col * tile_w
    y = padding + row * tile_h
    blit(atlas, x, y, render(tile))
    metadata[tile.name] = { x, y, w: tile.width, h: tile.height, index: i }

  Return (atlas, metadata)
```

---

## 6. MCP Integration — The AI Layer

### 6.1 Transport

Using `rmcp` 1.2.0 (official MCP Rust SDK):
- Primary: stdio transport (`transport-io` feature) — for Claude Code / Claude Desktop
- Secondary: HTTP+SSE (`transport-streamable-http-server`) — for web integrations

### 6.2 State Model

The MCP server maintains an in-memory `PaxFile` that tools mutate. This is the
"document" the LLM builds up during a conversation.

```rust
struct McpState {
    file: PaxFile,
    render_cache: HashMap<String, Vec<u8>>,  // tile_name → last rendered PNG bytes
    refinement_count: HashMap<String, u32>,  // tile_name → SELF-REFINE iteration count
}
```

The state is ephemeral per session — each conversation starts fresh. The LLM can
export the accumulated .pax file at any time via `pixl.get_file()`.

The `refinement_count` tracks how many times each tile has been refined. Per
SELF-REFINE research, the tool signals diminishing returns after 3 iterations.

### 6.3 Tool Definitions

```
pixl.session_start()
    Entry point for any MCP session. Returns available themes, palettes,
    existing tiles/stamps if any, and a suggested workflow.
    The LLM calls this first to understand what's available before generating.

pixl.get_palette(theme_name)
    Returns the full symbol table for a theme's palette — symbol chars,
    hex colors, and semantic roles. The LLM reads this before writing any tile
    to know what characters are available.

pixl.create_palette(name, symbols)
    Create or replace a palette.
    Returns: palette summary with symbol count.

pixl.set_theme(name, palette, roles, scale?, max_palette_size?, light_source?)
    Create or replace a theme with semantic color roles and constraints.
    Returns: theme summary with role→symbol mapping.
    If light_source set: all subsequent create_tile responses include a 4×4
    reference shading quad showing expected shadow placement.

pixl.create_stamp(name, palette, size, grid)
    Create a reusable macro-block.
    Returns: rendered preview (base64 PNG at 16× zoom).

pixl.create_tile(name, palette, size, grid_or_rle, encoding?, symmetry?,
                 edge_class?, tags?, semantic?)
    Create a tile. Auto-extracts edges. Auto-classifies edge classes if not provided.
    Returns: {
      ok: bool,
      preview: base64 PNG at 16× zoom,     ← VISION FEEDBACK
      edges_extracted: { n, e, s, w },
      edge_class_auto: { n, e, s, w },
      validation: { errors: [], warnings: [] },
      compatible_neighbors: { n: [...], e: [...], s: [...], w: [...] }
    }

pixl.compose_tile(name, palette, size, layout)
    Create a tile via stamp composition.
    Returns: same as create_tile.

pixl.create_sprite(name, palette, size, frames, fps, loop?)
    Create animated sprite. Frame 1 = full grid, rest = delta.
    Returns: preview of frame 1 + frame count confirmation.

pixl.add_frame(sprite_name, encoding, grid_or_delta, base?)
    Add a frame to an existing sprite.
    Returns: rendered frame preview.

pixl.validate(check_edges?, check_semantic?)
    Validate entire in-memory PAX file.
    Returns: { errors: [], warnings: [], stats: { tiles, sprites, stamps } }

pixl.render_tile(name, scale?)
    Render a tile to PNG.
    Returns: base64 PNG.

pixl.render_atlas(include?, columns?, padding?)
    Pack tiles into atlas.
    Returns: base64 PNG + JSON metadata.

pixl.generate_tilemap(width, height, seed?, constraints?)
    Run WFC to produce a tilemap.
    Returns: rendered PNG + tile name grid + seed used.

pixl.suggest_compatible(tile_name, direction)
    List tiles compatible with a given tile's edge in a direction.
    Returns: compatible tile names with edge classes.

pixl.check_edge_pair(tile_a, direction, tile_b)
    Check if tile_b can be placed in the given direction relative to tile_a.
    Most common LLM need: "I just wrote tile B — can it go East of tile A?"
    Returns: { compatible: bool, reason: string, edge_a: string, edge_b: string }
    ~20 lines in handlers.rs. Dramatically improves the generation loop vs.
    running full validate or scanning suggest_compatible results.

pixl.refine_tile(name, region_x, region_y, region_w, region_h, grid_or_rle, encoding?)
    Patch a sub-region of an existing tile (for iterative refinement).
    Coordinates are in tile-pixels (1 char = 1 pixel). Region grid uses the
    same encoding as tile creation (raw grid or RLE, default: grid).
    If region extends beyond tile boundary → error with actual tile dimensions.
    The patched region overwrites existing pixels; surrounding pixels are unchanged.
    Returns: updated full tile preview (16× zoom) + refinement count + diff summary.

pixl.get_file()
    Return the full .pax source of the current in-memory state.
    Returns: PAX file as string.

pixl.list_tiles()
    List all tiles with edge classes, tags, and semantic properties.
    Returns: tile summary array.

pixl.list_stamps()
    List available stamps with sizes.
    Returns: stamp summary array.
```

### 6.4 The SELF-REFINE Vision Loop

**Research basis:** Madaan et al., "Self-Refine: Iterative Refinement with
Self-Feedback", NeurIPS 2023. Formally proven: generate → self-critique →
refine, no additional training required. 10–20% improvement per pass, capped
at 3 passes for optimal cost/quality.

Every tool that creates or modifies a tile returns:

1. **Rendered PNG at 16× zoom** — the LLM examines this visually
2. **Refinement count** — how many times this tile has been refined
3. **Refinement guidance** — after pass 3, tool suggests accepting the result

The tool description explicitly instructs the LLM:

> "Examine the preview image. If the rendered tile does not match your intent,
> use pixl.refine_tile() to patch specific regions. You have refined this tile
> {n}/3 times. After 3 refinements, accept the best result."

This is not a nice-to-have — it's the core architecture. The difference between
"generate and hope" and "generate, see, fix" is the difference between
unreliable and production-quality output.

Implementation: `pixl-render` has a `preview()` function that renders at
configurable zoom (default 16×) with optional grid lines at tile boundaries.
The MCP state tracks `refinement_count[tile_name]` and surfaces it in every
response.

### 6.5 Progressive Refinement Workflow

For complex tiles, the MCP tools support a 3-pass workflow that maps directly
to the three-tier encoding:

**Pass 1 — Structure (compose):** LLM uses `pixl.compose_tile()` with stamps
to lay out the overall structure. No pixel-level reasoning needed. This is
always reliable regardless of tile size.

**Pass 2 — Detail (grid):** LLM uses `pixl.refine_tile()` to fill in specific
8×8 regions with custom pixel grids. Each sub-region is within reliable accuracy.
The tool assembles them and returns the full preview.

**Pass 3 — Variation (automatic):** Tool generates procedural variants from a
base tile — random crack placement, moss density variation, subtle color
shifting within palette. No LLM intervention needed.

### 6.6 Narrative-to-Map Pipeline (Killer Demo)

The highest-impact demo combines the Narrative-to-Scene research with PAX's
semantic WFC:

```
Input:  "A dark forest dungeon with a boss chamber in the southeast corner
         and three loot rooms"

Step 1: LLM extracts spatial predicates (Object-Relation-Object triples)
        → boss_chamber LOCATED_AT southeast
        → loot_room COUNT 3
        → loot_room CONNECTED_TO corridor
        → dungeon BIOME dark_forest

Step 2: Predicates become WFC semantic constraints
        → boss_chamber tile has affordance:hazard, size≥8×8, position bias SE
        → loot_room tiles have affordance:interactive, count=3
        → all rooms connected via affordance:walkable paths

Step 3: WFC assembles the map with edge AND semantic correctness

Step 4: Rendered PNG in ~30 seconds

Output: A playable dungeon layout with correct tile transitions,
        accessible rooms, and semantically coherent placement
```

This is the screenshot that goes viral on r/gamedev and r/indiegaming. It's
comprehensible to non-technical audiences in a way that "LLM-native tileset
format" is not.

**Implementation:** This is a post-V1 MCP tool (`pixl.narrate_map()`) that
wraps the WFC engine with an LLM preprocessing step. The WFC and semantic
constraint engine from V1 are the foundation — the narrative layer is
orchestration on top.

---

## 7. Implementation Phases

**Status: V1 COMPLETE. All 6 phases shipped + V1.1/V1.2 features.**

**Authoritative format spec:** `docs/specs/pax.md` (PAX 2.0)
**All format details, algorithms, and type definitions** live in the spec.

### Completion Summary

| Phase | Status | Tests | Key Deliverables |
|-------|--------|-------|-----------------|
| Phase 1 — Parser & Validator | DONE | 75 | types, parser, grid, RLE, symmetry, compose, template, theme, edges, rotate, cycle, blueprint, validate, style, stampgen, resolve |
| Phase 2 — Rendering | DONE | 20 | renderer, atlas, GIF, preview, import |
| Phase 3 — WFC Engine | DONE | 29 | adjacency, WFC, semantic constraints, autotile, narrate |
| Phase 4 — MCP Server | DONE | 4 | 19 MCP tools, state management |
| Phase 5 — Game Engine Export | DONE | 8 | TexturePacker, Tiled, Godot, Unity, GBStudio |
| Phase 6 — Polish | DONE | — | README, CI, cargo fmt, clippy |
| V1.1 — Import Bridge | DONE | +5 | diffusion import, platformer + gameboy examples |
| V1.2 — Style Latent | DONE | +7 | style extraction, scoring, MCP tools |
| V1.2 — Stamp Generation | DONE | +6 | 8 procedural patterns |
| V1.5 — Narrate Pipeline | DONE | +6 | predicate parser, WFC pins, path validation |
| Studio Integration | DONE | — | HTTP API (20 endpoints), generate/context, pixl_backend.dart |

**Total: 136 tests, ~11K LOC Rust, 15 CLI commands, 19 MCP tools, 20 HTTP endpoints**

### Original Phase Details (for reference)

### Phase 1 — Core Format & Parser (Week 1–3)

**Goal:** Parse any valid `.pax` file into `PaxFile`, validate it, round-trip.
Budget 3 weeks — theme inheritance has edge cases (cycle detection, missing
parent roles), compose grammar needs careful error propagation, and custom
serde deserializers require testing.

**Start here:** Write 3 example `.pax` files by hand BEFORE any parser code.
Parse them mentally. Cheap format-ambiguity discovery.

Tasks:
- [ ] Write `dungeon.pax`, `platformer.pax`, `gameboy.pax` example files
- [ ] Define all types in `types.rs` (spec Section 8.2, 9.1–9.4)
- [ ] Custom serde deserializer for palette `HashMap<char, Rgba>` (spec Section 4)
- [ ] TOML → PaxFile deserialization (`parser.rs`)
- [ ] Grid string parser (`grid.rs`)
- [ ] RLE decoder/encoder — every row explicit, no silent repetition (`rle.rs`)
- [ ] Symmetry expansion — quad/h/v (`symmetry.rs`)
- [ ] Compose resolver — `@stamp_ref` and `_` void only, no inline pixels (`compose.rs`)
- [ ] Tile template inheritance resolver — no template chains (`template.rs`)
- [ ] Tile auto-rotation — 4way/flip/8way variant generation (`rotate.rs`)
- [ ] Color cycling definitions — index rotation logic (`cycle.rs`)
- [ ] Palette swap definitions — full and partial swap (`parser.rs`)
- [ ] Spriteset + sprite + frame parsing — `[[...]]` array-of-tables (`parser.rs`)
- [ ] Frame resolution — grid/delta/linked encoding types
- [ ] Theme resolver — roles, constraints, inheritance, `extends` cycle detection (`theme.rs`)
- [ ] Theme constraint evaluation — declarative checks, not expressions (spec Section 3)
- [ ] Edge extraction + FNV-1a auto-classification (`edges.rs`, `fnv` crate)
- [ ] Auto-fix edge classes from grid content (`--fix` flag)
- [ ] All validation rules from spec Section 16 (`validate.rs`)
- [ ] Palette size hard error, atlas mixed-size error
- [ ] Blueprint system — built-in anatomy models in `pixl-core` (`blueprint.rs`)
- [ ] `Blueprint::resolve(w, h)` → pixel-coordinate landmarks
- [ ] `Blueprint::render_guide(w, h)` → text map for LLM/tool consumption
- [ ] Built-in models: humanoid_chibi (6-head), humanoid_realistic (8-head)
- [ ] Eye size rules by canvas size (spec Section 18.4)
- [ ] `pixl blueprint 32x48 --model chibi` CLI command
- [ ] `pixl validate` + `pixl check --fix` CLI commands
- [ ] Unit tests for every parser + algorithm

**Deliverable:** `pixl validate dungeon.pax` passes. `pixl check --fix` works.
`pixl blueprint 32x48` prints anatomy guide.

### Phase 2 — Rendering & Display (Week 3–5)

**Goal:** Render tiles, sprites, atlases. All display algorithms from spec
Section 14.

Tasks:
- [ ] Tile → `ImageBuffer<Rgba<u8>>` renderer — nearest-neighbor only (`renderer.rs`)
- [ ] Scale factor support (1x through 16x)
- [ ] Palette swap application at render time (`renderer.rs`)
- [ ] Color cycling application at render time (`renderer.rs`)
- [ ] Layer compositing — normal/multiply/screen/add blend modes (`composite.rs`)
- [ ] Animation frame selection — uniform and variable duration (`animation.rs`)
- [ ] Palette LUT texture generation — grayscale index + LUT rows (`palette_lut.rs`)
- [ ] 16x zoom preview with grid lines for SELF-REFINE loop (`preview.rs`)
- [ ] Atlas packer — uniform-size grid layout, mixed-size validation error (`atlas.rs`)
- [ ] TexturePacker JSON Hash metadata — NO `animationTags` (that's Aseprite) (`atlas.rs`)
- [ ] Aseprite-compatible `frameTags` JSON for animation data
- [ ] Sprite frame rendering — resolve grid/delta/linked frames
- [ ] Animated GIF export (`gif.rs`)
- [ ] Spritesheet export (`spritesheet.rs`)
- [ ] 9-slice tile rendering — corners fixed, edges tiled, center tiled (`renderer.rs`)
- [ ] `pixl render`, `pixl atlas`, `pixl gif`, `pixl preview` CLI commands
- [ ] Snapshot tests: render examples, compare against golden PNGs

**Deliverable:** `pixl render dungeon.pax --tile wall_solid --scale 4 --out wall.png`

### Phase 3 — WFC Engine (Week 5–6)

**Goal:** Generate valid, game-sensible tilemaps from edge-compatible tile sets.

Tasks:
- [ ] Adjacency rule builder from edge classes (`adjacency.rs`)
- [ ] Variant group expansion — group members share edge compatibility (`adjacency.rs`)
- [ ] Core WFC — entropy observation with NaN-safe weight clamping (`wfc.rs`)
- [ ] Weighted random collapse
- [ ] AC-3 constraint propagation
- [ ] `forbids` rules — hard prune during propagation (`semantic.rs`)
- [ ] `requires` rules — soft weight bias at collapse time only (`semantic.rs`)
- [ ] Backtracking with snapshot stack (`backtrack.rs`)
- [ ] Seed-based deterministic RNG
- [ ] Contradiction diagnostics
- [ ] Constraint painting — pins, zones, paths (`wfc.rs`)
- [ ] Path validation — BFS on collapsed grid, retry on blocked path
- [ ] BITMASK_TO_47 generated in `build.rs`, validated by tests (`autotile.rs`)
- [ ] Dual-grid autotiling — 5 types / 15 tiles (`dual_grid.rs`)
- [ ] `pixl wfc`, `pixl autotile` CLI commands
- [ ] Tests: deterministic WFC with fixed seeds, known-good outputs
- [ ] Test: BITMASK_TO_47 validates against cr31.co.uk reference

**Deliverable:** `pixl wfc dungeon.pax --width 12 --height 8 --seed 42 --out map.png`

### Phase 4 — MCP Server (Week 6–8)

**Goal:** Full MCP integration. Claude can create a complete tileset with
palette swaps, animations, and WFC maps in one conversation.

Tasks — Discovery tools:
- [ ] MCP server scaffold with `rmcp` stdio transport (`server.rs`)
- [ ] In-memory `PaxFile` state + refinement counter (`state.rs`)
- [ ] `pixl.session_start()` — theme, palette, stamps, light_source hint
- [ ] `pixl.get_palette()` — symbol table with roles
- [ ] `pixl.list_tiles()`, `pixl.list_stamps()`, `pixl.list_sprites()`

Tasks — Creation tools:
- [ ] `pixl.create_tile()` with auto-edge, edge context pixels in response
- [ ] `pixl.compose_tile()` — stamp composition
- [ ] `pixl.create_stamp()`
- [ ] `pixl.create_spriteset()` + `pixl.add_sprite()`
- [ ] `pixl.define_palette_swap()` — full and partial
- [ ] `pixl.define_cycle()`

Tasks — Refinement tools (SELF-REFINE loop):
- [ ] `pixl.refine_tile()` — sub-region patching with iteration tracking
- [ ] `pixl.check_edge_pair()` — pairwise edge compatibility
- [ ] `pixl.render_with_swap()` — preview palette swap on tile/sprite
- [ ] `pixl.render_cycle_frame()` — preview cycle animation frame
- [ ] `pixl.get_edge_context()` — actual border pixels for visual continuity

Tasks — Validation & generation:
- [ ] `pixl.validate()`
- [ ] `pixl.generate_wfc_map()` — with constraint painting support
- [ ] `pixl.generate_autotile_set()` — blob_47 or dual_grid
- [ ] `pixl.render_atlas()` + `pixl.get_file()`

Tasks — Infrastructure:
- [ ] Base64 PNG in every create/render response
- [ ] 16x zoom preview default
- [ ] Light source 4x4 reference quad injection
- [ ] Edge context pixels in `session_start()` and `create_tile()` responses
- [ ] SELF-REFINE tracking — count in responses, "accept" after 3
- [ ] `pixl mcp` CLI command
- [ ] Integration test: full tile → validate → WFC → atlas workflow
- [ ] MCP tool descriptions optimized for LLM comprehension

**Deliverable:** `pixl mcp` added to Claude Code config → dungeon tileset in one conversation.

### Phase 5 — Game Engine Export (Week 8–9)

**Goal:** Export to every major 2D game engine via two standard formats.

Tasks:
- [ ] TexturePacker JSON Hash with 9-slice `border` metadata (`texturepacker.rs`)
- [ ] Aseprite-compatible `frameTags` JSON for animation data
- [ ] Tiled TMJ tileset + tilemap with collision + z_order (`tiled.rs`)
- [ ] Godot .tres tileset with collision shapes (`godot.rs`)
- [ ] Unity tilemap JSON metadata (`unity.rs`)
- [ ] GBStudio 160x144 PNG grid layout (`gbstudio.rs`)
- [ ] Palette LUT texture export for shader-based runtime swaps (`palette_lut.rs`)
- [ ] `pixl export` CLI command with `--format` flag
- [ ] Test: import into Tiled, verify tileset + collision

**Deliverable:** `pixl export dungeon.pax --format tiled --out dungeon/`

### Phase 6 — Polish & Ship V1 (Week 9–10)

**Goal:** Production-ready V1 release.

Tasks:
- [ ] Error messages with file position, context, fix suggestion
- [ ] `--help` text with examples for every CLI command
- [ ] CI: `cargo test`, `cargo clippy`, `cargo fmt --check`
- [ ] Cross-compilation: macOS (arm64/x86_64), Linux (x86_64/arm64), Windows
- [ ] Release binaries via GitHub Actions
- [ ] README: quickstart, three-tier encoding table (front and center), MCP setup
- [ ] Example: complete dungeon tileset with palette swaps + WFC map + atlas
- [ ] Publish to crates.io + GitHub Releases

**Deliverable:** `cargo install pixl` works. Binary on GitHub.

---

## 8. Tech Stack & Dependencies

### 8.1 Rust Dependencies

```toml
# pixl-core
serde = { version = "1", features = ["derive"] }
toml = "0.8"
fnv = "1.0"              # FNV-1a for edge hashing (NOT DefaultHasher — that's SipHash)
thiserror = "2"

# pixl-render
image = { version = "0.25", default-features = false, features = ["png", "gif"] }
base64 = "0.22"

# pixl-wfc
rand = "0.9"
fixedbitset = "0.5"      # 0.5.7 confirmed on crates.io

# pixl-mcp
rmcp = { version = "1.2", features = ["server", "transport-io", "macros"] }
tokio = { version = "1", features = ["full"] }
serde_json = "1"

# pixl-cli
clap = { version = "4", features = ["derive"] }
```

### 8.2 Minimum Rust Version

MSRV: **1.75** (for `rmcp` compatibility and async trait stabilization).

### 8.3 WASM Target (Future)

`pixl-core` and `pixl-render` compile to `wasm32-unknown-unknown`.
Required: `image` with `default-features = false`.
Excluded: `pixl-mcp`, `pixl-cli` (not applicable to WASM).

---

## 9. Roadmap — What's Done, What's Next

### Shipped

| Feature | Status | CLI / API |
|---------|--------|-----------|
| V1.0 Core (6 phases) | DONE | Parser, renderer, WFC, MCP, export, CI |
| V1.1 Diffusion Import | DONE | `pixl import` with Lanczos + Bayer dither |
| V1.1 Examples | DONE | dungeon.pax, platformer.pax, gameboy.pax |
| V1.1 Theme Library | DONE | `pixl new <theme>` — 6 themes with stamps |
| V1.2 Style Latent | DONE | `pixl style` / `pixl_learn_style` / `pixl_check_style` |
| V1.2 Procedural Stamps | DONE | `pixl generate-stamps` — 8 patterns |
| V1.2 Project Sessions | DONE | `pixl project init/add-world/status/learn-style` |
| V1.3 HTTP API | DONE | `pixl serve` — 20 endpoints (axum) |
| V1.5 Narrate Pipeline | DONE | `pixl narrate` / `pixl_narrate_map` |
| Studio Integration | DONE | generate/context, pixl_backend.dart |
| MCP Tool Descriptions | DONE | 19 tools with LLM-optimized descriptions |
| WASM Playground | DONE | 8 wasm-bindgen exports, live PAX editor in browser |
| Dogfood Fixes | DONE | 10 bugs found + fixed from first real usage session |

### Remaining — Future

**Procedural Variation Engine:**
- Auto-generate N tile variants conditioned on style latent
- Crack placement, moss/erosion density, color jitter
- `pixl vary <tile> --count 4 --seed 42`

**Skeletal Animation (V2):**
- Body part sprites + RotSprite rotation + bone interpolation

**Fine-tuned PAX LoRA (V2+):**
- Train on GameTileNet corpus converted to .pax format

### V1.6 — GameTileNet Corpus Integration (~1–2 weeks)

**Conversion pipeline** (this is real work, not just "copy files"):
- PNG → indexed palette quantization (reduce to 16 colors per tile)
- Symbol assignment from quantized palette
- TOML `.pax` stamp generation with size, palette, grid
- GameTileNet affordance label → PAX semantic tag mapping
- Batch validation of all 2,142 converted stamps
- Tool: `pixl import-corpus <gameTileNet-dir/> --palette <name>`

**Integration:**
- Ship as built-in stamp library in `corpus/`
- Stamp browser MCP tool: `pixl.browse_stamps(affordance?, biome?, tags?)`
- LLM can search: "show me walkable forest floor stamps" → filtered results
- Adopt GameTileNet's 361 normalized tags as the canonical tag vocabulary

### V2.0 — PIXL Studio (Flutter Desktop)

- Desktop app in `studio/` talking to `pixl` Rust backend (via HTTP API or FFI)
- Live preview panel — edit .pax text, see rendered tiles instantly
- Tile palette browser — visual grid of all tiles with affordance labels
- WFC map preview — generate and preview maps interactively
- Stamp library browser with GameTileNet corpus, drag-and-drop composition
- Narrative-to-map UI: text prompt → live map generation
- Export wizard for game engines (TexturePacker + Tiled + GBStudio)
- Theme editor with live palette preview

### V2.x — Advanced Features

- **Skeletal animation system** — body part sprites + RotSprite rotation +
  bone interpolation. Author 6-8 body parts, define 3-4 skeletal keyframe
  poses, system generates complete spritesheets. 4-directional movement via
  mirror. RotSprite preserves pixel aesthetic (no new colors). See spec
  Section 19.1 for full format specification.
- Parallax layer definitions
- Sound effect tags on tiles (footstep type, ambient)
- Collaborative editing via CRDT-based .pax sync
- Fine-tuned small LLM (3B–7B) specifically trained on PAX format
- Public stamp/theme registry (npm-style for pixel art building blocks)
- Training data pipeline: convert OpenGameArt CC0 assets to .pax for LLM training

---

## 10. Success Metrics

### V1 Launch Criteria

| Metric | Target |
|--------|--------|
| LLM creates coherent 12-tile tileset in one conversation | < 20 tool calls |
| Tiles validate on first LLM attempt | ≥ 80% |
| WFC produces valid maps from LLM-authored tiles | ≥ 90% |
| Total Rust LOC (excluding tests) | < 8,000 |
| MCP tool response time (render) | < 100ms |
| MCP tool response time (WFC 20×20) | < 500ms |
| Binary size (release, stripped) | < 10MB |
| External runtime dependencies | Zero |
| Cross-platform | macOS, Linux, Windows |

### Quality Gates Per Phase

- Phase 1: `pixl validate` passes on all 3 example files, 100% parser test coverage
- Phase 2: Rendered PNGs match golden snapshots pixel-for-pixel
- Phase 3: WFC with fixed seed produces identical output across runs
- Phase 4: End-to-end MCP test completes full tile → validate → WFC → atlas workflow
- Phase 5: Exported files import cleanly in Tiled 1.11+
- Phase 6: `cargo clippy` zero warnings, all tests green on CI

---

## 11. Open Decisions — Resolution Status

All prior open decisions are resolved by the PAX 2.0 spec (`docs/specs/pax.md`):

| Decision | Resolution |
|----------|-----------|
| TOML delimiter | `'''...'''` confirmed |
| Edge class naming | Freeform strings + FNV-1a auto-generated fallback |
| Semantic rule language | `forbids` (hard, propagation) + `requires` (soft, collapse) |
| Stamp size | Fixed-size per compose row. `_` void block added |
| File scope | Single file for V1. `include` in V1.1 |
| Palette ordering | `HashMap<char, Rgba>` with custom deserializer |
| Tile rotation | `auto_rotate` field, tool generates variants at parse time |
| Variant groups | `[wfc_rules.variant_groups]` table |
| Compose grammar | `@stamp_ref` and `_` only — no inline pixels in V1 |
| Theme constraints | Declarative boolean checks, not expression language |
| Frame syntax | `[[...]]` array-of-tables with inline frame arrays |
| Animation tags | Aseprite-compatible `frameTags` (NOT TexturePacker `animationTags`) |
| Bitmask table | Generated in `build.rs`, validated against cr31.co.uk |
| Hashing | FNV-1a via `fnv` crate (NOT `DefaultHasher` which is SipHash) |

### Remaining open

**Go wrapper vs pure Rust.** The PAX 2.1 addendum proposes a Go wrapper around
Rust libpax via C FFI. Current implementation is pure Rust (rmcp for MCP, clap
for CLI). The Go wrapper adds MCP transport flexibility and CLI ergonomics but
introduces a two-language build. Decision deferred — pure Rust for V1, evaluate
Go wrapper for V1.3 (HTTP API) based on real friction.

---

*Plan is authoritative for phases, tasks, and timeline.
Format spec is authoritative at `docs/specs/pax.md`.
Update both as decisions are made and phases complete.*
