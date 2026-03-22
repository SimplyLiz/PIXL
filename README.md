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
# Validate a .pax file
pixl validate examples/dungeon.pax

# Render a tile to PNG
pixl render examples/dungeon.pax --tile wall_solid --scale 4 --out wall.png

# 16x zoom preview (for SELF-REFINE visual inspection)
pixl preview examples/dungeon.pax --tile floor_moss --out preview.png --grid

# Pack tiles into a sprite atlas with TexturePacker JSON
pixl atlas examples/dungeon.pax --out atlas.png --map atlas.json --scale 2

# Show anatomy blueprint for character sprites
pixl blueprint 32x48

# Start MCP server for Claude Code integration
pixl mcp --file examples/dungeon.pax
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
"." = "#00000000"    # transparent
"#" = "#2a1f3d"      # stone dark
"+" = "#4a3a6d"      # stone lit
"~" = "#1a3a5c"      # water

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

Full format specification: [docs/specs/pax.md](docs/specs/pax.md)

## MCP Integration

Add to your Claude Code MCP config:

```json
{
  "mcpServers": {
    "pixl": {
      "command": "pixl",
      "args": ["mcp"]
    }
  }
}
```

Then ask Claude to create pixel art tilesets. The MCP server provides 10 tools:

- `pixl_session_start` — palette symbols, theme, workflow guidance
- `pixl_create_tile` — create tile with auto edge classification + 16x preview
- `pixl_check_edge_pair` — verify two tiles can be placed adjacent
- `pixl_render_tile` — render any tile to PNG
- `pixl_validate` — full file validation
- `pixl_get_blueprint` — anatomy guide for character sprites
- `pixl_list_tiles`, `pixl_get_palette`, `pixl_get_file`, `pixl_delete_tile`

Every create/render response includes a base64 PNG at 16x zoom — the
SELF-REFINE vision loop (Madaan et al., NeurIPS 2023) lets the LLM see what
it drew and fix issues iteratively.

## Export Formats

- **TexturePacker JSON Hash** — 48+ game engines (Unity, Godot, Phaser, Bevy...)
- **Tiled TMJ/TSJ** — with collision shapes, importable to Godot and Unity
- **Godot .tres** — TileSet resource with physics layers
- **GBStudio** — 128px-wide tileset PNG for Game Boy style games

## Architecture

```
PIXL/
├── tool/                    Rust workspace
│   ├── pixl-core/           Format types, parser, validator, blueprint
│   ├── pixl-render/         Tile renderer, atlas, GIF, preview
│   ├── pixl-wfc/            Wave Function Collapse engine
│   ├── pixl-mcp/            MCP server (rmcp, stdio)
│   ├── pixl-export/         TexturePacker, Tiled, Godot, Unity, GBStudio
│   └── pixl-cli/            CLI binary
├── studio/                  PIXL Studio (Flutter, future)
├── docs/specs/pax.md        PAX 2.0 format specification
└── examples/dungeon.pax     Complete example tileset
```

## Building

```bash
cd tool
cargo build --release
cargo test
```

## License

MIT OR Apache-2.0
