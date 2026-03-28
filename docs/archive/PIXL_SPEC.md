# PIXL — Pixel Intelligence eXchange Language
## A Groundbreaking LLM-Native Format for Pixel Art Game Assets

**Version 0.1 — Concept & Specification**
**Author: Synthesis from research 2024–2025**

---

## Executive Summary

PIXL is a human-readable, LLM-writable text format for pixel art that is *already its own source of truth* — no decompression, no decoding, no translation needed to understand what it contains. Paired with a Go/Rust CLI/API/MCP tool, it enables any sufficiently capable LLM to generate complete, coherent, tileable game art assets — from 8×8 icons to 64×64 detailed character sprites — all constrained by a global style sheet that guarantees visual coherence across an entire game.

This does not exist yet. What exists is:
- Binary formats (PNG, Aseprite) — not LLM-writable
- SVG — verbose, spatial reasoning hostile
- ASCII art tools — not renderable, no palette, no tile constraints
- Prompt-to-image AI — non-deterministic, not editable, not tileable, not coherent

PIXL fills this gap by being the first format specifically designed for **LLM authorship** of pixel art.

---

## Research Foundations

### 1. The LLM Spatial Reasoning Gap

Research (SkyPilot blog, 2024) confirmed that current multimodal LLMs fail at spatial structure in ASCII conversion tasks. The root cause: LLMs reason in **semantic space**, not pixel space. The fix is not better spatial training — it's **a format that maps to how LLMs already think**. Characters should carry semantic meaning. `S` should mean *stone*, not "this specific shade of gray."

### 2. Indexed Color + BPE = Natural LLM Token Space

Two papers converge here:
- **BPE Image Tokenization** (Zhang et al., 2024, ICLR 2025): Applying Byte-Pair Encoding to quantized visual tokens dramatically improves LLM image understanding. Frequent visual patterns merge into single tokens — exactly like words in text.
- **Multidimensional BPE** (Elsner et al., 2024): 2D BPE on image patches reduces sequence length by 30%+ and improves generation quality, because condensed sequences are easier to model.

**Implication for PIXL:** Instead of writing pixel-by-pixel, the LLM writes in *named sub-tile patterns* — exactly analogous to BPE tokens. Frequent 4×4 and 8×8 blocks get semantic names, and the LLM composes tiles from this vocabulary.

### 3. Wave Function Collapse — Constraint Engine

WFC (Gumin, 2016; extensively documented) is the ideal backend for PIXL's tileability guarantees:
- Input: Tile definitions + edge constraint declarations
- Output: Valid tilemaps where every edge matches
- Extension: Sub-complete tilesets (N-WFC, 2023) enable infinite, aperiodic generation in polynomial time
- Key insight from Boris the Brave's WFC tips: designing tiles by **corner/edge behavior** rather than content is the most powerful design principle — and this maps exactly to PIXL's edge declaration syntax

### 4. Grammar-Based Style Coherence

Procedural Content Generation literature (Survey, 2024) identifies generative grammars as the primary mechanism for cross-asset stylistic coherence. PIXL's `@theme` system is a declarative grammar that establishes:
- Color relationships (not just values)
- Pixel scale and density rules
- Semantic color roles (shadow, highlight, accent, danger, void)
- Theme inheritance and override

### 5. Existing LLM Pixel Art Experiments

The MCP/Aseprite experiment (ljvmiranda921, 2025) showed Claude Opus 4 can produce creative pixel art via tool-calling but that **per-pixel tools** are the wrong abstraction. Drawing pixels one at a time is how you get 300 tool calls for a 16×16 sprite. The insight: the tool's granularity should match how LLMs actually think — in shapes, regions, and patterns, not coordinates.

---

## The PIXL Format

### Architecture Overview

```
PIXL has five hierarchical layers:

  Layer 4: Map          ← assembled from tile groups
  Layer 3: Tile Group   ← named clusters (corner_NW, wall_cross, etc.)
  Layer 2: Tile         ← 8×8 to 64×64, composed from micro-tiles
  Layer 1: Micro-tile   ← named 4×4 reusable fragments
  Layer 0: Symbol       ← single character = single palette entry

LLMs primarily work at Layers 2-4.
The renderer works bottom-up: Symbol → Micro-tile → Tile → Group → Map.
```

### File Extension: `.pixl`

A `.pixl` file is UTF-8 plain text. It can contain one or more of: themes, palettes, micro-tiles, tiles, sprite sheets, maps.

---

### Layer 0: Symbols & Palette

Each symbol is a **single printable character** mapped to a semantic color role.

```pixl
@palette dungeon_dark {
  // Semantic symbol → color role → hex
  '.' = void       #00000000   // transparent
  '#' = stone      #2a1f3d
  '+' = lit_stone  #4a3a6d     // lighten(stone, 40%)
  '~' = water      #1a3a5c
  'g' = moss       #2d5a27
  'o' = gold       #c8a035
  'r' = blood      #8b1a1a
  's' = shadow     darken(stone, 30%)
  'h' = highlight  lighten(lit_stone, 20%)
  'W' = wood       #5a3a1a
  'D' = door       #7a5a2a
}
```

**Design principles:**
- Lowercase = dark/background tones
- Uppercase = structural/interactive elements
- `.` always = transparent/void
- Characters chosen for visual mnemonics where possible (`~` for water, `g` for grass, `o` for gold)
- A palette has at most 62 symbols (26 lower + 26 upper + 10 digits) — enough for any pixel art palette, tight enough that LLMs never need to invent new symbols

---

### Layer 0.5: The Theme System

Themes are the "CSS" of PIXL. They define palette + render parameters + semantic relationships.

```pixl
@theme dark_fantasy {
  @palette   dungeon_dark      // which palette to use
  @scale     2                 // pixel upscale factor
  @canvas    16                // default tile canvas size
  @dither    none              // dithering mode: none | bayer | ordered

  // Semantic color roles — the LLM thinks in these, not hex
  --void      = '.'
  --bg        = '#'            // primary background
  --fg        = '+'            // primary foreground/lit
  --accent    = 'o'            // highlight accent
  --danger    = 'r'
  --neutral   = 's'

  // Relationship rules (LLM can validate its own palette choices)
  @rule highlight_contrast: luminance(--fg) > luminance(--bg) + 30%
  @rule accent_distinct:    hue_distance(--accent, --bg) > 60deg
}

@theme dark_fantasy extends light_fantasy {
  // Theme inheritance — child overrides specific values
  @palette   dungeon_dark
}
```

---

### Layer 1: Micro-tiles (Named Pattern Fragments)

Micro-tiles are the BPE tokens of PIXL — frequently occurring 4×4 or 8×8 sub-patterns that get semantic names. They enable the LLM to compose tiles from vocabulary rather than drawing pixel-by-pixel.

```pixl
@micro wall_cap 4x4 {
  // A top-edge wall segment, used in many tiles
  ####
  ++++
  ++++
  ####
}

@micro corner_nw 4x4 {
  ####
  #+..
  #+..
  ##..
}

@micro crack_v 4x4 {
  .#..
  ##..
  .#..
  .##.
}
```

**LLM note:** Micro-tiles are a pre-defined vocabulary. The LLM doesn't need to invent them — the theme ships with a library. The LLM just selects and arranges them.

---

### Layer 2: Tiles

Tiles are the primary unit. They reference micro-tiles with `[name]` or write pixels directly.

```pixl
@tile wall_basic 16x16 {
  @theme      dark_fantasy
  @layer      collision
  @tiling     repeat             // WFC: this tile is repeat-tileable

  // Edge declarations (used by WFC engine)
  @edge N = [################]   // must match with tiles whose S edge = this
  @edge S = [################]
  @edge E = [################]
  @edge W = [################]

  // Tags
  @tags { walkable=false, blocks_light=true, variant_group=wall }

  // Grid: can use symbols, micro-tile references, or run-length encoding
  grid {
    ################
    #+++++++++++++##
    #+++++++++++++##
    ##++++++#++####
    ###++++####+++#   // note: LLM writes at symbol level for custom detail
    ###++######+++#
    ##+++#+++###++#
    ##++++++++++###
    ##+++++++++####
    ##+++++++######
    ###+++++++++###
    #+++++++++++++#
    #+++++++++++++#
    #+++++++++++++#
    #+++++++++++++#
    ################
  }
}
```

#### Tile Composition — The Power Feature

Tiles can be composed from micro-tiles using a **placement grid**:

```pixl
@tile wall_elaborate 16x16 {
  @compose {
    // 4×4 quadrants, each filled by a named micro-tile
    [corner_nw] [wall_cap  ] [wall_cap  ] [corner_ne]
    [wall_side ] [crack_v   ] [detail_rn ] [wall_side ]
    [wall_side ] [wall_base ] [wall_base ] [wall_side ]
    [corner_sw] [floor_edg ] [floor_edg ] [corner_se]
  }
}
```

This is how LLMs work most naturally — selecting from a vocabulary and arranging.

#### Run-Length Encoding in Grid Lines

```pixl
grid {
  16#          // 16 stone pixels — RLE syntax: count+symbol
  1#+12+1#+1#  // border-lit-border
  16#
}
```

#### Symmetry Declarations

```pixl
@tile gem 16x16 {
  @symmetry horizontal vertical   // write only top-left quadrant

  grid {
    ........
    ...oo...
    ..oooo..
    .oohoo..   // 'h' = highlight. Renderer mirrors the rest.
    ..oooo..
    ...oo...
    ........
    ........
  }
}
```

---

### Layer 3: Sprites & Animation

```pixl
@sprite hero_walk 16x32 {
  @theme dark_fantasy
  @frames 4
  @fps    8
  @loop   cycle

  frame 1 {
    // standing
    grid { ... }
  }

  frame 2 {
    // step right
    @diff from=1 {
      // only changed pixels vs frame 1
      // syntax: row:col=symbol
      20:7=+, 20:8=+, 22:6=#, 22:9=#
    }
  }

  frame 3 {
    @diff from=1 { ... }   // LLMs write diffs, not full frames
  }

  frame 4 {
    @diff from=2 { ... }   // reuse frame 2 diff, mirrored
    @mirror horizontal
  }
}
```

#### Skeleton Animation (for complex characters)

```pixl
@skeleton hero {
  @bones {
    body   = [8x8 at 0,0]
    head   = [8x8 at 4,-8] parent=body
    arm_l  = [4x8 at -4,2] parent=body
    arm_r  = [4x8 at 8,2]  parent=body
    leg_l  = [4x8 at 2,8]  parent=body
    leg_r  = [4x8 at 6,8]  parent=body
  }

  @animation walk {
    @fps 8
    @frames 4
    keyframe 0 { arm_l.rotate=10, arm_r.rotate=-10, leg_l.rotate=-15, leg_r.rotate=15 }
    keyframe 2 { arm_l.rotate=-10, arm_r.rotate=10, leg_l.rotate=15, leg_r.rotate=-15 }
  }
}
```

---

### Layer 3.5: Tile Groups & Autotiles

This is the WFC-ready layer. A tile group defines a complete set of topological variants.

```pixl
@tilegroup dungeon_wall {
  @theme dark_fantasy

  // 47-tile autotile set — complete Wang tile system
  // Each tile name encodes which neighbors are also wall
  // N=north, S=south, E=east, W=west (present = wall)

  tile NSEW { ... }   // surrounded on all sides
  tile NSE  { ... }   // wall on N, S, E — open W
  tile NSW  { ... }
  tile NEW  { ... }
  tile SEW  { ... }
  tile NS   { ... }   // vertical corridor
  tile EW   { ... }   // horizontal corridor
  tile N    { ... }   // end cap facing N
  tile S    { ... }
  tile E    { ... }
  tile W    { ... }
  tile none { ... }   // isolated wall pixel

  // Corner variants (concave)
  tile inner_NE { ... }
  tile inner_NW { ... }
  tile inner_SE { ... }
  tile inner_SW { ... }

  // The WFC adjacency rules are DERIVED automatically from the names above
  // No manual rule specification needed
  @wfc auto
}
```

---

### Layer 4: Maps

```pixl
@map dungeon_test 20x15 {
  @theme dark_fantasy
  @tilegroup dungeon_wall
  @tilegroup dungeon_floor

  // Explicit map (LLM-authored)
  layout {
    WWWWWWWWWWWWWWWWWWWW
    W..................W
    W.WWWWWW..WWWWWW..W
    W.W......W......W..W
    W.W..WW..W..WW..W..W
    W.W..W...W..W...W..W
    W.W..WWWWW..WWWWW..W
    W..................W
    W..WWWWW....WWWWW..W
    W..W...W....W...W..W
    W..W...WWWWWW...W..W
    W..W..........W....W
    W..WWWWWWWWWWWWW...W
    W..................W
    WWWWWWWWWWWWWWWWWWWW
  }

  // Symbol mapping for this map (different from tile-level symbols)
  where {
    W = dungeon_wall  // resolved to autotile variant automatically
    . = dungeon_floor
  }
}

@map dungeon_procedural 40x40 {
  @theme dark_fantasy
  @tilegroup dungeon_wall
  @tilegroup dungeon_floor
  @generator wfc {
    @seed 42
    @density wall=0.35
    @guarantee connected=true
    @rooms { min=5 max=12 size=5x5..12x12 }
  }
}
```

---

## The Toolchain: `pixl` (Go or Rust)

### CLI

```
pixl render <file.pixl>              → PNG output
pixl render --atlas <file.pixl>      → sprite atlas PNG + tilemap.json
pixl render --frames <sprite.pixl>   → individual frame PNGs or GIF
pixl validate <file.pixl>            → edge compatibility, palette check
pixl wfc <tilegroup.pixl> --size 20x15 --seed 42  → generate map
pixl pack <dir/>                     → pack all .pixl into atlas
pixl serve                           → HTTP API server
pixl mcp                             → MCP server (stdio)
pixl new theme <name>                → scaffold a new theme
pixl new tile <name> --theme <t>     → scaffold a new tile with palette
```

### HTTP API

```
POST /render        body: { pixl: "...", format: "png" }  → PNG bytes
POST /validate      body: { pixl: "..." }                 → errors[]
POST /wfc           body: { tilegroup: "...", size: [20,15], seed: 42 }
GET  /themes        → list built-in themes
GET  /microtiles    → list available micro-tiles for a theme
```

### MCP Tools (the LLM integration layer)

```
pixl.list_themes()
  → returns available themes and their palette symbols + semantic roles

pixl.get_palette(theme: "dark_fantasy")
  → returns symbol table + color roles + composition rules
  → LLM uses this to understand what characters are available before generating

pixl.validate_tile(pixl_source: "...")
  → returns errors: edge inconsistency, unknown symbols, size mismatch

pixl.check_tileable(tile_a: "...", tile_b: "...", direction: "N")
  → returns: compatible: bool, reason: string

pixl.render_tile(pixl_source: "...")
  → returns: png_base64: string (for visual verification)

pixl.render_map(map_source: "...", tilegroup_sources: ["..."])
  → returns: png_base64: string

pixl.generate_wfc_map(tilegroup: "...", width: 20, height: 15, seed: 42)
  → returns: map_pixl: string (a complete @map block the LLM can inspect)

pixl.suggest_edge_compatible_tiles(edge_N: "####", edge_W: "####", tilegroup: "...")
  → returns: compatible_tiles: string[]
  → LLM calls this to know what tile it can place next in a manual map
```

---

## The LLM Workflow

### Generating a complete dungeon tile set

```
1. LLM calls pixl.list_themes() → sees "dark_fantasy"
2. LLM calls pixl.get_palette("dark_fantasy") → gets symbol table
3. LLM generates @tilegroup dungeon_wall (16 topological variants)
4. For each tile, LLM calls pixl.validate_tile() → fix errors iteratively
5. LLM calls pixl.generate_wfc_map() → gets a test map
6. LLM calls pixl.render_map() → gets PNG to verify
7. Repeat for floor, door, chest, stairs, torch, etc.
8. LLM generates character sprites using same theme
9. pixl pack → single atlas.png + tilemap.json
```

**Total: A complete coherent dungeon art set in one conversation.**

### Why this is reliable

- LLM never touches hex codes — only semantic symbols
- LLM never guesses tileability — the tool validates it
- LLM never guesses what's available — the tool tells it
- Style coherence is guaranteed by the shared theme
- Errors are caught immediately via validate, not after rendering
- Frame diffs mean animation is tractable (LLM writes 20% of the pixels)
- Symmetry declarations cut the LLM's work in half or quarters

---

## Key Innovations vs. Prior Art

| Property | PNG/Aseprite | SVG | ASCII Art | PIXL |
|---|---|---|---|---|
| LLM-writable | ✗ | partial | partial | ✓ |
| Semantic symbols | ✗ | ✗ | ✗ | ✓ |
| Tileable by spec | ✗ | ✗ | ✗ | ✓ |
| Style inheritance | ✗ | partial | ✗ | ✓ |
| WFC compatible | ✗ | ✗ | ✗ | ✓ |
| Renderable directly | ✓ | ✓ | ✗ | ✓ |
| Diffable/versionable | ✗ | partial | ✓ | ✓ |
| Animation | ✓ | partial | ✗ | ✓ |
| MCP tool native | ✗ | ✗ | ✗ | ✓ |
| Game engine export | ✓ | partial | ✗ | ✓ |

---

## What Makes This Groundbreaking

### 1. First LLM-native visual authorship format
Not "describe what you want and AI generates it" — the LLM *is* the author, writing the source. This is the difference between prompting DALL-E and writing code. PIXL gives LLMs the equivalent of a programming language for pixel art.

### 2. Semantic compression that mirrors BPE
The micro-tile vocabulary system is a human-readable implementation of the same insight as BPE image tokenization: frequent visual patterns should be named and reused, not written from scratch each time. An LLM writing `[corner_nw][wall_cap][wall_cap][corner_ne]` is working at the natural token level for its architecture.

### 3. Validation loops enable autonomous iteration
The LLM can write, validate, fix, validate again — fully autonomously via MCP. This is how Claude Code works for software. PIXL gives the same loop for art.

### 4. WFC as constraint solver, not generator
Most WFC tools generate random content. PIXL uses WFC as a *validator and completer* — the LLM designs the tiles, WFC checks and fills gaps. The human (or LLM) retains authorial intent while getting formal tileability guarantees.

### 5. The format IS the game art pipeline
PIXL is source control for art. A game studio that adopts it gets:
- Git diffs on art assets
- Code review on sprites
- CI/CD for art (validate all tiles on every commit)
- LLM-assisted art generation that stays in-style
- Reproducible procedural map generation from seeds

---

## Implementation Roadmap

### Phase 1 — Core Renderer (2–3 weeks, Go)
- Parser for @palette, @theme, @tile with grid
- Symbol → pixel renderer → PNG export
- CLI: `pixl render`
- Basic RLE in grids

### Phase 2 — Validation + WFC (2–3 weeks)
- Edge declaration parser
- Tileability checker
- WFC map generator (simple tiled model)
- CLI: `pixl validate`, `pixl wfc`
- Autotile name-to-edge-rule derivation

### Phase 3 — Micro-tiles + Composition (1–2 weeks)
- Micro-tile registry
- @compose placement grid
- Symmetry declarations
- Theme library (dark_fantasy, light_fantasy, sci_fi, nature)

### Phase 4 — Animation (1–2 weeks)
- @frames, @fps, @loop
- @diff syntax
- GIF export
- @mirror for frame symmetry

### Phase 5 — MCP + API (1 week)
- All MCP tools defined above
- HTTP API
- Streaming validation responses

### Phase 6 — Atlas + Export (1 week)
- `pixl pack` → sprite atlas
- Tiled-compatible JSON tilemap export
- Unity/Godot import helper

**Total: ~10 weeks solo. ~5 weeks with parallel tracks.**

---

## Format Versioning & Interoperability

- `.pixl` source files are version-tagged: `@pixl 0.1`
- Canonical output is always PNG (universal)
- JSON sidecar for metadata: tile names, edge types, tags, frame timings
- Godot `.tres` and Tiled `.tsj` export planned for Phase 6
- PIXL themes are sharable as `.pixlt` files (theme-only subset)

---

## Open Questions / Design Decisions

1. **Character set**: UTF-8 allows box-drawing characters (╔╗╚╝═║) which are visually mnemonic for stone structures — but requires careful font handling. Consider ASCII-only mode.

2. **Palette size limit**: 62 symbols is generous for most pixel art (NES: 4 colors per tile, GB: 4 colors, SNES: 16 per tile). Should there be a `@strict` mode that enforces platform limits?

3. **Grid column alignment**: Should each row be padded to exact width? Tab-stop alignment would help human readability at the cost of a whitespace-sensitivity requirement.

4. **Micro-tile library curation**: Who owns the canonical micro-tile library? Should it be theme-scoped or global? A public registry (like npm for micro-tiles) is worth considering.

5. **LLM fine-tuning**: Training a small model (3B–7B) specifically on PIXL format would dramatically improve generation reliability. A PIXL dataset could be created by converting open-source pixel art (OpenGameArt.org) to PIXL format.

---

## Conclusion

PIXL is a convergence of four deep research streams — BPE tokenization theory, WFC constraint propagation, procedural grammar systems, and LLM spatial reasoning research — into a single coherent format designed from first principles for LLM authorship.

It is buildable in 10 weeks. It would be the first tool that lets an LLM generate a complete, coherent, tileable, game-ready pixel art asset set from a single prompt. The open-source release of the spec, reference renderer, and theme library would create immediate value for indie game developers and the AI tooling community.

**The gap is real. The technology is ready. The format just doesn't exist yet.**
