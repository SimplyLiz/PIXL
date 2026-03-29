# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

PIXL is an LLM-native pixel art toolchain. It bridges the gap between what LLMs can reason about (symbolic composition) and what they struggle with (raw pixels above 12x12). The core insight is three-tier encoding: grid (≤16px, raw characters), RLE (17-32px, run-length rows), compose (33-64px, named stamp placement). With symmetry declarations, a 32x32 tile becomes a 16x16 grid.

## Repository Structure

- **`tool/`** — Rust workspace (6 crates). All Rust work happens from this directory.
- **`studio/`** — Flutter desktop app (PIXL Studio). Riverpod for state management.
- **`training/`** — Python ML pipeline (MAP-Elites, LoRA fine-tuning, MLX).
- **`docs/`** — Specs (`pax.md`), research, guides, user docs.
- **`tool/examples/`** — Reference `.pax` files (dungeon, platformer, gameboy).

## Build & Test Commands

### Rust (from `tool/`)
```bash
cargo build --release           # Build all crates
cargo test                      # Run all tests (~136)
cargo test -p pixl-core         # Test a single crate
cargo test test_name            # Run a specific test
cargo clippy --all-targets -- -W warnings   # Lint (CI enforced)
cargo fmt --check               # Format check (CI enforced)
cargo run -- <command>          # Run CLI (e.g. cargo run -- validate examples/dungeon.pax)
```

### Flutter (from `studio/`)
```bash
flutter pub get                 # Install dependencies
flutter test                    # Run tests
flutter run -d macos            # Dev run
flutter build macos --release   # Production build
```

### CI validates
- Rust build + test on Ubuntu, macOS, Windows
- Clippy and fmt on Ubuntu only
- Example validation: `validate dungeon.pax`, `render wall_solid`, `atlas pack`

## Rust Workspace (`tool/`)

| Crate | Role |
|-------|------|
| **pixl-core** | Parser, types, validator, grid/RLE/compose encoding, style latent, blueprint, edge classes, OKLab color space |
| **pixl-render** | Tile renderer, atlas packing, GIF animation, preview, backdrop, pixelize |
| **pixl-wfc** | Wave Function Collapse with semantic constraints, `narrate` (map generation) |
| **pixl-mcp** | MCP server (rmcp) + HTTP API (axum) — 24 MCP tools, 35+ REST endpoints |
| **pixl-export** | Game engine exporters: TexturePacker, Tiled, Godot, Unity, GBStudio |
| **pixl-cli** | CLI binary, 25+ subcommands via clap |

Dependency flow: `pixl-core` → `pixl-render` / `pixl-wfc` → `pixl-export` / `pixl-mcp` → `pixl-cli`

Tests are **inline** (`#[cfg(test)]` modules within source files), not in a separate `tests/` directory.

## Flutter Studio (`studio/`)

- State management: **flutter_riverpod** — providers in `lib/providers/`
- Backend communication: HTTP client (`lib/services/pixl_backend.dart`) talks to `pixl serve --port 3742`
- Linting: `flutter_lints` (analysis_options.yaml)
- Tests in `studio/test/` (3 test files)

## PAX Format

`.pax` files are TOML. They contain themes, palettes, stamps, tiles, sprites, WFC rules, atlas config. The format spec is at `docs/specs/pax.md` (PAX 2.1). Key concepts:

- **Edge classes** — directional tile compatibility (`n/e/s/w`) driving WFC
- **Style latent** — 8-property fingerprint for visual consistency across sessions
- **Symmetry declarations** — reduce effective grid size (quad, horizontal, vertical)
- **Stamps** — reusable 2x2–8x8 pixel macros for compose mode
- **Blueprint** — anatomical coordinate guides for character sprites

## Key Architectural Patterns

- **SELF-REFINE loop**: every create/render returns a 16x preview PNG so the LLM sees what it drew and iterates
- **MCP server** (`pixl mcp --file <path>`) exposes tools for Claude Code integration
- **HTTP API** (`pixl serve --port 3742`) exposes REST endpoints for Studio
- The MCP server and HTTP API share the same engine code; MCP tools map to the same core functions as HTTP endpoints
- **OKLab color space** used everywhere for perceptual color distance

## Version Management

Single-source version in `/VERSION` file (currently 2.0.0). Bump script at `tool/scripts/bump_version.sh`.

## Important Files

- `docs/specs/pax.md` — PAX 2.1 format specification (the source of truth for .pax format)
- `docs/specs/backdrop.md` — Backdrop format extension
- `tool/crates/pixl-mcp/src/tools.rs` — MCP tool definitions
- `tool/crates/pixl-mcp/src/http.rs` — HTTP API endpoint definitions
- `tool/crates/pixl-cli/src/main.rs` — CLI command definitions
