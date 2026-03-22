# Blueprint System Research

## Core Insight

"Build your sprites around the eyes." Professional pixel artists work from
anatomy grids with explicit ratios. These are exact and size-dependent.

## Anatomy Ratios (Pixel Art Standard)

- Eyes halfway between head top and chin
- Face divided into 3 equal parts: hairline → eyes → nose → chin
- Distance between eyes = one eye width
- Mouth width = distance between pupils
- Chibi (game characters): 6-head proportions
- Realistic (large sprites): 8-head proportions

## Size-Dependent Rules

| Canvas | Eye | Pupil | Facial Features |
|--------|-----|-------|-----------------|
| 8x8    | N/A | N/A   | None possible   |
| 16x16  | N/A | N/A   | Color region    |
| 16x32  | 2x1 | No    | Eyes only       |
| 24x32  | 2x2 | 1x1   | Eyes + pupil    |
| 32x48  | 3x3 | 1x1   | Full face       |
| 32x64  | 4x4 | 2x2   | Detailed face   |

## Implementation

Lives in pixl-core as queryable data structures.
Blueprint::resolve(w, h) → pixel-coordinate landmarks
Blueprint::render_guide(w, h) → text map for any consumer

Consumers: MCP server, CLI, PIXL Studio, fine-tuned models.
NOT baked into prompt templates — single source of truth in core.

## Status: V1 Phase 1 (core data structure, no rendering dependency)
