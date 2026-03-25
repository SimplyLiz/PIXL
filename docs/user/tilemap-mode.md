# Tilemap Mode

Tilemap mode lets you paint a 2D map using tiles from your session — like Tiled or LDTK, but integrated with PIXL's AI generation.

## Switching Modes

Click the **Pixel / Tilemap** toggle in the top bar. The center viewport switches between the pixel canvas and the tilemap grid.

## Tilemap Canvas

The tilemap is a grid of cells, each containing a tile reference (or empty). Empty cells show a checkerboard pattern. Placed tiles render as their actual preview images.

### Tools

| Tool | Shortcut | Description |
|------|----------|-------------|
| Stamp | `T` | Paint the selected tile onto grid cells. Click or drag. |
| Eraser | `E` | Clear cells (remove tile). Click or drag. |
| Fill | `G` | Flood fill empty/matching cells with the selected tile. |
| Eyedropper | `I` | Pick a tile from a cell and set it as active brush. |

### Controls

| Action | How |
|--------|-----|
| Pan | `Space` + drag |
| Zoom | Scroll wheel |
| Toggle grid | `H` |
| Undo | `Cmd+Z` |
| Redo | `Cmd+Shift+Z` |

### Map Size

Set the grid dimensions in the **Palette tab** (right panel) when in tilemap mode. Width and height are configurable from 2 to 64 tiles.

## Selecting a Tile Brush

Click a tile in the **Tile Picker** strip at the bottom of the canvas. In tilemap mode, clicking a tile selects it as the active stamp brush (highlighted with a primary-colored border).

## Loading from WFC

After generating a map with the **WFC Map** dialog:
1. Click **"Load into Tilemap"** in the WFC result
2. The generated tile grid is loaded into the tilemap canvas
3. Studio switches to tilemap mode automatically
4. You can then edit the map — fix individual cells, add details, swap tiles

## Workflow

A typical workflow:

1. **Generate tiles** via chat: "generate a dungeon tileset with walls, floors, and corners"
2. Switch to **Tilemap mode**
3. Select a floor tile → stamp it across the map
4. Select wall tiles → paint the borders
5. Use **Fill** to flood-fill large regions
6. Add details with specific tiles (torches, cracks, moss)
7. Export via the Export menu

## Export

In tilemap mode, the Export menu includes game engine export options:
- **Tiled** (.tmx + .tsx)
- **Godot** (tileset resource)
- **TexturePacker** (JSON + spritesheet)
- **GB Studio** (tilemap format)
- **Unity** (tilemap asset)

Select a format, pick an output directory, and the engine writes the files.
