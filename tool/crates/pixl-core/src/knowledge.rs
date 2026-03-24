/// Pixel art knowledge base — loads Ingestible corpus export and provides
/// BM25 search with KG expansion for context injection into LLM prompts.
///
/// Search strategy:
/// 1. **BM25** over a rich per-passage index that concatenates content, summary,
///    hypothetical questions (pre-computed HyDE), concepts, and keywords.
///    BM25 handles TF-IDF weighting, stemming, and stop-word removal.
/// 2. **KG expansion**: entities matched in the query are traversed 1-hop through
///    the knowledge graph to find related passages BM25 might miss.
/// 3. **Score fusion**: BM25 scores + KG boost, top-k returned.

use bm25::{Document, Language, SearchEngineBuilder};
use serde::Deserialize;
use std::collections::HashMap;
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

/// In-memory pixel art knowledge base with BM25 search and KG expansion.
pub struct KnowledgeBase {
    /// Stored passage data (content, summary, title) for result construction.
    passages: Vec<StoredPassage>,
    /// BM25 search engine over the enriched passage index.
    engine: bm25::SearchEngine<usize>,
    /// concept (lowercase) → passage indices, for KG-expanded retrieval.
    concept_to_passages: HashMap<String, Vec<usize>>,
    /// entity (lowercase) → related entity names, for 1-hop traversal.
    entity_relations: HashMap<String, Vec<String>>,
}

struct StoredPassage {
    content: String,
    summary: String,
    source_title: String,
}

impl KnowledgeBase {
    /// Load a knowledge base from an Ingestible corpus-export JSON file.
    /// Returns None if the file doesn't exist or can't be parsed.
    pub fn load(path: &Path) -> Option<Self> {
        let data = std::fs::read_to_string(path).ok()?;
        let corpus: CorpusFile = serde_json::from_str(&data).ok()?;

        let mut passages = Vec::new();
        let mut concept_to_passages: HashMap<String, Vec<usize>> = HashMap::new();
        let mut entity_relations: HashMap<String, Vec<String>> = HashMap::new();

        // Build the BM25 search engine with enriched documents.
        // Each passage is indexed with its content + summary + hypothetical questions +
        // concepts + keywords concatenated. This gives BM25 the richest possible
        // vocabulary to match against — especially the hypothetical questions, which
        // are pre-computed HyDE (the LLM already generated questions this passage answers).
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

                // Build the enriched searchable text for BM25.
                // Hypothetical questions are the key — they bridge the vocabulary gap
                // between user prompts and passage content.
                let searchable = format!(
                    "{}\n{}\n{}\n{}\n{}",
                    p.content,
                    p.summary,
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
                    .entry(entity_name.clone())
                    .or_default()
                    .extend(related);
            }
        }

        eprintln!(
            "knowledge: loaded {} passages, {} concepts, {} entities (BM25 indexed)",
            passages.len(),
            concept_to_passages.len(),
            entity_relations.len(),
        );

        Some(KnowledgeBase {
            passages,
            engine,
            concept_to_passages,
            entity_relations,
        })
    }

    /// Search for passages relevant to a query string.
    ///
    /// Strategy:
    /// 1. BM25 search over enriched passage index (content + summary +
    ///    hypothetical questions + concepts + keywords). BM25 handles stemming,
    ///    TF-IDF weighting, and stop-word removal automatically.
    /// 2. KG expansion: tokenize query, find matching entities, traverse 1-hop
    ///    relations, boost passages linked to related entities.
    /// 3. Fuse scores: BM25 score + KG boost, return top-k.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<KnowledgeResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        // Phase 1: BM25 search — retrieve more candidates than needed for fusion
        let bm25_limit = (top_k * 3).max(10);
        let bm25_results = self.engine.search(query, bm25_limit);

        let mut scores: HashMap<usize, f32> = HashMap::new();
        for result in &bm25_results {
            scores.insert(result.document.id, result.score);
        }

        // Phase 2: KG expansion — find entities mentioned in query, traverse 1-hop
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower
            .split(|c: char| !c.is_alphanumeric() && c != '-')
            .filter(|w| w.len() > 2)
            .collect();

        for word in &query_words {
            if let Some(related) = self.entity_relations.get(*word) {
                for related_name in related {
                    if let Some(indices) = self.concept_to_passages.get(related_name.as_str()) {
                        for &idx in indices {
                            // KG boost: add a fraction of the best BM25 score
                            let boost = bm25_results
                                .first()
                                .map(|r| r.score * 0.15)
                                .unwrap_or(1.0);
                            *scores.entry(idx).or_default() += boost;
                        }
                    }
                }
            }
        }

        // Also try multi-word entity matches from the query
        for entity in self.entity_relations.keys() {
            if entity.contains(' ') && query_lower.contains(entity.as_str()) {
                if let Some(related) = self.entity_relations.get(entity) {
                    for related_name in related {
                        if let Some(indices) = self.concept_to_passages.get(related_name.as_str())
                        {
                            let boost = bm25_results
                                .first()
                                .map(|r| r.score * 0.2)
                                .unwrap_or(1.0);
                            for &idx in indices {
                                *scores.entry(idx).or_default() += boost;
                            }
                        }
                    }
                }
            }
        }

        // Phase 3: Rank and return top-k
        let mut ranked: Vec<(usize, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        ranked
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
            .collect()
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

        // Search for dithering — should find relevant passages via BM25 + hypothetical questions
        let results = kb.search("dithering techniques Bayer pattern", 3);
        assert!(!results.is_empty(), "should find dithering passages");
        assert!(results[0].score > 0.0);
        eprintln!("dithering results:");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }

        // Search for WFC content
        let results = kb.search("wave function collapse tilemap", 3);
        assert!(!results.is_empty(), "should find WFC passages");
        eprintln!("WFC results:");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }

        // Creative prompt — should still find relevant passages via hypothetical questions
        let results = kb.search("dark stone wall with moss growing", 3);
        eprintln!("creative prompt results:");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }

        // Retro style query — should find hardware constraint passages
        let results = kb.search("make it look retro like a NES game", 3);
        assert!(!results.is_empty(), "should find retro/NES passages");
        eprintln!("retro results:");
        for r in &results {
            eprintln!("  [{:.2}] {} — {}", r.score, r.source_title, &r.summary[..r.summary.len().min(60)]);
        }
    }
}
