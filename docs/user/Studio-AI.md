# AI Chat & Generation

The left panel in Studio connects to an AI assistant for tile generation, quality feedback, and style guidance. Works with Claude, GPT, or any LLM provider.

## Setting up

Open **Settings** (gear icon in the top bar) and configure your AI provider:

- **Anthropic** — paste your Claude API key
- **OpenAI** — paste your GPT API key
- **Custom** — any OpenAI-compatible endpoint

The chat panel activates once a provider is configured.

## What you can ask

### Generate tiles

> "Create a 16×16 dungeon wall tile with stone texture"

The AI generates a PAX character grid, creates the tile in your session, and shows a rendered preview. It uses the active palette and theme constraints automatically.

### Generate sprites from images

> "Generate a wizard sprite with a purple hat and staff"

Uses the diffusion bridge (DALL-E) to create a reference image, then converts it to clean pixel art with your palette. See [Sprite Generation](./Sprite-Generation) for details.

### Critique your art

> "Check the quality of my wall tile"

Runs structural validators — outline coverage, centering, contrast, fragmentation — and returns specific fix instructions with row numbers.

### Refine tiles

> "Fix the outline on rows 3-5 of the wizard tile"

Patches specific rows of the tile grid and re-renders. The AI sees the updated preview and can continue refining.

### Create variations

> "Make 4 variants of the floor tile with cracks and moss"

Generates controlled mutations of an existing tile while preserving edge compatibility for WFC.

### Build maps

> "Generate a 12×8 dungeon with a boss room in the southeast"

Creates a WFC map from spatial rules and renders it as a preview.

## The SELF-REFINE loop

The AI follows a generate-see-fix cycle:

1. **Generate** a tile
2. **See** the rendered result (the AI receives the preview image)
3. **Critique** — run structural checks automatically
4. **Fix** specific rows if issues are found
5. **Re-check** until the tile passes

This loop runs automatically during generation. You see the final result after quality checks have passed.

## Style consistency

When you accept or reject tiles, the AI learns your preferences:

- Accepted tiles → style profile updated (light direction, density, palette usage)
- Rejected tiles → constraints recorded ("too sparse", "bad edges")
- Next generation → pre-filtered against your style profile

After a few rounds, the AI produces tiles that match your established look without being told.

## Reference images

Before generating, the AI can look at your existing tiles as rendered images — not just text grids. It matches the proportions, shading, and detail level of your existing work.

## Using without Studio

The same AI tools are available through the MCP server for Claude Code or any MCP client:

```bash
pixl mcp --file tileset.pax
```

See [MCP Tools](./MCP-Tools) for the full tool catalog.
