# LLM-Based Tile Generation Research

Research compiled March 2026. Covers TileGPT, LLM-guided procedural content
generation, WFC+AI hybrids, pixel art diffusion, prompt engineering for tiles,
and RAG for creative generation. Focus on practical techniques applicable to
PIXL's narrate/resolve pipeline.

---

## 1. TileGPT (Autodesk Research, May 2024)

**Paper:** "Generative Design through Quality-Diversity Data Synthesis and
Language Models" — Gaier, Stoddart, Villaggi, Sudhakaran

**Core idea:** LLM generates high-level layout concepts, WFC resolves them into
constraint-valid detailed designs. The LLM is a "strategic director," not a
pixel-level generator. This is the same decomposition PIXL uses (narrate → resolve).

### Two-Stage Pipeline

1. **LLM Concept Phase:** Fine-tuned DistilGPT2 (96M params) with frozen BART
   text encoder and cross-attention for prompt conditioning. Outputs simplified
   tile categories as single characters ('A', 'B', 'C') on a 25x15 grid.
2. **WFC Refinement Phase:** Abstract categories expand into full tile variants
   (rotations, reflections). LLM outputs become pre-constraints (fixed boundary
   conditions) for WFC. Shannon entropy guides cell collapse ordering.

### Training Data via MAP-Elites

Random WFC sampling produces heavily skewed distributions — certain feature
combinations almost never appear. MAP-Elites (quality-diversity optimization)
fixes this by searching the space of WFC-generable designs across user-defined
feature dimensions, producing 50,000 labeled designs with uniform coverage.

**Key finding:** MAP-Elites training data is *essential* for prompt adherence.
Without it, the model ignores textual guidance. Gini coefficient analysis shows
MAP-Elites vastly outperforms random sampling for distribution uniformity.

### Training Details

- 500,000 steps, batch size 16, Adam optimizer
- Tokenization: tile complexities abstracted to functional categories
- Evaluated on 243 prompts x 100 designs = 24,300 generations

### Results & Limitations

- Superior fidelity across most feature categories
- Weak on "total units" — determined more by WFC stochasticity than LLM intent
- Domain was architectural site layouts (prefab housing), not game levels
- Limited to linear feature ranges; no non-linear conditioning
- No qualitative/aesthetic labels — purely quantitative metrics

### Relevance to PIXL

TileGPT validates the narrate→resolve decomposition. Their training data problem
(needing MAP-Elites to get diverse examples) maps directly to our challenge of
generating good PAX examples for few-shot prompting. Their tile abstraction
approach (categories as single chars) parallels PIXL's PAX TOML representation.

---

## 2. MarioGPT (NeurIPS 2023)

**Paper:** "MarioGPT: Open-Ended Text2Level Generation through Large Language
Models" — Sudhakaran et al.

TileGPT's direct ancestor. Fine-tuned GPT-2 on 37 annotated Super Mario Bros
levels from the Video Game Level Corpus (VGLC). Levels encoded as text strings,
each column a sequence of tile characters.

### Key Techniques

- Frozen BART encoder for text conditioning (same pattern as TileGPT)
- Levels represented as character sequences — one char per tile type
- Text prompts control features: "many pipes, few enemies, high elevation"
- 88% playable level rate without external agent validation
- Novelty scoring via edit-distance from training examples

### Practical Insight

MarioGPT shows that very small datasets (37 levels) suffice for fine-tuning
when the representation is compact and well-structured. The text encoding of
tile grids is the critical enabler — it converts spatial generation into
sequence prediction, which is exactly what language models do well.

---

## 3. Word2World (May 2024)

**Paper:** "Word2World: Generating Stories and Worlds through Large Language
Models" — Nasir, James et al.

No fine-tuning. Uses frontier LLMs (GPT-4, Claude 3) directly through
multi-step prompting to generate playable top-down RPG worlds from stories.

### Pipeline

1. LLM generates 4-5 paragraph story with 8 objectives
2. LLM extracts: characters, tile info, goals, walkable tiles, interactive tiles
3. Two-step generation: environment tiles first, then character/object placement
4. Multiple refinement rounds with evaluation feedback
5. DistilBERT embeddings match tile descriptions to visual assets (cosine similarity)
6. AStar pathfinding validates traversability (max 1000 iterations)

### LLM Comparison Results

| Model           | Playability | Coherence | Completion |
|-----------------|-------------|-----------|------------|
| Claude 3 Opus   | 8/10        | 10/10     | 7/10       |
| GPT-4 Turbo     | 9/10        | 9/10      | 10/10      |
| Claude 3 Sonnet  | 7/10        | 7/10      | 7/10       |
| GPT-3.5 Turbo   | 7/10        | 5/10      | 5/10       |

### Critical Insight

"Direct generation through prompts is very unlikely to give desired results" —
the decomposed pipeline is mandatory. Two-step generation (environment then
characters) achieved 90% playability vs. much worse for single-step. Goal
extraction is critical — omitting it reduced playability to near zero.

Cost: $0.50-$1.00 per generation via API. Larger models consistently better.

### Relevance to PIXL

Word2World's approach is closest to PIXL's narrate flow. The finding that
decomposed multi-step prompting dramatically outperforms single-shot generation
validates our narrate→resolve split. Their asset matching via embeddings is
worth studying — we could use similar semantic matching for theme selection.

---

## 4. Narrative-to-Scene Generation (2025)

**Paper:** "Narrative-to-Scene Generation: An LLM-Driven Pipeline for 2D Game
Environments" — arxiv 2509.04481

Builds on Word2World with stronger spatial reasoning. Uses GameTileNet dataset
for semantic tile retrieval.

### Pipeline

1. LLM generates ~100-word adventure story
2. Extracts three temporal keyframes
3. Decomposes into "[Object] [Relation] [Object]" predicate triples
4. Semantic matching via all-MiniLM-L6-v2 Sentence Transformer embeddings
5. Procedural terrain via Cellular Automata
6. Constraint-driven object placement
7. Multi-layer visual composition

### GameTileNet Dataset

Semantic dataset of low-res game tiles from OpenGameArt.org. Five affordance
types: Terrain, Environmental Object, Interactive Object, Item/Collectible,
Character/Creature. Annotated with names, group labels, supercategories.
Published at AAAI AIIDE 2025.

### Spatial Constraint Handling

Normalizes natural language relations to controlled ontology (above/below,
left/right, on-top-of). Two-phase placement: random initialization on walkable
terrain, then iterative constraint refinement.

Results: 72% predicate satisfaction (range 56-89%). Cosine similarity for tile
matching averages 0.41. Affordance match rate 0.42 (high variance 0.27-0.55).

### Key Limitation

Visual similarity does not guarantee gameplay semantics. A "lantern" might match
to a "torch" visually but have wrong affordances. The gap between embedding
space proximity and functional game behavior remains unsolved.

---

## 5. WFC + AI Hybrid Approaches

### Markovian WFC (2025)

**Paper:** "A Markovian Framing of WaveFunctionCollapse" — arxiv 2509.09919

Reformulates WFC as a Markov Decision Process. State: binary tensor of tile
feasibility at each cell. Action: logit vector specifying tile choice at next
uncollapsed cell. Invalid tiles masked to zero probability.

**Key result:** MDP-based evolution converges 84% of runs vs. 16% for baselines
on path-length objectives. WFC handles constraint propagation; the learned agent
handles aesthetic/objective optimization.

**Practical takeaway:** Don't make neural networks learn adjacency constraints.
Externalize constraints to WFC, let the learned component focus on high-level
objectives. Explicitly encoding adjacency constraints outperforms implicit
learning in highly constrained domains.

### RL + WFC (FDG 2021)

Deep RL agent decides *which tile to place* at each WFC step. WFC determines
*where* to place (minimum entropy cell). Three-step loop: define tiles and
connectivity, WFC selects location, RL selects tile.

Applied to architectural space generation. Enables goal-driven generation
(maximize room connectivity, minimize corridors) while maintaining WFC's
constraint satisfaction guarantees.

### Nested WFC (2024)

Hierarchical WFC with meta-tiles and leaf tiles. Abstract nodes represent
functional regions (room, corridor), which decompose into concrete tile sets.
Enables large-scale generation without exponential constraint complexity.

### Growing Grid WFC (FDG 2020)

Augments WFC with Growing Grid neural network. Accepts high-level description
as image, returns solutions based on learned parameters and module constraints.
Combines spatial learning with WFC's local consistency.

### Consensus for PIXL

The research consistently shows: **keep WFC for constraint satisfaction, add
intelligence on top for objective optimization.** The narrate→resolve pattern
(LLM plans, WFC resolves) is the winning architecture. Attempts to replace WFC
with pure neural approaches lose the hard constraint guarantees that make
generated content actually valid.

---

## 6. Pixel Art Diffusion Models

### SD-piXL (October 2024)

**Paper:** "SD-piXL: Generating Low-Resolution Quantized Imagery via Score
Distillation" — arxiv 2410.06236

The most technically rigorous approach to palette-constrained pixel art
generation. Does not train a new model — uses pretrained SDXL via score
distillation.

**Architecture:** Operates on H x W x n logit tensor (n = palette size). For
each pixel position, logits determine probability of selecting each palette
color. Gumbel-Softmax reparameterization enables differentiable optimization
through discrete color choices.

**Palette enforcement:** Two modes:
- Argmax: hard constraint, guaranteed palette adherence, no interpolation
- Softmax: convex color combinations, smoother but relaxed

**Score distillation:** Latent SDS loss has noise term (variance reduction) and
semantic term (prompt guidance, scale factor s=40). ControlNet conditioning on
Canny edges and DPT depth maps provides structural guidance.

**Additional:** FFT-based smoothness loss (weight 20) minimizes high-frequency
artifacts. Initialization from input images via nearest-palette-color mapping.

**Relevance to PIXL:** SD-piXL's logit-per-pixel-per-palette-color approach is
the right mathematical framework for palette-constrained generation. If PIXL
ever adds a diffusion bridge, this architecture handles the palette problem
correctly — no post-hoc quantization needed.

### Retro Diffusion (2025)

Commercial product built on FLUX architecture. Key technical details:

- FLUX Dev naturally produces near-grid-aligned pixels; additional training
  closes the gap
- Style variation through prompt engineering alone (no per-style LoRAs)
- Supports styles: general, retro, simple, detailed, anime, game asset,
  portrait, texture, UI, item sheet, character turnaround, 1-bit
- Post-processing pipeline: clustering for ideal colors per section, then
  color quantization
- **Honest admission:** "models still have trouble being specifically limited
  to a set number of colors, or generating perfect sections of squares"
- 1-bit mode (two colors) is hardest — every pixel placement becomes critical
- Directional seamless tiling forthcoming but not shipped yet

### Pixel Art XL LoRA (nerijs/pixel-art-xl)

Community LoRA for SDXL. Widely used baseline for pixel art generation. Works
well for single sprites; less reliable for tilesets or sheets with spatial
structure. Available on HuggingFace and Civitai.

### 8bitdiffuser 64x

LoRA specifically targeting 64x64 pixel art. Claims perfect grid alignment at
that resolution. Multiple versions (v1-v4) suggest iterative improvement of
grid fidelity — the hardest problem in diffusion-based pixel art.

### Key Challenges (Across All Approaches)

1. **Palette adherence:** Models cannot reliably limit to N colors without
   post-processing. SD-piXL solves this mathematically; all others approximate.
2. **Grid alignment:** Sub-pixel artifacts and anti-aliasing creep in. FLUX
   is better than SD; neither is perfect.
3. **Tileset coherence:** Generating a single sprite is much easier than
   generating a coherent tileset where pieces fit together.
4. **Edge compatibility:** No diffusion model natively handles tile edge
   matching constraints. Requires either post-processing or WFC-style
   constraint solvers.

---

## 7. Prompt Engineering for Tile Generation

### Constraint Priority (PixelLab Findings)

State style and color constraints *before* the subject. "pixel art, 4-color
palette, 16x16, dark fantasy, wall tile" beats "wall tile in pixel art style."
Leading with constraints primes the model and reduces constraint violations.

### The Realism Trap

Conflicting constraints cause silent failures. "Photorealistic knight, 4-color
palette" — the model ignores the palette because realism requires many colors.
Rule: never combine "realistic/detailed/photorealistic" with strict palette or
resolution constraints.

### PixelLab Inpainting Workflow

For tileset generation via inpainting (applicable to any inpainting model):

1. Start with 128x128 canvas (8x8 tiles at 16x16)
2. Generate first section with full inpaint mask
3. Expand outward with partially overlapping selections
4. Critical: leave non-inpainted reference areas within each selection
5. Description should describe what's in the *middle* of the selection
6. Adjust init image strength between passes for consistency

**Why this works:** The model can only see content inside the selection. Overlap
provides context continuity. Full inpainting of the entire selection destroys
all reference context.

### LLM Tool-Calling for Pixel Art (Miranda, 2025)

Tested five LLMs drawing pixel art via Aseprite MCP server with primitive tools
(draw_pixels, draw_line, draw_rectangle, fill_area, draw_circle).

**Results:**
| Model          | Score (0-4) |
|----------------|-------------|
| Claude Opus 4  | 2.5         |
| Claude Sonnet 4 | 2.0        |
| GPT-4.1        | 1.25        |
| GPT-4o         | 0.75        |
| Qwen 3 32B     | 0.25        |

Claude Opus 4 maintained consistent character concepts across attempts. Qwen
"blamed the tools" when results were poor. Static images far easier than
animation sequences. The author notes MCP server development requires
"significant time investment" — tooling overhead is real.

### Negative Prompts

Use negative prompts to exclude: anti-aliasing, gradients, blur, glow, noise,
transparency, photorealism, high resolution, smooth shading. These are the most
common failure modes in AI pixel art generation.

---

## 8. RAG for Creative/Visual Generation

### Survey Landscape (2025)

"Retrieval Augmented Generation and Understanding in Vision: A Survey and New
Outlook" (arxiv 2503.18016) catalogs three RAG frameworks for visual tasks:

1. **Text-based RAG:** LLMs retrieve factual context to guide image generation.
   FAI (Fact-Augmented Intervention) injects demographic/historical knowledge.
2. **Vision-based RAG:** RA-Diffusion retrieves reference images from database
   during generation. Smaller models + large retrieval DB can match larger
   models. ImageRAG dynamically retrieves relevant images per text prompt.
3. **Multimodal RAG:** ReMoDiffuse combines semantic and kinematic similarity
   for hybrid retrieval. Cross-modal fusion during generation.

### Practical Techniques

- **iRAG:** Tackles "factuality hallucination" by augmenting generated objects
  with retrieved real images, improving realism of specific objects.
- **RealRAG:** Self-reflective contrastive learning for fine-grained object
  generation; outperforms on unseen object categories.
- **Domain-specific RAG:** Garment, traffic, architectural layout databases
  enable targeted generation for specific visual domains.

### Key Insight for PIXL

RAG reduces training requirements — a smaller model with access to a good
retrieval database can match a larger model's output quality. For PIXL, this
means our PAX theme files and example tilesets function as a retrieval corpus.
Few-shot examples in prompts are essentially manual RAG. A more systematic
approach would embed theme/tileset descriptions and retrieve relevant examples
based on the narrate prompt.

The GameTileNet dataset's embedding-based retrieval (all-MiniLM-L6-v2, cosine
similarity) is a concrete implementation of creative RAG — narrative entities
are embedded and matched to visual tiles. This achieves 0.41 average cosine
similarity and 72% spatial predicate satisfaction.

---

## 9. Synthesis: What This Means for PIXL

### Validated Architecture

Every successful system in this survey uses the same decomposition PIXL uses:
- TileGPT: LLM concept → WFC resolve
- Word2World: LLM story → LLM tiles → pathfinding validation
- Narrative-to-Scene: LLM narrative → predicate extraction → constraint placement
- Markovian WFC: learned objective → WFC constraint propagation

The narrate→resolve split is not just convenient — it is the empirically
dominant architecture. No system that asks a single model to handle both
creative planning and constraint satisfaction performs well.

### Training Data Quality Matters More Than Quantity

- MarioGPT: 37 levels, compact text encoding, works well
- TileGPT: MAP-Elites needed for diverse examples; random sampling fails
- Word2World: zero training, multi-step prompting, $0.50-$1.00/generation

For PIXL: curate PAX examples carefully. A small set of high-quality,
diverse examples beats a large set of redundant ones. MAP-Elites-style
diversity search over WFC outputs could generate better few-shot examples.

### The Palette Problem Is Unsolved in Diffusion

Every diffusion approach struggles with strict palette limits. SD-piXL's
logit-space formulation is the only mathematically correct solution, but
requires optimization per image (not real-time). Retro Diffusion uses
post-processing quantization and admits imperfection. For PIXL, the current
approach (WFC with explicit tile definitions) sidesteps this entirely —
palettes are enforced by construction, not learned.

### Semantic Gap Between Visual Similarity and Game Function

Narrative-to-Scene's affordance mismatch (0.42 match rate) highlights a
fundamental problem: embeddings capture visual similarity, not gameplay
semantics. PIXL's PAX format explicitly encodes tile roles (wall, floor,
door) rather than relying on visual similarity, which is the right approach.

### Edge Compatibility Remains a Hard Constraint Problem

No AI model reliably generates tiles with matching edges. WFC solves this by
construction (adjacency rules). Diffusion models need either post-processing
or explicit constraint conditioning (ControlNet on edge maps). PIXL's
WFC-based resolve step handles this correctly.

### Implementation Status (March 2026)

The TileGPT architecture has been implemented in PIXL's training pipeline:

- **MAP-Elites data synthesis** (`training/map_elites.py`): pyribs-based QD search over WFC parameter space (tile weights, seeds, predicates). 2D archive on wall_ratio × room_count. Achieved 75-80% coverage across 8 themes, producing ~1,250 diverse labeled maps.
- **Feature-conditioned training** (`training/prepare_me_data.py`, `train_me.sh`): LoRA fine-tune on Qwen2.5-3B with structured labels (`theme:X, size:WxH, layout:open, rooms:few, border:enclosed`). Rank 16, 24 layers, epoch-by-epoch with resume.
- **LM + WFC generation** (`training/generate_map.py`): Free-text prompt → structured label → LM tile-name grid → WFC refinement with graceful degradation (pin reduction on contradiction).
- **CLI extensions**: `--weight`, `--pin`, `--format json` flags on `pixl narrate` enable Python↔Rust interop for the ML pipeline.
- **Image pipeline improvements** (`training/prepare_matched.py`): Rich feature labels (density, symmetry, edge complexity) and stratified sampling for uniform dataset coverage.

Full guide: [docs/guides/map-generation-training.md](../guides/map-generation-training.md)

---

## Sources

- [TileGPT](https://tilegpt.github.io/) — Gaier et al., Autodesk Research
- [MarioGPT](https://github.com/shyamsn97/mario-gpt) — NeurIPS 2023
- [Word2World](https://arxiv.org/abs/2405.06686) — Nasir et al., 2024
- [Narrative-to-Scene](https://arxiv.org/abs/2509.04481) — 2025
- [GameTileNet](https://arxiv.org/abs/2507.02941) — AAAI AIIDE 2025
- [Markovian WFC](https://arxiv.org/abs/2509.09919) — 2025
- [SD-piXL](https://arxiv.org/abs/2410.06236) — 2024
- [Retro Diffusion](https://retrodiffusion.ai/) / [Technical Blog](https://runware.ai/blog/retro-diffusion-creating-authentic-pixel-art-with-ai-at-scale)
- [Pixel Art XL LoRA](https://huggingface.co/nerijs/pixel-art-xl)
- [PixelLab Tile Guide](https://www.pixellab.ai/docs/guides/map-tiles)
- [LLM Pixel Art via MCP](https://ljvmiranda921.github.io/notebook/2025/07/20/draw-me-a-swordsman/)
- [RAG Vision Survey](https://arxiv.org/abs/2503.18016) — 2025
- [WFC + RL](https://dl.acm.org/doi/fullHtml/10.1145/3472538.3472541) — FDG 2021
- [PCG with LLMs Survey](https://arxiv.org/abs/2410.15644) — 2024
