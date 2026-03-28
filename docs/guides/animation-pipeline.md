# Animation Pipeline Guide

How PIXL handles sprite animation, color cycling, and backdrop effects
end-to-end.

---

## 1. Sprite Animation (`animate.rs`)

The `pixl-core/src/animate.rs` module resolves raw spriteset frames into
fully materialized pixel grids. This is the core library function used by
both the CLI (`pixl render-sprite`) and the MCP server.

### Frame Encoding Types

| Encoding | How it works | TOML field |
|----------|-------------|------------|
| `grid` | Full character grid per frame | `grid = '''...'''` |
| `delta` | Copy a base frame, apply pixel changes | `base = 1`, `changes = [{x, y, sym}]` |
| `linked` | Alias to another frame (zero-copy) | `link_to = 1` |

### Mirror Support

Frames can be flipped without storing duplicate data:

| Mirror | Effect |
|--------|--------|
| `"h"` | Flip horizontal (walk-left from walk-right) |
| `"v"` | Flip vertical |
| `"hv"` | Flip both (180 rotation) |

### API

```rust
use pixl_core::animate;

// Resolve all frames of a sprite
let frames = animate::resolve_sprite_frames(
    &sprite,       // SpriteRaw
    width, height, // sprite dimensions
    &palette,      // Palette
    sprite.fps,    // default FPS
)?;

// Apply color cycling at a given tick
let cycled = animate::resolve_frames_with_cycles(
    &frames,
    &cycle_refs,   // Vec<&Cycle>
    &palette,
    tick,          // u64
);
```

### CLI Usage

```bash
# Animated GIF (respects per-frame duration + loop mode)
pixl render-sprite game.pax --spriteset hero --sprite walk_right -o walk.gif --scale 4

# Horizontal spritesheet PNG
pixl render-sprite game.pax --spriteset hero --sprite idle -o idle.png --scale 4
```

### Sprite Scale (Neo Geo / Super Scaler)

Sprites can define a `scale` factor for distance-based sizing:

```toml
[[spriteset.enemies.sprite]]
name = "goblin_far"
fps = 8
scale = 0.5     # render at half size (nearest-neighbor)
```

---

## 2. Color Cycling (`cycle.rs`)

Color cycling rotates palette entries at runtime — the classic technique for
water shimmer, lava flow, and fire without storing animation frames.

### How It Works

A cycle defines a list of symbols whose colors rotate:

```toml
[cycle.water_shimmer]
palette   = "dungeon"
symbols   = ["~", "h", "+"]
direction = "forward"    # "forward" | "backward" | "ping-pong"
fps       = 8
```

At each tick, symbol `~` takes the color of `h`, `h` takes `+`, `+` takes `~`.
The pixel grid never changes — only the palette-to-color mapping shifts.

### Integration Points

| Context | How cycles apply |
|---------|-----------------|
| **Tiles** | `cycles = ["water_shimmer"]` on `[tile.*]` |
| **Spritesets** | `cycles = ["torch_flicker"]` on `[spriteset.*]` |
| **Backdrop zones** | `behavior = "cycle"` with `cycle = "water_shimmer"` |
| **Tilemap layers** | `cycles = ["lightning"]` on `[tilemap.*.layer.*]` |

### API

```rust
use pixl_core::cycle;

// Get effective color for a symbol at a given tick
let color = cycle::cycle_color_at_frame('~', &cycle, &palette, tick);
```

---

## 3. Backdrop Animation (10 Zone Behaviors)

Backdrop zones define rectangular regions with procedural animation effects.
The engine generates frames at runtime — no stored frames.

### Behavior Reference

| Behavior | Source Hardware | Visual Effect |
|----------|---------------|---------------|
| `cycle` | General | Rotate palette colors uniformly |
| `wave` | SNES HDMA | Cycle with per-row phase offset (water reflections) |
| `flicker` | General | Random subset of cycle pixels active (fire, torches) |
| `scroll_down` | General | Shift pixels downward with wrap (waterfalls) |
| `hscroll_sine` | SNES HDMA | Per-scanline horizontal sine distortion (heat haze) |
| `vscroll_sine` | Genesis VSRAM | Per-column vertical sine distortion (waterfall columns) |
| `color_gradient` | General | Per-pixel tint interpolation (atmospheric perspective) |
| `palette_ramp` | Konami raster | Per-scanline palette entry replacement (sky gradients) |
| `mosaic` | GBA MOSAIC | Pixelation with independent X/Y block sizes |
| `window` | GBA WIN0/WIN1 | Layer visibility control within a rectangle |

### Global Animation Clock (Neo Geo Auto-Animation)

Instead of per-tile animation arrays, tiles can opt into a shared clock:

```toml
[anim_clock.water]
fps = 6
frames = 4
mode = "loop"

[backdrop_tile.water_a]
anim_clock = "water"   # cycles water_a_0 → water_a_1 → water_a_2 → water_a_3
```

All tiles sharing a clock stay perfectly synchronized. Companion tiles must
be named `{base}_0` through `{base}_{frames-1}`.

---

## 4. Core Tilemaps (`tilemap.rs`)

The `pixl-core/src/tilemap.rs` module implements PAX spec section 10:
multi-layer game tilemaps with z-ordering, collision, and WFC constraints.

### Layer Properties

| Field | Type | Description |
|-------|------|-------------|
| `z_order` | int | Render order (lower = behind) |
| `blend` | string | Blend mode (normal, additive, multiply, screen) |
| `collision` | bool | Layer participates in physics |
| `collision_mode` | string | `"full"` or `"top_only"` (one-way platforms) |
| `layer_role` | string | `"background"`, `"platform"`, `"foreground"`, `"effects"` |
| `cycles` | [string] | Layer-wide color cycling |
| `scroll_factor` | float | Parallax (0.0=far, 1.0=near) |

### Tile References in Grids

Tilemap grids use whitespace-separated tile names with optional flip flags
and brightness modifiers:

```
floor_stone           # plain
floor_stone!h         # flip horizontal
floor_stone!hv        # 180° rotation
wall_corner!dh        # 90° clockwise
floor_stone:shadow    # Genesis shadow (halve RGB)
floor_stone:highlight # Genesis highlight (halve + midpoint)
wall!h:shadow         # flip + shadow combined
.                     # empty cell
```

### WFC Constraint Painting

```toml
[tilemap.dungeon.constraints]

[[tilemap.dungeon.constraints.pins]]
x = 0, y = 0, to_x = 19, to_y = 0
tile = "wall_solid"

[[tilemap.dungeon.constraints.paths]]
from = { x = 0, y = 7 }
to   = { x = 19, y = 7 }
```

---

## 5. The Convert + Backdrop Pipeline

End-to-end workflow from AI-generated image to animated PAX backdrop:

```
AI Image (1024x1536)
    │
    ▼
pixl convert image.png          → pixl_convert/medium/image.png (160x240, 32 colors)
    │
    ▼
pixl backdrop-import image.png  → scene.pax (tile-decomposed, 55KB)
    │                             ├── [palette.*] + [palette_ext.*]
    │                             ├── [backdrop_tile.*] × ~150 unique tiles
    │                             ├── [backdrop.scene] with tilemap
    │                             └── suggested animation zones (commented)
    ▼
Edit scene.pax                  → add [cycle.*] + [[backdrop.scene.zone]] entries
    │
    ▼
pixl backdrop-render scene.pax  → static.png or animated.gif
```

### Pixelize Internals (`pixelize.rs`)

The `pixl-render/src/pixelize.rs` module handles:

1. **Lanczos3 downsampling** — high-quality resize to target resolution
2. **Median-cut quantization** — reduce to N colors with perceptual weighting
3. **Palette splitting** — top 16 colors → single-char symbols, rest → multi-char (`2a`-`4z`)
4. **Tile slicing** — divide image into 16x16 blocks
5. **Deduplication** — FNV hash comparison, identical tiles share one definition
6. **RLE encoding** — colon separator for multi-char symbols (`1:2a`, `5:2f`)
7. **PAX generation** — writes complete `.pax` file with palette, tiles, tilemap
