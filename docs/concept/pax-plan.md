# PAX — Pixel Art eXchange Format
## Complete Technical Specification & Go Implementation Plan

**Version:** 0.1-draft  
**Author:** Lisa / TasteHub GmbH  
**Date:** March 2026

---

## 1. Research Synthesis

### 1.1 The Core Problem: LLMs Cannot See Grids

Research from ASCIIBench (NeurIPS 2025 Workshop), the "Stuck in the Matrix" paper (Oct 2025), and extensive testing confirms a fundamental architectural limitation: **LLMs are inherently biased toward sequential processing and fail catastrophically at 2D spatial reasoning** when grid dimensions exceed ~12×12.

Key findings from the literature:

- **Tokenization destroys spatial adjacency.** Characters that are visually neighbors in a grid (e.g., column-adjacent) are separated by an entire row's worth of tokens. Self-attention cannot establish strong patterns between spatially related but sequentially distant tokens.
- **Performance degrades ~42-84% as grid complexity increases.** Simple 8×8 grids are manageable; 32×32 grids are unreliable; 64×64 grids are essentially impossible to produce correctly in a single pass.
- **Coordinate-based representations outperform grid-based ones.** Research on spatial reasoning in LLMs (Martorell 2025) shows that explicit (x,y) coordinate formats encoded in JSON yield significantly higher accuracy than ASCII/topographic layouts.
- **Symbolic reasoning works; pixel reasoning doesn't.** LLMs excel at understanding that "# means stone wall" and reasoning about where stone walls should go. They fail at counting exact pixel positions.

### 1.2 Existing Approaches & Their Limitations

| Approach | What It Does | Why It's Not Enough |
|----------|-------------|-------------------|
| **Aseprite MCP** (pixel-mcp, Go) | Exposes draw_pixel, draw_line, etc. via MCP | LLM must emit hundreds of individual pixel coordinates — slow, error-prone, no semantic understanding |
| **LLM4SVG** (CVPR 2025) | Fine-tuned LLM to generate/understand SVG | SVG is verbose, not grid-aligned, bad for pixel art specifically |
| **Diffusion models** (FLUX, SD + PixelArt LoRA) | Generate pixel art images from text | No structural control, can't guarantee tileability, palette coherence, or specific game-engine constraints |
| **WFC from sample image** | Constraint-solving tilemap generation | Requires pre-existing sample art — doesn't help with initial tile creation |
| **Raw ASCII grid output** | LLM writes characters in grid | Works up to ~12×12, fails beyond; no palette, no tiling, no validation |

### 1.3 The Insight: Work With LLM Strengths, Compensate for Weaknesses

The PAX format is designed around what LLMs **can** do well:

1. **Symbolic reasoning** — assign meaning to single characters (#=wall, .=void) and reason about placement semantics
2. **Small grid production** — reliably produce 8×8 or 16×16 character grids
3. **Hierarchical composition** — define small blocks, then compose them into larger structures via named references
4. **Rule following** — validate edge constraints when given explicit edge-color annotations
5. **Style adherence** — maintain a palette defined once, referenced everywhere

And compensates for what they **cannot** do:

1. **Large grid accuracy** → Macro-block composition (build 32×32 from four 16×16 quadrants, or 64×64 from sixteen 16×16)
2. **Pixel-perfect counting** → RLE (Run-Length Encoding) as an alternative representation the LLM can choose
3. **Cross-row spatial reasoning** → Edge annotations that reduce tiling validation to string comparison
4. **Color space math** → Palette defined once, symbols used everywhere

### 1.4 Key Algorithms from Literature

**Wave Function Collapse (WFC)** — Gumin 2016, extensively studied since. Core algorithm:
1. Initialize grid where each cell can be any tile (superposition)
2. Pick lowest-entropy cell (fewest remaining possibilities)
3. Collapse it to one tile (weighted random by frequency)
4. Propagate constraints to neighbors (arc consistency — remove impossible tiles)
5. Repeat until all cells collapsed or contradiction

PAX uses WFC for **tilemap assembly** — not tile creation. LLM creates tiles with edge constraints; WFC assembles them into coherent maps.

**Wang Tiles** — Hao Wang 1961. Square tiles with colored edges, placed so matching edges meet. For `k` edge colors: minimum viable set is `2k²` tiles (each N,W color combo needs ≥2 tiles for non-periodicity). PAX adopts the **corner-matching variant** (Wang 2-corner / blob tiles) using a 4-bit bitmask index (16 tiles) for terrain transitions.

**Autotiling Bitmask** — Assign each neighbor direction a bit weight (N=1, E=2, S=4, W=8 for 4-connected; add NE=16, SE=32, SW=64, NW=128 for 8-connected). Sum weights of same-type neighbors → unique index into tileset. The "blob" variant (47 unique tiles from 256 combinations) is the game-industry standard.

**Run-Length Encoding (RLE)** — For LLM-friendly sprite authoring of larger sprites. Instead of a 32×32 grid of characters, the LLM writes `8# 4+ 4# 16.` etc. Combined with palette indexing, this is highly compressible and LLM-writable.

---

## 2. The PAX Format Specification

### 2.1 File Structure

PAX files use TOML for metadata and a custom grid syntax for pixel data. A `.pax` file is UTF-8 text.

```
┌─────────────────────────────────┐
│  [pax]                          │  ← Format header + version
│  [palette.<name>]               │  ← Named color palettes
│  [stamp.<name>]                 │  ← Reusable macro-blocks (4×4, 8×8)
│  [tile.<name>]                  │  ← Tile definitions (8×8 to 32×32)
│  [sprite.<name>]                │  ← Multi-frame animated sprites
│  [tilemap.<name>]               │  ← Composed tilemaps
│  [atlas]                        │  ← Atlas export configuration
└─────────────────────────────────┘
```

### 2.2 Header

```toml
[pax]
version = "0.1"
name = "dungeon_tileset"
author = "claude"
created = "2026-03-22T12:00:00Z"
```

### 2.3 Palette Definition

```toml
[palette.dungeon]
# Format: symbol = "#RRGGBB" or symbol = "#RRGGBBAA"
# Single printable ASCII char, no whitespace
"." = "#00000000"   # transparent
"#" = "#2a1f3d"     # stone dark
"+" = "#4a3a6d"     # stone lit
"~" = "#1a3a5c"     # water
"g" = "#2d5a27"     # moss
"o" = "#c8a035"     # gold/torch glow
"r" = "#8b1a1a"     # blood/danger
"w" = "#e8e0d4"     # bone white

[palette.gameboy]
"." = "#0f380f"
"1" = "#306230"
"2" = "#8bac0f"
"3" = "#9bbc0f"
```

**Design rationale:** Symbols are single characters, chosen to be mnemonic (`#` looks like a wall, `~` looks like water). The LLM works in **meaning-space**, never needing to think about hex colors while drawing. Palette is defined once and inherited by all tiles/sprites in the file.

### 2.4 Stamps (Macro-blocks)

Stamps are small reusable patterns (2×2 to 8×8) that tiles can reference by name. This is the key mechanism for enabling LLMs to create sprites larger than ~16×16.

```toml
[stamp.brick_2x2]
palette = "dungeon"
size = "4x4"
grid = """
##++
#+++
++##
+##.
"""

[stamp.eye_3x3]
palette = "dungeon"
size = "3x3"
grid = """
.w.
wro
.w.
"""
```

### 2.5 Tile Definition — Grid Mode

For tiles up to 16×16 that the LLM can reliably produce as raw grids:

```toml
[tile.wall_basic]
palette = "dungeon"
size = "16x16"
# Edge colors for Wang-tile matching (N E S W)
# Each edge is a string of symbols from the grid's border row/col
edges = { n = "################", e = "################", s = "################", w = "################" }
# Edge-class (simplified): reduce full edge to a short hash for WFC
edge_class = { n = "solid", e = "solid", s = "solid", w = "solid" }
tags = ["wall", "interior"]
weight = 1.0  # frequency hint for WFC
grid = """
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
"""
```

### 2.6 Tile Definition — RLE Mode

For sprites 32×32 and above, the LLM produces run-length encoded rows:

```toml
[tile.big_wall]
palette = "dungeon"
size = "32x32"
encoding = "rle"
edge_class = { n = "solid", e = "solid", s = "solid", w = "solid" }
# RLE format: <count><symbol> pairs, row-by-row
# "16# 16+" means 16 '#' followed by 16 '+'
rle = """
32#
2# 12+ 6# 12+
2# 12+ 6# 12+
32#
8# 16+ 8#
8# 16+ 8#
32#
"""
# Omitted rows repeat the previous row
```

### 2.7 Tile Definition — Stamp Composition Mode

For the largest sprites, compose from named stamps:

```toml
[tile.castle_gate]
palette = "dungeon"
size = "32x32"
encoding = "compose"
edge_class = { n = "solid", e = "solid", s = "open", w = "solid" }
# Grid of stamp references, each stamp fills its natural size
# '@' prefix = stamp reference, bare char = single-pixel fill
layout = """
@brick_2x2 @brick_2x2 @brick_2x2 @brick_2x2
@brick_2x2 @brick_2x2 @brick_2x2 @brick_2x2
@brick_2x2 @arch_top   @arch_top   @brick_2x2
@brick_2x2 @arch_left  @arch_right @brick_2x2
@brick_2x2 ....        ....        @brick_2x2
@brick_2x2 ....        ....        @brick_2x2
@brick_2x2 @brick_2x2 @brick_2x2 @brick_2x2
@brick_2x2 @brick_2x2 @brick_2x2 @brick_2x2
"""
```

### 2.8 Sprite (Animation)

```toml
[sprite.hero_idle]
palette = "dungeon"
size = "16x16"
frames = 2
fps = 4
loop = true

[sprite.hero_idle.frame.1]
grid = """
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
"""

[sprite.hero_idle.frame.2]
# Delta mode: only specify changed pixels
encoding = "delta"
changes = [
    { x = 4, y = 8, sym = "+" },
    { x = 5, y = 8, sym = "w" },
    { x = 6, y = 9, sym = "+" },
]
```

### 2.9 Tilemap

```toml
[tilemap.dungeon_room_1]
size = "8x6"          # in tiles (not pixels)
tile_size = "16x16"   # pixel size of each tile
layers = ["floor", "walls", "objects"]

[tilemap.dungeon_room_1.layer.floor]
grid = """
floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone
floor_stone floor_stone floor_stone floor_water floor_water floor_stone floor_stone floor_stone
floor_stone floor_stone floor_stone floor_water floor_water floor_stone floor_stone floor_stone
floor_stone floor_stone floor_stone floor_water floor_water floor_stone floor_stone floor_stone
floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone
floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone floor_stone
"""
```

### 2.10 Atlas Export Config

```toml
[atlas]
format = "png"
padding = 1           # px between tiles in atlas
columns = 8           # tiles per row in atlas sheet
include = ["wall_*", "floor_*"]  # glob patterns
output = "dungeon_atlas.png"
map_output = "dungeon_atlas.json"  # Tiled/Godot-compatible metadata
```

---

## 3. Algorithm Specifications

### 3.1 RLE Parser

```
Input:  "8# 4+ 12# 8."
Output: ['#','#','#','#','#','#','#','#','+','+','+','+','#','#',...]

Algorithm:
  for each token (split by space):
    parse leading digits → count (default 1 if absent)
    remaining char → symbol
    emit symbol × count
  
  validate: sum of all counts == row width
```

Time complexity: O(n) where n = decompressed row width.

### 3.2 Edge Extraction & Classification

```
Given a tile grid[h][w]:
  edge_n = grid[0][0..w]         # first row
  edge_s = grid[h-1][0..w]       # last row  
  edge_w = grid[0..h][0]         # first column
  edge_e = grid[0..h][w-1]       # last column

Edge class (simplified hash):
  if all symbols identical → "solid_<sym>"
  if first_half == second_half (symmetric) → "sym_<hash4>"
  else → "mixed_<hash8>"
  
  where hash = first 4/8 chars of SHA-256 of edge string
```

Two tiles can be placed adjacent if their touching edge classes match. Full edge string comparison is the **strict** mode; edge class matching is the **relaxed** mode for WFC.

### 3.3 Wave Function Collapse (Tilemap Assembly)

This is the core algorithm for assembling tilemaps from PAX tiles. Implementation based on Gumin's original WFC with the "simple tiled model" variant.

```
Data structures:
  Cell: set of possible tile IDs (initially all tiles)
  Grid: 2D array of Cells
  AdjacencyRules: map[(tile_id, direction)] → set[compatible_tile_ids]
  FrequencyHints: map[tile_id] → float (from tile weight field)
  PropagationStack: stack of (x, y) coordinates to propagate

Algorithm WFC(width, height, tiles, rules):
  1. INITIALIZE
     for each cell (x,y):
       cell.possibilities = set(all tile IDs)
     
  2. MAIN LOOP
     while any cell has |possibilities| > 1:
       
       a. OBSERVE (pick lowest entropy cell)
          entropy(cell) = -Σ p_i * log(p_i)
            where p_i = freq[tile_i] / Σ freq[possible tiles]
          chosen = argmin(entropy) over uncollapsed cells
          // tie-break: random
       
       b. COLLAPSE
          pick tile from chosen.possibilities 
            weighted by FrequencyHints
          chosen.possibilities = {picked_tile}
          push chosen onto PropagationStack
       
       c. PROPAGATE (arc consistency / AC-3)
          while stack not empty:
            (cx, cy) = stack.pop()
            for each direction d in {N, E, S, W}:
              (nx, ny) = neighbor(cx, cy, d)
              if out of bounds: continue
              
              // Compatible tiles for neighbor given current cell
              allowed = union of rules[(t, d)] for t in cell(cx,cy).possibilities
              
              // Remove incompatible from neighbor
              before = |cell(nx,ny).possibilities|
              cell(nx,ny).possibilities &= allowed
              after = |cell(nx,ny).possibilities|
              
              if after == 0: 
                return CONTRADICTION  // backtrack or restart
              if after < before:
                push (nx, ny) onto stack  // entropy changed, propagate further
  
  3. RESULT
     return grid where each cell has exactly one tile

Adjacency rules construction:
  for each pair of tiles (A, B):
    if A.edge_class.e == B.edge_class.w:
      rules[(A, EAST)] += {B}
      rules[(B, WEST)] += {A}
    if A.edge_class.s == B.edge_class.n:
      rules[(A, SOUTH)] += {B}
      rules[(B, NORTH)] += {A}
```

**Backtracking extension** (optional, for complex tilesets):

```
When CONTRADICTION detected:
  1. Save snapshots at each collapse step
  2. Pop last snapshot
  3. Remove the tile choice that led to contradiction
  4. If cell now empty → pop another snapshot (deeper backtrack)
  5. Re-collapse and re-propagate
  Max backtracks before full restart: 100
  Max restarts: 10
```

### 3.4 Stamp Composition Resolver

```
Algorithm resolve_compose(layout, stamps, tile_size):
  # Parse layout into grid of references
  # Each cell is either a stamp name or a literal pixel block
  
  pixel_grid = new Grid(tile_size.w, tile_size.h)
  
  cursor_y = 0
  for each row in layout:
    cursor_x = 0
    max_height = 0
    for each token in row:
      if token starts with '@':
        stamp = stamps[token[1:]]
        blit(pixel_grid, cursor_x, cursor_y, stamp.grid)
        cursor_x += stamp.width
        max_height = max(max_height, stamp.height)
      else:
        # Literal inline pixels
        for each char in token:
          pixel_grid[cursor_y][cursor_x] = char
          cursor_x += 1
        max_height = max(max_height, 1)
    cursor_y += max_height
  
  return pixel_grid
```

### 3.5 Delta Frame Resolver (Animation)

```
Algorithm resolve_delta(base_frame, delta):
  result = deep_copy(base_frame)
  for each change in delta.changes:
    result[change.y][change.x] = change.sym
  return result
```

### 3.6 Atlas Packing (Bin-Packing)

Simple grid packing since all tiles are same-size in a PAX file:

```
Algorithm pack_atlas(tiles, columns, padding):
  tile_w = tiles[0].width + padding
  tile_h = tiles[0].height + padding
  rows = ceil(len(tiles) / columns)
  
  atlas_w = columns * tile_w + padding
  atlas_h = rows * tile_h + padding
  atlas = new Image(atlas_w, atlas_h, transparent)
  
  metadata = {}
  for i, tile in enumerate(tiles):
    col = i % columns
    row = i / columns
    x = padding + col * tile_w
    y = padding + row * tile_h
    blit(atlas, x, y, render(tile))
    metadata[tile.name] = { x, y, w: tile.width, h: tile.height }
  
  return atlas, metadata
```

### 3.7 Edge Validation

```
Algorithm validate_tileset(tiles):
  errors = []
  
  for each tile:
    # 1. Grid dimensions match declared size
    if grid_rows != size.h or any row_len != size.w:
      errors += "Size mismatch in tile {name}"
    
    # 2. All symbols exist in palette
    for sym in grid:
      if sym not in palette:
        errors += "Unknown symbol '{sym}' in tile {name}"
    
    # 3. Edge extraction matches declared edges
    extracted = extract_edges(grid)
    if tile.edges and extracted != tile.edges:
      errors += "Edge mismatch in tile {name}"
    
    # 4. Edge class compatibility (can tiles actually tile?)
    for direction in [N, E, S, W]:
      opposite = opposite_dir(direction)
      compatible = [t for t in tiles 
                    if t.edge_class[opposite] == tile.edge_class[direction]]
      if len(compatible) == 0:
        errors += "Tile {name} edge {direction} has no compatible neighbors"
  
  # 5. Symmetry check for tileable tiles
  for tile where tile.tags contains "tileable":
    if tile.edge_class.n != tile.edge_class.s:
      warnings += "Tile {name} not vertically tileable"
    if tile.edge_class.e != tile.edge_class.w:
      warnings += "Tile {name} not horizontally tileable"
  
  return errors, warnings
```

### 3.8 Autotile Bitmask Index Computation

For generating autotile-compatible tilesets (47-tile blob tileset):

```
Algorithm compute_bitmask(tilemap, x, y):
  # 8-neighbor bitmask (Wang blob)
  # Bit weights:  NW=1 N=2 NE=4 W=8 E=16 SW=32 S=64 SE=128
  
  this_type = tilemap[y][x].type
  mask = 0
  
  neighbors = [(-1,-1,1), (0,-1,2), (1,-1,4),
               (-1,0,8),            (1,0,16),
               (-1,1,32), (0,1,64), (1,1,128)]
  
  for (dx, dy, bit) in neighbors:
    nx, ny = x+dx, y+dy
    if in_bounds(nx, ny) and tilemap[ny][nx].type == this_type:
      mask |= bit
  
  # Corner cleanup: corner only counts if both adjacent edges present
  if (mask & 1) and not (mask & 2 and mask & 8):   mask &= ~1   # NW
  if (mask & 4) and not (mask & 2 and mask & 16):  mask &= ~4   # NE
  if (mask & 32) and not (mask & 8 and mask & 64):  mask &= ~32  # SW
  if (mask & 128) and not (mask & 16 and mask & 64): mask &= ~128 # SE
  
  return BITMASK_TO_47_INDEX[mask]  # lookup table: 256 → 47 unique tiles
```

---

## 4. Go Implementation Plan

### 4.1 Project Structure

```
pax/
├── cmd/
│   └── pax/
│       └── main.go              # CLI entry point
├── pkg/
│   ├── format/
│   │   ├── parser.go            # TOML + grid parser
│   │   ├── parser_test.go
│   │   ├── rle.go               # RLE encoder/decoder
│   │   ├── rle_test.go
│   │   ├── types.go             # Core data types
│   │   └── validate.go          # Format validation
│   ├── render/
│   │   ├── renderer.go          # Pixel grid → image.RGBA
│   │   ├── renderer_test.go
│   │   ├── atlas.go             # Atlas packer
│   │   ├── atlas_test.go
│   │   ├── gif.go               # Animated GIF export
│   │   └── spritesheet.go       # Spritesheet export
│   ├── compose/
│   │   ├── stamp.go             # Stamp resolver
│   │   ├── delta.go             # Delta frame resolver
│   │   └── compose_test.go
│   ├── tiling/
│   │   ├── edges.go             # Edge extraction + classification
│   │   ├── validate.go          # Tileability validation
│   │   ├── wfc.go               # Wave Function Collapse
│   │   ├── wfc_test.go
│   │   ├── autotile.go          # Bitmask autotile computation
│   │   └── autotile_test.go
│   ├── mcp/
│   │   ├── server.go            # MCP JSON-RPC server (stdio)
│   │   ├── tools.go             # Tool definitions
│   │   └── handlers.go          # Request handlers
│   └── export/
│       ├── tiled.go             # Tiled JSON export
│       ├── godot.go             # Godot .tres export
│       └── unity.go             # Unity tilemap metadata
├── schemas/
│   └── pax-v0.1.schema.json    # JSON Schema for validation
├── examples/
│   ├── dungeon.pax
│   ├── platformer.pax
│   └── gameboy.pax
├── go.mod
├── go.sum
├── Makefile
└── README.md
```

### 4.2 Core Types (`pkg/format/types.go`)

```go
package format

import "image/color"

type PAXFile struct {
    Header   Header
    Palettes map[string]*Palette
    Stamps   map[string]*Stamp
    Tiles    map[string]*Tile
    Sprites  map[string]*Sprite
    Tilemaps map[string]*Tilemap
    Atlas    *AtlasConfig
}

type Header struct {
    Version string
    Name    string
    Author  string
    Created string
}

type Palette struct {
    Name    string
    Symbols map[rune]color.RGBA
}

type Stamp struct {
    Name    string
    Palette string
    Width   int
    Height  int
    Grid    [][]rune // resolved pixel grid
}

type Encoding int
const (
    EncodingGrid    Encoding = iota
    EncodingRLE
    EncodingCompose
)

type EdgeClass struct {
    N string
    E string
    S string
    W string
}

type Tile struct {
    Name      string
    Palette   string
    Width     int
    Height    int
    Encoding  Encoding
    EdgeClass EdgeClass
    Tags      []string
    Weight    float64  // WFC frequency hint, default 1.0
    Grid      [][]rune // resolved pixel grid (always available after parse)
    
    // Raw source (for round-tripping)
    RawGrid    string
    RawRLE     string
    RawCompose string
}

type Sprite struct {
    Name    string
    Palette string
    Width   int
    Height  int
    FPS     int
    Loop    bool
    Frames  []*Frame
}

type Frame struct {
    Index    int
    Encoding Encoding // grid or delta
    Grid     [][]rune
    Changes  []DeltaChange // for delta encoding
}

type DeltaChange struct {
    X   int
    Y   int
    Sym rune
}

type Tilemap struct {
    Name     string
    Width    int    // in tiles
    Height   int    // in tiles
    TileW    int    // pixel width of each tile
    TileH    int    // pixel height of each tile
    Layers   []TilemapLayer
}

type TilemapLayer struct {
    Name string
    Grid [][]string // tile names
}

type AtlasConfig struct {
    Format    string // "png"
    Padding   int
    Columns   int
    Include   []string // glob patterns
    Output    string
    MapOutput string
}
```

### 4.3 CLI Commands (`cmd/pax/main.go`)

```
pax render <file.pax> [--tile <name>] [--sprite <name>] --out <output.png>
    Render a single tile or sprite to PNG.
    If sprite, renders first frame (or all frames with --all-frames).

pax atlas <file.pax> --out <atlas.png> [--map <atlas.json>]
    Pack all tiles into a sprite atlas PNG + optional JSON metadata.

pax validate <file.pax> [--check-edges] [--strict]
    Validate format, palette references, grid dimensions.
    --check-edges: verify all tiles have compatible neighbors.
    --strict: full edge string comparison (not just edge classes).

pax wfc <file.pax> --width <W> --height <H> --out <tilemap.png>
    Generate a WxH tilemap using WFC from the tiles in the file.
    Outputs rendered PNG and optional JSON tilemap data.

pax autotile <file.pax> --terrain <type> --out <autotile_atlas.png>
    Generate a 47-tile blob autotile set from a PAX terrain definition.

pax preview <file.pax>
    Render a quick 4x4 tilemap preview of all tiles side by side.

pax gif <file.pax> --sprite <name> --out <sprite.gif> [--scale <N>]
    Export animated sprite as GIF with optional upscaling.

pax export <file.pax> --format <tiled|godot|unity> --out <output>
    Export tilemap data in game-engine-specific formats.

pax mcp [--port <port>]
    Start as MCP server (stdio or SSE transport).
    Exposes tools for LLM-driven tile creation and validation.
```

### 4.4 MCP Tool Definitions (`pkg/mcp/tools.go`)

```
pax.create_palette(name, symbols)
    → Create a new palette. Returns palette summary.

pax.create_tile(name, palette, size, grid_or_rle, edge_class?, tags?)
    → Create a tile from grid or RLE data. Auto-extracts edges.
    → Returns: tile summary + rendered base64 PNG preview.

pax.create_stamp(name, palette, size, grid)
    → Create a reusable macro-block.

pax.compose_tile(name, palette, size, layout)
    → Create a tile via stamp composition.

pax.create_sprite(name, palette, size, frames, fps)
    → Create an animated sprite. First frame = full grid, rest = delta.

pax.validate(check_edges?)
    → Validate entire file. Returns errors + warnings.

pax.render_tile(name, scale?)
    → Render tile to PNG, return base64 image.

pax.render_atlas(include?, columns?, padding?)
    → Pack tiles into atlas, return base64 PNG + JSON metadata.

pax.generate_tilemap(width, height, seed?, constraints?)
    → Run WFC to generate a tilemap. Return rendered PNG + tile grid.

pax.suggest_edge_compatible(tile_name, direction)
    → Given a tile and direction, list all compatible neighbor tiles.

pax.get_file()
    → Return the full PAX file source (for inspection/iteration).

pax.list_tiles()
    → List all tiles with their edge classes and tags.
```

### 4.5 Key Dependencies

```go
// go.mod
module github.com/tastehub/pax

go 1.22

require (
    github.com/BurntSushi/toml v1.3.2      // TOML parser
    github.com/mark3labs/mcp-go v0.20.0     // MCP server SDK
)

// Standard library only for rendering:
//   image, image/color, image/png, image/gif
//   encoding/json
//   crypto/sha256 (edge hashing)
//   math/rand (WFC)
//   container/heap (WFC entropy priority queue)
```

### 4.6 Implementation Phases

**Phase 1 — Core Format (Week 1)**
- [ ] TOML parser for PAX files
- [ ] Grid parser (raw grid + RLE decoder)
- [ ] Palette resolver (symbol → color.RGBA)
- [ ] Type system + validation
- [ ] `pax validate` command
- [ ] Unit tests for parser + RLE

**Phase 2 — Rendering (Week 1-2)**
- [ ] Tile → image.RGBA renderer
- [ ] PNG export (with configurable scale factor)
- [ ] Atlas packer (grid layout)
- [ ] JSON metadata export (Tiled-compatible)
- [ ] `pax render` + `pax atlas` commands
- [ ] Animated GIF export for sprites

**Phase 3 — Composition (Week 2)**
- [ ] Stamp resolver
- [ ] Compose-mode tile builder
- [ ] Delta frame resolver for animations
- [ ] `pax preview` command

**Phase 4 — Tiling Intelligence (Week 2-3)**
- [ ] Edge extraction from grids
- [ ] Edge classification (solid/sym/mixed hashing)
- [ ] Adjacency rule builder
- [ ] WFC implementation with priority queue
- [ ] Backtracking support
- [ ] `pax wfc` command
- [ ] Autotile bitmask computation
- [ ] `pax autotile` command

**Phase 5 — MCP Server (Week 3)**
- [ ] MCP stdio transport setup
- [ ] Tool registration
- [ ] State management (in-memory PAX file)
- [ ] Base64 PNG preview in tool responses
- [ ] `pax mcp` command
- [ ] Integration test: Claude → MCP → render loop

**Phase 6 — Game Engine Export (Week 3-4)**
- [ ] Tiled JSON export (.tmj)
- [ ] Godot .tres tileset export
- [ ] Unity tilemap JSON
- [ ] `pax export` command

---

## 5. Design Decisions & Rationale

### 5.1 Why TOML, not YAML or JSON?

- TOML has **multi-line literal strings** (`"""..."""`) — perfect for embedding grids without escape sequences
- TOML is more readable than JSON for human editing
- TOML has a well-defined Go parser (BurntSushi/toml) that's battle-tested
- YAML's whitespace sensitivity would conflict with grid data alignment

### 5.2 Why single-character symbols?

- LLMs produce them reliably (one token per symbol)
- Grid alignment is visually obvious (monospaced text)
- Maximum palette size = ~94 printable ASCII characters (well beyond any practical pixel art palette)
- The constraint prevents LLMs from accidentally creating multi-char sequences that break grid alignment

### 5.3 Why three encoding modes?

The research shows clear thresholds in LLM spatial accuracy:

| Grid Size | LLM Accuracy | Best Encoding |
|-----------|-------------|---------------|
| ≤ 8×8     | High (~90%+) | Raw grid |
| 9-16      | Moderate (70-85%) | Raw grid with careful prompting |
| 17-32     | Low (40-60%) | RLE (sequential, not spatial) |
| 33-64     | Very low (<30%) | Stamp composition (no spatial reasoning needed) |
| > 64      | Impossible | Multi-tile composition via tilemap |

RLE shifts the task from spatial to sequential — "8 wall pixels then 4 light pixels" is natural language, not grid reasoning. Composition shifts it to naming — "put the brick pattern in the top-left" requires zero pixel-level reasoning.

### 5.4 Why edge classes instead of full edge comparison?

Full edge comparison (16 or 32 characters must match exactly) is too restrictive for LLM-authored tiles — a single misplaced pixel on an edge breaks compatibility. Edge classes allow "fuzzy" matching:

- `"solid"` — all same symbol (walls, floors)
- `"sym_<hash>"` — symmetrically patterned edge (decorative borders)
- `"open"` — all transparent (open areas, doorways)
- Custom class names — `"grass_top"`, `"water_edge"` — semantic grouping

The LLM annotates these explicitly, and the renderer validates them against the actual grid.

### 5.5 Why WFC over simpler tiling?

Simple left-to-right, top-to-bottom placement (stochastic Wang tiling) works for textures but produces incoherent maps for game levels. WFC's constraint propagation ensures:

- No isolated unreachable areas (with path constraints)
- Biome coherence (via tile weights and tag grouping)
- Contradiction detection (knows when a tileset can't produce a valid map)

The stochastic approach is available as a fast fallback (`--fast` flag) for when WFC is overkill.

---

## 6. MCP Workflow: End-to-End Example

Here's what a typical LLM ↔ PAX MCP session looks like:

```
User: "Create a dark dungeon tileset with stone walls, mossy floors, 
       and a water feature. I need it Godot-compatible."

LLM → pax.create_palette("dungeon", {
  ".": "#00000000", "#": "#2a1f3d", "+": "#4a3a6d",
  "~": "#1a3a5c", "g": "#2d5a27", "o": "#c8a035"
})

LLM → pax.create_tile("wall_solid", "dungeon", "16x16", 
  grid="################\n##++##++##...", 
  edge_class={n:"solid", e:"solid", s:"solid", w:"solid"},
  tags=["wall"])
  ← Returns: {ok: true, preview: "data:image/png;base64,...", 
              edges_valid: true}

LLM → pax.create_tile("floor_moss", "dungeon", "16x16",
  grid="gggg+g+ggg+g+ggg\n...",
  edge_class={n:"floor", e:"floor", s:"floor", w:"floor"},
  tags=["floor"])

LLM → pax.create_tile("wall_floor_n", "dungeon", "16x16",
  grid="################\n##++##++##++####\n...\ngggg+g+ggg+g+ggg",
  edge_class={n:"solid", e:"mixed_a3f2", s:"floor", w:"mixed_b1c4"},
  tags=["transition"])

... (creates ~12 tiles total)

LLM → pax.validate(check_edges=true)
  ← {errors: [], warnings: ["tile 'wall_corner_nw' edge W has only 1 compatible neighbor"]}

LLM → pax.generate_tilemap(8, 6, constraints={border: "wall_solid"})
  ← {preview: "data:image/png;base64,...", tilemap: [...]}

LLM → pax.render_atlas(columns=4, padding=1)
  ← {atlas: "data:image/png;base64,...", metadata: {...}}

LLM → pax.export("godot")
  ← {tres_file: "...", atlas_path: "dungeon_atlas.png"}
```

The LLM sees rendered previews at every step, can catch errors via validation, and iterates until the tileset is coherent — all within a single conversation.

---

## 7. Novel Contributions

1. **Semantic grid encoding** — first format designed specifically for LLM authoring, where symbols carry meaning not just color
2. **Three-tier encoding** (grid → RLE → compose) — scales across the full range of LLM spatial capability
3. **Edge class system** — fuzzy Wang tile matching that tolerates LLM imprecision while maintaining visual coherence
4. **Integrated WFC** — tile creation and map assembly in one tool, with the LLM in the authoring loop
5. **MCP-native design** — format + renderer + validator as a single MCP server, enabling closed-loop LLM workflows

---

## 8. Success Metrics

- An LLM can create a coherent 12-tile dungeon tileset in a single conversation (<20 tool calls)
- All tiles validate without errors on first try ≥80% of the time
- WFC produces playable tilemaps from LLM-authored tiles without contradiction ≥90% of the time
- Total Go codebase: <3000 LOC (excluding tests)
- MCP response time: <100ms for render, <500ms for WFC on 20×20 tilemap
- Zero external runtime dependencies (no Python, no Node, just the Go binary)
