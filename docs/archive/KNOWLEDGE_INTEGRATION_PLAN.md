# Knowledge Integration Plan

**Product:** PIXL — Pixel Intelligence eXchange Layer
**Feature:** Context-Aware Pixel Art Knowledge System
**Date:** 2026-03-24
**Team:** Lisa + Claude
**Status:** Implemented (BM25 knowledge base is live in engine)

---

## Table of Contents

1. [Motivation](#1-motivation)
2. [Architecture Overview](#2-architecture-overview)
3. [Tier 1 — Embedded Knowledge (Compile-Time)](#3-tier-1--embedded-knowledge-compile-time)
4. [Tier 2 — Full Corpus (Runtime, Opt-In)](#4-tier-2--full-corpus-runtime-opt-in)
5. [Tier 3 — Developer & User Expansion](#5-tier-3--developer--user-expansion)
6. [Context Injection Design](#6-context-injection-design)
7. [Dataset Pipeline](#7-dataset-pipeline)
8. [Implementation Phases](#8-implementation-phases)
9. [File Layout](#9-file-layout)
10. [Open Decisions](#10-open-decisions)

---

## 1. Motivation

### Problem

PIXL builds enriched system prompts for Claude with palette symbols, theme constraints, style latents, feedback, and few-shot examples. But it has no domain knowledge injection — the model relies entirely on its training data for pixel art technique knowledge.

The current `studio/assets/knowledge_base.md` (132 lines) is a static guide used only by the Flutter Studio. It is not injected into the backend system prompt, and it covers topics at surface level.

### Goal

Give PIXL a structured, searchable pixel art knowledge base that:
- Injects relevant technique knowledge into Claude's context during generation
- Understands relationships between concepts (dithering relates to palettes, autotiling relates to WFC)
- Can be expanded by developers and optionally by users
- Adds zero runtime dependencies — pure data, loaded by Rust

### Prior Art

A corpus of 26 documents was ingested via [Ingestible](https://github.com/SimplyLiz/Ingestible) with full enrichment and knowledge graph extraction. The result:

| Metric | Count |
|--------|-------|
| Source documents | 26 |
| Hierarchical passages (L3) | 76 |
| KG triples | ~1,300 |
| Unique entities | ~1,750 |
| Unique concepts | ~1,920 |
| Cross-document shared entities | 111 |
| Corpus JSON size | 2.5 MB |

This corpus is stored at `corpus/pixelart-knowledge-base.json` and serves as the input for all three tiers below.

---

## 2. Architecture Overview

Three tiers, each independent. Implement in order — each tier delivers value alone.

```
Tier 1 (compile-time)          Tier 2 (runtime)              Tier 3 (expansion)
========================       ======================        ========================
Condensed topic files          Full corpus JSON              Dev scripts + user CLI
include_str!() embedded        Loaded at startup             Runs Ingestible pipeline
Keyword → topic selection      Concept index + KG graph      Rebuilds corpus JSON
~500-800 tokens injected       TF-IDF search over passages   Optional user feature
Zero overhead                  2.5 MB memory                 Dev-only dependency
```

### Design Principles

- **No Python at runtime.** Ingestible is a build-time/dev-time tool only.
- **No vector database.** 76 passages do not need embeddings — keyword/concept matching suffices.
- **Select, don't dump.** Inject 1-3 relevant sections per prompt, not the entire knowledge base.
- **Supplement, don't replace.** Knowledge augments the existing style latent + feedback system.

---

## 3. Tier 1 — Embedded Knowledge (Compile-Time)

### What

Distill the 26-document corpus into ~10 focused topic files optimized for LLM consumption (not human reading). Embed them at compile time via `include_str!()`.

### Topic Files

```
tool/knowledge/
  topics.toml          # topic metadata: name, tags, description
  color-theory.md      # hue shifting, palette ramps, gamut masking, value contrast
  dithering.md         # ordered/Bayer, Floyd-Steinberg, fill vs transitional, when to use
  shading.md           # light sources, cel vs soft, shadow types, common mistakes
  animation.md         # keyframes, walk cycles, sub-pixel, smear frames, timing
  tilesets.md          # autotiling bitmask, Wang/blob tiles, terrain transitions, seamless
  sprites.md           # proportions at 8/16/32/64px, silhouette, readability
  procgen.md           # WFC algorithm, noise functions, cellular automata sprites
  retro.md             # NES/SNES/GB/C64 constraints and how they shaped style
  effects.md           # fire, water, smoke, particles, magic
  palettes.md          # PICO-8/DB16/DB32/Endesga hex values, palette selection guide
  isometric.md         # 2:1 line, grid sizes, projection, shadow casting
```

Each file is condensed to ~200-400 tokens — the essential rules and relationships, not tutorial prose.

### topics.toml Schema

```toml
[[topic]]
name = "color-theory"
file = "color-theory.md"
tags = ["color", "palette", "hue", "ramp", "saturation", "warm", "cool"]
description = "Color theory for pixel art: hue shifting, palette ramps, value contrast"

[[topic]]
name = "dithering"
file = "dithering.md"
tags = ["dither", "pattern", "bayer", "checkerboard", "stipple", "noise"]
description = "Dithering techniques and when to use them"

# ... etc
```

### Rust Integration

```rust
// pixl-core/src/knowledge/embedded.rs

pub struct EmbeddedKnowledge {
    topics: Vec<Topic>,
}

struct Topic {
    name: &'static str,
    tags: &'static [&'static str],
    content: &'static str,  // include_str!()
}

impl EmbeddedKnowledge {
    /// Select topics relevant to a user prompt.
    /// Returns up to `max_topics` content strings.
    pub fn select(&self, prompt: &str, max_topics: usize) -> Vec<&str> {
        // 1. Tokenize prompt into lowercase words
        // 2. Score each topic by tag overlap count
        // 3. Return top-k by score (min score = 1 tag match)
    }
}
```

### Injection Point

In `handlers.rs`, between theme and style latent:

```rust
// handlers.rs — handle_generate_context()

let knowledge_sections = state.knowledge.select(&prompt, 2);
let knowledge_text = if knowledge_sections.is_empty() {
    String::new()
} else {
    format!(
        "\nRelevant technique knowledge:\n{}\n",
        knowledge_sections.join("\n\n")
    )
};

// Insert into system prompt template
format!(
    "...\n{theme_text}\n{knowledge_text}\n{style_text}\n..."
)
```

### Token Budget

| System prompt section | Tokens (current) | Tokens (with T1) |
|-----------------------|-------------------|-------------------|
| Palette symbols | ~200 | ~200 |
| Theme constraints | ~100 | ~100 |
| **Knowledge (new)** | **0** | **~500-800** |
| Style latent | ~150 | ~150 |
| Preference profile | ~100 | ~100 |
| Layer/edge context | ~200 | ~200 |
| Few-shot examples | ~300 | ~300 |
| Avoid constraints | ~100 | ~100 |
| Rules | ~150 | ~150 |
| **Total** | **~1,300** | **~1,800-2,100** |

500-800 additional tokens is well within Claude's budget and meaningfully below the cost of one few-shot tile example.

---

## 4. Tier 2 — Full Corpus (Runtime, Opt-In)

### What

Ship `pixelart-knowledge-base.json` as a runtime data file. Load it at startup into a lightweight in-memory search structure. Enables deeper, more targeted knowledge retrieval than Tier 1.

### Data File

```
tool/knowledge/pixelart-knowledge-base.json   (2.5 MB)
```

Format: Ingestible `ingestible-corpus` v1 — array of documents, each with:
- Hierarchical chapters → sections → passages
- Per-passage: content, summary, concepts, keywords, KG triples
- Document-level: knowledge graph with entity index, concept frequency map
- Corpus-level: shared entities across documents

### Rust Data Structures

```rust
// pixl-core/src/knowledge/corpus.rs

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct Corpus {
    pub documents: Vec<Document>,
    pub corpus_graph: CorpusGraph,
}

#[derive(Deserialize)]
pub struct Document {
    pub doc_id: String,
    pub title: String,
    pub domain: String,
    pub executive_summary: String,
    pub key_concepts: Vec<String>,
    pub chapters: Vec<Chapter>,
    pub knowledge_graph: KnowledgeGraph,
    pub concepts: ConceptIndex,
}

#[derive(Deserialize)]
pub struct Chapter {
    pub title: String,
    pub summary: String,
    pub sections: Vec<Section>,
}

#[derive(Deserialize)]
pub struct Section {
    pub title: String,
    pub summary: String,
    pub passages: Vec<Passage>,
}

#[derive(Deserialize)]
pub struct Passage {
    pub id: String,
    pub content: String,
    pub summary: String,
    pub concepts: Vec<String>,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub kg_triples: Vec<KGTriple>,
}

#[derive(Deserialize)]
pub struct KGTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f64,
}

#[derive(Deserialize)]
pub struct KnowledgeGraph {
    pub triples: Vec<KGTriple>,
    pub entities: HashMap<String, Entity>,
}

#[derive(Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub relations: Vec<Relation>,
    pub source_chunks: Vec<String>,
}

#[derive(Deserialize)]
pub struct Relation {
    pub predicate: String,
    pub target: String,
    pub direction: String,  // "incoming" | "outgoing"
}
```

### Search Engine

No vector DB needed. Build a simple inverted index at load time:

```rust
// pixl-core/src/knowledge/search.rs

pub struct KnowledgeSearch {
    corpus: Corpus,
    concept_to_passages: HashMap<String, Vec<usize>>,  // concept → passage indices
    keyword_to_passages: HashMap<String, Vec<usize>>,   // keyword → passage indices
    all_passages: Vec<PassageRef>,                       // flat list for scoring
}

impl KnowledgeSearch {
    pub fn load(path: &Path) -> Result<Self, Error>;

    /// Search by query string. Returns top-k passages.
    /// Algorithm:
    /// 1. Tokenize query into words
    /// 2. Look up each word in keyword + concept indices
    /// 3. Score passages by hit count (concept hits weighted 2x)
    /// 4. Optional: traverse KG 1-hop from matched entities
    /// 5. Return top-k by score
    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchResult>;

    /// Get all passages related to an entity via KG traversal.
    pub fn related(&self, entity: &str, hops: usize) -> Vec<SearchResult>;
}

pub struct SearchResult {
    pub passage_content: String,
    pub passage_summary: String,
    pub score: f32,
    pub source_doc: String,
    pub concepts: Vec<String>,
}
```

### Integration with Context Builder

```rust
// handlers.rs

let knowledge_text = if let Some(ref search) = state.knowledge_search {
    let results = search.search(&prompt, 3);
    if results.is_empty() {
        String::new()
    } else {
        let mut text = String::from("Relevant pixel art knowledge:\n");
        for r in &results {
            text.push_str(&format!("- {}\n", r.passage_summary));
            text.push_str(&r.passage_content);
            text.push('\n');
        }
        text
    }
} else {
    // Fall back to Tier 1 embedded knowledge
    state.embedded_knowledge.select(&prompt, 2).join("\n\n")
};
```

### Loading

```rust
// state.rs — McpState

pub struct McpState {
    // ... existing fields ...
    pub embedded_knowledge: EmbeddedKnowledge,           // always available (Tier 1)
    pub knowledge_search: Option<KnowledgeSearch>,       // loaded if file exists (Tier 2)
}

impl McpState {
    pub fn new() -> Self {
        let knowledge_search = KnowledgeSearch::load(
            Path::new("knowledge/pixelart-knowledge-base.json")
        ).ok();  // None if file doesn't exist — graceful degradation

        Self {
            embedded_knowledge: EmbeddedKnowledge::new(),
            knowledge_search,
            // ...
        }
    }
}
```

### Tier 2 is opt-in: if the JSON file is absent, PIXL falls back to Tier 1 embedded knowledge.

---

## 5. Tier 3 — Developer & User Expansion

### Developer Workflow

Scripts in the PIXL repo to rebuild the knowledge base using Ingestible:

```
scripts/
  build-knowledge-base.sh    # full pipeline: ingest dataset → corpus-export
  add-document.sh             # add a single doc and re-export
  condense-topics.sh          # regenerate Tier 1 topic files from corpus
  README.md                   # contributor guide
```

#### build-knowledge-base.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

# Requires: pip install ingestible (dev dependency only, not shipped)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PIXL_ROOT="$(dirname "$SCRIPT_DIR")"
DATASET="$PIXL_ROOT/dataset/pixelart"
CORPUS="$PIXL_ROOT/corpus"
OUTPUT="$PIXL_ROOT/tool/knowledge/pixelart-knowledge-base.json"

echo "Ingesting $DATASET..."
INGEST_EXTRACT_KNOWLEDGE_GRAPH=true \
INGEST_DATA_DIR="$CORPUS" \
ingest ingest "$DATASET" --parallel=4 -v

echo "Exporting corpus..."
INGEST_DATA_DIR="$CORPUS" \
ingest corpus-export -o "$OUTPUT"

echo "Done. Knowledge base: $OUTPUT"
echo "Documents: $(jq '.document_count' "$OUTPUT")"
echo "KG triples: $(jq '[.documents[].knowledge_graph.triples | length] | add' "$OUTPUT")"
```

#### add-document.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

FILE="$1"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PIXL_ROOT="$(dirname "$SCRIPT_DIR")"

# Copy to dataset
cp "$FILE" "$PIXL_ROOT/dataset/pixelart/"

# Ingest just this file
INGEST_EXTRACT_KNOWLEDGE_GRAPH=true \
INGEST_DATA_DIR="$PIXL_ROOT/corpus" \
ingest ingest "$FILE" -v

# Re-export corpus
INGEST_DATA_DIR="$PIXL_ROOT/corpus" \
ingest corpus-export -o "$PIXL_ROOT/tool/knowledge/pixelart-knowledge-base.json"

echo "Added $(basename "$FILE") to knowledge base."
```

### User-Supplied Knowledge (Future)

A `pixl learn` CLI command that lets users ingest their own pixel art books:

```bash
# User has a pixel art PDF
pixl learn ~/Books/pixel-logic.pdf

# Or a collection of tutorials
pixl learn ~/pixel-art-guides/ --recursive
```

Implementation:
1. Check if `ingest` CLI is available on PATH
2. If yes: shell out to `ingest ingest <path>` + `ingest corpus-export`
3. If no: use a bundled lightweight markdown parser (no enrichment, no KG)
4. Merge user corpus with built-in corpus at load time

This is explicitly **not MVP**. Defer until Tier 1 + 2 are proven.

---

## 6. Context Injection Design

### Selection Strategy

Not all knowledge is relevant to every generation request. The context builder must select wisely.

#### Tier 1 Selection (keyword matching)

```
User prompt: "dark stone wall with moss"
Matched tags: "wall" → tilesets.md, "stone" → tilesets.md, "moss" → color-theory.md
Injected: tilesets.md (seamless tiles, terrain transitions) + color-theory.md (organic colors)
```

#### Tier 2 Selection (concept + KG search)

```
User prompt: "dark stone wall with moss"
Concept hits: "terrain transitions" (3 passages), "seamless tiling" (2 passages)
KG traversal: "terrain transitions" → related to "autotiling" → related to "blob tileset"
Injected: Top 3 passages by relevance score
```

### Prompt Template (Updated)

```
You are a pixel art expert generating PAX format tiles.

{palette_text}
{theme_text}

Relevant technique knowledge:
{knowledge_text}

{style_text}
{preference_text}
Canvas size: {size_str}
Type: {tile_type}

{layer_context}
{edge_context}
{examples_text}
{avoid_text}

Rules:
- Use ONLY symbols from the palette above
- Output a raw character grid (one row per line)
- Grid must be exactly {size_str} characters
- Shadows go bottom-right of structures
- Highlights go top-left of surfaces
- For WFC compatibility, edges should match neighboring tiles
- Suggest a target_layer for this tile
```

Knowledge goes between theme and style — it provides general technique grounding before the session-specific constraints.

### Fallback Chain

```
1. Tier 2 search (if corpus loaded) → top 3 passages
2. Tier 1 embedded (if Tier 2 unavailable or no results) → top 2 topics
3. No knowledge (if both fail) → existing behavior, no regression
```

---

## 7. Dataset Pipeline

### Source Data

The dataset lives in the PIXL repo at `dataset/pixelart/`. Currently 26 markdown files with YAML frontmatter (title, source URL, topic, fetch date).

### Ingestion Pipeline

```
dataset/pixelart/*.md
    │
    ▼
Ingestible Pipeline (dev-time only)
    │  Stage 1: Parse markdown
    │  Stage 2: Build structure (headings → chapters/sections)
    │  Stage 3: Chunk into passages (~250-500 tokens each)
    │  Stage 4: LLM enrichment (summaries, concepts, keywords, KG triples)
    │  Stage 5: Embed (E5-large-v2 vectors + BM25 + concept index)
    │  Stage 6: Store to disk
    │
    ▼
corpus/documents/{doc_id}/     (intermediate, gitignored)
    │
    ▼
ingest corpus-export --format complete
    │
    ▼
tool/knowledge/pixelart-knowledge-base.json    (committed, 2.5 MB)
    │
    ▼
Condense script (extract key passages → topic files)
    │
    ▼
tool/knowledge/*.md            (committed, ~15 KB total)
```

### What Gets Committed

```
dataset/pixelart/*.md                              # source documents (committed)
tool/knowledge/topics.toml                          # topic index (committed)
tool/knowledge/*.md                                 # condensed topic files (committed)
tool/knowledge/pixelart-knowledge-base.json         # full corpus (committed)
corpus/                                             # intermediate data (gitignored)
```

### Expanding the Dataset

To add new knowledge:

1. Place markdown/PDF/text files in `dataset/pixelart/`
2. Run `scripts/build-knowledge-base.sh`
3. Optionally update condensed topic files
4. Commit the updated JSON + topic files

The dataset should be expanded with:
- Pixel art composition and layout theory
- Pixel art fonts and text rendering
- Environment design (interiors, landscapes, dungeons)
- Advanced palette techniques (palette cycling, color animation)
- Game-specific style guides (roguelike, platformer, RPG)
- Sub-pixel rendering and anti-aliasing deep dives

---

## 8. Implementation Phases

### Phase 1 — Tier 1: Embedded Knowledge (1-2 days)

**Deliverables:**
- [ ] Condense corpus into ~10 topic files in `tool/knowledge/`
- [ ] Create `topics.toml` with tags
- [ ] Implement `EmbeddedKnowledge` struct in `pixl-core`
- [ ] Add `select()` method with tag-based matching
- [ ] Wire into `handle_generate_context()` in `handlers.rs`
- [ ] Add `{knowledge_text}` to system prompt template
- [ ] Test: verify relevant topics selected for sample prompts
- [ ] Test: verify token budget stays under 2,500

**No new dependencies. No new data files at runtime.**

### Phase 2 — Tier 2: Runtime Corpus (2-3 days)

**Deliverables:**
- [ ] Define `Corpus`/`Document`/`Passage`/`KGTriple` serde structs in `pixl-core`
- [ ] Implement `KnowledgeSearch` with inverted concept/keyword index
- [ ] Implement `search()` with weighted scoring
- [ ] Implement `related()` for KG entity traversal
- [ ] Load corpus JSON in `McpState` (optional — graceful if absent)
- [ ] Wire Tier 2 search into context builder with Tier 1 fallback
- [ ] Ship `pixelart-knowledge-base.json` in `tool/knowledge/`
- [ ] Test: search quality on sample queries
- [ ] Test: fallback works when JSON absent

**New runtime data: 2.5 MB JSON. No new binary dependencies.**

### Phase 3 — Tier 3: Dev Scripts + Dataset Management (1 day)

**Deliverables:**
- [ ] Create `scripts/build-knowledge-base.sh`
- [ ] Create `scripts/add-document.sh`
- [ ] Create `scripts/README.md` with contributor guide
- [ ] Add `corpus/` to `.gitignore`
- [ ] Document expansion workflow in repo README or CONTRIBUTING.md

**Dev-only. No runtime impact.**

### Phase 4 — Evaluation & Tuning (1-2 days)

**Deliverables:**
- [ ] Generate tiles with and without knowledge injection
- [ ] Compare: technique accuracy, palette usage, tileability
- [ ] Tune: topic selection weights, max injected tokens, passage count
- [ ] Tune: balance between knowledge and few-shot examples
- [ ] Document findings

### Phase 5 — Future: User `pixl learn` (deferred)

- [ ] Design CLI UX for `pixl learn <path>`
- [ ] Implement Ingestible detection + fallback parser
- [ ] User corpus merge with built-in corpus
- [ ] Studio UI for browsing/managing knowledge base

---

## 9. File Layout

### After Tier 1 + 2 + 3

```
PIXL/
  dataset/
    pixelart/                           # 26+ source markdown files
      fundamentals-pixel-art-basics.md
      color-theory-palettes.md
      dithering-techniques.md
      ...
  corpus/                               # gitignored, intermediate Ingestible data
  scripts/
    build-knowledge-base.sh
    add-document.sh
    README.md
  tool/
    knowledge/
      topics.toml                       # topic metadata + tags
      color-theory.md                   # condensed topic files (~200-400 tokens each)
      dithering.md
      shading.md
      animation.md
      tilesets.md
      sprites.md
      procgen.md
      retro.md
      effects.md
      palettes.md
      isometric.md
      pixelart-knowledge-base.json      # full corpus (2.5 MB)
    crates/
      pixl-core/
        src/
          knowledge/
            mod.rs                      # pub mod embedded; pub mod corpus; pub mod search;
            embedded.rs                 # Tier 1: EmbeddedKnowledge
            corpus.rs                   # Tier 2: serde types for corpus JSON
            search.rs                   # Tier 2: KnowledgeSearch inverted index
      pixl-mcp/
        src/
          handlers.rs                   # Updated: knowledge injection in context builder
          state.rs                      # Updated: holds EmbeddedKnowledge + Option<KnowledgeSearch>
```

---

## 10. Open Decisions

### Must Decide Before Phase 1

1. **Topic file format** — Should condensed topics be markdown (human-readable) or structured TOML (machine-parseable)?
   - Recommendation: Markdown. Easier to author, `include_str!()` just works.

2. **Selection threshold** — Minimum tag overlap to inject a topic?
   - Recommendation: 1 tag match minimum, max 2 topics. Test and tune.

### Must Decide Before Phase 2

3. **Corpus shipping** — Should `pixelart-knowledge-base.json` be committed to the repo or downloaded on first run?
   - Recommendation: Commit. 2.5 MB is fine for a git repo. Avoids network dependency.

4. **Search scoring** — Pure keyword overlap or TF-IDF?
   - Recommendation: Start with keyword overlap (concept hits 2x weight). TF-IDF only if results are poor.

5. **KG traversal depth** — 1-hop or 2-hop entity expansion?
   - Recommendation: 1-hop. 2-hop risks pulling in loosely related passages.

### Deferred

6. **User knowledge ingestion** — How to handle users without Python/Ingestible?
   - Defer to Phase 5. Likely: bundled lightweight parser in Rust (no enrichment).

7. **Knowledge in local inference** — Should the LoRA-tuned local model also get knowledge context?
   - Probably yes, but smaller context window means fewer passages. Tune separately.

8. **Studio knowledge browser** — Should the Flutter UI expose the knowledge base for browsing?
   - Nice-to-have. The knowledge base is primarily for LLM context, not human reference.
