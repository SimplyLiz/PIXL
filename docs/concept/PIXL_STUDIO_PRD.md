# PIXL Studio — Product Requirements Document
## AI-Native Pixel Art Editor with Expert Chat
**Version 0.1 — MVP Spec**

---

## Vision

PIXL Studio is the first pixel art editor where an AI expert sits alongside your canvas — not to replace your creativity, but to handle the technical craft: palette constraints, WFC tileability, color theory, dithering rules, and PIXL source generation. You draw and decide. The AI validates, suggests, and generates.

Target user: indie game developers and pixel artists who want to produce coherent, tileable game art sets faster than doing it fully by hand, and better than diffusion models alone can manage.

---

## Problem Statement

Existing tools fall into two camps:
- **Traditional editors** (Aseprite, LibreSprite): powerful for artists but no AI, no tileability validation, no style consistency enforcement
- **AI image generators** (Midjourney, Leonardo, PixelLab): non-deterministic, non-editable, non-tileable, palette-inconsistent

PIXL Studio is the hybrid: a real pixel art editor (precise control, indexed palette, layers) with an AI expert that knows the craft and generates PIXL-format source the renderer can validate before touching a pixel.

---

## Core Principles

1. **The LLM advises, the artist decides.** Generation is a suggestion, not a result.
2. **Every generated asset is inspectable.** The PIXL source is always visible and editable.
3. **Coherence is enforced by the system.** Palette, style, and tile constraints are baked into the session — you can't accidentally go off-style.
4. **Validation closes the loop.** WFC edge checking runs before render. Bad tiles never appear silently.

---

## MVP Feature Set

### 1. Canvas Editor

#### 1a. Pixel drawing surface
- Fixed-size canvas: 8×8, 16×16, 32×32, 48×48, 64×64 (selectable at project creation)
- Pixel-perfect zoom: 2×, 4×, 8×, 14×, 20×, 32×
- Tools: Pencil, Eraser, Bucket fill, Eyedropper, Rectangle select, Move
- Symmetry mode: horizontal, vertical, both (write one quadrant, mirror the rest)
- Grid overlay toggle
- Current pixel position + color display in status bar

#### 1b. Color system (indexed palette only in MVP)
- Session palette: 2–64 colors max
- Palette imported from active theme (dark_fantasy, sci_fi, nature, custom)
- Palette slot: click to select foreground color, shift-click for background
- Palette editor: add/remove/reorder swatches, click to edit hex (no free color entry — must pick from palette)
- Theme palette lock: optional — prevents adding non-theme colors

#### 1c. Layers (MVP: simplified)
- 2 layers: Base + Detail
- Toggle visibility per layer
- Merge layers to flat on export
- No blend modes in MVP

#### 1d. Undo/redo
- 50-step history minimum
- Per-pixel granularity (not stroke-level)

---

### 2. AI Expert Chat

The chat is not generic — it is a specialized pixel art and game design expert loaded with a curated knowledge base.

#### Knowledge Base Contents (loaded as system context)
- **Color theory for pixel art**: limited palette design, ramp construction, value contrast, hue shifting
- **Dithering techniques**: pattern, ordered (Bayer), checkerboard, selective
- **Outline techniques**: self-outline, drop shadow, selective outline, no-outline styles
- **Tileability rules**: edge consistency, WFC adjacency theory, Wang tile sets
- **Sprite anatomy**: silhouette priority, light source consistency, anti-aliasing rules for pixel art
- **Animation principles**: squash/stretch adapted to pixel scale, key frame delta strategy
- **PIXL format spec**: full syntax, micro-tile library, theme system, edge declarations
- **Game art patterns**: dungeon tiles, overworld tiles, UI elements, character sprite sheets
- **Platform constraints**: NES (4 colors/sprite), SNES (16 colors/tile), Game Boy (4 shades) — as style guides
- **Style vocabulary**: how to describe "gritty", "retro", "vibrant", "lo-fi", and translate to palette decisions

#### Chat capabilities in MVP
- Ask technique questions, get targeted expert answers
- Request PIXL source generation: "generate a 16×16 dungeon wall tile, dark fantasy style"
- Request palette suggestions with rationale
- Ask "is this tile WFC-compatible?" → chat triggers validation, reports result
- Describe a sprite concept → get the PIXL source → push to canvas in one click
- Knowledge tag toggles: enable/disable sub-topics to keep context focused

#### Chat is NOT
- A general-purpose AI (scoped strictly to pixel art + game design domain)
- A replacement for the canvas (it generates source, the tool renders)
- Able to directly edit the canvas (it proposes, artist confirms)

---

### 3. AI Generation

#### Generation flow
1. Artist describes what they want (text prompt in chat or dedicated prompt field)
2. System enriches prompt with active theme, palette constraints, canvas size, tile type
3. Claude generates PIXL source for the tile/sprite
4. PIXL renderer validates (edge compatibility, palette compliance, size)
5. Preview renders in a side panel — not yet on canvas
6. Artist accepts (push to canvas), requests variation (re-generate with temperature bump), or rejects

#### Prompt enrichment (automatic, invisible to user)
Active theme + palette → constraints appended to every generation prompt:
```
Generate a 16×16 pixel art tile using exactly these symbols:
  '.' = transparent
  '#' = #2a1f3d (dark stone, primary structure)
  '+' = #4a3a6d (lit stone, interior surface)
  'g' = #2d5a27 (moss accent, ≤10% of interior)
  'h' = #6a5a9d (highlight, ≤5% of interior)
  's' = #1a1228 (deep shadow)
All four edge rows must be solid '#' for WFC compatibility.
Style: dark fantasy dungeon. Gritty, weathered stone, occasional moss.
```

#### Variation system
- "Variations (3)" button generates 3 alternatives at once
- Displayed as small previews in a strip below the main canvas
- User clicks to select one → pushes to main canvas

#### Tile group generation
- "Generate full tilegroup" → generates all 13 (4-bit) or all 47 (8-bit/Wang) variants
- Takes 15–30 seconds for full set
- Progress shown per-tile in sidebar
- On completion: variant strip populates with all tiles

---

### 4. Style System

#### Theme selection
- Built-in themes: dark_fantasy, light_fantasy, sci_fi, nature, retro_8bit
- Custom theme creation: pick palette + give it a name → saved locally
- Theme applied at session level — all generation and validation uses it

#### Style chips (per-session modifiers)
Additional style constraints on top of theme:
- **Dithering**: none, bayer, ordered, selective
- **Outline**: none, self, drop-shadow, selective
- **Mood**: gritty, clean, vibrant, pastel, monochrome

Style chips are injected into every generation prompt automatically.

#### Theme preview
Color swatch row + palette character mapping shown in sidebar. Artist can compare themes before switching.

---

### 5. Validation

On every generated or manually drawn tile, the validation panel shows:
- **Edge compatibility**: are the four edges consistent for tiling? (runs WFC simple-tiled check)
- **Palette compliance**: any pixels using non-palette colors?
- **Symbol consistency**: any PIXL symbols not mapped to palette?
- **Size**: matches expected canvas size?

Validation runs instantly on save, and can be triggered manually via cmd+shift+V.

Visual indicator in tile info sidebar: green ✓ / red ✗ per check.

---

### 6. Export

- **Export single tile**: PNG (at canvas size or scaled 2×/4×/8×)
- **Export PIXL source**: `.pixl` file for the current tile/sprite
- **Export atlas**: pack all tiles in current session into a spritesheet PNG + tilemap JSON (Tiled/Godot compatible)
- **Export animation**: GIF from sprite frames

---

### 7. Project / Session Management

- Project = a named session with: active theme, canvas size, palette, tile group, all tiles
- Projects saved locally as `.pixlproj` (zip of `.pixl` files + metadata)
- Recent projects list on open screen
- No cloud sync in MVP (local files only)

---

## Tech Stack

### Frontend: Flutter (Desktop)

**Rationale:**
- Lisa and the TasteHub team are already expert Flutter developers
- Flutter Desktop (macOS, Windows, Linux) is mature and production-ready
- `CustomPainter` gives full control over pixel-perfect canvas rendering — exactly what a pixel art editor needs
- Chat UI, panel layout, and state management are well-served by Flutter's widget system
- Single codebase for all desktop targets
- Riverpod for state management (session state, canvas state, chat history)

**Key Flutter packages:**
- `riverpod` — state management
- Custom `CustomPainter` canvas (no package needed — just the raw API)
- `flutter_markdown` — rendering AI chat messages
- `file_picker`, `path_provider` — file system access
- `http` — backend API calls
- `shared_preferences` — local settings
- `gif` — GIF export (or custom frame encoder)

**NOT Flutter Web** (MVP): canvas performance and file system access are better native desktop.

---

### Backend: Go

**Rationale:**
- Consistent with the PIXL renderer (same repo)
- Fast, small binary, easy to embed or run as subprocess
- Handles: AI API proxy, PIXL rendering, WFC engine, atlas packing

**Backend structure:**
```
pixlstudio-backend/
  cmd/server/       → HTTP API server (go run .)
  internal/
    pixl/           → PIXL parser + renderer
    wfc/            → Wave Function Collapse engine
    ai/             → Anthropic API proxy + prompt builder
    atlas/          → Sprite atlas packer
    validate/       → Tile validation logic
```

**API endpoints (all local, no cloud in MVP):**
```
POST /api/generate          → { prompt, theme, size } → PIXL source
POST /api/render            → { pixl_source } → PNG bytes (base64)
POST /api/validate          → { pixl_source } → validation report
POST /api/wfc/map           → { tilegroup, w, h, seed } → map PIXL
POST /api/chat              → { messages, knowledge_tags } → AI response
GET  /api/themes            → list available themes
GET  /api/microtiles        → list micro-tile library for a theme
POST /api/atlas/pack        → { tiles[] } → atlas PNG + JSON
```

**The backend runs locally** (started by the Flutter app as a subprocess on launch, or always-on daemon). No internet required except for AI API calls.

---

### AI: Anthropic Claude API

- Model: Claude Sonnet 4.6 (fast, smart, cost-effective for iteration)
- System prompt: large (~8k tokens) — contains full knowledge base + PIXL spec + active theme
- Temperature: 0.3 for tile generation (deterministic), 0.7 for chat answers (more conversational)
- API key: entered by user in Settings, stored in OS keychain (not in code)

**Prompt architecture:**
```
system: [pixel art knowledge base]
        [PIXL format spec]
        [active theme: dark_fantasy palette + rules]
        [canvas constraints: 16×16, tiling, edge rules]
user:   [conversation history]
        [current request]
```

Knowledge base is a single curated Markdown document (~5k tokens) authored once, maintained in the repo.

---

### Local Storage

- Projects: `~/Documents/PixlStudio/projects/` (`.pixlproj` files)
- Themes: `~/Documents/PixlStudio/themes/` (`.pixlt` files)
- Settings: OS keychain (API key) + `SharedPreferences` (app settings)
- No SQLite, no cloud in MVP

---

## UI Layout

### Three-panel layout

```
┌──────────────────────────────────────────────────────────┐
│  TOPBAR: Logo · File Edit View Generate Export · badges  │
├────────────┬───────────────────────────────┬─────────────┤
│            │                               │             │
│  AI CHAT   │      CANVAS VIEWPORT          │  TOOLS      │
│            │      (pixel editor)           │  PANEL      │
│  230px     │      flex: 1                  │  180px      │
│            │                               │             │
│            ├───────────────────────────────┤             │
│            │  VARIANT STRIP (tile previews)│             │
└────────────┴───────────────────────────────┴─────────────┘
│  STATUS BAR: pos · color · tile name · canvas size       │
└──────────────────────────────────────────────────────────┘
```

### Left panel: AI Chat (230px)
- Active knowledge tags (toggleable chips)
- Chat message thread (user/AI alternating)
- Input textarea + Send button
- Clean and focused — no decorative chrome

### Center: Canvas (flex: 1)
- Canvas toolbar: tool buttons, zoom, grid toggle, symmetry, current info
- Canvas viewport: the pixel grid, checkered background for transparency
- Bottom strip: tile variant previews for current tilegroup (13 or 47 tiles)

### Right panel: Tools (180px)
Scrollable stacked sections:
1. **Generate**: prompt field + Generate Tile + Variations buttons
2. **Style**: theme chips + style modifier chips + active palette preview
3. **Palette**: indexed color swatches, active swatch highlighted
4. **Layers**: 2-row layer list (MVP)
5. **Tile info**: type, edge compat, WFC status, group name

### Status bar
Position, current color hex, tile name, canvas size, version.

---

## Development Phases

### Phase 1 — Canvas + Palette (2 weeks)
- Flutter app shell, 3-panel layout
- CustomPainter pixel canvas with pencil + eraser + fill tools
- Indexed palette display + color selection
- Zoom + grid toggle
- Undo/redo (50 steps)
- File open/save (PNG + .pixlproj)

### Phase 2 — Backend + PIXL (2 weeks)
- Go backend with HTTP API (local subprocess)
- PIXL parser + PNG renderer
- `/api/render`, `/api/validate` endpoints
- Validation panel in Flutter UI
- Tile info sidebar (edge compat, WFC status)

### Phase 3 — AI Chat + Generation (2 weeks)
- Anthropic API proxy in Go backend
- Knowledge base Markdown document (authored once)
- Chat UI in Flutter with markdown rendering
- `/api/chat` endpoint
- `/api/generate` endpoint with prompt enrichment
- Accept/reject/variation flow

### Phase 4 — Style System + Themes (1 week)
- Built-in themes (dark_fantasy, sci_fi, nature)
- Theme selection UI
- Style chip modifiers
- Theme palette lock
- Custom theme creation

### Phase 5 — Tile Groups + WFC (2 weeks)
- WFC engine in Go (simple tiled model)
- `/api/wfc/map` endpoint
- Tilegroup generation flow (all 13 variants)
- Variant strip UI
- Edge rule auto-derivation from tile names

### Phase 6 — Export (1 week)
- Single tile PNG export (scaled)
- PIXL source export
- Atlas packing (`/api/atlas/pack`)
- GIF animation export
- Tiled + Godot JSON export

**Total MVP: ~10 weeks solo or ~6 weeks with two devs (Flutter + Go parallel)**

---

## Open Questions for MVP

1. **Backend process model**: Embed Go renderer in a shared library (CGo FFI to Flutter) vs. sidecar HTTP process? Sidecar is simpler to develop, FFI is tighter. Recommend sidecar for MVP.

2. **API key UX**: Where does the user enter their Anthropic API key? Settings screen, first-run wizard, or env variable? First-run wizard is most user-friendly.

3. **Canvas size at session start**: Should changing canvas size mid-session be allowed (with crop/scale dialog)? Or is it locked at project creation? Locked for MVP simplicity.

4. **Knowledge base curation**: Who authors and maintains the pixel art knowledge base document? This is a one-time 4–6 hour investment and is a significant quality lever. Should be authored by Lisa + a pixel art practitioner.

5. **Offline generation fallback**: If no API key (or offline), should there be a template-based generation fallback? For MVP: no — just show a clear "configure API key" prompt.

---

## Success Metrics (MVP)

- Time to generate a complete 13-tile dungeon wall autotile set: < 5 minutes
- % of generated tiles passing WFC validation on first generation: > 80%
- Palette compliance on all generated tiles: 100% (enforced by prompt constraints)
- Artist accepts AI generation without manual edits: > 50% of cases (target)
