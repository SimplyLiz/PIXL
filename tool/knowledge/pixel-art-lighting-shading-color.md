# Pixel Art Lighting, Shading, and Color Theory Reference

> Compiled from tutorials and techniques by Slynyrd (Raymond Schlitter), Pedro Medeiros
> (Saint11 / MiniBoss), AdamCYounis, Brandon James Greer, Derek Yu, Pixel Parmesan, the
> OpenGameArt community, and the Liberated Pixel Cup style guide. Intended as an AI
> knowledge base for generating pixel art tiles.

---

## 1. Light Source Placement and Consistency

### 1.1 Establishing a Light Direction

Every tile, sprite, and environment piece must share a single dominant light direction.
This decision is made before any shading work begins and governs highlight placement,
shadow direction, and color selection for the entire tileset.

**Top-left convention (most common):**
- The de facto standard in 2D pixel art and the most widely expected by players.
- Highlights land on top and left edges; shadows fall on bottom and right edges.
- Works well for both top-down and side-view tilesets.

**Top / directly-above convention:**
- Preferred when tiles need to be horizontally mirrored without breaking lighting
  consistency (Slynyrd uses this for side-view tilesets specifically so left and right
  variants can be simple mirrors).
- Creates symmetrical shadow distribution -- shadows appear equally on left and right
  bottom edges.
- Natural for top-down views where the "camera" looks straight down.

**Front lighting:**
- Light faces the viewer. Rarely used for tilesets because it flattens forms.
- Acceptable for UI elements or menu sprites where depth is unimportant.

**Rule:** Once a direction is chosen, every tile in the set must obey it. If a tile
appears inconsistent when placed next to others, the light direction is the first thing
to audit.

### 1.2 Ambient vs Directional vs Point Lighting

**Directional light (sunlight, moonlight):**
- Infinite distance, parallel rays. A flat surface perpendicular to the light receives
  uniform illumination -- no gradient across its face. This is a critical rule: flat
  surfaces get a single flat color, not a gradient. Gradients on flat planes are a
  hallmark of beginner work (OpenGameArt Chapter 4).
- Produces hard, well-defined cast shadows in clear atmosphere. In hazy / overcast
  conditions shadows become soft and diffuse.

**Ambient light:**
- Non-directional fill that prevents shadows from going pure black.
- In outdoor scenes the sky acts as ambient fill, tinting shadows slightly blue.
- In indoor/dungeon scenes ambient light is minimal, increasing contrast.
- Technique: introduce a secondary, dimmer light from a different angle. Highlight only
  the edges of shadowed areas while keeping shadow cores dark (OpenGameArt).

**Point light (torches, lava, crystals):**
- Radial falloff: brightest at source, dimming with distance.
- At pixel scale, show falloff through discrete color steps rather than smooth gradients.
- Warm point lights (torches) tint nearby surfaces orange/yellow and push shadows toward
  complementary cool tones (blue-purple).
- A point light creates highlights on whichever side of an object faces the source,
  regardless of the global directional light -- this means objects near a torch may have
  highlights on their "shadow side."

### 1.3 Maintaining Lighting Across a Tileset

- Use a shared palette with pre-defined light/mid/shadow colors per material.
- Test tiles in all 8-neighbor arrangements to catch seam inconsistencies.
- When mirroring tiles, it only works if the light source is symmetrical (directly above)
  or if you are mirroring along the light axis. A top-left lit tile mirrored horizontally
  becomes top-right lit and will clash.
- Drop shadows should use a single consistent color and opacity across all tiles. The
  Liberated Pixel Cup standard specifies `#322125` at 60% opacity for all drop shadows.

---

## 2. Shadow Techniques at Pixel Scale

### 2.1 Cast Shadows

Cast shadows are projected by one object onto another surface.

**Direction:** Always opposite the light source. Top-left light = shadows fall
bottom-right.

**Length:** In tilesets, keep cast shadow length to a maximum of one tile width regardless
of the object's actual height. While unrealistic, this convention prevents shadows from
spanning multiple tiles and creating tiling conflicts (Slynyrd, Pixelblog 43). The effect
remains convincing because pixel art operates on aesthetic convention, not photorealism.

**Hardness:**
- Clear atmosphere (outdoor, space): crisp, hard-edged shadows. In vacuum/space,
  shadows are razor-sharp and elongated.
- Hazy/overcast atmosphere: soft, diffuse shadows. Use dithering or an intermediate
  shadow color to soften edges.
- Indoor/dungeon: shadows are sharp near objects but can be softer further from point
  light sources.

**Color:** Never pure black. Use the receiving surface's darkest shade, optionally
shifted toward purple or blue. This keeps the shadow grounded in the material below
it. The LPC style guide mandates hue-shifting shadows toward purple as they darken.

### 2.2 Form Shadows (Self-Shadows)

Form shadows show an object's own volume -- the side of a cube facing away from light,
the underside of a sphere.

**At 16x16:**
- You have very few pixels to work with. Use hard value steps, not smooth gradients.
- A typical small object needs only 2-3 shading steps: light face, mid-tone, shadow face.
- Flat shading (uniform color per face) is often the correct approach at this scale.
  Reserve gradients for larger surfaces (32x32+) or curved forms.

**The terminator line:**
- The boundary between lit and shadow areas. Pedro Medeiros recommends sharp terminators
  at pixel scale to avoid banding. A sharp transition reads as "form" even at tiny sizes;
  a gradient reads as "mush."

**Bounce light:**
- A subtle brightening on the shadow side, caused by light reflecting off the ground or
  nearby surfaces. At pixel scale this is often just one pixel of a slightly lighter
  shadow color along the bottom edge. Adds significant perceived volume.

### 2.3 Shadow Color Theory

**Why pure black destroys depth:**
- Black has no hue or saturation information. It reads as a void, not a shadow.
- It eliminates any sense of reflected environment light.
- Adjacent colors appear garish next to black due to extreme contrast.

**How to pick shadow hues:**
- Start from the object's local color.
- Decrease brightness.
- Shift hue toward cool (blue/purple) for outdoor daylight scenes.
- Shift hue toward warm (red/brown) for warm indoor light.
- Increase saturation slightly in mid-shadows, then decrease again in the darkest values.
- The Liberated Pixel Cup rule: shadow hues shift toward purple; highlight hues shift
  toward yellow.

**Real-world basis:** Outdoor shadows are tinted by the blue sky (the main ambient
source). Indoor shadows near a warm fire shift toward the complementary of orange --
blue-purple. This is not arbitrary artistic choice; it is how light behaves.

### 2.4 Ambient Occlusion at Pixel Scale

Ambient occlusion (AO) darkens areas where surfaces meet and block ambient light:
corners, crevices, where walls meet floors, under overhangs, between planks.

**Implementation in tiles:**
- Apply a soft, darker shade at tile corners and where geometry elements meet (e.g.,
  between bricks, between wood planks).
- Where two materials intersect, the overlapping material casts a thin shadow onto the
  surface below (e.g., grass edge casts shadow onto dirt using dirt's darkest shade).
- AO color is never pure black -- use a dark version of the receiving surface's hue.
- AO is subtle: usually 1-2 pixels wide. It grounds objects and prevents the "floating"
  look.
- In procedural/auto-tiled setups, AO can be approximated by darkening pixels adjacent
  to occupied neighbor tiles.

---

## 3. Highlight and Specular Techniques

### 3.1 Highlight Placement

Highlights fall on surfaces that face the light source most directly.

**Top surfaces** of objects receive the strongest highlight in a top-lit scene.
**Front-facing edges** catch light in a top-left scheme.
**Curved surfaces** place the highlight at the point where the surface normal points
most directly at the light. On a sphere this is a small bright spot; on a cylinder it
is a vertical stripe.

**Endesga's bevel tip:** Sharp edges and bevels catch narrow, bright highlights that
define form crisply. Adding a single bright pixel along a ridge communicates edge
geometry efficiently at small scales.

### 3.2 Specular vs Diffuse Reflection

**Diffuse surfaces (wood, stone, cloth, dirt):**
- Light scatters in all directions. No bright specular spot.
- Highlights are broad, soft, and close in value to the midtone.
- The highlight, if present, is a slightly lighter version of the base color.

**Specular surfaces (metal, water, glass, polished stone):**
- Light reflects directionally, creating a sharp, bright highlight.
- Extreme value contrast: specular metals jump from very dark to very bright with little
  mid-tone (the "skip the midtone" rule for metals -- OpenGameArt Chapter 7).
- Highlight color may shift toward the light source color (warm yellow for sunlight)
  rather than the material color.
- Metals are never simply gray. They always carry a hue, either intrinsic (copper =
  orange-brown, gold = yellow) or from ambient/reflected light.

### 3.3 Material-Specific Highlight Behavior

| Material | Highlight shape | Value range | Hue shift | Notes |
|----------|----------------|-------------|-----------|-------|
| **Metal (steel)** | Sharp, narrow, high contrast | Full black-to-white, skipping gray | Toward light source | Reflects environment; never plain gray |
| **Metal (gold/copper)** | Sharp, narrow | Full range within warm hues | Greenish shadows for copper | Intrinsic color dominates |
| **Wood** | Absent or very subtle | Narrow mid-range | Minimal | Grain pattern dominates; only varnished wood gets specular |
| **Stone (granite)** | Dull, broad | Narrow range | Minimal | Small-scale bumps shown via tiny shadow dots |
| **Stone (marble)** | Moderate, smooth | Wider than granite | Slight | More anti-aliasing; smoother surface |
| **Water/Glass** | Bright, sharp, diagonal bands | Very wide (near-white highlights) | Toward sky/environment | Semi-transparent; secondary highlight from internal reflection |
| **Cloth/Fabric** | Soft, follows folds | Moderate range | Follows fold geometry | Defined by fold structure, not surface finish |
| **Clay/Dirt** | Nearly absent | Very narrow | Warm shifts | Roughness dominates; minimal reflection |
| **Hair/Fur** | Line or triangle shapes | Moderate | Depends on hair color | Highlights define strand direction and volume |

### 3.4 Rim Lighting and Backlighting

Rim light is a bright outline effect from a secondary light source behind the subject.

**When to use:**
- To separate a sprite from a dark background.
- To show a strong environmental light source (sunset behind a character, lava glow from
  behind a wall).
- To add drama and atmosphere.

**Implementation:**
- Apply a thin (1-pixel) bright line along the edge opposite the main light source.
- Rim light color matches the backlight source (orange for lava, blue for moonlight).
- Use sparingly. Adding rim light to every tile in a set looks over-produced.
- Pedro Medeiros lists rim light as a secondary effect to add after primary shading is
  complete.

---

## 4. Color Ramp Construction

### 4.1 The HSB Model

All ramp construction should be done in HSB (Hue, Saturation, Brightness), not RGB.
RGB changes are unpredictable for artistic purposes; HSB gives direct control over the
three perceptual axes.

### 4.2 Hue Shifting in Ramps

A straight ramp (same hue, just lighter/darker) looks dull and muddy. Effective ramps
shift hue across the brightness range.

**The standard warm-highlight / cool-shadow pattern:**
- As brightness increases, hue shifts toward yellow (warm).
- As brightness decreases, hue shifts toward blue/purple (cool).
- This mimics outdoor daylight where the sun is warm and sky-fill shadows are cool.

**The inverse (for warm environments):**
- Warm light source (torchlight): shadows shift cool (blue-purple).
- Cool light source (moonlight, underwater): shadows shift warm (brown-red).
- The rule is always: highlights shift toward the light's hue; shadows shift toward
  the complement.

**Slynyrd's Mondo palette formula:**
- 9 swatches per ramp.
- 20 degrees of positive hue shift between each swatch.
- 8 ramps total, each offset by 45 degrees, cycling through 360 degrees of the color
  wheel.
- Adjacent ramps share overlapping hue ranges, creating natural harmony.

### 4.3 Saturation Shifting Across a Ramp

Saturation does NOT stay constant. It follows a curve:

- **Darkest values:** Low-to-moderate saturation. Very dark + high saturation =
  overly rich and heavy.
- **Mid-values:** Peak saturation. This is where color is most vivid.
- **Brightest values:** Saturation decreases toward washed-out / near-white. High
  brightness + high saturation = "eye-burning" neon (Slynyrd).

Saturation never reaches 0% or 100% in a well-constructed ramp (Slynyrd).

**Important nuance (Pixel Parmesan):** Different hues peak in saturation at different
brightness levels. Yellow peaks at high brightness. Blue peaks at low brightness. Red
peaks in mid-tones. A universal "saturation curve" does not exist -- it must be tuned
per hue.

### 4.4 How Many Steps Per Ramp

| Tile size | Recommended ramp steps | Notes |
|-----------|----------------------|-------|
| 8x8 (Celeste-scale) | 3-5 colors per material | Celeste uses 3-5 colors per tileset |
| 16x16 | 3-5 colors per material | Sweet spot: 1 highlight, 1 base, 1-2 shadow, 1 dark/outline |
| 32x32 | 4-7 colors per material | Enough room for subtle gradation without banding |
| Full sprite sheets | 5-9 per ramp | Slynyrd's Mondo uses 9 per ramp for maximum range |

**Anti-banding rule:** If two adjacent ramp steps create visible parallel bands that
follow the contour of a shape, either merge them into one step or break the pattern
by varying cluster shapes (Pedro Medeiros).

### 4.5 Building Ramps for Specific Materials

**Grass/Foliage:**
- Base hue: green (90-140 degrees).
- Highlight shift: toward yellow-green (70-90 degrees).
- Shadow shift: toward blue-green or teal (160-200 degrees).
- High saturation in mid-tones; desaturated shadows.

**Stone/Rock:**
- Base hue: desaturated blue-gray or warm gray (20-40 or 200-220 degrees).
- Narrow hue shift across the ramp.
- Shadows: shift slightly purple. Highlights: shift slightly yellow.
- Low saturation throughout; texture comes from value variation, not color variation.

**Wood:**
- Base hue: orange-brown (20-40 degrees).
- Highlight shift: toward yellow (40-55 degrees).
- Shadow shift: toward red-brown or purple-brown (0-20 degrees).
- Moderate saturation; grain pattern provides most of the visual interest.

**Metal (steel):**
- Base hue: desaturated blue or blue-gray.
- Skip mid-tones: jump from dark to highlight.
- Highlights can approach near-white with a slight warm shift.
- Shadows are dark and saturated.

**Water:**
- Base hue: blue-cyan (190-220 degrees).
- Highlight shift: toward cyan-white.
- Shadow shift: toward deep blue-purple.
- High transparency effect: secondary internal highlights from light passing through.

**Skin:**
- Base hue: peach/orange (15-30 degrees).
- Highlight shift: toward yellow (35-45 degrees).
- Shadow shift: toward red, then toward purple in deepest shadows.
- Subsurface scattering effect: shadows are warmer and more saturated than expected.

---

## 5. Atmospheric and Environmental Lighting

### 5.1 Torchlight and Warm Point Lights (Dungeons)

- Color temperature: orange-yellow (30-50 degrees, high saturation, moderate brightness).
- Nearby surfaces receive warm tint -- shift base colors toward orange.
- Shadows pushed cool (blue-purple) by contrast.
- Radial falloff shown through discrete color steps. Tiles near the torch use the warm
  palette variant; tiles further away revert to the ambient (dark, cool) palette.
- Flickering can be suggested with 2-3 animation frames that subtly shift highlight
  positions and warm tint intensity.
- Multiple torches create overlapping warm zones; where two lights meet, surfaces are
  brighter and more saturated.

**Practical tile approach for dungeons:**
- Define a "lit" and "unlit" variant for each tile material.
- Lit variant: warmer hues, higher value, stronger contrast.
- Unlit variant: cooler hues, lower value, compressed contrast.
- The game engine or renderer handles the blending, but the tile artist must define
  both ends of the range.

### 5.2 Underwater Lighting

- Global tint: blue-green (180-210 degrees).
- Saturation decreases with depth.
- Contrast compresses with depth (shadows and highlights converge toward mid-blue).
- Light rays from above: narrow diagonal bright bands, slightly brighter and more
  cyan than surrounding water.
- Caustics (light patterns on the sea floor): bright, shifting patches of cyan-white
  on the bottom surface.
- Objects at depth lose red first (red light is absorbed soonest), then orange, then
  yellow. Deep objects trend toward blue-green monochrome.

**Tile implementation:**
- Use a base palette that is already shifted cool and desaturated.
- Floor tiles can include subtle caustic patterns as lighter pixel clusters.
- Vertical depth can be shown by further desaturating and darkening lower tiles.

### 5.3 Sunset / Sunrise Color Temperature

**Sunrise (dawn):**
- Sky: gradient from deep blue/purple (top) to pink-orange (horizon).
- Light color: warm pink-orange, low intensity, low contrast.
- Shadows: long, cool blue-purple, soft-edged.
- Surfaces: warm golden tint on anything facing east / the light source.

**Sunset (golden hour):**
- Dominant warm palette: oranges, reds, yellows.
- Intense, saturated warm highlights.
- Long, cool blue-purple shadows stretching opposite the sun.
- Atmospheric haze increases: background layers become more orange and less saturated.

**Midday:**
- Bright, high-contrast, saturated colors.
- Short shadows (sun is high).
- Blue sky provides strong cool ambient fill in shadows.

**Night / Moonlight:**
- Cool blue-silver palette.
- Very low contrast; everything compresses toward dark blue.
- Highlights are desaturated cool blue, not white.
- Point lights (torches, windows) become dominant warm accents against the cool base.

### 5.4 Fog and Distance Desaturation

Atmospheric perspective at tile scale uses the same principle as in traditional painting:
objects further from the camera become desaturated, lower contrast, and shift toward the
atmospheric color (usually a blue-gray or warm haze depending on setting).

**Implementation in tilesets:**
- Background / distant layers: reduce saturation, compress value range, shift hue toward
  sky/fog color.
- Celeste uses desaturated versions of tileset colors for background layers, explicitly
  described as simulating "atmospheric interference" to create a far-off look.
- Foreground: full saturation, full contrast.
- Middle ground: slightly desaturated.
- Background: significantly desaturated, hue-shifted toward atmospheric color.

**Fog specifically:**
- Uniform lightening and desaturation of all colors toward a fog color (white-gray for
  normal fog, green-gray for swamp fog, blue-gray for mountain fog).
- Tiles in fog lose detail -- reduce texture variation and flatten value range.
- A partially transparent fog layer over tiles is the simplest implementation.

---

## 6. Tile-Specific Lighting Rules

### 6.1 How Walls vs Floors Receive Light

In a top-down or 3/4 view:

**Floor tiles (horizontal surfaces):**
- Receive the most direct light from an overhead source.
- Generally brighter and more uniformly lit.
- Texture and material pattern dominate; shading is subtle.
- Shadows on floors come from objects above them (cast shadows), not from the floor's
  own form.

**Wall tiles (vertical surfaces):**
- Lit on the face that angles toward the light source; shadowed on the opposite face.
- Top edge of walls catches a bright highlight (light hitting the top plane).
- Vertical surfaces are darker than horizontal surfaces under overhead light (Gleaner
  Heights devlog: "horizontal planes tend to be brighter than vertical ones").
- Side-view wall tiles require a clear light/shadow split: one face lit, one face
  shadowed, top edge highlighted.

**The LPC standard:** Props and walls must show significant lighting variation between
top and side surfaces to integrate visually within scenes.

### 6.2 Corner Tiles: Where Shadows Fall

**Inner corners (concave, e.g., inside corner of a room):**
- AO is strongest here. Both walls block ambient light.
- Darken 1-2 pixels into the corner on both wall faces.
- Floor at the inner corner also darkens slightly (ambient occlusion from two walls).

**Outer corners (convex, e.g., outside corner of a building):**
- The protruding edge catches light on the side facing the source.
- The opposite side falls into shadow.
- Cast shadow falls on the ground from the corner, extending in the light's opposite
  direction.

**Wall-floor junctions:**
- A 1-pixel dark line or AO gradient where the wall meets the floor.
- This "grounding shadow" is critical -- without it, walls appear to float.
- Use the floor's darkest shade for this line, not the wall's.

### 6.3 Transition Tiles: Lighting at Material Boundaries

Where two materials meet (grass-to-stone, dirt-to-water, etc.):

**The overlapping material casts shadow:**
- The material "on top" (e.g., grass overhanging a cliff) casts a thin shadow onto the
  material below using the lower material's own darkest shade.
- Edge grass strands should NOT use bright highlight colors -- they fall in the darker
  zone of the boundary, using mid-tones or shadow values (Gleaner Heights).

**Material brightness hierarchy:**
- Horizontal planes are brighter than vertical planes.
- Organic materials (grass, dirt) are typically less reflective than worked stone.
- The transition edge itself gets a shadow line: 1 pixel of the lower material's dark
  color directly beneath the upper material's edge.

**Avoiding seams:**
- Transition tile edges should use overlapping color clusters that break the grid line.
- Test all permutations of adjacent tiles to verify shadow continuity.
- Create shadow variant tiles (e.g., "grass tile with brick shadow overlay") for
  combinations that need special treatment.

### 6.4 Self-Illuminating Tiles (Lava, Magic, Crystals)

Self-illuminating tiles are simultaneously a surface and a light source.

**The glow structure (inside out):**
1. Core: near-white or max-saturation bright color (yellow-white for lava, cyan-white
   for magic).
2. Hot zone: fully saturated bright hue (orange for lava, blue for magic crystals).
3. Falloff zone: less saturated, darker version of the hue.
4. Edge / crust: darkest, most desaturated color. For lava this is dark brown-black
   rock crust; for crystals it is the ambient shadow color.

**Effect on adjacent tiles:**
- Tiles adjacent to a self-illuminating tile should receive a warm (or cool, depending on
  the glow color) tint on their nearest edge.
- This can be baked into variant tiles ("stone wall next to lava" variant with warmer
  near-edge colors) or handled by the rendering engine with a light overlay.

**Animation:**
- 2-4 frame subtle pulsing: shift the core between slightly brighter/dimmer.
- Shift highlight pixel positions slightly between frames to suggest liquid movement
  (lava) or energy fluctuation (magic).

**The key rule:** A self-illuminating tile overrides the global light direction on
nearby surfaces. The tile itself has no "shadow side" -- it is uniformly bright at
its core. But it creates a new local light source that affects everything around it.

---

## 7. Common Anti-Patterns to Avoid

### 7.1 Pillow Shading

Placing bright colors at the center and dark colors at all edges, creating a puffy,
sourceless look. The fix: commit to a single light direction. Shadows go on the side
opposite the light, not uniformly around the perimeter.

### 7.2 Banding

Parallel bands of color that follow a contour, creating a "topographic map" look. The
fix: vary cluster shapes, merge unnecessary steps, or break bands with texture details.
If two shading bands create a visible parallel boundary, they are banding.

### 7.3 Dirty / Muddy Colors

Using unsaturated mid-gray shadows or straight black for shading. The fix: every shadow
color should carry a hue. Shift toward cool (blue/purple) for outdoor shadows, warm
(orange/red-brown) for indoor warm-lit shadows.

### 7.4 Gradient on Flat Surfaces

Applying a smooth gradient across a flat plane. Real directional light illuminates flat
surfaces uniformly. Save gradients for curved surfaces only.

### 7.5 Inconsistent Light Direction Across Tiles

Mixing top-left and top-right lighting within a single tileset. The result is an
environment that looks broken. If tiles were designed to be mirrored, verify the
light source is vertically symmetric (directly above).

### 7.6 Over-Shading at Small Scales

Using too many color steps on a 16x16 tile creates noise, not detail. At 16x16, 3-5
colors per material is the sweet spot. Use the minimum number of values that reads
correctly at 1x zoom.

---

## 8. Dithering for Lighting and Shading

Dithering is a pattern of alternating pixels of two colors that creates the illusion
of an intermediate shade. At typical view distances, the eye blends the pattern.

### When to Use Dithering for Shading

- To simulate gradual shadow falloff with a limited palette.
- To add texture to large flat surfaces (dirt, stone, sand).
- To create soft transitions between light zones in atmospheric effects (fog edges,
  light falloff from a torch).
- Most useful in retro / low-color-count styles (4-16 color palettes).

### When NOT to Use Dithering

- When sufficient palette colors exist for smooth ramps.
- On small sprites (16x16 or less) where checkerboard patterns read as noise.
- Across material boundaries (dithering grass into stone looks like a rendering error).

### Patterns

- **50/50 checkerboard:** The most common pattern. Creates an even blend.
- **75/25 scattered:** One color dominates; sparse pixels of the second color create a
  subtle shift.
- **Ordered dithering (Bayer matrix):** Regular geometric patterns. Clean but can look
  mechanical.
- **Noise / random dithering:** Irregular placement. More organic but can look messy.

---

## 9. Game Art Analysis: How Specific Games Handle Tileset Lighting

### 9.1 Celeste (Matt Thorson, Noel Berry)

- **Tile size:** 8x8 pixels.
- **Palette:** 3-5 colors per tileset. One shared dark infill color across all terrain
  types for visual coherence.
- **Lighting:** Top-down convention. Dark colors on bottom edges, light colors on top
  edges. Simple and consistent.
- **Background layers:** Same auto-tiling rules as foreground but with desaturated colors
  to simulate atmospheric distance.
- **Edge handling:** Edges are never perfectly straight -- transparent pixels break grid
  rigidity. 1-4 variants of straight-edge tiles swap randomly to prevent repetition.
- **Philosophy:** Impressionistic rendering given the tiny tile size. Readability over
  detail.

### 9.2 Stardew Valley (ConcernedApe)

- **Palette discipline:** Entire world uses a unified, curated palette. Multiple tile
  variants placed semi-randomly prevent monotony without breaking cohesion.
- **Lighting:** Consistent overhead light. Indoor scenes use warmer, lower-contrast
  palettes.
- **Seasonal variants:** Same tiles with shifted palettes for spring/summer/fall/winter,
  demonstrating how hue shifting can transform mood without redesigning geometry.

### 9.3 Shovel Knight (Yacht Club Games)

- **Three background layers** with parallax scrolling and atmospheric perspective.
- **Palette constraints:** NES-era color limitations as a deliberate style choice,
  forcing efficient use of limited ramps.
- **Lighting:** Scene-based mood lighting with entire palette shifts per level (warm
  underground, cool overworld, etc.).

### 9.4 The Liberated Pixel Cup Standard (OpenGameArt)

- **Light direction:** Primarily from above, with minimal left-side directionality.
- **Shadow color:** Uniform `#322125` at 60% opacity for all drop shadows.
- **Color ramps:** Hue shifts mandatory -- shadows toward purple, highlights toward
  yellow. Same-hue ramps are explicitly prohibited.
- **Interiors:** Cooler overall coloration with reduced contrast. Underground spaces
  push this further (cooler, darker, flatter).
- **Tile size:** 32x32 base grid, 16x16 sub-tiles.

---

## 10. Practical Workflow Summary for AI Tile Generation

When generating pixel art tiles, apply these rules in order:

1. **Establish light direction** before rendering any detail. Top-left is the safe
   default.

2. **Build the color ramp** per material using HSB:
   - 3-5 steps for 16x16 tiles.
   - Hue-shift toward yellow/warm in highlights, toward blue/cool in shadows.
   - Peak saturation in mid-tones; desaturated at extremes.
   - Never use pure black or 100% saturation.

3. **Block in flat colors** per face/surface first. Flat surfaces get one color. Only
   curved or angled surfaces get gradients.

4. **Add form shadows** using hard-edged value steps. Sharp terminators, not gradients.
   Bounce light on shadow edges where appropriate (1 pixel lighter).

5. **Add cast shadows** in the direction opposite the light. Max 1-tile length. Use the
   receiving surface's dark shade, hue-shifted toward purple/blue.

6. **Add ambient occlusion** in corners, crevices, and wall-floor junctions. 1-2 pixels
   of the surface's darkest shade.

7. **Add highlights** appropriate to the material:
   - Diffuse materials: subtle, broad highlight near the base color.
   - Specular materials: sharp, bright highlight with high value contrast.

8. **Apply atmospheric modifications** if the tile belongs to a themed environment
   (warm tint for torchlit, cool tint for underwater, desaturated for fog/distance).

9. **Handle self-illuminating elements** last: paint glow core outward (white -> bright
   saturated -> dim -> dark edge). Flag adjacent tile edges for warm/cool tinting.

10. **Test in context:** Place the tile in a grid with its neighbors and verify lighting
    consistency, shadow continuity, and seam invisibility.

---

## Sources

- [Slynyrd - Pixelblog 6: Light and Shadow](https://www.slynyrd.com/blog/2018/6/15/pixelblog-6-light-and-shadow)
- [Slynyrd - Pixelblog 1: Color Palettes](https://www.slynyrd.com/blog/2018/1/10/pixelblog-1-color-palettes)
- [Slynyrd - Pixelblog 28: Side View Tiles](https://www.slynyrd.com/blog/2020/5/21/pixelblog-28-side-view-tiles)
- [Slynyrd - Pixelblog 43: Top Down Tiles Part 2](https://www.slynyrd.com/blog/2023/3/26/pixelblog-43-top-down-tiles-part-2)
- [Slynyrd - Pixelblog 20: Top Down Tiles](https://www.slynyrd.com/blog/2019/8/27/pixelblog-20-top-down-tiles)
- [Slynyrd - Full Tutorial Catalogue](https://www.slynyrd.com/pixelblog-catalogue)
- [Pedro Medeiros - Basic Shading (Pixel Grimoire #4)](https://medium.com/pixel-grimoire/how-to-start-making-pixel-art-4-f57f51dcfa02)
- [Pedro Medeiros - Basic Color Theory (Pixel Grimoire #6)](https://medium.com/pixel-grimoire/how-to-start-making-pixel-art-6-a74f562a4056)
- [Pedro Medeiros - Anti-Alias and Banding (Pixel Grimoire #5)](https://medium.com/pixel-grimoire/how-to-start-making-pixel-art-4-ff4bfcd2d085)
- [Pedro Medeiros / Saint11 - Pixel Art Tutorials Collection](https://saint11.org/blog/pixel-art-tutorials/)
- [Studio MiniBoss - Tutorial Archive](https://blog.studiominiboss.com/pixelart)
- [AdamCYounis - Pixel Art Class: Lighting & Shading Basics](https://www.pinterest.com/pin/pixel-art-class-lighting-shading-basics--110971578309827867/)
- [Brandon James Greer - Hue Shifting in Pixel Art](https://www.last.fm/music/Brandon+James+Greer/_/Hue+Shifting+in+Pixel+Art+(Color+Tutorial))
- [Derek Yu - Pixel Art Tutorial (makegames)](https://www.derekyu.com/makegames/pixelart.html)
- [Pixel Parmesan - Color Theory for Pixel Artists: It's All Relative](https://pixelparmesan.com/blog/color-theory-for-pixel-artists-its-all-relative)
- [Pixel Parmesan - Dithering for Pixel Artists](https://pixelparmesan.com/blog/dithering-for-pixel-artists)
- [OpenGameArt - Chapter 4: Shadow and Light](https://opengameart.org/content/chapter-4-shadow-and-light)
- [OpenGameArt - Chapter 7: Textures and Dithering](https://opengameart.org/content/chapter-7-textures-and-dithering)
- [Liberated Pixel Cup Style Guide](https://lpc.opengameart.org/static/LPC-Style-Guide/build/styleguide.html)
- [Celeste Tilesets Step-by-Step (Aran P. Ink)](https://aran.ink/posts/celeste-tilesets)
- [Gleaner Heights: A Study in Pixels - Rocks and Grass](https://gleanerheights.blogspot.com/2016/06/a-study-in-pixels-rocks-and-grass.html)
- [Lospec - Pixel Art Lighting Tutorials](https://lospec.com/pixel-art-tutorials/tags/lighting)
- [Lospec - Pixel Art Shading Tutorials](https://lospec.com/pixel-art-tutorials/tags/shading)
- [Lospec - Hue Shift Palette Collection](https://lospec.com/palette-list/tag/hue%20shift)
- [Endesga - Pixelart Quicktip: Bevels and Highlights](https://lospec.com/pixel-art-tutorials/tags/shading)
