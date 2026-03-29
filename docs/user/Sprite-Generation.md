# Sprite Generation

Describe what you need and PIXL draws it, or import any AI-generated pixel art image and reconstruct it as a clean, game-ready tile.

## From text to pixel art

Tell PIXL what you want — "a wizard with a purple hat and glowing staff" — and it generates a sprite. Under the hood, an image model draws the character at high resolution, then PIXL's pipeline turns it into true pixel art.

```bash
pixl generate-sprite tileset.pax \
  --prompt "wizard with purple hat and staff" \
  --name wizard \
  --out wizard.png
```

## How the conversion works

AI image generators produce images that *look* like pixel art but aren't — they're rendered at 1024×1024 with anti-aliased edges and thousands of colors. PIXL converts these into real pixel art through a multi-step pipeline:

### 1. Pixel grid detection

The AI image has each "art pixel" rendered as a 20-40 pixel block. PIXL scans for this repeating grid pattern and finds the actual art resolution — usually 24-32 pixels.

### 2. Center-sampling

Instead of blurry downscaling, PIXL picks the center pixel of each block. This avoids the smeared edges that normal image resizing creates.

### 3. Background removal

AI generators often add glows and halos around sprites, even when asked for a transparent background. PIXL flood-fills from the image corners to detect and strip these automatically.

### 4. Palette extraction

The dominant colors are pulled from the image and organized into a clean indexed palette, darkest to lightest. The actual darkest and lightest pixels are always preserved — your outlines and highlights won't get averaged away.

### 5. Anti-aliasing cleanup

Stray blended pixels between color regions get snapped to their nearest clean neighbor. The result is crisp pixel art, not a soft downscale.

### 6. Outline enforcement

Any boundary pixel that's too light gets darkened to ensure the sprite has a readable silhouette — the single most important quality rule in pixel art.

## Import your own images

Already have AI-generated pixel art from Midjourney, DALL-E, or Stable Diffusion?

```bash
pixl convert wizard.png --width 32 --colors 32
```

PIXL detects the pixel grid, samples cleanly, strips the background, extracts a palette, and outputs a true 32×32 sprite with 32 indexed colors.

## Palette management

### Auto-extracted palette

Every generated sprite comes with an auto-extracted palette in PAX format — copy and paste it into your `.pax` file:

```toml
[palette.auto]
"." = "#00000000"
"#" = "#1a1020ff"
"a" = "#4a3668ff"
"b" = "#6a5090ff"
```

### Remap to your project palette

When you're ready to integrate into your existing tileset, remap the auto-extracted colors to your project palette. PIXL uses perceptual color matching (OKLab) to find the closest color for each symbol.

## Quality checks

Every generated sprite is automatically analyzed:

- **Outline coverage** — what % of the silhouette has dark border pixels
- **Centering** — is the subject in the middle of the tile
- **Canvas utilization** — is the sprite filling the space or floating in a void
- **Contrast** — are adjacent colors distinct enough to read
- **Fragmentation** — are there disconnected floating pixels

If something's off, PIXL tells you exactly what row to fix and how.
