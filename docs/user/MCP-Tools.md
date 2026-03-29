# MCP Tools

PIXL's MCP server exposes 28 tools that any AI assistant (Claude, GPT, etc.) can call through the Model Context Protocol.

## Starting the server

```bash
pixl mcp --file tileset.pax
```

The server runs over stdio. Claude Code and other MCP clients connect automatically.

## Tool categories

### Discovery

| Tool | What it does |
|------|-------------|
| `pixl_session_start` | Returns theme, palette, stamps, tiles. Call this first. |
| `pixl_get_palette` | Full symbol table with hex colors and roles |
| `pixl_list_tiles` | All tiles with edge classes and tags |
| `pixl_list_themes` | Available themes with palette and constraints |
| `pixl_list_stamps` | Reusable stamp blocks with sizes |
| `pixl_list_composites` | Composites with variants and animations |

### Creation

| Tool | What it does |
|------|-------------|
| `pixl_create_tile` | Create tile from character grid, returns preview + edge analysis |
| `pixl_generate_sprite` | DALL-E → quantize → PAX tile with auto-palette |
| `pixl_upscale_tile` | Nearest-neighbor grid upscale (8→16, 16→32) |
| `pixl_load_source` | Load a .pax string into the session |

### Quality

| Tool | What it does |
|------|-------------|
| `pixl_critique_tile` | Structural analysis + rendered preview + fix instructions |
| `pixl_refine_tile` | Patch specific rows, re-render, re-critique |
| `pixl_check_seams` | Composite seam continuity warnings |
| `pixl_show_references` | Render matching tiles as visual examples |
| `pixl_remap_tile` | Remap tile from one palette to another |

### Style

| Tool | What it does |
|------|-------------|
| `pixl_learn_style` | Extract 8-property style fingerprint |
| `pixl_check_style` | Score a tile against the session style (0-1) |
| `pixl_generate_context` | Build full generation prompt with rendered examples |

### Rendering

| Tool | What it does |
|------|-------------|
| `pixl_render_tile` | Render tile to PNG at specified scale |
| `pixl_render_sprite_gif` | Animated GIF from spriteset |
| `pixl_render_composite` | Render composite with variant/animation |

### Map generation

| Tool | What it does |
|------|-------------|
| `pixl_narrate_map` | WFC map from spatial predicate rules |
| `pixl_check_edge_pair` | Test if two tiles can be adjacent |
| `pixl_check_completeness` | Find missing transition tiles |

### Export

| Tool | What it does |
|------|-------------|
| `pixl_pack_atlas` | Sprite atlas PNG + TexturePacker JSON |
| `pixl_export` | Game engine format (Tiled, Godot, Unity) |
| `pixl_get_file` | Full .pax TOML source |

## The SELF-REFINE loop

The recommended workflow for AI-assisted tile creation:

```
1. pixl_generate_sprite  →  generates tile + preview
2. pixl_critique_tile    →  structural analysis + fix instructions
3. pixl_refine_tile      →  patch specific rows
4. Repeat 2-3 until critique passes (max 3 rounds)
```

The AI sees the rendered preview image at each step — it's looking at what it drew, not guessing blind.
