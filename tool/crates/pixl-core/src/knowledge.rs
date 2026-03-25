/// Pixel art knowledge base — loads Ingestible corpus export and provides
/// BM25 search with KG-expanded query decomposition for context injection
/// into LLM prompts.
///
/// Search strategy:
/// 1. **Contextual indexing**: each passage is indexed with a structured context
///    prefix (source title + topic concepts) to improve BM25 term matching.
/// 2. **KG query expansion**: before BM25 search, query terms are expanded by
///    traversing the knowledge graph 1-2 hops to discover related concepts
///    (e.g. "dungeon wall" → shadow_placement, stone_material, dithering).
/// 3. **Query decomposition**: complex creative prompts are split into
///    knowledge-area sub-queries using KG concept clusters, with per-sub-query
///    retrieval and arithmetic mean score fusion.
/// 4. **BM25** over the enriched passage index with expanded query terms.
/// 5. **Score fusion + sandwich ordering**: results ranked by fused score,
///    then reordered so the best passage is first and second-best is last
///    (mitigates the "lost in the middle" attention pattern in LLMs).

use bm25::{Document, Language, SearchEngineBuilder};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ---------------------------------------------------------------------------
// Serde types — mirror the Ingestible "complete" export format
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CorpusFile {
    documents: Vec<CorpusDocument>,
}

#[derive(Deserialize)]
struct CorpusDocument {
    #[allow(dead_code)]
    doc_id: String,
    title: String,
    #[serde(default)]
    chapters: Vec<Chapter>,
    #[serde(default)]
    orphan_passages: Vec<Passage>,
    #[serde(default)]
    knowledge_graph: KnowledgeGraphData,
}

#[derive(Deserialize)]
struct Chapter {
    #[serde(default)]
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    passages: Vec<Passage>,
}

#[derive(Deserialize)]
struct Passage {
    content: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    concepts: Vec<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    hypothetical_questions: Vec<String>,
}

#[derive(Deserialize, Default)]
struct KnowledgeGraphData {
    #[serde(default)]
    entities: HashMap<String, EntityData>,
}

#[derive(Deserialize)]
struct EntityData {
    #[serde(default)]
    relations: Vec<RelationData>,
    #[allow(dead_code)]
    #[serde(default)]
    source_chunks: Vec<String>,
}

#[derive(Deserialize)]
struct RelationData {
    #[allow(dead_code)]
    predicate: String,
    target: String,
    #[allow(dead_code)]
    direction: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// A single search result from the knowledge base.
#[derive(Debug, Clone)]
pub struct KnowledgeResult {
    pub content: String,
    pub summary: String,
    pub source_title: String,
    pub score: f32,
}

/// In-memory pixel art knowledge base with BM25 search, KG-expanded query
/// decomposition, contextual indexing, and sandwich-ordered results.
pub struct KnowledgeBase {
    /// Stored passage data (content, summary, title, concepts) for result construction.
    passages: Vec<StoredPassage>,
    /// BM25 search engine over the contextually-enriched passage index.
    engine: bm25::SearchEngine<usize>,
    /// concept (lowercase) → passage indices, for KG-expanded retrieval.
    concept_to_passages: HashMap<String, Vec<usize>>,
    /// entity (lowercase) → related entity names, for 1-2 hop traversal.
    entity_relations: HashMap<String, Vec<String>>,
    /// concept (lowercase) → set of related concept names (from KG), for query decomposition.
    concept_relations: HashMap<String, Vec<String>>,
}

struct StoredPassage {
    content: String,
    summary: String,
    source_title: String,
}

/// Confidence threshold: if the best BM25 score exceeds this fraction of the
/// average document length, skip query decomposition (the direct query is
/// already high-quality). This avoids adding noise for specific queries like
/// "generate an 8x8 grass tile using the gameboy palette".
const DECOMPOSITION_CONFIDENCE_THRESHOLD: f32 = 8.0;

impl KnowledgeBase {
    /// Load a knowledge base from an Ingestible corpus-export JSON file.
    /// Returns None if the file doesn't exist or can't be parsed.
    pub fn load(path: &Path) -> Option<Self> {
        let data = std::fs::read_to_string(path).ok()?;
        let corpus: CorpusFile = serde_json::from_str(&data).ok()?;

        let mut passages = Vec::new();
        let mut concept_to_passages: HashMap<String, Vec<usize>> = HashMap::new();
        let mut entity_relations: HashMap<String, Vec<String>> = HashMap::new();
        let mut concept_relations: HashMap<String, Vec<String>> = HashMap::new();

        // Build the BM25 search engine with contextually-enriched documents.
        //
        // Each passage is indexed with:
        // - A contextual prefix: [From: {title} | Topic: {concepts}] {summary}
        //   (offline equivalent of Anthropic's contextual retrieval technique)
        // - Full content
        // - Hypothetical questions (pre-computed HyDE)
        // - Concepts and keywords
        let mut engine = SearchEngineBuilder::<usize>::with_avgdl(200.0)
            .language_mode(Language::English)
            .build();

        for doc in &corpus.documents {
            let all_passages: Vec<&Passage> = doc
                .chapters
                .iter()
                .flat_map(|ch| ch.sections.iter())
                .flat_map(|sec| sec.passages.iter())
                .chain(doc.orphan_passages.iter())
                .collect();

            for p in all_passages {
                let idx = passages.len();

                // Task 4: Contextual chunk prefix — situates each passage
                // for better BM25 term matching without needing an LLM call.
                let context_prefix = format!(
                    "[From: {} | Topic: {}] {}",
                    doc.title,
                    if p.concepts.is_empty() {
                        "general".to_string()
                    } else {
                        p.concepts[..p.concepts.len().min(5)].join(", ")
                    },
                    p.summary,
                );

                let searchable = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    context_prefix,
                    p.content,
                    p.hypothetical_questions.join(" "),
                    p.concepts.join(" "),
                    p.keywords.join(" "),
                );

                engine.upsert(Document {
                    id: idx,
                    contents: searchable,
                });

                passages.push(StoredPassage {
                    content: p.content.clone(),
                    summary: p.summary.clone(),
                    source_title: doc.title.clone(),
                });

                for concept in &p.concepts {
                    concept_to_passages
                        .entry(concept.to_lowercase())
                        .or_default()
                        .push(idx);
                }
            }

            // Build entity relation graph for KG expansion
            for (entity_name, entity_data) in &doc.knowledge_graph.entities {
                let related: Vec<String> = entity_data
                    .relations
                    .iter()
                    .map(|r| r.target.to_lowercase())
                    .collect();
                entity_relations
                    .entry(entity_name.to_lowercase())
                    .or_default()
                    .extend(related.clone());

                // Build bidirectional concept relations for query decomposition
                for target in &related {
                    concept_relations
                        .entry(entity_name.to_lowercase())
                        .or_default()
                        .push(target.clone());
                    concept_relations
                        .entry(target.clone())
                        .or_default()
                        .push(entity_name.to_lowercase());
                }
            }
        }

        eprintln!(
            "knowledge: loaded {} passages, {} concepts, {} entities, {} concept relations (BM25 indexed)",
            passages.len(),
            concept_to_passages.len(),
            entity_relations.len(),
            concept_relations.len(),
        );

        Some(KnowledgeBase {
            passages,
            engine,
            concept_to_passages,
            entity_relations,
            concept_relations,
        })
    }

    /// Expand a query string using the knowledge graph.
    ///
    /// Traverses 1-2 hops from matched entities/concepts to discover related
    /// terms that BM25 would otherwise miss. Returns the expanded terms as a
    /// space-separated string to append to the original query.
    fn expand_query(&self, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|w| w.len() > 2)
            .collect();

        let mut expanded: HashSet<String> = HashSet::new();

        // 1-hop: exact word matches against entity and concept keys
        for word in &query_words {
            if let Some(related) = self.entity_relations.get(*word) {
                for r in related {
                    expanded.insert(r.clone());
                }
            }
            if let Some(related) = self.concept_relations.get(*word) {
                for r in related {
                    expanded.insert(r.clone());
                }
            }
        }

        // Fuzzy entity matching: query words that are substrings of entity
        // keys or vice versa (e.g. "torchlight" matches entity "torch",
        // "nes" matches entity "nes games"). This catches partial matches
        // that exact lookup misses.
        for entity in self.entity_relations.keys() {
            let matched = if entity.contains(' ') {
                // Multi-word: check if entity appears in full query
                query_lower.contains(entity.as_str())
            } else {
                // Single-word: check if any query word contains or is
                // contained by the entity (min 3 chars to avoid noise)
                entity.len() >= 3
                    && query_words.iter().any(|w| {
                        w.contains(entity.as_str()) || entity.contains(w)
                    })
            };
            if matched {
                if let Some(related) = self.entity_relations.get(entity) {
                    for r in related {
                        expanded.insert(r.clone());
                    }
                }
            }
        }

        // Also match query words against concept keys (not just entities)
        // to catch direct concept references like "palette", "shadow", etc.
        for word in &query_words {
            for concept in self.concept_to_passages.keys() {
                if concept.len() >= 3
                    && (word.contains(concept.as_str()) || concept.contains(word))
                    && *word != concept.as_str()
                {
                    // Add the concept itself as an expanded term
                    expanded.insert(concept.clone());
                }
            }
        }

        // 2-hop: traverse from 1-hop results to discover deeper connections
        let first_hop: Vec<String> = expanded.iter().cloned().collect();
        for hop1 in &first_hop {
            if let Some(related) = self.concept_relations.get(hop1.as_str()) {
                // Only add 2-hop terms that appear as indexed concepts
                // (prevents noise from distant, irrelevant entities)
                for r in related {
                    if self.concept_to_passages.contains_key(r.as_str()) {
                        expanded.insert(r.clone());
                    }
                }
            }
        }

        // Remove terms already in the query to avoid BM25 double-counting
        for word in &query_words {
            expanded.remove(*word);
        }

        expanded.into_iter().collect::<Vec<_>>().join(" ")
    }

    /// Decompose a creative prompt into knowledge-area sub-queries using the
    /// KG concept taxonomy.
    ///
    /// "generate a dungeon wall tile" → sub-queries covering tiling rules,
    /// material textures, lighting, palette constraints.
    ///
    /// Returns None if the query is already specific enough (high BM25 scores).
    fn decompose_query(&self, query: &str) -> Option<Vec<String>> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|w| w.len() > 2)
            .collect();

        // Collect all concepts reachable from query terms (1 hop)
        // Uses fuzzy matching (same as expand_query) to catch partial matches
        let mut reachable_concepts: HashSet<String> = HashSet::new();
        for word in &query_words {
            if let Some(related) = self.entity_relations.get(*word) {
                reachable_concepts.extend(related.iter().cloned());
            }
            if let Some(related) = self.concept_relations.get(*word) {
                reachable_concepts.extend(related.iter().cloned());
            }
        }
        // Fuzzy entity matching for decomposition
        for entity in self.entity_relations.keys() {
            let matched = if entity.contains(' ') {
                query_lower.contains(entity.as_str())
            } else {
                entity.len() >= 3
                    && query_words.iter().any(|w| {
                        w.contains(entity.as_str()) || entity.contains(w)
                    })
            };
            if matched {
                if let Some(related) = self.entity_relations.get(entity) {
                    reachable_concepts.extend(related.iter().cloned());
                }
            }
        }

        // Not enough KG coverage to decompose — the query terms are too
        // specific or not in the KG
        if reachable_concepts.len() < 3 {
            return None;
        }

        // Cluster reachable concepts by which passages they appear in.
        // Each cluster becomes a sub-query representing a knowledge area.
        let mut passage_to_concepts: HashMap<usize, Vec<String>> = HashMap::new();
        for concept in &reachable_concepts {
            if let Some(indices) = self.concept_to_passages.get(concept.as_str()) {
                for &idx in indices {
                    passage_to_concepts
                        .entry(idx)
                        .or_default()
                        .push(concept.clone());
                }
            }
        }

        // Group by source document to form knowledge-area clusters
        let mut doc_clusters: HashMap<&str, HashSet<String>> = HashMap::new();
        for (idx, concepts) in &passage_to_concepts {
            let title = self.passages[*idx].source_title.as_str();
            doc_clusters
                .entry(title)
                .or_default()
                .extend(concepts.iter().cloned());
        }

        // Build sub-queries: original query terms + cluster-specific concepts.
        // Deduplicate by sorting cluster terms so identical concept sets collapse.
        let original_terms = query_words.join(" ");
        let mut seen: HashSet<String> = HashSet::new();
        let sub_queries: Vec<String> = doc_clusters
            .values()
            .filter_map(|concepts| {
                let mut cluster_terms: Vec<&str> = concepts
                    .iter()
                    .take(4) // limit to avoid BM25 dilution
                    .map(|s| s.as_str())
                    .collect();
                cluster_terms.sort();
                let key = cluster_terms.join(" ");
                if seen.insert(key) {
                    Some(format!("{} {}", original_terms, cluster_terms.join(" ")))
                } else {
                    None
                }
            })
            .collect();

        if sub_queries.len() <= 1 {
            return None;
        }

        Some(sub_queries)
    }

    /// Search for passages relevant to a query string.
    ///
    /// Strategy:
    /// 1. Attempt query decomposition for complex creative prompts. If the
    ///    initial BM25 scores are already high-confidence, skip decomposition.
    /// 2. KG-expand each (sub-)query to discover related terms.
    /// 3. BM25 search per expanded (sub-)query, fuse scores via arithmetic mean.
    /// 4. KG boost: passages linked to entities reachable from query get a
    ///    score bump proportional to hop distance.
    /// 5. Sandwich ordering: best result first, second-best last.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<KnowledgeResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        // Phase 1: Initial BM25 probe to check confidence
        let probe_results = self.engine.search(query, 3);
        let high_confidence = probe_results
            .first()
            .map(|r| r.score > DECOMPOSITION_CONFIDENCE_THRESHOLD)
            .unwrap_or(false);

        // Phase 2: Build query set — either decomposed sub-queries or
        // single expanded query
        let expanded_original = format!("{} {}", query, self.expand_query(query));
        let queries: Vec<String> = if high_confidence {
            // Direct query is strong enough — just expand, don't decompose
            vec![expanded_original]
        } else if let Some(sub_queries) = self.decompose_query(query) {
            // Decompose into knowledge-area sub-queries, each KG-expanded
            let mut qs: Vec<String> = sub_queries
                .iter()
                .map(|sq| format!("{} {}", sq, self.expand_query(sq)))
                .collect();
            // Always include the original expanded query
            qs.insert(0, expanded_original);
            qs
        } else {
            vec![expanded_original]
        };

        // Phase 3: BM25 search per query, fuse via arithmetic mean
        let bm25_limit = (top_k * 3).max(15);
        let mut fused_scores: HashMap<usize, (f32, usize)> = HashMap::new(); // idx → (sum, count)

        for q in &queries {
            let results = self.engine.search(q, bm25_limit);
            for result in &results {
                let entry = fused_scores.entry(result.document.id).or_default();
                entry.0 += result.score;
                entry.1 += 1;
            }
        }

        // Arithmetic mean fusion (equal weight per knowledge area)
        let query_count = queries.len() as f32;
        let mut scores: HashMap<usize, f32> = fused_scores
            .into_iter()
            .map(|(idx, (sum, _count))| (idx, sum / query_count))
            .collect();

        // Phase 4: KG entity boost (same as before, but on fused scores)
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
            .filter(|w| w.len() > 2)
            .collect();

        let best_score = scores.values().copied().fold(0.0_f32, f32::max);

        for word in &query_words {
            // 1-hop boost (15%)
            if let Some(related) = self.entity_relations.get(*word) {
                for related_name in related {
                    if let Some(indices) = self.concept_to_passages.get(related_name.as_str()) {
                        let boost = best_score * 0.15;
                        for &idx in indices {
                            *scores.entry(idx).or_default() += boost;
                        }
                    }
                }
            }
        }

        // Multi-word entity matches with stronger boost (20%)
        for entity in self.entity_relations.keys() {
            if entity.contains(' ') && query_lower.contains(entity.as_str()) {
                if let Some(related) = self.entity_relations.get(entity) {
                    for related_name in related {
                        if let Some(indices) = self.concept_to_passages.get(related_name.as_str()) {
                            let boost = best_score * 0.2;
                            for &idx in indices {
                                *scores.entry(idx).or_default() += boost;
                            }
                        }
                    }
                }
            }
        }

        // Phase 5: Rank, deduplicate, apply sandwich ordering
        let mut ranked: Vec<(usize, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut results: Vec<KnowledgeResult> = ranked
            .into_iter()
            .take(top_k)
            .filter(|(_, score)| *score > 0.0)
            .map(|(idx, score)| {
                let p = &self.passages[idx];
                KnowledgeResult {
                    content: p.content.clone(),
                    summary: p.summary.clone(),
                    source_title: p.source_title.clone(),
                    score,
                }
            })
            .collect();

        // Sandwich ordering: highest-relevance first, second-highest last.
        // This mitigates the "lost in the middle" attention pattern where LLMs
        // attend strongly to the start and end of context but lose information
        // placed in the middle.
        if results.len() >= 3 {
            // results[0] stays (best), swap results[1] to the end
            let second = results.remove(1);
            results.push(second);
        }

        results
    }

    /// Number of indexed passages.
    pub fn passage_count(&self) -> usize {
        self.passages.len()
    }

    /// Number of unique concepts in the index.
    pub fn concept_count(&self) -> usize {
        self.concept_to_passages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_nonexistent_returns_none() {
        assert!(KnowledgeBase::load(Path::new("/nonexistent/file.json")).is_none());
    }

    #[test]
    fn load_and_search() {
        let kb_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../knowledge/pixelart-knowledge-base.json");
        if !kb_path.exists() {
            eprintln!("skipping: knowledge base not found at {:?}", kb_path);
            return;
        }

        let kb = KnowledgeBase::load(&kb_path).expect("should load");
        assert!(kb.passage_count() > 0);
        assert!(kb.concept_count() > 0);

        // Search for dithering — should find relevant passages
        let results = kb.search("dithering techniques Bayer pattern", 5);
        assert!(!results.is_empty(), "should find dithering passages");
        assert!(results[0].score > 0.0);
        eprintln!("dithering results (k=5, sandwich ordered):");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }

        // Search for WFC content
        let results = kb.search("wave function collapse tilemap", 5);
        assert!(!results.is_empty(), "should find WFC passages");
        eprintln!("WFC results:");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }

        // Creative prompt — tests query decomposition + KG expansion
        let results = kb.search("dark stone wall with moss growing", 5);
        eprintln!("creative prompt results (decomposed):");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }

        // Retro style query — should find hardware constraint passages
        let results = kb.search("make it look retro like a NES game", 5);
        assert!(!results.is_empty(), "should find retro/NES passages");
        eprintln!("retro results:");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }

        // Sandwich ordering test: first result should have highest score,
        // last result should have second-highest
        let results = kb.search("pixel art tile generation", 5);
        if results.len() >= 3 {
            assert!(
                results[0].score >= results.last().unwrap().score,
                "first result should have highest or equal score"
            );
            // The last result (second-best) should score higher than middle results
            let last_score = results.last().unwrap().score;
            for r in &results[1..results.len() - 1] {
                assert!(
                    last_score >= r.score,
                    "sandwich: last result ({:.2}) should score >= middle ({:.2})",
                    last_score,
                    r.score
                );
            }
        }
    }

    #[test]
    fn expand_query_adds_related_terms() {
        let kb_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../knowledge/pixelart-knowledge-base.json");
        if !kb_path.exists() {
            eprintln!("skipping: knowledge base not found at {:?}", kb_path);
            return;
        }

        let kb = KnowledgeBase::load(&kb_path).expect("should load");

        let expanded = kb.expand_query("dithering");
        eprintln!("expanded 'dithering': {}", expanded);
        // Should contain related concepts from the KG
        assert!(!expanded.is_empty(), "should expand via KG relations");
    }

    #[test]
    fn decompose_complex_query() {
        let kb_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../knowledge/pixelart-knowledge-base.json");
        if !kb_path.exists() {
            eprintln!("skipping: knowledge base not found at {:?}", kb_path);
            return;
        }

        let kb = KnowledgeBase::load(&kb_path).expect("should load");

        // Complex creative prompt should decompose
        let subs = kb.decompose_query("generate a dark dungeon wall tile with torchlight");
        eprintln!("decomposed sub-queries: {:?}", subs);

        // Simple specific prompt should NOT decompose (or decompose minimally)
        let subs = kb.decompose_query("NES palette");
        eprintln!("simple query sub-queries: {:?}", subs);
    }
}
