# Drawing & Painting

Everything about pixel mode — tools, shortcuts, symmetry, color picking, and reference images.

## Tools

| Tool | Shortcut | What it does |
|------|----------|-------------|
| Pencil | `B` | Draw pixels one at a time. Click or drag. |
| Eraser | `E` | Clear pixels (set to transparent). |
| Fill | `G` | Flood fill a contiguous region. |
| Eyedropper | `I` | Pick a color from the canvas. |
| Line | `L` | Click start point, drag to end point. Preview shown while dragging. |
| Rectangle | `R` | Drag corner to corner. Hold **Shift** for filled rectangle. |
| Select | `S` | Drag to select a rectangular region. |
| Move | — | Move selected content around the canvas. |

## Selection

After selecting a region with **Select (S)**:

| Shortcut | Action |
|----------|--------|
| `Cmd+C` | Copy |
| `Cmd+X` | Cut (copy + clear) |
| `Cmd+V` | Paste at selection origin |
| `Delete` | Clear selected pixels |
| `Escape` | Deselect |

The selection appears as a cyan rectangle overlay.

## Symmetry

Cycle through symmetry modes in the tool strip:

- **Off** — normal drawing
- **H** — horizontal mirror (left/right)
- **V** — vertical mirror (top/bottom)
- **H+V** — four-way symmetry (all quadrants)

All drawing tools respect the active symmetry mode — draw on one side, the mirror updates automatically. Great for characters facing forward.

## Canvas controls

| Shortcut | Action |
|----------|--------|
| `Space` + drag | Pan the canvas |
| Scroll wheel | Zoom in/out |
| `+` / `-` | Zoom in/out |
| `Cmd+0` | Reset zoom and pan |
| `H` | Toggle grid overlay |
| `Cmd+Z` | Undo |
| `Cmd+Shift+Z` | Redo |
| `Cmd+S` | Quick save |
| `Cmd+/` | Show all shortcuts |

## Color picker

Click the edit icon next to the foreground color to open the HSV picker:

- **SV square** — drag to set saturation (horizontal) and value/brightness (vertical)
- **Hue bar** — drag to pick the base hue
- **RGB sliders** — fine-tune individual red, green, blue channels
- **Hex input** — type a 6-digit hex code directly
- **Preview** — old color vs new color side by side

## Blueprint overlay

Toggle the blueprint overlay (person icon in the top bar) to show semi-transparent anatomy guides for character sprites:

- Head, torso, limbs, feet positions
- Eye placement and size recommendations
- Adapts to the current canvas size (8×8 through 64×64)

At small sizes (8×8, 16×16), the blueprint omits features that won't be visible — no individual eyes at 8×8, just the head region.

## Reference image

Load any image as a semi-transparent underlay for tracing. The reference is scaled to fit the canvas and rendered below all pixel layers. Useful for converting a sketch or concept art into pixel art.

## Sprite animation preview

For tiles that belong to a spriteset, click **Play Animation** in the Tiles tab to preview the animation as an animated GIF in a popup. Shows all frames at the declared FPS so you can check timing without exporting.
