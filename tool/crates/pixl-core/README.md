# pixl-core

Core library for the [PIXL](https://github.com/SimplyLiz/PIXL) toolchain — an LLM-native pixel art system.

`pixl-core` provides the parser, type definitions, validator, and utilities for working with `.pax` files, a TOML-based pixel art tileset format designed for AI-assisted game development.

## Usage

```rust
let source = std::fs::read_to_string("tileset.pax").unwrap();
let pax = pixl_core::parser::parse(&source).unwrap();
let errors = pixl_core::validate::validate(&pax);
```

## What's inside

- **Parser** — `.pax` TOML to typed Rust structures
- **Types** — `Pax`, `Tile`, `Palette`, `Theme`, edge classes, sprites, backdrops
- **Validator** — tileset consistency checks (edges, palettes, sizes, semantic tags)
- **Edge classification** — automatic edge class extraction from grid content
- **Three-tier encoding** — `grid` (raw), `rle` (run-length), `compose` (stamp placement)
- **Style latent** — 8-property visual fingerprint using OKLab perceptual color space
- **Blueprint system** — anatomy coordinates for character sprites at any canvas size
- **Tileset completeness** — gap analysis for Wave Function Collapse connectivity
- **Theme library** — 8 built-in themes with curated palettes and starter tiles

## Part of PIXL

This crate is the foundation of the PIXL toolchain:

- **[PIXL CLI](https://github.com/SimplyLiz/PIXL)** — 25-command CLI for rendering, exporting, and generating tilesets
- **[PIXL Studio](https://github.com/SimplyLiz/PIXL/tree/main/studio)** — Flutter desktop editor with AI chat
- **PIXL MCP** — Model Context Protocol server for Claude Code / Claude Desktop integration

Install the full toolchain from [GitHub Releases](https://github.com/SimplyLiz/PIXL/releases) or via Homebrew (`brew install SimplyLiz/pixl/pixl-studio`).

## License

MIT OR Apache-2.0
