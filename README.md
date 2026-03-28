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

## Install

### CLI (macOS, Linux, Windows)

Download pre-built binaries from [GitHub Releases](https://github.com/SimplyLiz/PIXL/releases):

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/SimplyLiz/PIXL/releases/latest/download/pixl-v1.0.0-aarch64-apple-darwin.tar.gz | tar xz
sudo mv pixl /usr/local/bin/

# macOS (Intel)
curl -fsSL https://github.com/SimplyLiz/PIXL/releases/latest/download/pixl-v1.0.0-x86_64-apple-darwin.tar.gz | tar xz
sudo mv pixl /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/SimplyLiz/PIXL/releases/latest/download/pixl-v1.0.0-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv pixl /usr/local/bin/
```

Or build from source:

```bash
cd tool && cargo build --release
# Binary at tool/target/release/pixl
```

### Studio (macOS)

Install via [Homebrew](https://brew.sh):

```bash
brew install SimplyLiz/pixl/pixl-studio
```

Or download the `.dmg` from [GitHub Releases](https://github.com/SimplyLiz/PIXL/releases).

### As a Rust library

Add `pixl-core` to your project to parse, validate, and work with `.pax` files:

```bash
cargo add pixl-core
```

[![crates.io](https://img.shields.io/crates/v/pixl-core.svg)](https://crates.io/crates/pixl-core)

## Quick Start

```bash
# Start a new project from a built-in theme
pixl new dark_fantasy --out my_tileset.pax

# Start with AI-generated tiles (outputs enriched prompts for each tile type)
pixl new dark_fantasy --out my_tileset.pax --generate

# Validate a .pax file
pixl validate my_tileset.pax

# Analyze tileset completeness — find missing transition tiles for WFC
pixl validate my_tileset.pax --completeness

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

# Convert AI-generated images to true 1:1 pixel art
pixl convert ai_image.png                                # 3 presets (small/medium/large)
pixl convert ai_image.png --width 160 --colors 32        # single resolution

# Import as animated PAX backdrop (tile-decomposed scene)
pixl backdrop-import pixelized.png --name waterfall --colors 32 -o scene.pax
pixl backdrop-render scene.pax --name waterfall -o static.png --scale 4
pixl backdrop-render scene.pax --name waterfall -o anim.gif --frames 8 --scale 4

# Render sprite animation as GIF or spritesheet
pixl render-sprite examples/dungeon.pax --spriteset hero --sprite walk -o walk.gif
pixl render-sprite examples/dungeon.pax --spriteset hero --sprite idle -o idle.png  # spritesheet

# Extract style fingerprint from reference tiles (uses OKLab color space)
pixl style examples/dungeon.pax

# Generate tile variants from a base tile
pixl vary examples/dungeon.pax --tile wall_solid --count 4

# Generate procedural stamps
pixl generate-stamps brick_bond --size 4

# Generate a map from spatial predicates
pixl narrate examples/dungeon.pax --width 12 --height 8 \
  -r "region:entrance:floor_moss:2x2:northwest" \
  -r "region:chamber:floor_stone:3x3:southeast" \
  --out dungeon_map.png

# Narrate with weight overrides and cell pinning (for ML pipeline)
pixl narrate examples/dungeon.pax -w floor_stone:5.0 --pin 0,0:wall_solid --format json

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

Add to your Claude Code MCP config (`~/.claude/settings.json`):

```json
{
  "mcpServers": {
    "pixl": {
      "command": "pixl",
      "args": ["mcp", "--file", "examples/dungeon.pax"]
    }
  }
}
```

For Claude Desktop, add to `~/Library/Application Support/Claude/claude_desktop_config.json`.

24 MCP tools available:
- `pixl_session_start` — palette, theme, stamps, workflow
- `pixl_create_tile` — create with auto edge classification + 16x preview + edge context
- `pixl_narrate_map` — spatial predicates to rendered dungeon map
- `pixl_learn_style` / `pixl_check_style` — style latent extraction + scoring
- `pixl_render_tile`, `pixl_check_edge_pair`, `pixl_validate`
- `pixl_render_sprite_gif` — animated GIF preview for sprites
- `pixl_generate_context` — enriched prompt builder for AI generation
- `pixl_generate_tile` — local LoRA inference for tile generation
- `pixl_list_tiles`, `pixl_list_themes`, `pixl_list_stamps`
- `pixl_get_file`, `pixl_delete_tile`, `pixl_get_blueprint`
- `pixl_pack_atlas`, `pixl_load_source`, `pixl_vary_tile`
- `pixl_convert_sprite` — convert AI images to true 1:1 pixel art
- `pixl_backdrop_import`, `pixl_backdrop_render` — backdrop scenes
- `pixl_new_from_template` — create project from built-in theme
- `pixl_export` — export to game engine formats
- `pixl_check_completeness` — analyze tileset gaps for WFC
- `pixl_generate_transition_context` — enriched prompts for missing transition tiles

## HTTP API (PIXL Studio)

`pixl serve --port 3742` exposes 35 REST endpoints:

```
GET  /health                  POST /api/session
POST /api/palette             GET  /api/themes
GET  /api/stamps              GET  /api/tiles
POST /api/tile/create         POST /api/tile/render
POST /api/tile/delete         POST /api/tile/edge-check
POST /api/tile/vary           POST /api/validate
POST /api/narrate             POST /api/style/learn
POST /api/style/check         POST /api/blueprint
POST /api/sprite/gif          GET  /api/file
POST /api/generate/context    POST /api/generate/tile
POST /api/atlas/pack          POST /api/load
POST /api/feedback            GET  /api/feedback/stats
GET  /api/feedback/constraints POST /api/training/export
GET  /api/training/stats      POST /api/new
POST /api/export              GET  /api/check/completeness
POST /api/tile/generate-transition
POST /api/convert             POST /api/backdrop/import
POST /api/backdrop/render     POST /api/tool
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

8 built-in themes with curated palettes and stamps. Start a project instantly:

```bash
pixl new dark_fantasy --out dungeon.pax   # Purple stone, dark shadows
pixl new light_fantasy --out castle.pax   # Warm marble, gold trim
pixl new sci_fi --out station.pax         # Neon blue on dark panels
pixl new nature --out forest.pax          # Greens, browns, water
pixl new gameboy --out gb_game.pax        # 4-color GB green
pixl new nes --out nes_game.pax           # 4-color NES brown
pixl new snes --out snes_game.pax         # 16-color SNES palette
pixl new gba --out gba_game.pax           # 16-color GBA palette
```

Each theme includes: palette, semantic color roles, light source direction,
`max_palette_size` constraint, 6 starter tiles (wall, floor, floor variant,
corner, door, decoration), and 2-5 stamps for compose mode. Validates clean
out of the box.

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
- **Sprite Conversion** — convert AI-generated "fake pixel art" to true 1:1 pixel art. 3 presets (small/medium/large) with OKLab perceptual color quantization. See [docs/guides/sprite-conversion.md](docs/guides/sprite-conversion.md).
- **Backdrop Scenes** — large animated backgrounds (160x240+) stored as tile-decomposed PAX scenes with 10 procedural animation zone types (cycle, wave, flicker, scroll, HDMA sine, gradient, mosaic, window, palette ramp, global clock). See [docs/specs/backdrop.md](docs/specs/backdrop.md).
- **Tileset Completeness** — analyze edge class connectivity, identify missing transition tiles, generate enriched prompts to fill gaps.
- **Map Generation Training** — TileGPT-style pipeline: MAP-Elites data synthesis → LoRA fine-tuning → LM + WFC generation. See [docs/guides/map-generation-training.md](docs/guides/map-generation-training.md).
- **OKLab Color Space** — perceptual color distance for image import, style latent extraction, and palette quantization.

## Architecture

```
PIXL/
├── tool/                    Rust workspace (6 crates)
│   ├── pixl-core/           Format types, parser, validator, blueprint, style latent
│   ├── pixl-render/         Tile renderer, atlas, GIF, preview, backdrop, pixelize
│   ├── pixl-wfc/            Wave Function Collapse, semantic constraints, narrate
│   ├── pixl-mcp/            MCP server + HTTP API (rmcp + axum)
│   ├── pixl-export/         TexturePacker, Tiled, Godot, Unity, GBStudio
│   └── pixl-cli/            CLI binary (25 commands)
├── training/                ML training pipeline (Python/MLX)
│   ├── map_elites.py        MAP-Elites QD data synthesis
│   ├── generate_map.py      LM + WFC map generation
│   └── adapters/            Trained LoRA adapters
├── studio/                  PIXL Studio (Flutter desktop app)
├── docs/
│   ├── specs/pax.md         PAX 2.0 format specification
│   ├── specs/backdrop.md    Backdrop format extension (large animated backgrounds)
│   ├── guides/              Local inference, map generation, sprite conversion, animation pipeline
│   └── research/            Research synthesis and design docs
└── examples/
    ├── dungeon.pax          Dark fantasy tileset
    ├── platformer.pax       Forest side-scroller
    └── gameboy.pax          4-color Game Boy
```

## Building from Source

```bash
cd tool
cargo build --release
cargo test  # 136 tests
```

### Studio from Source

```bash
cd studio
flutter pub get
flutter run -d macos
```

See [Getting Started](docs/user/getting-started.md) for detailed Studio setup.

## Links

- [Website](https://pixl.dev)
- [GitHub Releases](https://github.com/SimplyLiz/PIXL/releases)
- [pixl-core on crates.io](https://crates.io/crates/pixl-core)
- [Homebrew Tap](https://github.com/SimplyLiz/homebrew-pixl)
- [PAX Format Spec](docs/specs/pax.md)
- [User Documentation](docs/user/)

## License

MIT OR Apache-2.0
