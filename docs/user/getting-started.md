# Getting Started

Get PIXL running and create your first tileset in under five minutes.

## Install

### PIXL Studio (recommended)

Download the desktop app for Mac, Windows, or Linux from [GitHub Releases](https://github.com/SimplyLiz/PIXL/releases/latest). The Rust engine is bundled inside — no separate install needed.

On macOS with Homebrew:

```bash
brew install SimplyLiz/pixl/pixl-studio
```

### CLI only

Download prebuilt binaries from [GitHub Releases](https://github.com/SimplyLiz/PIXL/releases/latest) (macOS, Linux, Windows), or install via Cargo:

```bash
cargo install pixl
```

Or build from source:

```bash
git clone https://github.com/SimplyLiz/PIXL.git
cd PIXL/tool
cargo build --release
```

The binary is at `tool/target/release/pixl`.

## Create your first tileset

### 1. Start from a template

```bash
pixl new dark_fantasy -o my_tileset.pax
```

This creates a `.pax` file with a dark fantasy palette, theme constraints, and a few starter stamps. Available themes: `dark_fantasy`, `light_fantasy`, `sci_fi`, `nature`, `gameboy`, `nes`.

### 2. Validate

```bash
pixl validate my_tileset.pax
```

### 3. Render a tile

```bash
pixl render my_tileset.pax --tile wall_solid --scale 8 --out wall.png
```

### 4. Generate a map

```bash
pixl narrate my_tileset.pax --width 12 --height 8 --out map.png
```

## Using with Claude

PIXL includes an MCP server that connects directly to Claude Code or any MCP-compatible AI assistant:

```bash
pixl mcp --file my_tileset.pax
```

Then ask Claude to create tiles, critique your art, generate sprites, or build maps — all through natural conversation.

## Next steps

- [PAX Format](./PAX-Format) — understand the file format
- [Sprite Generation](./Sprite-Generation) — generate art from descriptions
- [CLI Reference](./CLI-Reference) — all available commands
