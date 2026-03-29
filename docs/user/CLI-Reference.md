# CLI Reference

All PIXL commands available from the terminal.

## Tile operations

| Command | What it does |
|---------|-------------|
| `pixl render FILE --tile NAME --scale N --out PATH` | Render a tile to PNG |
| `pixl preview FILE --tile NAME --out PATH` | 16× zoom preview with optional grid |
| `pixl critique FILE --tile NAME` | Structural quality analysis |
| `pixl upscale FILE --tile NAME --factor 2 --out PATH` | Nearest-neighbor grid upscale |

## Validation

| Command | What it does |
|---------|-------------|
| `pixl validate FILE` | Check palette, grid, edges, composites |
| `pixl validate FILE --check-edges` | Include edge compatibility checks |
| `pixl validate FILE --check-seams` | Check composite tile boundary continuity |
| `pixl validate FILE --completeness` | Find missing WFC transition tiles |
| `pixl check FILE --fix` | Auto-classify edges from grid content |

## Generation

| Command | What it does |
|---------|-------------|
| `pixl generate-sprite FILE --prompt "..." --name NAME --out PATH` | DALL-E → pixel art |
| `pixl narrate FILE --width W --height H -r "rule" --out PATH` | WFC map generation |
| `pixl new THEME --out PATH` | Create tileset from template |
| `pixl blueprint SIZE` | Show character anatomy landmarks |

## Conversion

| Command | What it does |
|---------|-------------|
| `pixl convert IMAGE --width W --colors N` | AI image → pixel art |
| `pixl import IMAGE --size WxH --pax FILE --palette NAME` | Quantize to specific palette |
| `pixl backdrop-import IMAGE --name NAME --colors N --tile-size T` | Image → PAX backdrop |

## Sprites & animation

| Command | What it does |
|---------|-------------|
| `pixl render-sprite FILE --spriteset NAME --sprite NAME --out PATH` | Render animated GIF |
| `pixl render-composite FILE --composite NAME --out PATH` | Render composite sprite |
| `pixl vary FILE --tile NAME --count N` | Generate tile variants |

## Atlas & export

| Command | What it does |
|---------|-------------|
| `pixl atlas FILE --out PATH --map JSON` | Pack sprite atlas + JSON |
| `pixl export FILE --format tiled --out DIR` | Export to game engine format |
| `pixl style FILE` | Extract style fingerprint |

## Server

| Command | What it does |
|---------|-------------|
| `pixl mcp --file FILE` | Start MCP server (for Claude) |
| `pixl serve --port 3742 --file FILE` | Start HTTP API (for Studio) |

## Common flags

- `--scale N` — render scale factor (default varies by command)
- `--out PATH` / `-o PATH` — output file path
- `--file FILE` — input .pax file
