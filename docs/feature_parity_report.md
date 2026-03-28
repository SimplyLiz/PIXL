# Feature Parity Report

**Generated:** 2026-03-28
**Scope:** Every PAX engine feature vs its exposure across CLI, MCP, HTTP, and Studio.

Legend: **yes** = implemented, **--** = not exposed, **partial** = exists but incomplete

---

## 1. Tile CRUD

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Create tile | yes | -- | yes | yes | yes |
| Render tile | yes | yes | yes | yes | yes |
| Delete tile | yes | -- | yes | yes | yes |
| List tiles | yes | -- | yes | yes | yes |
| Validate | yes | yes | yes | yes | yes |
| Edge check | yes | yes | yes | yes | yes |
| Completeness analysis | yes | yes | yes | yes | yes |
| Check/fix edges | yes | yes | auto | auto | auto |
| Preview (16x zoom) | yes | yes | yes | yes | yes |
| Vary tile (mutations) | yes | yes | yes | yes | -- |
| Generate transition context | yes | -- | yes | yes | yes |

**Skipped (deliberate):**
- Create/delete/list tiles have no CLI commands — tiles are authored in `.pax` text files, not via imperative CLI. The CLI is file-oriented.
- Vary tile has no Studio button — it's an AI-assisted workflow triggered via chat.

---

## 2. Palette, Theme & Color

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Palette symbols | yes | implicit | yes | yes | yes (palette tab) |
| Extended palette (17-48 colors) | yes | implicit | implicit | implicit | -- |
| Palette swap | yes | implicit | implicit | implicit | -- |
| Color cycling (cycle) | yes | implicit | implicit | implicit | partial (name only) |
| Theme selection | yes | yes | yes | yes | yes (new project) |
| Theme roles | yes | implicit | yes | yes | yes (palette tab) |
| Theme constraints | yes | yes (validate) | yes (validate) | yes | -- |
| Color profile (sRGB/linear) | yes | implicit | implicit | implicit | -- |

**Skipped (deliberate):**
- Extended palette, palette swap, and color profile are format-level metadata. They work in the engine and render correctly. Editing them means editing the `.pax` file. No visual editor needed for V1.

**TODO:**
- Cycle editing (fps, direction, symbols) — the zone property panel only lets you pick a cycle by name, not create/edit cycles. Medium priority.

---

## 3. Stamps & Compose

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Stamp definitions | yes | -- | yes (list) | yes (list) | yes (stamp picker) |
| Tile compose mode | yes | implicit | implicit | implicit | -- |
| Generate procedural stamps | yes | yes | -- | -- | -- |

**Skipped (deliberate):**
- `generate-stamps` is a CLI batch tool that emits TOML to stdout. Not interactive.
- Stamp CRUD — stamps are declared in `.pax`. No visual stamp editor for V1.

---

## 4. Sprites & Animation

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Spriteset definition | yes | -- | -- | -- | -- |
| Frame resolution (grid/delta/linked/mirror) | yes | yes | yes | yes | yes |
| Render sprite GIF | yes | yes | yes | yes | yes (preview dialog) |
| Sprite scale (Neo Geo) | yes | implicit | implicit | implicit | -- |
| Animation tags | yes | implicit | implicit | implicit | -- |
| Frame-based tile animation | yes | implicit | implicit | implicit | -- |

**Skipped (deliberate):**
- Spriteset CRUD — sprites are authored in `.pax`. Visual sprite editing is a V2 feature (skeletal animation system planned).
- Sprite scale — applied during rendering, no UI control needed yet.

---

## 5. Tilemaps (Core)

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Tilemap types | yes | -- | -- | -- | partial (viewport stub) |
| Tilemap layers (z_order, blend, collision) | yes | -- | -- | -- | -- |
| Layer roles (background/platform/foreground/effects) | yes | -- | -- | -- | -- |
| Collision modes (full/top_only) | yes | -- | -- | -- | -- |
| WFC constraint painting (pins/zones/paths) | yes | yes (narrate) | yes | yes | yes (WFC dialog) |
| Object placement | yes | -- | -- | -- | -- |
| Tile run groups (left/middle/right) | yes | -- | -- | -- | -- |

**Skipped (scope):**
- Tilemap editing is the biggest remaining gap. The types exist, WFC generates maps, but there's no API to create/edit/save tilemaps directly. The Studio tilemap viewport has painting tools but no backend integration for tilemap persistence.
- Object placement and tile runs are parsed from `.pax` but have no exposure anywhere. Future feature.

---

## 6. Backdrop System

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Backdrop import (image → PAX) | yes | yes | yes | yes | yes (dialog + editor) |
| Backdrop render (static) | yes | yes | yes | yes | yes (viewport) |
| Backdrop render (animated) | yes | yes | yes | yes | yes (play/pause/export) |
| Multi-layer support | yes | yes | yes | yes | yes (layer list) |
| Layer opacity | yes | implicit | implicit | implicit | yes (slider) |
| Layer parallax (scroll_factor) | yes | implicit | implicit | implicit | yes (slider) |
| Layer blend mode | yes | implicit | implicit | implicit | yes (label) |
| Layer visibility | yes | -- | -- | -- | yes (toggle) |
| Layer fade (GBA BLDY) | yes | implicit | implicit | implicit | **--** |
| Layer scroll_lock (Genesis window) | yes | implicit | implicit | implicit | **--** |
| Layer offset_x/offset_y | yes | implicit | implicit | implicit | **--** |
| Backdrop tile (per-tile frame animation) | yes | implicit | implicit | implicit | -- |
| Global animation clock (anim_clock) | yes | implicit | implicit | implicit | -- |
| Extended RLE (colon separator) | yes | implicit | implicit | implicit | -- |

**TODO (Studio panel gaps):**
- Layer fade — need target dropdown (black/white) + amount slider
- Layer scroll_lock — need toggle + rect inputs (x, y, w, h)
- Layer offset — need x/y number fields

**Skipped (deliberate):**
- Anim clock editing — declared in `.pax`, consumed at render time. Creating clocks from the UI is a V2 feature.
- Per-tile frame animation — same, authored in TOML.

---

## 7. Zone Behaviors (10 total)

All 10 behaviors are selectable in the Studio dropdown. Parameter editors:

| Behavior | Engine | Studio Dropdown | Studio Params | Missing Params |
|----------|:---:|:---:|:---:|---|
| `cycle` | yes | yes | yes (cycle name) | -- |
| `wave` | yes | yes | yes (cycle, phase_rows) | -- |
| `flicker` | yes | yes | partial (cycle only) | **density, seed** |
| `scroll_down` | yes | yes | **--** | **speed, wrap** |
| `hscroll_sine` | yes | yes | partial (amplitude, period) | **speed** |
| `vscroll_sine` | yes | yes | partial (amplitude, period) | **speed** |
| `color_gradient` | yes | yes | **--** | **from, to, direction** |
| `palette_ramp` | yes | yes | **--** | **symbol, from, to** |
| `mosaic` | yes | yes | yes (size_x, size_y) | -- |
| `window` | yes | yes | **--** | **layers_visible, blend_override, opacity_override** |

**TODO:** Add missing parameter fields to `_ZoneProperties` in `backdrop_panel.dart`.

---

## 8. Per-Tile Modifiers

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Flip flags (!h, !v, !d) | yes | implicit | implicit | implicit | -- |
| Shadow/highlight (:shadow, :highlight) | yes | implicit | implicit | implicit | -- |
| NineSlice (left/right/top/bottom) | yes | implicit | implicit | implicit | -- |
| visual_height_extra | yes | implicit | implicit | implicit | -- |

**Skipped (scope):**
- These are per-tile-reference modifiers applied in tilemap strings. Exposing them in Studio requires a tile-level placement editor within the tilemap/backdrop viewport — effectively the tilemap painting feature. Out of scope until tilemap editing is built.

---

## 9. WFC & Map Generation

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Narrate map (predicate DSL) | yes | yes | yes | yes | yes (WFC dialog) |
| Edge class (N/E/S/W) | yes | yes | yes | yes | yes (auto on create) |
| Corner class (NE/SE/SW/NW) | yes | -- | -- | -- | -- |
| Semantic (affordance/collision) | yes | implicit | implicit | implicit | -- |
| Collision polygons | yes | -- | -- | -- | -- |
| WFC rules (forbids/requires) | yes | implicit | implicit | implicit | -- |
| Weight overrides | yes | yes (--weight) | -- | -- | -- |
| Pin overrides | yes | yes (--pin) | -- | -- | -- |
| Auto-rotate variants | yes | implicit | implicit | implicit | -- |

**TODO:**
- Weight and pin overrides not exposed in MCP/HTTP narrate. CLI has them. Minor gap.

**Skipped (deliberate):**
- Corner class — used internally by 8-neighbor WFC adjacency checks. No user-facing API needed; it works automatically when tiles define `corner_class` in the `.pax` file.
- Collision polygons — game engine concern, not editor concern for V1.

---

## 10. Import & Conversion

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Import (image → tile grid) | yes | yes | -- | -- | -- |
| Convert (AI → pixel art) | yes | yes | yes | yes | yes (dialog) |
| Backdrop import (image → tiles) | yes | yes | yes | yes | yes (dialog) |
| Corpus (batch PNG → stamps) | yes | yes | -- | -- | -- |

**Skipped (deliberate):**
- `import` is CLI-only — superseded by `convert` for most workflows. Import with specific dithering + palette quantization is a niche power-user operation.
- `corpus` is a batch offline tool for training data prep.

---

## 11. Export

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Atlas PNG + JSON | yes | yes | yes | yes | yes |
| TexturePacker format | yes | yes | yes | yes | yes |
| Tiled format | yes | yes | yes | yes | yes |
| Godot format | yes | yes | yes | yes | yes |
| Unity format | yes | -- | yes | yes | yes |
| GB Studio format | yes | -- | yes | yes | yes |
| PNG single tile | yes | yes | yes | yes | yes |
| PAX source save | yes | -- | yes | yes | yes |
| GIF sprite animation | yes | yes | yes | yes | yes |
| GIF backdrop animation | yes | yes | yes | yes | yes |
| Animation frame tags (TexturePacker) | yes | -- | -- | -- | -- |

**TODO:**
- `pack_atlas_with_tags()` exists but no caller passes spriteset data. The frame tags are defined but never populated at export time. Low priority.

---

## 12. Style & Blueprint

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Learn style (8-property fingerprint) | yes | yes | yes | yes | yes |
| Check style (score vs latent) | yes | -- | yes | yes | yes |
| Blueprint (anatomy landmarks) | yes | yes | yes | yes | yes |
| Generate context (enriched prompt) | yes | -- | yes | yes | yes |
| Generate tile (local LoRA) | yes | -- | yes | yes | yes |

**Skipped (deliberate):**
- Check style has no CLI command — it's an interactive scoring workflow.
- Generate context/tile are server-side AI workflows, not CLI batch operations.

---

## 13. Feedback & Training

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Record feedback | yes | -- | **partial** | yes | yes |
| Feedback stats | yes | -- | **partial** | yes | yes |
| Feedback constraints | yes | -- | **partial** | yes | yes |
| Export training JSONL | yes | -- | **partial** | yes | yes |
| Training stats | yes | -- | **partial** | yes | yes |

**"partial" = handler exists in dispatch table, HTTP route works, but NOT listed in `tools.rs` `tool_definitions()`.** MCP clients (Claude Desktop, Cursor) cannot discover these tools. They work if called explicitly but won't appear in tool listings.

**TODO (critical):** Add these 5 tools to `tool_definitions()` in `tools.rs`.

---

## 14. Project Management

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Project init | yes | yes | -- | -- | -- |
| Project add-world | yes | yes | -- | -- | -- |
| Project status | yes | yes | -- | -- | -- |
| Project learn-style | yes | yes | -- | -- | -- |
| New from template | yes | yes | **partial** | yes | yes |

**Skipped (deliberate):**
- `.pixlproject` management is a CLI workflow for organizing multi-world game projects. Studio doesn't need it — projects are opened via Open PAX.

**TODO:** `pixl_new_from_template` handler exists, HTTP works, but not in `tool_definitions()`.

---

## 15. Sprite Generation & Quality (NEW — 2026-03-28)

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Diffusion bridge (DALL-E) | yes | yes | yes | yes | -- |
| Auto-palette extraction | yes | yes | yes | yes | -- |
| Palette remap (OKLab) | yes | -- | yes | yes | -- |
| Pixel grid detection | yes | yes | yes | yes | -- |
| Center-sampling | yes | yes | yes | yes | -- |
| Background flood-fill removal | yes | yes | yes | yes | -- |
| AA artifact cleanup | yes | yes | yes | yes | -- |
| Outline enforcement | yes | yes | yes | yes | -- |
| Structural critique | yes | yes | yes | yes | -- |
| Refinement (row patching) | yes | -- | yes | yes | -- |
| 8→16 upscale | yes | yes | yes | yes | -- |
| Visual references | yes | -- | yes | yes | -- |

## 16. Composite Sprites (NEW — 2026-03-28)

| Feature | Engine | CLI | MCP | HTTP | Studio |
|---------|:---:|:---:|:---:|:---:|:---:|
| Composite layout | yes | yes | yes | yes | yes |
| Variants (slot overrides) | yes | yes | yes | yes | yes |
| Animation (frame swaps) | yes | yes | yes | yes | yes |
| Per-tile offsets | yes | yes | yes | yes | -- |
| Seam checking | yes | yes | yes | yes | -- |
| Composite atlas packing | yes | yes | -- | -- | -- |
| Composite rendering | yes | yes | yes | yes | yes |

**Note:** Studio composite mode has list panel, variant/animation selectors, and preview viewport. No in-viewport editing yet (tiles edited individually in pixel mode).

---

## Priority Action Items

### P0 — MCP tool discovery (9 missing tool definitions)

These handlers and HTTP routes work, but MCP clients can't discover them:

1. `pixl_record_feedback`
2. `pixl_feedback_stats`
3. `pixl_feedback_constraints`
4. `pixl_export_training`
5. `pixl_training_stats`
6. `pixl_new_from_template`
7. `pixl_check_completeness`
8. `pixl_generate_transition_context`
9. `pixl_export`

**Fix:** Add 9 entries to `tool_definitions()` in `tools.rs`.

### P1 — Studio zone parameter editors

6 zone behaviors have dropdowns but missing parameter fields:

| Behavior | Missing fields |
|----------|---------------|
| `scroll_down` | speed, wrap |
| `color_gradient` | from, to, direction |
| `palette_ramp` | symbol, from, to |
| `window` | layers_visible, blend_override, opacity_override |
| `flicker` | density, seed |
| `hscroll_sine` / `vscroll_sine` | speed |

**Fix:** Extend `_ZoneProperties` in `backdrop_panel.dart`.

### P2 — Studio layer sub-features

3 layer properties exist in engine but not in Studio panel:

| Property | What it does |
|----------|-------------|
| `fade` | GBA BLDY darken/brighten (target + amount) |
| `scroll_lock` | Genesis window plane (fixed rect) |
| `offset_x/y` | Pixel offset for layer positioning |

**Fix:** Add controls to `_LayerRow` in `backdrop_panel.dart`.

### P3 — MCP narrate weight/pin overrides

CLI has `--weight` and `--pin` flags. MCP/HTTP `pixl_narrate_map` doesn't accept them.

**Fix:** Extend the MCP handler to accept `weights` and `pins` in the args.

### Deliberately Skipped (not planned for V1)

| Feature | Why skipped |
|---------|-------------|
| Tilemap direct editing API | Tilemap painting is V2 scope. Maps are generated via WFC. |
| Object placement API | Multi-tile objects are a planned future feature. |
| Tile run exposure | Auto-tiling runs are parsed but not used in any workflow. |
| Visual sprite authoring | Skeletal animation system planned for V2. |
| NineSlice editing | UI scaling feature, not needed for game tile workflows. |
| Collision polygon editing | Game engine concern, not pixel art editor concern. |
| Anim clock visual editor | Clocks are simple (fps + frames), easier to author in TOML. |
| Extended palette editor | Power users edit `.pax` directly for 17-48 color palettes. |
| Per-tile flip/shadow editor | Requires tilemap-level tile placement UI. |
| Project management in Studio | CLI workflow for multi-world games. |
