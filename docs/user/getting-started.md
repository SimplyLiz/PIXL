# Getting Started with PIXL Studio

## Prerequisites

- **macOS** (Apple Silicon recommended for local AI)
- **Flutter** installed (`flutter doctor` should pass)
- **Rust toolchain** installed (`rustup`, `cargo`)
- An LLM API key (Anthropic, OpenAI, Gemini) OR Ollama running locally

## Build & Run

```bash
# 1. Build the PIXL engine
cd tool && cargo build --release

# 2. Run Studio
cd studio && flutter run -d macos
```

Studio automatically starts the engine on launch. If it can't find the binary, you'll see "Engine not connected" in the status bar — make sure `tool/target/release/pixl` exists.

## First Launch

On first launch, you'll see the **Auto-Learn opt-in dialog**. This asks whether accepted tiles should be saved as training data for the local LoRA model. All data stays on your machine. You can change this anytime in Settings > Training.

## Opening a Project

1. Click **Open PAX** in the top bar to load a `.pax` file
2. Click **New** to create a project from a theme template (Dark Fantasy, Sci-Fi, Nature, Game Boy, etc.) or start with a blank canvas
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
