# Getting Started with PIXL Studio

## Install

### Option 1: Homebrew (recommended)

```bash
brew install SimplyLiz/pixl/pixl-studio
```

### Option 2: Download

Download the latest `.dmg` from [GitHub Releases](https://github.com/SimplyLiz/PIXL/releases) and drag PIXL Studio to your Applications folder.

### Option 3: Build from source

Prerequisites: macOS, [Flutter](https://flutter.dev), [Rust toolchain](https://rustup.rs)

```bash
# 1. Build the PIXL engine
cd tool && cargo build --release

# 2. Run Studio
cd studio && flutter pub get && flutter run -d macos
```

Studio automatically starts the engine on launch. If it can't find the binary, you'll see "Engine not connected" in the status bar — make sure `tool/target/release/pixl` exists.

## CLI

You also need the `pixl` CLI for rendering, exporting, and MCP server. Install it separately:

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/SimplyLiz/PIXL/releases/latest/download/pixl-v1.0.0-aarch64-apple-darwin.tar.gz | tar xz
sudo mv pixl /usr/local/bin/
```

Or build from source: `cd tool && cargo build --release`

## Prerequisites

- **macOS** (Apple Silicon recommended for local AI)
- An LLM API key (Anthropic, OpenAI, Gemini) OR Ollama running locally

## First Launch

On first launch, you'll see the **Auto-Learn opt-in dialog**. This asks whether accepted tiles should be saved as training data for the local LoRA model. All data stays on your machine. You can change this anytime in Settings > Training.

## Opening a Project

1. Click **Open PAX** in the top bar to load a `.pax` file
2. Click **New** to open the **New from Template** dialog:
   - Pick a theme (Dark Fantasy, Light Fantasy, Sci-Fi, Nature, Retro 8-bit, Game Boy)
   - Each theme comes with 6 starter tiles: wall, floor, floor variant, corner, door (N/S), and decoration
   - Or choose **Blank Canvas** for an empty project
3. Recent files appear in the clock icon menu next to Open PAX

The engine loads the file automatically and populates tiles, palettes, and themes.

## Layout

```
┌──────────────────────────────────────────────────┐
│ Top Bar: mode toggle, file actions, canvas controls │
├────────┬────┬──────────────────┬──┬───────────────┤
│  Chat  │Tool│   Canvas         │Tab│  Right Panel  │
│ Panel  │Strip  (pixel/tilemap) │Bar│  (4 tabs)     │
│        │    │   Tile Picker    │  │               │
├────────┴────┴──────────────────┴──┴───────────────┤
│ Status Bar                                        │
└───────────────────────────────────────────────────┘
```

- **Chat Panel** (left): AI expert chat, tile generation, accept/reject flow
- **Tool Strip** (thin vertical): drawing tools, mode-aware
- **Canvas** (center): pixel editor or tilemap painter
- **Tile Picker** (below canvas): session tiles as selectable thumbnails
- **Right Panel** (tabbed): Palette, Style, Generate, Tiles

## Editor Modes

Toggle between **Pixel** and **Tilemap** mode using the toggle in the top bar.

### Pixel Mode
Traditional pixel-by-pixel editing with layers, symmetry, and palette.

### Tilemap Mode
Paint tiles on a 2D grid. Select a tile from the picker strip, then stamp it on the map canvas. Useful after generating a tileset.

## Saving

- **Cmd+S**: Quick-save to the last opened .pax file
- **Export menu**: PNG (4x/8x), PAX source, atlas, game engine formats

## Next Steps

- [Drawing Tools](drawing-tools.md) — all tools and shortcuts
- [AI Features](ai-features.md) — generation, style transfer, auto-tag
- [Tilemap Mode](tilemap-mode.md) — painting maps with tiles
- [Settings & Training](settings.md) — LLM providers, local inference, training
