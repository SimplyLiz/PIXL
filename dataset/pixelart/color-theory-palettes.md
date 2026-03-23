---
title: "Color Palettes for Pixel Art"
source: "https://www.slynyrd.com/blog/2018/1/10/pixelblog-1-color-palettes"
topic: "color-theory"
fetched: "2026-03-23"
---

# Color Palettes for Pixel Art

By Raymond Schlitter (SLYNYRD)

## Foundation: Understanding HSB

Pixel art color work begins with HSB (Hue, Saturation, Brightness):

- **Hue**: The actual color (0–360°)
- **Saturation**: The intensity or purity of a color (0–100%)
- **Brightness**: The amount of black or white mixed with a color (0–100%)

This framework provides precise control over custom color creation.

## Color Ramps: Building Blocks

A color ramp is an ordered sequence of related colors arranged by brightness.

### Saturation Management

As colors reach high brightness, saturation must decrease to avoid harsh, eye-straining results. Conversely, very dark colors with high saturation become overly heavy. Saturation peaks in the midtones.

### Hue-Shifting

Rather than creating "straight ramps" that only adjust brightness and saturation, introduce gradual hue transitions. "Positive hue shift usually results in more natural colors, warming as they become brighter."

**The principle**: Shadows shift cooler (toward blue/purple), highlights shift warmer (toward yellow/orange). This mimics how natural light works — sunlight is warm, ambient/sky light in shadows is cool.

Hue-shifting creates visual harmony across multiple ramps and prevents monotonous palettes.

## Building a Complete Palette

Schlitter's "Mondo" palette demonstrates the methodology:

1. **Establish parameters**: Determine swatches per ramp (e.g., 9) and hue increment (e.g., 20°)
2. **Create the base ramp**: Start with a middle-value color at peak vibrancy
3. **Adjust increments**: Brightness increases consistently left-to-right; saturation peaks centrally
4. **Expand systematically**: Duplicate and shift hues across the color wheel
5. **Add neutrals**: Include desaturated versions for natural tones and grays

## Practical Application

Color selection from a well-structured palette becomes intuitive when ramps "criss-cross," ensuring surrounding colors harmonize naturally. This approach maintains visual consistency across game art and illustrations while using surprisingly few colors per individual piece.

## Common Palette Sizes

- **4 colors**: Game Boy style, extreme constraint
- **16 colors**: PICO-8, classic pixel art standard
- **32 colors**: Popular modern pixel art standard (e.g., DB32, Endesga 32)
- **64 colors**: Rich but still manageable
