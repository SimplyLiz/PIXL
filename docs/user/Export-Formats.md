# Export Formats

PIXL exports to every major game engine and level editor.

## Supported formats

| Format | Engine | What you get |
|--------|--------|-------------|
| **Tiled** | Tiled map editor | `.tsj` tileset + `.tmj` tilemaps + PNG atlas |
| **Godot** | Godot Engine | TileMap resources + PNG |
| **Unity** | Unity | Tile palette data + PNG atlas |
| **TexturePacker** | 48+ engines | JSON Hash metadata + PNG atlas |
| **GB Studio** | GB Studio | Game Boy compatible export |
| **PNG** | Any | Individual tile PNGs at any scale |
| **GIF** | Any | Animated sprites with frame timing |

## CLI export

```bash
# Export everything to Tiled format
pixl export tileset.pax --format tiled --out ./export/

# Export to Godot
pixl export tileset.pax --format godot --out ./export/

# Just the atlas
pixl atlas tileset.pax --out atlas.png --map atlas.json --scale 2
```

## Tiled export

The Tiled export produces:

- **`tileset.tsj`** — Tiled tileset JSON with collision shapes and `pax_name` properties
- **`tileset.png`** — packed atlas image
- **`{tilemap_name}.tmj`** — one Tiled map per `[tilemap.*]` section in your PAX file

### Multi-layer depth sorting

When your PAX file defines multi-tile objects with `above_player_rows` / `below_player_rows`, the tilemap export generates 3 depth layers instead of 1:

| Layer | Purpose |
|-------|---------|
| `below_player` | Object rows below the player (trunks, building bases) |
| `terrain` | Ground tiles + base_tile fill under objects |
| `above_player` | Object rows above the player (canopy, rooftops) |

This gives you the standard top-down depth setup in any engine that imports Tiled maps. Your player/entity layer goes between `below_player` and `above_player` in the engine's draw order.

```toml
# PAX example — cottage with depth splitting
[object.cottage]
size_tiles        = "3x4"
base_tile         = "grass_plain"
above_player_rows = [0, 1]        # roof renders in front of player
below_player_rows = [2, 3]        # base renders behind player

tiles = '''
roof_l      roof_c      roof_r
wall_win_l  wall_door   wall_win_r
wall_base_l wall_base_c wall_base_r
shadow_l    shadow_c    shadow_r
'''

# Place it in a tilemap
[[tilemap.village.objects]]
object = "cottage"
x = 5
y = 3
```

Objects with neither list specified default all rows to `above_player`.

## TexturePacker JSON Hash

The atlas JSON uses the TexturePacker JSON Hash format — the most widely supported sprite atlas standard. It works out of the box with:

Phaser, Pixi.js, libGDX, Cocos2d-x, HaxeFlixel, MonoGame, Bevy, Macroquad, LÖVE, Defold, and dozens more.

```json
{
  "frames": {
    "wall_solid": {
      "frame": { "x": 1, "y": 1, "w": 32, "h": 32 },
      "sourceSize": { "w": 32, "h": 32 },
      "pivot": { "x": 0.5, "y": 0.5 }
    }
  },
  "meta": {
    "app": "pixl",
    "image": "atlas.png",
    "format": "RGBA8888"
  }
}
```

## Composite atlas

Composite sprites get their own atlas file (`atlas_composites.png`) with every variant and animation frame packed as separate entries:

```
warrior          — base layout
warrior:shield   — shield variant
warrior:walk:1   — walk frame 1
warrior:walk:2   — walk frame 2
```

## 9-slice support

Tiles with 9-slice borders include the border data in the atlas JSON, so your UI framework can stretch them correctly.
