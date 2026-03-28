# Cognitive Vault Integration with PIXL Studio

## The Problem CV Solves for PIXL

PIXL Studio's AI Expert Chat currently loads a static ~8K token knowledge base into the system prompt. This covers color theory, dithering, tileability, PIXL format spec, game art patterns, platform constraints, and style vocabulary. It works for MVP, but it has three limits:

1. **Fixed budget.** 8K tokens is a surface-level overview of each topic. The LLM knows dithering exists but doesn't know when ordered Bayer is better than pattern dithering for NES-style water tiles at 4 colors. Deeper knowledge means a bigger system prompt, which eats into the conversation budget.

2. **Static.** When you discover a better edge declaration strategy or a new WFC constraint pattern, you edit the system prompt and redeploy. There's no way to grow the knowledge base without bloating context.

3. **No provenance.** The LLM asserts "use value contrast of at least 30% between adjacent ramp colors" but can't tell you *where* that rule comes from. Is it from the PixelLogic book? A GDC talk? A convention you made up? When the rule produces bad results, there's nothing to audit.

**Cognitive Vault replaces the static knowledge base with a queryable, token-efficient, source-tracked knowledge store.** The LLM gets deeper answers at lower token cost, and every answer cites its source.

---

## Architecture: Two Channels

CV connects to PIXL through two channels — not one. Each serves a different purpose:

| Channel | Who calls it | When | Why |
|---------|-------------|------|-----|
| **REST API** | PIXL's Rust backend | Automatically, before and during generation | Backend has full control — pre-fetches knowledge, enriches prompts, validates against stored patterns. The LLM never even knows it happened. |
| **MCP** | The LLM itself | During conversation, when it needs to reason about domain knowledge | The LLM decides it needs deeper context and queries CV directly. Conversational, exploratory. |

This is the difference between the system being smart (API) and the LLM being smart (MCP). Both matter.

```
┌──────────────────────────────────────────────────────────────┐
│                    PIXL Studio (Flutter)                      │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ AI Chat  │  │   Canvas     │  │  Tools / Palette       │ │
│  └────┬─────┘  └──────────────┘  └────────────────────────┘ │
│       │                                                      │
│  ┌────▼──────────────────────────────────────────────────┐   │
│  │  Rust Backend (pixl-core + pixl-mcp)                  │   │
│  │                                                        │   │
│  │  ┌─────────────────────┐  ┌─────────────────────────┐ │   │
│  │  │ Prompt Enrichment   │  │ Tile Validation         │ │   │
│  │  │                     │  │                         │ │   │
│  │  │ Before every LLM    │  │ After every generation  │ │   │
│  │  │ call: query CV API  │  │ check tile against      │ │   │
│  │  │ for relevant domain │  │ stored edge patterns    │ │   │
│  │  │ knowledge, inject   │  │ and accepted tiles      │ │   │
│  │  │ into prompt context │  │ via CV API              │ │   │
│  │  └────────┬────────────┘  └───────────┬─────────────┘ │   │
│  │           │ CV REST API               │ CV REST API    │   │
│  │           │                           │                │   │
│  └───────────┼──────────┬────────────────┼───────────────┘   │
│              │          │                │                    │
│         ┌────▼────┐ ┌───▼────┐    ┌─────▼──────┐            │
│         │ LLM     │ │pixl-mcp│    │  CV MCP    │            │
│         │ (Claude)│ │validate│    │  cv_query   │            │
│         │         │ │render  │    │  cv_expand  │            │
│         │         │ │wfc     │    │  cv_verify  │            │
│         └─────────┘ └────────┘    └─────┬──────┘            │
└──────────────────────────────────────────┼──────────────────┘
                                           │
                              ┌────────────▼──────────────┐
                              │    Cognitive Vault         │
                              │                           │
                              │  REST API (:4000)         │
                              │  POST /query              │
                              │  POST /entries/bulk       │
                              │  GET  /documents/{id}     │
                              │                           │
                              │  MCP Server (stdio)       │
                              │  cv_query, cv_expand, ... │
                              │                           │
                              │  ANCS + CCE (internal)    │
                              └───────────────────────────┘
```

### Channel 1: REST API (Backend-Driven, Automatic)

The Rust backend calls CV's REST API directly. The artist and the LLM don't see these calls — they happen behind the scenes.

**Prompt Enrichment (before every LLM call):**
```
Artist types: "Generate a 16×16 dungeon wall tile, weathered stone"

Before the LLM sees this prompt, the Rust backend:

1. POST /api/v1/vaults/pixl-knowledge/query
   { "query": "dungeon wall tile weathered stone pixel art technique",
     "maxTokens": 1500,
     "trustFilter": ["SUPPORTED"] }

2. CV returns 1,500 tokens of domain knowledge:
   - Stone texture technique (3-value noise, mortar lines)
   - 3 reference tiles in PAX format
   - Artist's previous rejection notes for this tile type

3. Backend injects this into the LLM prompt AUTOMATICALLY:
   ┌─────────────────────────────────────────────┐
   │ System: PIXL spec + theme + palette         │
   │                                             │
   │ [CV Domain Context — auto-injected]         │
   │ Stone texture: 3-value noise, base→mid→     │
   │ highlight. Mortar lines 2-3px. Reference:   │
   │ (3 PAX tiles). Prior feedback: clustered    │
   │ moss, not scattered.                        │
   │                                             │
   │ User: "Generate a 16×16 dungeon wall tile,  │
   │        weathered stone"                     │
   └─────────────────────────────────────────────┘

The LLM gets domain expertise without making a tool call.
```

**Tile Validation (after generation):**
```
LLM generates a tile → Rust backend validates edges via pixl-core

But also:

1. POST /api/v1/vaults/pixl-knowledge/query
   { "query": "edge patterns wall tiles dark_fantasy",
     "maxTokens": 500 }

2. CV returns: all accepted edge patterns for this theme

3. Backend checks: does the new tile's edge pattern match any
   known-good patterns? If not → warning to artist:
   "This tile has a novel edge pattern not seen in your existing
    tileset. WFC may not find valid placements."
```

**Project History (after accept/reject):**
```
Artist accepts a tile → Rust backend:

1. POST /api/v1/vaults/pixl-knowledge/entries/bulk
   { "entries": [{
       "title": "wall_stone_007 — accepted",
       "content": "<PAX source>",
       "tags": ["wall", "stone", "dark_fantasy", "accepted"],
       "enrichment": {
         "summary": "16×16 weathered stone wall, clustered moss",
         "conceptTags": ["stone-texture", "moss-accent", "WFC-compatible"]
       }
   }]}

Artist rejects a tile → same, but tagged "rejected" + reason:
   "tags": ["wall", "stone", "dark_fantasy", "rejected"],
   "enrichment": { "summary": "Rejected: moss too uniform, edges inconsistent" }
```

Every accept/reject feeds back into the vault. Future generations benefit from this history — automatically, via prompt enrichment.

### Channel 2: MCP (LLM-Driven, Conversational)

The LLM calls CV MCP tools when it's *reasoning* — during conversation, during SELF-REFINE, when the artist asks a question.

```
Artist: "My water tiles look flat. How do I make them feel deeper?"

The LLM decides it needs domain knowledge → calls cv_query tool:
  { query: "pixel art water depth color theory limited palette" }

CV returns compressed expert knowledge with sources.

LLM synthesizes an answer citing Color & Light p.204 and the
artist's current palette values — then suggests a specific fix.
```

The LLM also uses MCP during the SELF-REFINE loop:
```
Pass 1: tile has edge gap → validation fails
Pass 2: LLM calls cv_query("WFC edge gap stone tile fix")
        → gets the root cause (moss in edge rows) + fix principle
        → patches intelligently, not just the one pixel
```

### Why Both Channels

| Scenario | API | MCP | Why |
|----------|-----|-----|-----|
| **Prompt enrichment** | **Yes** | No | Backend knows what the artist asked. Fetches relevant knowledge before the LLM runs. No tool call overhead, no wasted tokens on the LLM deciding to query. |
| **Post-generation validation** | **Yes** | No | Backend checks new tiles against stored patterns. Deterministic, fast, doesn't need LLM reasoning. |
| **Project history write-back** | **Yes** | No | Accept/reject events come from the UI. Backend writes to CV directly. |
| **Artist asking a technique question** | No | **Yes** | The LLM needs to reason about the answer. It queries CV, synthesizes, explains. |
| **SELF-REFINE deep investigation** | No | **Yes** | The LLM is debugging a failed tile. It needs to explore — query, read, understand, fix. |
| **Narrative-to-map** | Both | **Yes** | Backend pre-fetches existing tiles for the theme (API). LLM queries design principles for layout (MCP). |
| **GameTileNet reference lookup** | **Yes** | Fallback | Backend knows the artist's current tile type/theme, pre-fetches relevant examples. LLM queries only if it wants to explore further. |

**API = system intelligence.** PIXL's backend is smart about what knowledge to fetch and when.
**MCP = model intelligence.** The LLM is smart about what questions to ask and how to use the answers.

Together: every LLM call arrives with relevant domain context pre-loaded (API), and the LLM can dig deeper when it needs to (MCP).

---

## What Gets Ingested

### Vault: `pixl-knowledge`

The vault is pre-populated with domain reference material, organized by topic. Each document is ingested through Ingestible's pipeline (parse → chunk → enrich) and stored in ANCS with full provenance, entities, and relations.

#### Core Domain Knowledge

| Topic | Sources | What the LLM Learns |
|-------|---------|---------------------|
| **Color theory** | Pixel Logic (book), Color & Light by Gurney, pixel art color ramp tutorials | How to build ramps with hue shifting, when to break value rules, warm/cool contrast for depth |
| **Dithering** | PixelJoint tutorials, academic papers on ordered dithering, GBA/NES hardware docs | Which dithering pattern works at which resolution, Bayer matrix math, why checkerboard fails at 8x8 |
| **Animation** | The Animator's Survival Kit (adapted), pixel animation guides, frame timing tables | Walk cycle frame counts by sprite size, squash/stretch at pixel scale, anticipation frames |
| **Tileability** | WFC papers (Gumin 2016, N-WFC 2023), Boris the Brave's WFC tips, Wang tile theory | Edge constraint design, corner behavior vs content, autotile variant requirements (13/47/256) |
| **Platform constraints** | NES/SNES/GBA/GB hardware reference docs | 4 colors/sprite, 16 colors/tile, scanline limits, palette bank switching |
| **Game design patterns** | GDC talks (transcripts), game design pattern catalogs, level design guides | Dungeon tile anatomy, overworld biome transitions, UI element conventions |
| **Style vocabulary** | Art direction documents, style guide templates, mood board methodology | How "gritty" translates to palette decisions (desaturated, high contrast, noise) |
| **PIXL format** | PIXL_SPEC.md, PAX plan, edge declaration reference | Full syntax, micro-tile patterns, theme system, semantic symbols — the living spec |

#### GameTileNet Corpus (2,142 tiles)

The GameTileNet stamp corpus from Chen & Jhala (AAAI AIIDE 2025) gets ingested as structured entries:

```
For each tile:
  - Content: PAX source (converted at build time)
  - Tags: affordance (walkable, obstacle, hazard, collectible), biome, style
  - Relations: edge-compatible-with, variant-of, same-theme-as
  - Metadata: original source (OpenGameArt), license (CC), dimensions
```

The LLM can query: "show me all 16x16 walkable grass tiles in nature theme" and get back source examples + edge declarations it can reference when generating new tiles.

#### Living Project Knowledge

As artists use PIXL Studio, the vault accumulates project-specific knowledge:

| What | When Stored | How Used Later |
|------|------------|----------------|
| **Accepted tiles** | Artist clicks "Accept → Canvas" | "Generate a tile that matches the style of the ones I already accepted" |
| **Rejected tiles + reason** | Artist clicks "Reject" + optional note | "Avoid the pattern that was rejected in tile X" |
| **Palette decisions** | Artist modifies theme palette | "The dark_fantasy palette was adjusted — moss green is now #2d5a27, not #3a7a3a" |
| **WFC validation results** | Validation runs on export | "These edge patterns are known-good with the existing tileset" |
| **Design conversations** | Chat history (opt-in) | "The artist prefers hard outlines over self-outlines in this project" |

This knowledge is tracked with TruthKeeper dependencies. If the artist changes the palette, every tile that was generated with the old palette gets marked STALE.

---

## How It Works in Practice

### Example 1: Generating a Dungeon Wall Tile

**Without CV (current MVP approach):**
```
System prompt includes ~200 tokens on "dungeon tiles":
  "Dungeon walls should use solid edge rows for WFC compatibility.
   Use shadow colors in corners. Moss accents ≤10%."

LLM generates tile based on this thin guidance.
```

**With CV:**
```
1. Artist types: "Generate a 16×16 dungeon wall tile, weathered stone,
   occasional moss, dark fantasy theme"

2. LLM calls cv_query:
   { query: "dungeon wall tile design weathered stone pixel art",
     maxTokens: 1500 }

3. CV returns (compressed to 1,500 tokens):
   - From Pixel Logic ch.7: "Stone texture requires 3-value noise:
     base → mid → highlight. Never place highlight adjacent to shadow
     without a mid-tone bridge. Weathering = irregular dark patches
     at mortar lines, 2-3px wide."
   - From GameTileNet: 3 reference dungeon wall tiles in PAX format
     with edge declarations (actual source the LLM can study)
   - From project history: "Artist rejected tile_wall_003 because
     moss was too uniform — use clustered placement, not scattered"
   - Source: [Pixel Logic p.134, GameTileNet #0847, session 2026-03-20]

4. LLM generates tile with deep understanding of stone texture technique,
   real reference examples, and knowledge of this artist's preferences.
```

Token cost: +1,500 for the CV query. But the system prompt dropped from ~8K to ~3K (just PIXL spec + active theme). Net: **same budget, deeper knowledge, better results.**

### Example 2: SELF-REFINE Loop with Domain Knowledge

```
Pass 1: LLM generates tile → validation: "edge row 3 has a gap at col 12"

Pass 2: LLM calls cv_query:
  { query: "fixing WFC edge gaps in dungeon tiles, stone texture continuity" }

  CV returns: "Edge gaps in stone tiles are typically caused by placing
  moss or highlight symbols in edge rows. Fix: constrain edge rows to
  primary structure symbol (#) only. Interior moss should stop 1px
  before edges. Reference: Boris the Brave WFC tips, section 'Designing
  for Edges'." [Source: wfc-tips.md, p.3]

  LLM patches col 12 row 3, also preemptively checks rows 0 and 15.

Pass 3: Visual check passes. Tile accepted.
```

The LLM didn't just fix the symptom — it understood the root cause (moss in edge rows) because CV gave it the relevant WFC design principle.

### Example 3: Palette Assistance

```
Artist: "My water tiles look flat. How do I make them feel deeper?"

LLM calls cv_query:
  { query: "pixel art water depth color theory limited palette",
    maxTokens: 1000 }

CV returns:
  - From Color & Light (Gurney): "Water depth in limited palettes:
    use value steps of 15-20% between depth layers. Cool shift
    (toward blue-green) as depth increases. Surface highlights
    should be the warmest color in the water ramp."
  - From NES hardware docs: "At 4 colors, water depth requires
    dithering between the two darkest values. Checkerboard at 8×8
    reads as transparency, not depth — use 2×1 horizontal lines."
  - From project: "Current water palette: #1a3a5a, #2a5a8a, #4a8aba,
    #8acaff. Value steps: 16%, 15%, 22% — the jump to highlight is
    too large."
  [Sources: Color & Light p.204, NES PPU Reference, session palette]

LLM: "Your highlight (#8acaff) has a 22% value jump from the mid-tone.
Add an intermediate: #6aaada would give you 15% steps throughout. For
depth at 16×16, use horizontal 2×1 dithering between your two darkest
values — checkerboard will read as noise at this scale."
```

This is knowledge the static 8K system prompt can't carry — specific value percentages, hardware-specific dithering advice, awareness of the artist's current palette.

---

## Integration Points

### Full Tool & API Map

| Need | Channel | Endpoint / Tool | Who Initiates |
|------|---------|----------------|---------------|
| Pre-fetch domain knowledge for prompt | **API** | `POST /query` | Rust backend (automatic) |
| Check new tile against stored patterns | **API** | `POST /query` | Rust backend (automatic) |
| Store accepted/rejected tile | **API** | `POST /entries/bulk` | Rust backend (on UI event) |
| Store palette change | **API** | `POST /entries/bulk` | Rust backend (on UI event) |
| Fetch GameTileNet references | **API** | `POST /query` | Rust backend (before generation) |
| Check what goes stale after palette edit | **API** | `GET /documents/{id}/blast-radius` | Rust backend (on palette change) |
| Ask a technique question | **MCP** | `cv_query` | LLM (conversational) |
| Investigate SELF-REFINE failure | **MCP** | `cv_query` | LLM (during refinement) |
| Get full source of a reference tile | **MCP** | `cv_expand` | LLM (when studying examples) |
| Verify a design claim | **MCP** | `cv_verify` | LLM (when uncertain) |
| Validate edges | **pixl-mcp** | `pixl_validate` | LLM (during generation) |
| Render PAX to PNG | **pixl-mcp** | `pixl_render_tile` | LLM (during SELF-REFINE) |
| Get active palette | **pixl-mcp** | `pixl_get_palette` | LLM (during generation) |
| Generate WFC map | **pixl-mcp** | `pixl_narrate_map` | LLM (map generation) |

**Rule of thumb:**
- **CV API** = PIXL's backend making the LLM smarter before it runs
- **CV MCP** = the LLM making itself smarter during reasoning
- **pixl-mcp** = the LLM doing things (validate, render, WFC)

### Startup Flow

```
1. PIXL Studio launches
2. Starts pixl-mcp (Rust, local process)
3. Starts CV MCP (Node.js, local process, connects to local ANCS)
4. Rust backend connects to CV REST API (http://localhost:4000)
5. LLM system prompt loads:
   - PIXL format spec (essential, always in context)
   - Active theme + palette (changes per project)
   - Instruction: "Use cv_query for domain knowledge. Do not guess
     about color theory, dithering, or platform constraints — query
     the vault."
6. On every generation request:
   - Backend calls CV API → gets relevant domain context
   - Injects context into prompt (invisible to artist)
   - LLM generates with deep knowledge pre-loaded
   - LLM can still call cv_query MCP if it needs more
```

### Knowledge Base Toggle (PRD Feature)

The PRD specifies "Knowledge tag toggles: enable/disable sub-topics to keep context focused." With the dual-channel model, this controls *both* channels:

```
Toggle ON:  "Color Theory"
  → API prompt enrichment includes color-theory results
  → MCP cv_query scoped to include color-theory tags

Toggle OFF: "Animation"
  → API skips animation-related knowledge in enrichment
  → MCP cv_query excludes animation tags
```

The Rust backend reads the toggle state and passes it as a filter parameter to both API queries and as a scoping instruction in the LLM's system prompt for MCP queries.

### Feedback Loop: The Knowledge Flywheel

This is where the API channel creates compounding value:

```
                   ┌──────────────────────────┐
                   │  Artist generates tile   │
                   └────────────┬─────────────┘
                                │
           ┌────────────────────▼────────────────────┐
           │  Backend enriches prompt via CV API      │
           │  (references, techniques, past feedback) │
           └────────────────────┬────────────────────┘
                                │
                   ┌────────────▼─────────────┐
                   │  LLM generates tile      │
                   │  (better, because context)│
                   └────────────┬─────────────┘
                                │
                   ┌────────────▼─────────────┐
                   │  Artist accepts/rejects   │
                   └────────────┬─────────────┘
                                │
           ┌────────────────────▼────────────────────┐
           │  Backend writes result to CV via API     │
           │  (tile + decision + reason + palette)    │
           └────────────────────┬────────────────────┘
                                │
                         ┌──────▼──────┐
                         │  CV stores  │
                         │  with full  │
                         │  provenance │
                         └──────┬──────┘
                                │
                    ╔═══════════▼═══════════╗
                    ║  Next generation is   ║
                    ║  better because CV    ║
                    ║  knows what worked    ║
                    ║  and what didn't      ║
                    ╚═══════════════════════╝
```

Every tile generated, accepted, rejected, or modified feeds back into the vault. The 50th tile in a project benefits from the context of the first 49. The second project in the same theme benefits from the first. This is the flywheel the static 8K system prompt can never create.

---

## Vault Setup

### Initial Provisioning

```bash
# Create the vault
cv create-vault pixl-knowledge

# Ingest reference library
cv ingest --vault pixl-knowledge --profile documentation /path/to/pixel-logic-book.pdf
cv ingest --vault pixl-knowledge --profile documentation /path/to/color-and-light.pdf
cv ingest --vault pixl-knowledge --profile article /path/to/wfc-tips.md
cv ingest --vault pixl-knowledge --profile documentation /path/to/nes-ppu-reference.pdf
# ... more reference docs

# Ingest PIXL spec (living doc — watcher enabled)
cv ingest --vault pixl-knowledge --watch /Users/lisa/Work/Projects/PIXL/docs/concept/PIXL_SPEC.md

# Ingest GameTileNet corpus (structured entries)
cv ingest --vault pixl-knowledge --profile documentation /path/to/gametilenet-corpus/

# Verify
cv status --vault pixl-knowledge
# Documents: 47
# Entities: 1,283 (concepts, techniques, constraints, tiles)
# Relations: 3,891
# Tiers: T0=12%, T1=8%, T2=35%, T3=45%
```

### Token Economics

| Approach | System Prompt | Per Query | Total Context |
|----------|--------------|-----------|---------------|
| **Static KB (current MVP)** | ~8K tokens (all knowledge loaded) | 0 | 8K fixed overhead |
| **CV integration** | ~3K tokens (PIXL spec + theme only) | ~1-2K per cv_query | 3K + on-demand |

With CV, the LLM starts with a 5K token lighter context. Deep knowledge is loaded per-query, only when needed, compressed to fit. Over a 20-message conversation, the total knowledge consumed may be higher — but it's *relevant* knowledge, not the full 8K blob every turn.

---

## Narrative-to-Map Pipeline (Future)

This is where CV becomes essential, not optional. The PRD describes this flow:

```
Artist: "I want a dark forest dungeon with a boss chamber
        and three loot rooms connected by narrow corridors"
```

Without CV, the LLM has to invent spatial design rules from its training data. With CV:

```
1. LLM calls cv_query: "dungeon layout design boss chamber loot rooms corridors"

   CV returns:
   - From GDC talk "10 Principles of Dungeon Design": "Boss chambers
     should be 2-3x the size of standard rooms. Loot rooms are smaller
     than combat rooms. Corridors should be exactly 1 tile wide for
     tension. Place save/rest room before boss."
   - From level design guide: "Dark forest dungeons: root-like corridors,
     irregular room shapes, canopy ceiling tiles in large rooms"
   - From project tiles: 12 existing forest dungeon tiles with edge
     declarations already validated via WFC

2. LLM extracts spatial predicates:
   - boss_chamber(size: 3x) CONNECTED_BY corridor(width: 1) TO hub
   - loot_room(size: 1x, count: 3) CONNECTED_BY corridor TO hub
   - style: dark_forest, irregular_shapes

3. LLM calls pixl_narrate_map with:
   - Tile set: existing forest dungeon tiles from CV
   - Semantic constraints: from spatial predicates
   - Edge constraints: from tile declarations

4. WFC assembles map respecting both geometric and semantic rules
```

CV provides the design principles. pixl-mcp provides the execution. The LLM bridges them.

---

## What This Is Not

- **Not a training set.** CV doesn't fine-tune the model. It provides context at query time.
- **Not a vector database.** CV uses ANCS (hypergraph + PCST retrieval), which follows entity relations — not just embedding similarity. "Dithering for NES water" finds the NES hardware constraints AND the dithering technique AND the water tile patterns, even if they're in different documents.
- **Not required for MVP.** PIXL Studio works with the static 8K knowledge base. CV is the upgrade path for deeper, maintainable, auditable domain knowledge.
- **Not a replacement for pixl-mcp.** CV handles knowledge. pixl-mcp handles operations. They're complementary, not competing.

---

## Summary

| Question | Answer |
|----------|--------|
| What does CV give PIXL? | Deep, queryable domain knowledge with sources — not a static blob. Plus a feedback flywheel that compounds over every generation. |
| How does it connect? | **Two channels.** REST API for backend-driven automatic enrichment + validation. MCP for LLM-driven conversational queries. |
| What gets ingested? | Reference books, tutorials, specs, GameTileNet corpus, project history (accepts, rejects, palette changes) |
| Token impact? | System prompt drops from 8K to 3K. Backend injects 1-2K of relevant knowledge per generation via API. LLM can pull more via MCP. |
| Is it separate? | Yes. CV is a standalone knowledge store. PIXL is a client. Other tools can share the same vault. |
| Why two channels? | API = system intelligence (automatic, invisible, fast). MCP = model intelligence (conversational, exploratory, reasoning). Both are needed. |
| When is it needed? | Nice-to-have for MVP tile generation. Essential for narrative-to-map pipeline. The flywheel becomes the killer feature over time. |
