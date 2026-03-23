---
title: "Pixel Art Tutorial: Basics"
source: "https://www.derekyu.com/makegames/pixelart.html"
topic: "fundamentals"
fetched: "2026-03-23"
---

# Pixel Art Tutorial: Basics

By Derek Yu

## Core Definition

Pixel art, known as "dot art" in Japan, involves editing at the pixel level. It emphasizes creating vibrant artwork within tight constraints — much like how master brushstrokes convey emotion, individual pixels combine to meaningful effect.

## Fundamental Techniques

### Jaggies

Single pixels or small segments breaking line consistency create visual discord. The objective involves minimizing rather than eliminating jaggies entirely. Straight lines require single-pixel thickness; curved lines need consistent segment growth/shrinkage.

**Key rule**: When drawing curves, the length of line segments should change gradually and consistently. A sequence like 5-3-2-1-1-2-3-5 reads smoothly; 5-1-3-2 creates visible jaggies.

### Form and Volume

Conceptualize artwork as three-dimensional forms with clay-like properties. Shading sculpts volume rather than merely adding color. Well-defined characters maintain distinguishable large light/dark clusters when squinting.

### Anti-Aliasing (AA)

"In-between" colors soften blocky intersections where line segments meet. AA length corresponds to line segment length.

**Critical warning**: Avoid AA on sprite exteriors when background colors are unknown — it creates visible artifacts against unfamiliar backgrounds. This is especially important for game sprites that will be composited over varying backgrounds.

### Selective Outlining (Selout)

Replace pure black outlines with lighter colors toward light-struck areas, removing outlines entirely where sprites meet negative space. Deploy darker shadow colors for segmentation details (musculature, texture). This technique produces naturalistic appearance while reducing harsh segmentation.

### Dithering

Bridges color shades without introducing new hues through varying density noise patterns, similar to halftone printing or stippling. Creates texture through controlled randomness rather than solid color blocks. Most effective on expansive single-color areas or deliberately rough surfaces.

## Sprite Creation Workflow

### 96x96 Sprites

1. Create crude outline using pencil tool
2. Clean outline, reduce lines to single-pixel thickness
3. Apply base colors using paint bucket
4. Add shading simulating light source positioning
5. Implement anti-aliasing with shadow introduction
6. Apply selective outlining for naturalism
7. Add highlights, details, final refinements
8. Verify through horizontal flipping and desaturation testing

### 32x32 Sprites

Employ colored shapes rather than outlines initially. Individual pixels carry greater responsibility. Chibi (super-deformed) designs excel within limited space constraints through large heads and expressive features. Color, selective outlining, and anti-aliasing create perceived canvas expansion.

## Color Palette Considerations

Palettes define artistic style; 16 and 32-color palettes are popular standards. Beginning artists should select existing palettes rather than theorizing — pixel art allows straightforward palette swapping at any stage.

## File Format Guidance

**Critical**: Never use JPG format — lossy compression destroys crisp edges and palette integrity. PNG serves as the lossless standard for static work; animated GIFs handle animations.

## Development Perspective

Pixel art at professional quality demands extensive time investment. Single static sprites represent small components within complex game systems; maintaining broader project perspective prevents excessive micro-refinement on individual assets.
