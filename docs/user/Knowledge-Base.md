# Knowledge Base

PIXL ships with a curated library of pixel art techniques. When the AI generates a tile, it doesn't guess — it pulls in the relevant craft knowledge for the specific task.

## What's in the knowledge base

30+ documents covering the fundamentals of pixel art, organized into searchable topics:

- **Color theory** — palette design, color ramps, warm/cool balance
- **Dithering** — Bayer patterns, checkerboard, gradient approximation
- **Shading** — light direction, shadow placement, cel shading
- **Outlines** — when to use them, thickness, color selection
- **Animation** — frame timing, squash and stretch, walk cycles
- **Tiling** — seamless edges, transition tiles, autotiling rules
- **Hardware constraints** — NES, SNES, Game Boy, GBA limitations
- **Sprite design** — silhouettes, readability at small sizes, chibi proportions

## How it works

When you ask PIXL to generate a "dungeon wall tile," the knowledge base:

1. **Searches** for relevant passages about walls, dungeon aesthetics, tiling rules, and shadow placement
2. **Expands the search** using a concept graph — "dungeon wall" connects to stone materials, mortar patterns, WFC edge compatibility
3. **Ranks results** by relevance and injects the top passages into the AI's context
4. **Positions knowledge strategically** — the most relevant passage goes first, second-most goes last (where LLMs pay the most attention)

The result: the AI knows that stone walls need mortar line dithering, that shadows go bottom-right, and that WFC edges must be solid on wall tiles — before it draws a single pixel.

## 1,300+ cross-referenced concepts

The knowledge base isn't a flat list of documents. It includes a knowledge graph with 1,300+ concepts linked by relationships:

- "dithering" → connects to "Bayer matrix", "color reduction", "gradient approximation"
- "NES palette" → connects to "4 colors per sprite", "background palette sharing"
- "walk cycle" → connects to "frame count", "bob height", "arm swing"

When you search for one concept, related concepts are automatically discovered and included.

## Expandable

The knowledge base can be extended with your own documents. Add a markdown file about your game's art style, specific technique notes, or reference material — and it gets indexed alongside the built-in knowledge.
