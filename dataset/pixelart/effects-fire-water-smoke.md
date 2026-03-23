---
title: "Pixel Art Effects: Fire, Water, Smoke, and Particles"
source: "https://lospec.com/pixel-art-tutorials/tags/fire"
topic: "effects"
fetched: "2026-03-23"
---

# Pixel Art Effects: Fire, Water, Smoke, and Particles

## Fire

### Anatomy of Pixel Art Flames

A flame consists of layers:
1. **Core** (brightest): White or bright yellow at the base
2. **Inner flame**: Yellow to orange
3. **Outer flame**: Orange to red
4. **Tips**: Dark red to transparent/background

### Animation Technique

- **Frame count**: 4–8 frames for a convincing loop
- **Movement**: Flames rise and narrow — wider at base, pointed at top
- **Flickering**: Each frame shifts the outline irregularly
- **Sub-shapes**: Break the flame into 2–3 tongues that move independently
- **Color cycling**: Shift palette indices per frame for an ambient flicker without redrawing

### Small Scale Fire (8×8 to 16×16)

- 2–3 colors maximum (yellow core, orange body, red tips)
- 3–4 frames sufficient
- Exaggerate movement — subtlety is invisible at this scale
- Consider skipping the white core entirely

### Torch/Campfire Variants

- **Torch**: Narrow base, tall flame, 1px smoke wisps above
- **Campfire**: Wide base, multiple flame tongues, embers (scattered bright pixels rising)
- **Candle**: Tiny teardrop shape, minimal flicker, 2 frames can suffice

## Water

### Still Water

- **Color ramp**: 3–4 blues, darkest at bottom
- **Highlights**: Horizontal white/light blue lines suggest reflection
- **Animation**: Slowly shift highlight positions left/right, 4–8 frame cycle
- **Transparency**: For shallow water over terrain, use dithering between water color and ground color

### Flowing Water / Waterfalls

- **Direction**: Animated pixels move in flow direction (down for waterfalls, horizontal for rivers)
- **Splash zone**: At waterfall base, white pixels scatter outward for 2–3 frames
- **Foam**: Light edge along riverbanks — 1px white with occasional gaps
- **Frame timing**: Water feels best at 100–150ms per frame (faster than character animation)

### Ocean Waves

- **Sine movement**: Rows of pixels shift left/right in a sine pattern
- **Whitecaps**: Bright pixels appear at wave peaks, dissolve after 1–2 frames
- **Parallax**: Multiple wave layers at different speeds = depth

### Pixel Art Water Tips

- Water is about **horizontal** movement; fire is about **vertical**
- Reduce palette saturation for deeper water (desaturated dark blue vs. bright surface blue)
- Reflections mirror sprites vertically, shifted 1px per row, lower saturation

## Smoke

### Smoke Behavior

- Rises and expands (opposite of fire — fire narrows, smoke widens)
- Slower than fire — longer frame durations (150–200ms)
- Lower contrast than fire — uses grays/light colors
- Gradually becomes transparent (fade to background)

### Animation Approach

1. **Birth**: Small, opaque cluster near source
2. **Rise**: Moves upward, grows wider
3. **Dissipate**: Breaks into smaller clusters, colors fade to background
4. **Vanish**: Individual pixels disappear

### Smoke Styles

- **Campfire smoke**: Thin wisps, 1–2px wide, gentle drift
- **Explosion smoke**: Rapid expansion, dark initially, lightens as it dissipates
- **Dust**: Horizontal spread rather than rising, warm browns/tans
- **Steam**: Similar to smoke but lighter colors, faster dissipation

## General Particle Tips

### Embers / Sparks

- Single bright pixels that rise from fire/impact
- Random trajectory with upward bias
- Dim over 3–4 frames: white → yellow → orange → gone
- 1–3 on screen at a time prevents visual noise

### Impact Effects

- **Hit spark**: 3-frame sequence — tiny flash → star burst → fade
- **Dust poof**: Small circle expands and fades (landing, wall impact)
- **Slash trail**: 2 frames — bright arc → dim arc → gone

### Magic / Energy

- **Glow**: Use dithering around bright center to suggest soft light
- **Lightning**: 1–2px white zigzag lines, held for 1 frame, afterimage for 1 frame
- **Healing**: Rising green crosses or plus signs, 2px each, fade upward
- **Charge up**: Converging bright pixels toward center point

### Performance Considerations

- Keep particle count low — each particle competes for visual attention
- Particles should never obscure gameplay-critical elements
- Sync particle timing to game events, not just looping
- Reuse particle sprites with palette swaps (fire particles → ice particles by swapping colors)
