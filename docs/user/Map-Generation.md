# Map Generation

Draw a few tiles and PIXL generates entire maps that tile seamlessly. Describe a scene in plain language and the engine fills in the rest.

## Wave Function Collapse

PIXL uses Wave Function Collapse (WFC) — an algorithm that looks at the edges of your tiles and figures out which ones can go next to each other. You define the pieces, PIXL assembles the puzzle.

Every tile has edge classes that describe what its borders look like:

```toml
[tile.wall_solid]
edge_class = { n = "solid", e = "solid", s = "floor", w = "solid" }
```

Two tiles can sit next to each other only if their touching edges match. PIXL enforces this automatically.

## Describe scenes in plain language

Instead of placing tiles by hand, describe what you want:

```bash
pixl narrate tileset.pax --width 12 --height 8 \
  -r "border:wall_solid" \
  -r "region:boss_room:floor_stone:3x3:southeast" \
  -r "path:0,3:11,3"
```

This generates a 12×8 map with walls around the border, a boss room in the southeast corner, and a corridor connecting left to right.

## Semantic rules

Beyond geometric edge matching, you can define logical rules:

- **Forbids** — "a wall tile can never be directly above a water tile"
- **Requires** — "moss only appears adjacent to floor tiles"

These produce game-sensible maps, not just geometrically valid ones.

## Auto-edge classification

Don't want to label every edge by hand? PIXL can auto-classify edges from the pixel content — solid borders, open borders, and pattern-matched edges are detected automatically.

```bash
pixl check tileset.pax --fix
```

## Completeness analysis

PIXL tells you which transition tiles are missing from your set:

```bash
pixl validate tileset.pax --completeness
```

If you have "wall" and "floor" tiles but no "wall-to-floor" transition, PIXL flags it and suggests what edge classes the missing tile needs.
