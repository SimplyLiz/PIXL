# Drawing Tools

## Tool Strip

The vertical tool strip on the left shows available tools. In **pixel mode** you get drawing tools; in **tilemap mode** you get tile painting tools.

## Pixel Mode Tools

| Tool | Shortcut | Description |
|------|----------|-------------|
| Pencil | `B` | Draw pixels one at a time. Click or drag. |
| Eraser | `E` | Clear pixels (set to transparent). Click or drag. |
| Fill | `G` | Flood fill a contiguous region with the foreground color. |
| Eyedropper | `I` | Pick a color from the canvas and set it as foreground. |
| Line | `L` | Draw a straight line. Click start point, drag to end, release. Preview shown during drag. |
| Rectangle | `R` | Draw a rectangle outline. Hold **Shift** for filled rectangle. Click corner, drag to opposite corner. |
| Select | `S` | Drag to select a rectangular region. See [Selection](#selection) below. |
| Move | — | Move content (stub — not yet fully implemented). |

### Symmetry

The symmetry toggle in the tool strip cycles through:
- **Off** — normal drawing
- **H** — horizontal mirror (left/right)
- **V** — vertical mirror (top/bottom)
- **H+V** — four-way symmetry

All drawing tools (pencil, eraser, line, rect, fill) respect the active symmetry mode.

## Selection

Select a region with the **Select tool (S)**, then use:

| Shortcut | Action |
|----------|--------|
| `Cmd+C` | Copy selected pixels to clipboard |
| `Cmd+X` | Cut (copy + clear) |
| `Cmd+V` | Paste clipboard at selection origin |
| `Delete` / `Backspace` | Clear selected pixels |
| `Escape` | Deselect |

The selection is shown as a cyan rectangle overlay on the canvas.

## Tilemap Mode Tools

| Tool | Shortcut | Description |
|------|----------|-------------|
| Stamp | `T` | Place the selected tile on the grid. Click or drag to paint. |
| Eraser | `E` | Clear grid cells (remove tile). |
| Fill | `G` | Flood fill a region with the selected tile. |
| Eyedropper | `I` | Pick a tile from the grid and set it as the active brush. |

## Canvas Controls

| Shortcut | Action |
|----------|--------|
| `Space` + drag | Pan the canvas |
| Scroll wheel | Zoom in/out |
| `+` / `-` | Zoom in/out |
| `H` | Toggle grid overlay |
| `Cmd+Z` | Undo |
| `Cmd+Shift+Z` | Redo |
| `Cmd+S` | Quick save |
| `Cmd+/` | Show shortcuts dialog |

## Color Picker

Click the edit icon next to the foreground color hex value to open the **HSV Color Picker**:

- **SV square**: drag to set saturation (horizontal) and value (vertical)
- **Hue bar**: drag to set hue
- **RGB sliders**: fine-tune individual channels
- **Hex input**: type a 6-digit hex value
- **Preview**: old vs new color comparison

## Blueprint Overlay

Click the person icon in the top bar to toggle the **blueprint overlay** — semi-transparent cyan guides showing anatomical landmarks (head, torso, limbs, eyes) for character sprite creation. The blueprint adapts to the current canvas size.

## Reference Image

Load a reference image as a semi-transparent underlay on the canvas for tracing. The image is scaled to fit the canvas dimensions and rendered below your pixel layers.
