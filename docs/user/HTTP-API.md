# HTTP API

PIXL exposes a REST API for Studio and external integrations. Start it with:

```bash
pixl serve --port 3742 --file tileset.pax
```

All endpoints accept and return JSON. Image data is base64-encoded PNG.

## Discovery

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Returns `"pixl ok"` |
| POST | `/api/session` | Session info: theme, palette, stamps, tiles |
| POST | `/api/palette` | Full symbol table with hex colors |
| GET | `/api/themes` | Available themes |
| GET | `/api/stamps` | Available stamps with sizes |
| GET | `/api/tiles` | All tiles with edge classes and tags |
| GET | `/api/file` | Full .pax TOML source |

## Tile operations

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/tile/create` | Create tile from grid. Returns preview + edges. |
| POST | `/api/tile/render` | Render tile to base64 PNG |
| POST | `/api/tile/delete` | Remove tile from session |
| POST | `/api/tile/edge-check` | Test edge compatibility between two tiles |
| POST | `/api/tile/vary` | Generate tile variants |

## Generation

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/tile/generate-sprite` | DALL-E → quantize → PAX tile |
| POST | `/api/tile/upscale` | Nearest-neighbor grid upscale |
| POST | `/api/tile/references` | Render matching tiles as visual examples |
| POST | `/api/generate/context` | Build AI generation prompt with examples |
| POST | `/api/generate/tile` | Generate tile via local LoRA model |

## Quality

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/tile/critique` | Structural analysis + preview + fix instructions |
| POST | `/api/tile/refine` | Patch specific rows, re-critique |
| POST | `/api/validate` | Validate .pax file |

## Style & feedback

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/style/learn` | Extract style fingerprint from tiles |
| POST | `/api/style/check` | Score tile against session style |
| POST | `/api/feedback` | Record accept/reject feedback |
| GET | `/api/feedback/stats` | Feedback statistics |
| GET | `/api/feedback/constraints` | Learned constraints from feedback |

## Maps & composites

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/narrate` | WFC map generation from rules |
| GET | `/api/composites` | List composites with variants and animations |
| POST | `/api/composite/render` | Render composite sprite |
| GET | `/api/composite/check-seams` | Seam continuity warnings |
| POST | `/api/blueprint` | Character anatomy landmarks |

## Sprites & export

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/sprite/gif` | Animated GIF from spriteset |
| POST | `/api/atlas/pack` | Pack sprite atlas + JSON metadata |
| POST | `/api/export` | Export to game engine format |
| POST | `/api/convert` | AI image → pixel art conversion |
| POST | `/api/backdrop/import` | Image → PAX backdrop |
| POST | `/api/backdrop/render` | Render backdrop scene |

## Other

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/load` | Load .pax source string into session |
| POST | `/api/new` | Create from built-in theme template |
| GET | `/api/check/completeness` | WFC tileset completeness analysis |
| POST | `/api/tile/generate-transition` | AI prompt for missing transition tile |
| POST | `/api/training/export` | Export training data as JSONL |
| GET | `/api/training/stats` | Training data statistics |

## Generic tool endpoint

Any MCP tool can also be called via the generic endpoint:

```
POST /api/tool
{
  "tool": "pixl_critique_tile",
  "args": { "name": "wizard", "scale": 16 }
}
```

This is useful for tools that don't have a dedicated route.

## Response format

All endpoints return JSON. Image fields:

- `preview_b64` — base64 PNG (tile previews)
- `reference_b64` — base64 PNG (DALL-E reference image)
- `atlas_b64` — base64 PNG (packed atlas)
- `gif_b64` — base64 GIF (animated sprites)

Error responses include an `error` field with a human-readable message.
