# Pixel Art Material & Surface Rendering Reference

> A comprehensive reference for AI-assisted pixel art tile generation. Compiled from tutorials and techniques by Slynyrd (Raymond Schlitter), Pedro Medeiros (Saint11), AdamCYounis, Rappenem, UszatyArbuz, Aran P. Ink (Celeste tilesets), and community resources from Lospec, OpenGameArt, PixelJoint, and others.

---

## Table of Contents

1. [Universal Principles](#1-universal-principles)
2. [Stone & Brick](#2-stone--brick)
3. [Wood](#3-wood)
4. [Metal](#4-metal)
5. [Water & Liquid](#5-water--liquid)
6. [Grass & Vegetation](#6-grass--vegetation)
7. [Sand, Dirt & Mud](#7-sand-dirt--mud)
8. [Ice & Snow](#8-ice--snow)
9. [Fabric & Cloth](#9-fabric--cloth)
10. [Tile Transitions & Edge Behavior](#10-tile-transitions--edge-behavior)
11. [Common Mistakes](#11-common-mistakes)

---

## 1. Universal Principles

### Color Ramps & Hue Shifting

- Organize palettes into **ramps** -- groups of colors whose hues are adjacent, arranged dark to light.
- **Hue shift in shadows**: darker values shift toward blue/cool. **Hue shift in highlights**: lighter values shift toward yellow/warm. This mimics natural skylight (blue) and sunlight (yellow).
- Never build ramps by mixing in pure black or pure white. Shift hue and saturation instead.
- Avoid pure black (#000000) and pure white (#FFFFFF) except for outlines or special effects. In reality nothing is ever completely black.
- For a 3-color ramp, aim for ~15-20 unit differences in HSL between each step. More colors in a ramp means smaller incremental changes.
- Highly saturated colors work best as accents, not as dominant palette elements for natural materials.

### Shading Fundamentals (Pedro Medeiros)

- **Volume shadow**: self-projected soft shadow from the object's own form blocking light. The most common shadow type.
- **Projected shadow**: one object casting a sharp shadow onto another surface.
- **Terminator**: the transition zone between lit and shadowed areas. In pixel art, favor **sharp transitions** over soft gradients to avoid banding.
- **Specular highlight**: the brightest spot reflecting the light source. Glossy/reflective objects have small, focused highlights. Rough objects may have no specular at all.
- **Rim light**: bright outline effect when light comes from behind the object, usually from a secondary dimmer light.
- **Bounce light**: slightly brighter area in the shadow, caused by light reflecting off the ground or nearby surfaces.
- Flat faces should maintain **uniform color** across their surface. Reserve gradual color ramps for rounded shapes.

### Texture Creation Principles (Slynyrd)

- **Simplify**: break real-world details into simple abstract shapes. Pixel art resolution demands abstraction.
- **Repetition**: use a limited set of detail clusters repeated with varied distribution. Avoid monotonous uniform placement.
- **Balance**: distribute visual weight evenly within a tile. If one area has dominant detail, the repeating pattern becomes obvious.
- **Contrast**: vary texture density across surfaces. Completely homogeneous textures look unnatural and busy.
- **Orphan pixels**: isolated single pixels distract the eye. They are acceptable only when part of a larger texture cluster, never as standalone details.
- Good pixel art **eliminates all unnecessary detail**.

### Dithering for Materials

- Dithering creates the illusion of intermediate colors by interlacing pixels of two different colors in a pattern.
- **Ordered/checkerboard dithering**: most common, creates a regular pattern. Best for retro aesthetics and large gradient areas.
- **Transitional dithering**: used to smooth transitions between two colors or soften edges. Most useful at higher resolutions (32x32+).
- Dithering can serve double duty as both **shadow and texture** simultaneously (e.g., chainmail, rough stone).
- Use dithering sparingly on smooth surfaces -- it implies roughness. Reserve it for materials that are expected to be textured.
- Avoid placing dither patterns too close together; they create visual chaos when viewed at game scale.

### Scale Behavior Across Tile Sizes

| Aspect | 8x8 | 16x16 | 32x32 |
|---|---|---|---|
| **Detail capacity** | Minimal -- 1-2 texture hints max | Standard -- core material identity readable | Rich -- sub-features like individual cracks, knots |
| **Color count** | 2-3 per material | 3-5 per material | 4-8 per material |
| **Texture approach** | Color and value only, no room for pattern detail | Abstract cluster patterns, simplified grain/mortar | Recognizable patterns, visible grain direction, mortar lines |
| **Dithering** | Generally impractical | Sparse, 1-2 pixel transitions | Effective for gradients and material blending |
| **Identifiable features** | Color swatch communicates material | Silhouette shapes + color tell the story | Individual elements (bricks, planks, blades) are distinct |
| **Production time** | 5-10 minutes | 10-20 minutes | 20-60 minutes |
| **Tile variants needed** | 2-4 minimum | 3-6 for natural look | 4-10 for visual richness |

---

## 2. Stone & Brick

### Color Palette

- **Base stone**: use contextual colors that match the environment. Gray works generically, but reddish-brown for canyon rock, blue-gray for slate, warm beige for sandstone. A gray stone in a reddish canyon looks wrong.
- **Brick ramp**: 3-5 shades. Base brick color + 1 lighter highlight + 1-2 darker shadow tones + mortar color.
- **Mortar/grout**: typically 1-2 shades darker or lighter than brick, depending on style. Can be lighter (cement mortar) or darker (aged/dirty grout).
- Hue-shift shadows toward blue-gray for cool stone, toward brown for warm stone.

### Pixel-Level Techniques

**Brick patterns at 16x16:**
- With 1px grout lines, viable brick dimensions that divide evenly into 16px are: **15, 7, 3, and 1** pixels.
- Standard approach: 7px wide bricks with 1px grout, staggered rows (running bond pattern).
- Height and width of all bricks must adhere to these divisible dimensions to pattern seamlessly.

**Construction workflow (Slynyrd's 3-step method):**
1. **Lay the grout** -- establish the mortar line pattern first as the darkest value.
2. **Fill the bricks** -- apply base brick color within the mortar grid.
3. **Age and weather** -- add discoloration, damage, and value variation.

**Shading bricks:**
- Lighter color on top-left edges of each brick (light-facing).
- Darker color on bottom-right edges (shadow-facing).
- Darkest value at corners where bricks meet mortar (ambient occlusion).

**Cracks and damage:**
- Cracks follow 1px dark lines that break across a brick's face, sometimes extending into mortar.
- Use the mortar color or 1 shade darker for crack color.
- Cracks branch at angles -- avoid perfectly straight or perfectly diagonal lines.
- At 16x16, a single crack is 2-4 pixels long. At 32x32, cracks can be longer with branching.

**Weathering and moss:**
- Moss patches: 2-3 green pixels clustered in mortar joints or lower brick edges where moisture collects.
- Discoloration: scattered individual brick faces shifted 1-2 values lighter or darker than neighbors.
- At 16x16, weathering is suggested by 1-2 off-color bricks. At 32x32, individual bricks can have internal variation.

**Stone blocks (non-brick):**
- **Outline method** (Slynyrd): use angular lines for a sharp, hard look. Best for larger sprites.
- **Blocking method**: start with solid color blobs, then chisel detail by adding colors. Best for small sprites where outlines waste space.
- Surface bumps: small shadow touches (1-2px dark spots) representing holes and surface irregularity.
- Granite texture: large-scale surface irregularities with small shadow spots.
- Marble texture: smoother than granite, requires more colors and anti-aliasing, accepts brighter specular highlights.

### Wall vs Floor Distinction

- Identical brick patterns can serve as wall or floor tiles -- the difference is **contrast and value**.
- Walls: brighter highlights on top edges, darker sides (suggesting front-lit vertical surface).
- Floors: more uniform value, slight top-down perspective darkening at edges.
- Wall brick heights should appear narrower than floor brick widths (bricks laid broad-side-down).

### Scale Notes

- **8x8**: abandon individual bricks. Use 2-3 color blocks suggesting stonework.
- **16x16**: 2-3 visible brick rows with simplified mortar. Focus on pattern, not individual brick detail.
- **32x32**: individual bricks with internal shading, visible mortar texture, cracks, and moss patches.

---

## 3. Wood

### Color Palette

- Warm brown ramp: 4-6 shades from dark shadow brown through mid-tones to pale highlight.
- Hue-shift: shadows toward deep reddish-brown or cool gray-brown; highlights toward warm tan/yellow.
- Include 1-2 cool mid-tones for shading transitions to avoid muddy ramps.
- Weathered wood: desaturate and shift toward gray. Fresh wood: warmer, more saturated browns.

### Pixel-Level Techniques

**Grain direction:**
- Grain runs parallel to the plank's long axis. Always.
- Represent grain with **short directional strokes** (2-4px) and strategic speckling within planks.
- Avoid long continuous lines -- they break the pixel grid and look like scratches, not grain.
- Grain lines should be the same hue as the base but 1 value step darker or lighter.

**Plank patterns:**
- Define planks with 1px gap/shadow lines between boards.
- Stagger plank ends at different positions to avoid grid-like regularity.
- Each plank should vary slightly in base color (shift by 1 shade) to simulate natural variation.

**Knots:**
- Small concentric circles or ovals: 3-5px diameter at 16x16, 5-10px at 32x32.
- Dark ring (1px) surrounding a slightly lighter center.
- Place sparingly -- 0-1 per plank at 16x16, 1-2 at 32x32.
- Grain lines curve around knots.

**Bark texture:**
- Rougher than plank wood -- use deeper crevices (darker lines) with wider spacing.
- Curved flowing lines following the trunk's vertical axis.
- Concentric or wavy patterns suggesting growth rings on cross-sections.
- More color variation than planks: mix browns, grays, and subtle greens for lichen.

**Weathered vs fresh wood:**
- **Fresh**: saturated warm browns, smooth grain, uniform plank color, subtle highlights.
- **Weathered**: desaturated gray-browns, more prominent grain (raised grain from erosion), uneven plank coloring, possible green/dark spots (rot/algae), split or warped plank edges.

### Workflow

1. Decide tile dimensions and plank layout (rhythm and seam placement).
2. Block in plank shapes with base brown tones and gap lines.
3. Add grain detail with directional 1px strokes in a slightly different value.
4. Place highlights (top/left of plank surface) and shadow (bottom/right, in gaps).
5. Add variation: shift individual plank colors, add knots, weathering spots.

### Common Mistakes

- Making grain lines too long or continuous -- creates a "scratched" look, not wood grain.
- Abrupt color transitions along grain lines cause a "dented" appearance. Imagine grain as a subtle gradient.
- Over-texturing: sprinkling too many dark pixels randomly creates noise, not wood.
- Uniform plank coloring across the whole tile makes the repeating pattern obvious.

### Scale Notes

- **8x8**: 1-2 visible plank divisions. Grain implied by 1-2 slightly-off-color pixels. Color alone communicates "wood."
- **16x16**: 2-4 plank divisions visible. Short grain strokes (2px). Possible single knot.
- **32x32**: 3-6 planks with clear gaps. Visible grain direction with multiple stroke lengths. Knots with detail. Weathering variation per plank.

---

## 4. Metal

### Color Palette

- **Steel/iron**: gray ramp with blue or cool undertones. Use the full value range (near-black to near-white) for high contrast.
- **Copper**: warm orange-brown base. Shadows shift toward **green** tones (oxidation hint). Wide color variation.
- **Gold**: yellow-orange ramp. Shadows shift toward deep amber/brown, highlights toward pale yellow-white.
- **Rust**: break the base metal ramp with warm orange-brown patches that don't follow the underlying shading logic.
- Avoid neutral gray (#808080) for metal -- always push slightly warm or cool.
- For very bright metal surfaces where white isn't enough, tint the specular a different color (e.g., blue on white metal).

### Pixel-Level Techniques

**Reflectivity spectrum (key concept):**

Metal reflects light, not color. The core technique is displaying **the reflection of light within the surface** using sharp contrast.

- **High reflectivity (polished/new)**: sharp, high-contrast highlights. Near-white specular on near-black shadow. Creates a "brand new" look. Highlight is small and focused (1-3px).
- **Low reflectivity (worn/matte)**: diffused, blurred light. Softer contrast. Suggests micro-scratches and wear across the surface. Light spread over larger area.
- The **"light-middle-dark-middle-light" banding pattern** is the signature of metal. Alternating bands of value across the surface.

**Specular highlight placement:**
- The specular matches the surface form -- it sits where the surface angle reflects the light source directly at the viewer.
- On a flat plate: single bright spot in the upper portion.
- On a long object (sword blade): the specular can be 2-3px long, positioned along the edge catching light.
- On curved armor: the highlight follows the curve, tight and bright.
- For extreme reflectivity, add **radiating light rays** (1px lines extending from the specular point).

**Rust rendering:**
- Rust is a **layered, organic phenomenon** -- not a flat color overlay.
- Base rust color: warm reddish-brown to orange-brown.
- Apply rust in irregular patches, concentrated at edges, joints, and lower surfaces where water collects.
- Rust patches should break the underlying metal's shading logic -- they have their own local light response.
- Layer multiple rust tones: dark brown (deep corrosion) under bright orange (fresh rust) with occasional dark red.
- At 16x16: 3-5px scattered rust patches. At 32x32: visible rust with internal color variation.

**Rivets:**
- At 16x16: single bright pixel (highlight) with single dark pixel below (shadow). 2px total per rivet.
- At 32x32: 2x2 or 3x3 circles with highlight on upper-left, shadow on lower-right.
- Space rivets at regular intervals along edges or seam lines.
- Rivet highlights should match the metal's specular color.

**Armor plating:**
- Define plate boundaries with 1px dark lines (shadow in the seam).
- Each plate gets its own local highlight responding to the light direction.
- Overlapping plates: upper plate casts 1px shadow onto the plate below.
- Beveled edges: 1px bright line on the light-facing edge of each plate.

### Material Differentiation Summary

| Property | Polished Steel | Matte/Worn | Rusty | Copper | Gold |
|---|---|---|---|---|---|
| Contrast | Very high | Medium | Low-medium | High | High |
| Highlight size | Tiny, sharp | Spread, soft | Absent on rust | Medium, warm | Medium, warm |
| Shadow hue | Cool blue-gray | Neutral dark | Dark brown | Green-shifted | Deep amber |
| Highlight hue | Near-white, possible blue tint | Light gray | Orange on rust spots | Bright orange | Pale yellow |
| Surface pattern | Smooth value bands | Subtle noise | Irregular patches | Smooth with patina spots | Smooth bands |

### Common Mistakes

- Using the same shading approach as matte materials. Metal demands **higher contrast** than stone or wood.
- Making rust a flat overlay instead of organic, irregular patches.
- Forgetting environmental reflections -- metal should hint at what surrounds it (e.g., bounce light from a nearby floor).
- Pillow-shading metal. Metal highlights are directional and sharp, never uniform edge-inward gradients.

### Scale Notes

- **8x8**: a single highlight pixel against a dark base communicates "metal." No room for rivets or rust detail.
- **16x16**: visible specular (1-2px), possible plate seam lines, 1-2 rivet dots, small rust patch if needed.
- **32x32**: full value banding pattern, multiple rivets, detailed rust patches, plate overlaps with cast shadows.

---

## 5. Water & Liquid

### Color Palette

- Deep water: dark blue-teal (3-4 shades from near-black blue to mid-blue).
- Shallow water: lighter blue-green, more saturated.
- Highlights/foam: near-white with a blue or cyan tint.
- Depth is communicated through color: darker = deeper, lighter = shallower.
- Transparency at edges: blend toward the color of the ground beneath (sand, stone) at shorelines.

### Pixel-Level Techniques

**Surface pattern (top-down, static):**
- Form interconnected blob shapes using **single-pixel-wide lines** on the water surface.
- Start with one blob, draw branching lines that connect into a network.
- These lines represent light catching surface ripples.
- Highlight lines: 1 value brighter than the base water color.
- Shadow areas: 1-2px shadow positioned a couple pixels below each bright line (suggesting wave depth).
- Add sparkle with occasional bright-white pixels along highlight lines.

**Wave motion (Slynyrd):**
- Foundation: sine wave patterns built from **chains of ovals**.
- Circular oval chains = short, deep waves. Wide ovals = shallow, expansive waves.
- Asymmetrical oval pairing is more realistic: wave crests are steeper than troughs.
- Tile the wave pattern and test before animating.

**Animation (minimal frames):**
- Create **2 distinct water texture tiles** with different patterns.
- Transition between them to create simple looping animation.
- Insert a 50% opacity blended intermediate frame for smoother motion.
- Water pixels can cycle in a 1px circuit: up, left, down, right -- creating undulation.
- For ripples: 4-frame loop moving outward from a disturbance point.

**Waterfall construction:**
- **Mouth** (top): brightest area with reflective white highlights where water launches.
- **Flow** (middle): vertical bands that sag and darken as they descend, eventually fragmenting into droplets.
- **Splash** (bottom): synchronized looping impact animation with foam particles.

**Foam and shoreline:**
- Foam at edges: 1-2px white/light cyan irregular line along the water boundary.
- Foam animates by shifting 1px along the edge per frame.
- At shorelines, transition: ground color -> wet ground (darker) -> foam line -> shallow water -> deep water.
- Use 2-3 intermediate tiles for convincing shore transitions.

**Depth indication:**
- Shallow water: semi-transparent -- show ground color blended with water tint at ~50%.
- Medium depth: water color dominates, faint ground hints.
- Deep water: solid dark blue-teal, no ground visibility.
- Caustic light patterns (bright squiggly lines on the bottom) visible in shallow areas only.

**Reflections:**
- Static reflections: flip the source image vertically, shift down, and add a few highlight pixels.
- Animated reflections: add subtle ripple offset. Keep wave speed, width, and height low for natural look.
- Position the ripple source outside visible area for directional flow illusion.

### Common Mistakes

- Making water patterns too regular/geometric -- natural water is chaotic.
- Forgetting that water reflects its surroundings -- lighter areas should relate to sky/cloud highlights.
- Using too many animation frames. 2-4 frames is sufficient for most tile water.
- Making foam a solid white line instead of an irregular, broken edge.

### Scale Notes

- **8x8**: 2-3 shades of blue. A single wavy highlight line across the tile. No room for foam detail.
- **16x16**: visible ripple network (3-5 connected blob shapes). 1px foam at edges. 2-3 depth zones possible.
- **32x32**: complex ripple networks, animated foam, visible caustics in shallow areas, multiple depth gradient steps.

---

## 6. Grass & Vegetation

### Color Palette

- 3-4 shades of green minimum: dark shadow green, 2 mid-tone greens, bright highlight green.
- Hue-shift: shadows toward blue-green or teal; highlights toward yellow-green.
- Add accent colors sparingly: 1-2 flower colors (red, yellow, purple) for life.
- Dirt/ground peek-through: a warm brown visible between grass clusters.
- Avoid pure bright green (#00FF00) -- it reads as artificial. Desaturate and add complexity.

### Pixel-Level Techniques

**Blade patterns:**
- Grass blades as short upward-pointing marks: 1-2px tall at 16x16, 2-4px at 32x32.
- Cluster blades into **small islands of detail** (3-6px groups) rather than uniform coverage.
- Highlight the tips of blade clusters (upper pixels lighter).
- Shadow at the base where blades meet ground (darker pixels at bottom of clusters).
- Directional consistency: blades should lean slightly in one direction (wind implication).

**Ground scatter:**
- Mix flat green areas with textured vegetation zones for natural gradation.
- **Negative space** between clusters reduces noise and lets the tile breathe.
- Small scattered elements: pebbles (1-2px gray), fallen leaves (1-2px brown/orange), tiny flowers (1-2px color accent).

**Flower placement:**
- Flowers as 1-3px color accents sitting atop grass clusters.
- Never more than 1-2 flowers per 16x16 tile to avoid overwhelming the grass.
- Place flowers at cluster edges, not floating in empty space.
- Use 1 warm accent color that contrasts with the green (red, yellow, or white).

**Sparse vs dense grass:**
- **Sparse**: large areas of dirt/ground visible (60-70% ground, 30-40% grass). Isolated blade clusters with wide spacing.
- **Dense**: nearly full coverage (80-90% grass). Overlapping clusters, minimal ground visibility, more uniform color.
- Create visual gradation by making tiles that range from sparse to dense for transition zones.

**Seamless tiling (critical):**
- Achieve **homogeneous visual weight distribution** -- no area should be noticeably busier or emptier.
- Create clusters that **overlap tile edges and wrap around** to the opposite side to hide seams.
- Avoid placing prominent features (flowers, rocks) at tile center -- they create obvious repetition.
- Use 4+ tile variants mixed randomly to hide the grid pattern.

### Common Mistakes

- Filling every pixel with texture detail -- grass needs breathing room.
- Using too many greens (5+) at 16x16 creates blurry noise instead of readable texture.
- Placing all detail at tile center, making the repeat grid obvious.
- Making all blade clusters the same size and spacing.

### Scale Notes

- **8x8**: 2-3 green values. Texture implied by value variation, not visible blades. A couple darker pixels suggest shadow between blades.
- **16x16**: visible blade clusters (3-5 per tile). Clear highlight/shadow distinction. 1 possible flower accent.
- **32x32**: individual blade groups with internal shading. Multiple flowers, pebbles, variety elements. Visible dirt between sparse clusters.

---

## 7. Sand, Dirt & Mud

### Color Palette

**Sand:**
- Warm yellow-tan ramp: 3-4 shades. Light cream highlight, warm golden mid-tone, soft brown shadow.
- Avoid orange -- push toward yellow for dry sand, toward brown for wet sand.

**Dirt:**
- Brown ramp: 3-4 shades. Dark earthy brown shadow, mid brown, lighter brown highlight.
- Hue-shift: shadows toward cool gray-brown, highlights toward warm tan.
- Dry dirt is lighter and grayer. Wet dirt is darker and more saturated.

**Mud:**
- Darker, more saturated version of dirt palette with glossy highlight.
- Add a near-black brown for deep wet areas.
- Single bright highlight pixel suggests wet reflective surface.

### Pixel-Level Techniques

**Sand texture (Slynyrd):**
- Create **long angled calligraphic 'S' shaped clusters** for a pleasing wavy dune pattern.
- Add irregularity to break up the wave shapes -- avoid patterns that look like "ramen noodles."
- Clusters should be 1px wide lines of a lighter or darker value on the base sand color.
- Do NOT connect all lines into a continuous path.
- Wind-blown appearance: orient wave clusters in a consistent diagonal direction.

**Dirt texture:**
- Busier, more detailed patterns than grass (dirt is visually noisy by nature).
- Use small irregular specks (1-2px) of lighter and darker values scattered on the base.
- Dry/cracked dirt: 1px dark lines forming irregular polygonal shapes (like dried mud flats).
- Flat clay: smoother, fewer specks, more uniform color with subtle value shifts.

**Mud texture:**
- Similar to wet dirt but with **glossy highlights** -- 1-2px bright spots suggesting standing water.
- Smoother than dry dirt -- fewer sharp specks, more gradual value shifts.
- Edge behavior: mud oozes -- transitions to adjacent materials should be irregular and blobby.

**Granularity at pixel scale:**
- At 16x16, individual sand grains are not visible. Granularity is implied by **color noise** -- subtly varying pixel values across the surface.
- Use colors very close to each other to create a blur/noise effect. The goal is NOT to attract the player's attention.
- At 32x32, you can hint at larger granularity with scattered 1px specks.

**Wet vs dry distinction:**
- **Dry**: lighter, lower contrast, more specks of variation.
- **Wet**: darker overall, higher contrast, occasional bright highlight (reflected light), smoother transitions.
- Wet areas can have a 1px bright edge where water meets the surface.

**Footprint impressions (32x32 only):**
- 2-3px darker oval/shoe-shape pressed into the surface.
- Slightly raised rim (1px lighter) around the impression edge.
- Not feasible below 32x32 resolution.

### Common Mistakes

- Making sand too yellow/orange -- real beach sand is more beige/cream.
- Over-texturing dirt to the point of visual noise. Use restraint.
- Forgetting that paired busy textures (grass next to dirt) can exhaust the eye. Vary density between adjacent materials.
- Using perfectly random pixel noise instead of structured clusters for sand.

### Scale Notes

- **8x8**: pure color communication. 2 values for the material. No visible texture pattern.
- **16x16**: visible S-curve clusters for sand (3-4 per tile). Scattered specks for dirt. Wet/dry communicated by value.
- **32x32**: complex dune patterns, visible cracked-earth lines, footprint details, individual pebbles in dirt.

---

## 8. Ice & Snow

### Color Palette

**Ice:**
- Cool blue ramp: 4-5 shades from near-black blue to pale ice blue to near-white.
- Desaturated blues -- ice is not vivid blue. Push toward gray-blue.
- Highlights can be pure white or very pale cyan for crystalline sparkle.
- Ice is reflective: **add hints of surrounding environment colors** on its surface to make it feel part of the scene.

**Snow:**
- Near-white base with blue-gray shadows. Very low contrast.
- 3-4 shades: white/off-white highlight, pale blue-gray mid-tone, medium blue-gray for deep shadow.
- Snow shadows are **always cool blue**, never warm. Shadow color should be the coldest hue in the palette.
- Fresh snow is brighter (closer to white). Old/packed snow is slightly yellow-gray.

### Pixel-Level Techniques

**Ice surface:**
- Use **small jagged lines or dots** (1-2px) to mimic the uneven surface of ice.
- Highlight edges using brighter colors along the outer contour where light hits.
- Transparency effect: show underlying surface color bleeding through the ice, blended with the ice tint.
- Crystalline highlights: 1px pure white or pale cyan specks scattered across the surface (like sparkle on facets).
- Crack lines: 1px bright lines (lighter than surrounding ice) representing internal fractures. Branch at sharp angles.
- Smooth ice vs rough ice: smooth = fewer surface marks, larger highlight areas. Rough = more jagged detail, broken-up highlights.

**Crystalline/faceted ice:**
- Define sharp geometric edges with alternating light and dark faces.
- Each facet gets a distinct value based on its angle to the light source.
- Anti-alias edges between facets sparingly (1px only) for a gem-like quality.

**Snow surface:**
- Fresh/powder snow: very smooth, minimal texture. Almost pure white with gentle blue shadows in recesses.
- Packed snow: more visible surface texture. Small dimples (1-2px darker spots) and slight unevenness.
- Snow depth: indicated by how much of underlying objects (rocks, grass) pokes through.
- Footprints in snow: similar to sand but with blue-gray shadow color in the impression and a brighter rim.

**Snow coverage on objects:**
- Snow accumulates on **top surfaces only** (top of walls, roofs, branches).
- 2-4px thick white band on the top edge of any horizontal or near-horizontal surface.
- Snow edge should be slightly irregular (not a perfect straight line).
- Where snow meets the object, use 1px shadow (blue-gray) to show the snow sitting on top.

**Ice transparency:**
- Layer approach: draw the object behind the ice first, then overlay the ice color at reduced opacity.
- In practice at 16x16: mix ice-blue pixels with background-color pixels in an alternating pattern.
- Thicker ice = fewer background pixels showing through. Thin ice = nearly half background pixels.

### Common Mistakes

- Making snow pure white everywhere -- shadows are essential for reading the surface shape.
- Using warm colors in snow/ice shadows. Snow shadows are always cool blue.
- Making ice look like water -- ice needs hard edges and geometric highlights, water is soft and flowing.
- Forgetting that ice reflects its environment. Pure blue ice with no environmental color looks plastic.

### Scale Notes

- **8x8**: white with 1-2 blue-gray shadow pixels for snow. Blue-white with 1 highlight pixel for ice.
- **16x16**: visible snow shadow shapes, ice crack lines (1-2), crystalline sparkle dots. Snow depth suggested by transition tiles.
- **32x32**: detailed ice fracture patterns, snow surface dimples, visible transparency layering, ice with embedded color from objects below.

---

## 9. Fabric & Cloth

### Color Palette

- Fabric palette depends entirely on the intended material and dye color.
- Rule of thumb: **3-5 shades per fabric color** (deep shadow, shadow, mid-tone, highlight, bright highlight).
- **Silk**: needs the full range -- very bright highlights against deep shadows for that sheen.
- **Wool/matte cloth**: compressed range -- highlights are not much brighter than mid-tones.
- **Leather**: warm brown ramp similar to wood but with more prominent specular highlight.

### Pixel-Level Techniques

**Fold shadows (the core of fabric rendering):**
- Every fold is essentially a **small cylinder**: it has a highlight on the raised ridge, mid-tone on the sides, shadow in the valley, and subtle bounce light at the deepest point.
- In pixel art at small scale, this simplifies to: bright pixel on fold crest, 1-2 dark pixels in fold valley.
- Fold direction follows gravity and the draping structure. Folds radiate from points of tension (shoulders, waist, grip points).

**Fabric material differentiation:**

| Material | Highlight | Shadow | Fold Character | Key Visual Cue |
|---|---|---|---|---|
| **Silk** | Very bright, sharp, small | Deep, high contrast | Few large smooth folds | Abrupt shift from bright white to deep shadow |
| **Wool** | Soft, spread | Gentle, low contrast | Many small rounded folds | Rough edges, short textured strokes, thick appearance |
| **Cotton** | Medium, fairly sharp | Medium contrast | Natural draping folds | Clean, smooth edges, moderate fold depth |
| **Leather** | Focused specular spot | Deep brown | Stiff, minimal folds | Shiny highlight, thick/rigid form, minimal draping |
| **Linen** | Moderate | Moderate | Many fine wrinkles | Thin parallel fold lines, crisp edges |

**Pattern rendering at small scale:**
- At 16x16, fabric patterns (stripes, checks, plaid) must be **extremely simplified**.
- Stripes: 1px wide lines that follow the fabric's surface contour (curving with folds).
- Checks/plaid: 2x2px blocks that distort along fold lines.
- At 8x8: patterns are impossible. Communicate material through shading behavior alone.
- At 32x32: simple patterns become readable. Stripes can be 2px wide with fold distortion.
- Patterns must **follow the 3D surface** -- they curve, compress in shadows, and stretch on highlights.

**Leather specifically:**
- Smoother than cloth, stiffer folds (fewer, broader).
- 1px specular highlight where light hits the surface directly.
- Surface texture: very subtle -- occasional 1px darker dot suggesting pores (32x32 only).
- Worn leather: lighter color on edges and contact points (knees, elbows, straps).

### Shadow Diffusion by Material

- **Glossy materials (silk, satin)**: sharp shadow terminator. Abrupt transition from light to dark.
- **Matte materials (wool, canvas)**: soft/diffused shadows. Gradual transition, possibly using dithering at the boundary.
- **Stiff materials (leather, heavy cloth)**: shadows with hard edges following the structural folds, not soft draping.

### Common Mistakes

- Rendering fabric like a hard surface -- fabric folds, drapes, and wrinkles even when still.
- Pillow-shading cloth items. Light still comes from a direction; fabric folds respond to gravity and tension, not uniform edge-inward gradients.
- Forgetting that fold structure changes with material weight. Silk falls in smooth curves; burlap crumples in harsh angles.
- Over-detailing fabric texture at small scales. At 16x16, shading alone communicates the material.

### Scale Notes

- **8x8**: shading behavior only. 2-3 values. A bright highlight pixel = shiny material. No visible folds.
- **16x16**: 1-3 visible folds suggested by highlight/shadow placement. Material type communicated by contrast range.
- **32x32**: multiple fold structures, visible pattern distortion, leather grain hints, clear material differentiation.

---

## 10. Tile Transitions & Edge Behavior

### Core Principles

- Where two materials meet, the **top material casts a shadow** onto the material below (1-2px darker zone).
- Shadow colors at transitions should be **a darker shade of the bottom material**, not pure black or the top material's shadow.
- Building a tileset requires fitting each tile with all adjacent tiles. No tile can be designed in isolation.

### Autotile Architecture (Celeste method)

- Use a **3-5 color system per tileset**: 1-2 light (edge highlights), 1 mid-tone (bulk fill), 1-2 dark (transition zones).
- Share a common **infill color** across all tilesets. This enables seamless blending without needing unique transition tiles for every material pair.
- Create 1-4 **randomized edge variants** per straight edge to break repetition.
- Between exterior edges and infill, use **randomized intermediate tiles** (abstract shapes: blobs, circles, diamonds) that blend materials into larger cohesive shapes.

### Transition Tile Types

**Straight edges (4):** top, bottom, left, right borders of a material region.
**Outer corners (4):** where two straight edges meet at a convex corner.
**Inner corners (4):** where the material wraps around a concave corner (the most forgotten tile type).
**Variants:** 2-4 per edge type for randomized placement.

Minimum tileset for autotiling: ~12-16 unique tiles (4 edges + 4 outer corners + 4 inner corners). With variants: 24-48 tiles.

### Material-Specific Edge Behavior

**Grass to dirt:**
- Grass overlaps dirt with irregular organic edge (not a straight line).
- 1-2px dirt shadow where grass casts shade.
- Individual grass blade pixels extend over the dirt edge.
- Dedicated connection tiles bridge the two textures.

**Stone to grass:**
- Hard stone edge (straight or angular) against soft grass.
- Grass can creep up 1-2px onto the stone face.
- Dark shadow line (1px) on the grass side at the stone base.

**Water to land:**
- Foam/highlight line at the water boundary (1px light).
- Wet zone on land side: 1-2px darkened ground color.
- Shallow water gradient: land color blended with water color for 2-3px.

**Sand to water:**
- Gradual blend: dry sand -> wet sand (darker) -> foam line -> shallow water -> deep water.
- Each zone is 2-4px wide at 16x16, more at 32x32.

**Snow to other materials:**
- Snow sits on top with an irregular edge.
- 1px blue-gray shadow at the snow boundary on the lower material.
- Can create transparent snow autotile by removing the base ground, making snow overlay onto anything.

### Edge Rendering Tips

- Include **transparent pixels at tile edges** to break the outer contour and hide the grid.
- Dark colors toward **bottom edges** simulate lighting and suggest 3D depth.
- Use **props** (foreground vegetation, debris, snow piles) as natural transition bridges between tiled areas.
- Pair busy textures with calmer ones. Two highly-detailed materials adjacent creates visual noise.

### The Shadow Rule

- Cast shadows predominantly to one side (left OR right) with slight downward angle.
- Keep shadows within single tiles to avoid layering conflicts.
- Regardless of actual object height, **all shadows should be the same length** in a tileset. While unrealistic, it is convincing and only noticeable under close scrutiny.

---

## 11. Common Mistakes

### Universal Anti-Patterns

1. **Pillow shading**: shading from the outline inward creates a "pillowy" blob. Always shade from a consistent light direction. Solution: define a light source and commit to it.

2. **Banding**: several pixels in a value ramp following a similar shape/path create an illusory line between value zones. Solution: compress bands so transitions happen quickly, or position banded edges at natural terminators where the eye expects them.

3. **Too many similar colors**: colors that lack individual identity blend together and reduce readability. Each color should do as much work as possible. Solution: increase contrast between colors and reduce palette size.

4. **Naive coloring**: using "pure" versions of colors (bright green grass, pure gray stone, bright blue water) without considering reflected light or environmental color. Solution: observe real-world color complexity, desaturate, and hue-shift.

5. **Excessive highlighting**: adding little highlights everywhere is satisfying but creates noise and robs objects of color identity. Reserve highlights for a few sweet spots. Metals get longer highlight ramps; matte materials rarely need prominent highlights.

6. **Orphan pixels**: single disconnected pixels that distract the eye and read as noise rather than intentional texture. Especially problematic across animation frames.

7. **Over-texturing**: cramming excessive detail creates visual noise. Consider the scale at which the art will be viewed. Viewers interpret random pixels as noise, not design.

8. **Black line overuse**: thick internal black outlines consume pixel space and flatten forms. Use lighter-colored lines for lighter adjacent areas. Reserve pure black for the darkest shadow edges only.

9. **Ignoring tile repetition**: placing prominent features at tile center makes the repeating grid obvious. Distribute visual weight evenly and create features that wrap across tile edges.

10. **Uniform texture density**: real materials have variation -- worn areas, damage, natural growth patterns. A perfectly uniform texture reads as artificial.

### Material-Specific Pitfalls

- **Stone**: making all stones the same gray regardless of environment. Match geological context.
- **Wood**: continuous grain lines that look like scratches. Use short, broken strokes.
- **Metal**: shading metal like a matte material. Metal demands higher contrast and sharper highlights.
- **Water**: perfectly regular ripple patterns. Natural water is chaotic.
- **Grass**: filling every pixel with detail. Grass needs negative space.
- **Sand**: perfectly random noise instead of structured wavy clusters.
- **Ice**: using warm colors in shadows. Ice shadows are always cool blue.
- **Fabric**: rendering cloth like a hard surface. Fabric always drapes and folds.

---

## Sources & Further Reading

### Primary Tutorial Authors

- **Slynyrd (Raymond Schlitter)** -- Pixelblog series: [slynyrd.com/pixelblog-catalogue](https://www.slynyrd.com/pixelblog-catalogue)
  - Pixelblog 2: Texture
  - Pixelblog 13: Rocks
  - Pixelblog 20: Top Down Tiles
  - Pixelblog 43: Top Down Tiles Part 2
  - Pixelblog 45: Bricks, Walls, Doors, and More
  - Pixelblog 10: Water in Motion

- **Pedro Medeiros (Saint11)** -- [saint11.art/blog/pixel-art-tutorials](https://saint11.art/blog/pixel-art-tutorials/)
  - Basic Shading (Pixel Grimoire series on Medium)
  - Metals, Vegetation, Rock Formations, Ruins tutorials on Lospec

- **Aran P. Ink** -- Celeste Tilesets Step-by-Step: [aran.ink/posts/celeste-tilesets](https://aran.ink/posts/celeste-tilesets)

- **Rappenem** -- Crystal, Ice, Metal Shading tutorials on DeviantArt

- **UszatyArbuz** -- Shading/Textures comprehensive guide on DeviantArt

- **AdamCYounis** -- YouTube pixel art tutorials, Apollo palette on Lospec

### Community Resources

- [Lospec Pixel Art Tutorials](https://lospec.com/pixel-art-tutorials) -- searchable database by tag (texture, metal, wood, water, etc.)
- [OpenGameArt Pixel Art Guide](https://opengameart.org/content/chapter-7-textures-and-dithering) -- Chapters 5 (Color), 7 (Texture/Dithering), 8 (Tiles)
- [Derek Yu -- Pixel Art Common Mistakes](https://www.derekyu.com/makegames/pixelart2.html)
- [Arne's Pixel Art Tutorial](https://androidarts.com/pixtut/pixelart.htm) -- comprehensive technique reference
- [Wolthera -- Animating Water Tiles](https://wolthera.info/2019/06/animating-water-tiles-part-1-edges/)
