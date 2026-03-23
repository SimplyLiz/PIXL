# PIXL

**LLM-native pixel art toolchain.** Parse, validate, render, and generate
pixel art tilesets from `.pax` files — a TOML-based format designed for how
LLMs actually think.

## The Idea

LLMs fail at raw pixel reasoning above 12x12. They excel at symbolic
composition. PIXL bridges this gap with three-tier encoding:

| Grid Size | LLM Accuracy | Encoding | What LLM Writes |
|-----------|-------------|----------|-----------------|
| <= 16x16 | High (85-95%) | grid | Raw character grid |
| 17-32 | Moderate | rle | Run-length rows |
| 33-64 | Low (<40%) | compose | Named stamp placement |

With symmetry declarations, a 32x32 tile becomes a 16x16 grid (quad symmetry).
The LLM works within its reliable accuracy zone. The tool does the rest.

## Quick Start

```bash
# Start a new project from a built-in theme
pixl new dark_fantasy --out my_tileset.pax

# Validate a .pax file
pixl validate my_tileset.pax

# Render a tile (supports grid, RLE, compose, template, symmetry)
pixl render examples/dungeon.pax --tile wall_solid --scale 4 --out wall.png

# 16x zoom preview for SELF-REFINE visual inspection
pixl preview examples/dungeon.pax --tile floor_moss --out preview.png --grid

# Pack tiles into a sprite atlas
pixl atlas examples/dungeon.pax --out atlas.png --map atlas.json --scale 2

# Export to game engine format (Tiled, TexturePacker, Godot)
pixl export examples/dungeon.pax --format tiled --out dungeon/

# Auto-fix missing edge classes from grid content
pixl check examples/dungeon.pax --fix

# Import a reference image into PAX palette
pixl import reference.png --size 16x16 --pax dungeon.pax --palette dungeon

# Extract style fingerprint from reference tiles
pixl style examples/dungeon.pax

# Generate procedural stamps
pixl generate-stamps brick_bond --size 4

# Generate a map from spatial predicates
pixl narrate examples/dungeon.pax --width 12 --height 8 \
  -r "region:entrance:floor_moss:2x2:northwest" \
  -r "region:chamber:floor_stone:3x3:southeast" \
  --out dungeon_map.png

# Show anatomy blueprint for character sprites
pixl blueprint 32x48

# Manage a multi-world game project
pixl project init my_game --theme dark_fantasy
pixl project add-world my_game.pixlproject dungeon worlds/dungeon.pax
pixl project learn-style my_game.pixlproject dungeon
pixl project status my_game.pixlproject

# Start MCP server for Claude Code
pixl mcp --file examples/dungeon.pax

# Start HTTP API for PIXL Studio
pixl serve --port 3742 --file examples/dungeon.pax
```

## The PAX Format

A `.pax` file is TOML. It contains everything: themes, palettes, palette swaps,
color cycling, stamps, tiles, sprites, WFC rules, and atlas configuration.

```toml
[pax]
version = "2.0"
name    = "dungeon_tileset"
theme   = "dark_fantasy"

[theme.dark_fantasy]
palette          = "dungeon"
scale            = 2
max_palette_size = 16
light_source     = "top-left"

[palette.dungeon]
"." = "#00000000"
"#" = "#2a1f3d"
"+" = "#4a3a6d"
"~" = "#1a3a5c"

[tile.wall_solid]
palette    = "dungeon"
size       = "16x16"
edge_class = { n = "solid", e = "solid", s = "solid", w = "solid" }
semantic   = { affordance = "obstacle", collision = "full" }
grid = '''
################
##++##++##++####
#+++++++++++++##
##++########++##
################
...
'''
```

Full format specification: [docs/specs/pax.md](docs/specs/pax.md)

## MCP Integration (Claude Code)

Add to your Claude Code MCP config:

```json
{
  "mcpServers": {
    "pixl": {
      "command": "/path/to/pixl",
      "args": ["mcp", "--file", "examples/dungeon.pax"]
    }
  }
}
```

14 MCP tools available:
- `pixl_session_start` — palette, theme, stamps, workflow
- `pixl_create_tile` — create with auto edge classification + 16x preview + edge context
- `pixl_narrate_map` — spatial predicates to rendered dungeon map
- `pixl_learn_style` / `pixl_check_style` — style latent extraction + scoring
- `pixl_render_tile`, `pixl_check_edge_pair`, `pixl_validate`
- `pixl_render_sprite_gif` — animated GIF preview for sprites
- `pixl_generate_context` — enriched prompt builder for AI generation
- `pixl_list_tiles`, `pixl_list_themes`, `pixl_list_stamps`
- `pixl_get_file`, `pixl_delete_tile`, `pixl_get_blueprint`
- `pixl_pack_atlas`, `pixl_load_source`

## HTTP API (PIXL Studio)

`pixl serve --port 3742` exposes 20 REST endpoints:

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
POST /api/tool
```

The `/api/generate/context` endpoint builds enriched system prompts with
palette symbols, theme constraints, style latent, and edge context — the
Studio sends this to Anthropic and gets back valid PAX grids.

## Export Formats

- **TexturePacker JSON Hash** — 48+ game engines (Unity, Godot, Phaser, Bevy...)
- **Tiled TMJ/TSJ** — with collision shapes, importable to Godot and Unity
- **Godot .tres** — TileSet resource with physics layers
- **GBStudio** — 128px-wide tileset PNG for Game Boy style games

## Theme Library

6 built-in themes with curated palettes and stamps. Start a project instantly:

```bash
pixl new dark_fantasy --out dungeon.pax   # Purple stone, dark shadows
pixl new light_fantasy --out castle.pax   # Warm marble, gold trim
pixl new sci_fi --out station.pax         # Neon blue on dark panels
pixl new nature --out forest.pax          # Greens, browns, water
pixl new gameboy --out gb_game.pax        # 4-color GB green
pixl new nes --out nes_game.pax           # 4-color NES brown
```

Each theme includes: palette, semantic color roles, light source direction,
`max_palette_size` constraint, and 2-5 stamps for compose mode. Validates
clean out of the box.

## Project Sessions

Multi-world game projects with persistent style across sessions:

```bash
# Initialize a project
pixl project init my_game --theme dark_fantasy

# Add worlds (each is a separate .pax file)
pixl project add-world my_game.pixlproject dungeon worlds/dungeon.pax
pixl project add-world my_game.pixlproject ice_cave worlds/ice.pax

# Extract style latent from a world's tiles
pixl project learn-style my_game.pixlproject dungeon
# -> style latent saved to my_game.pixlproject

# Check project status
pixl project status my_game.pixlproject
# -> Project: my_game | Theme: dark_fantasy | Worlds: 2 | Progress: 43/120 tiles
```

The `.pixlproject` file stores: project metadata, world list (paths to .pax
files), style latent (8-property fingerprint extracted from reference tiles),
and progress tracking. Style persists across sessions — world 5 looks like
world 1 because the latent encodes lighting, shadow density, pixel density,
hue bias, and palette usage from the first tiles you authored.

## Key Features

- **Style Latent** — extract visual fingerprint from reference tiles, score new tiles against it. Makes session 5 look like session 1.
- **SELF-REFINE Loop** — every create/render returns 16x preview PNG. The LLM sees what it drew and fixes issues iteratively (Madaan et al., NeurIPS 2023).
- **Blueprint System** — anatomy-guided character sprites. Exact pixel coordinates for eyes, shoulders, knees at any canvas size.
- **Diffusion Import** — quantize any reference image into a PAX palette. Bridge from FLUX.2/SD to indexed pixel art.
- **Procedural Stamps** — 8 pattern types (brick, checker, diagonal, Bayer dither...) generate compose vocabulary without LLM authorship.
- **Narrate-to-Map** — spatial predicates to rendered dungeon. The killer demo.

## Architecture

```
PIXL/
├── tool/                    Rust workspace (6 crates)
│   ├── pixl-core/           Format types, parser, validator, blueprint, style latent
│   ├── pixl-render/         Tile renderer, atlas, GIF, preview, diffusion import
│   ├── pixl-wfc/            Wave Function Collapse, semantic constraints, narrate
│   ├── pixl-mcp/            MCP server + HTTP API (rmcp + axum)
│   ├── pixl-export/         TexturePacker, Tiled, Godot, Unity, GBStudio
│   └── pixl-cli/            CLI binary (15 commands)
├── studio/                  PIXL Studio (Flutter desktop app)
├── docs/
│   ├── specs/pax.md         PAX 2.0 format specification
│   └── plans/               Implementation plan
└── examples/
    ├── dungeon.pax          Dark fantasy tileset
    ├── platformer.pax       Forest side-scroller
    └── gameboy.pax          4-color Game Boy
```

## Building

```bash
cd tool
cargo build --release
cargo test  # 136 tests
```

## License

MIT OR Apache-2.0
