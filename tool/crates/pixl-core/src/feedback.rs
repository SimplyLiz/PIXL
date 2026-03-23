//! Feedback store — records accept/reject decisions with structured features.
//!
//! This is the local foundation for the knowledge flywheel:
//! - Accepted tiles refine the style latent
//! - Rejected tiles provide negative constraints
//! - Few-shot examples are selected from accepted tiles
//! - Acceptance rate tracks generation quality over time
//!
//! The Cognitive Vault integration replaces this with a queryable,
//! source-tracked knowledge store — but the data model is the same.

use crate::style::StyleLatent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A feedback event for a single tile decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEvent {
    pub tile_name: String,
    pub action: FeedbackAction,
    /// Style features at time of decision (from the tile itself).
    pub tile_features: Option<StyleLatent>,
    /// Style score against the session latent at time of decision.
    pub style_score: Option<f64>,
    /// Structured reject reason (not free text).
    pub reject_reason: Option<RejectReason>,
    /// The tile's grid at time of decision (for few-shot retrieval).
    pub grid: Option<Vec<Vec<char>>>,
    /// Tags from the tile.
    pub tags: Vec<String>,
    /// Target layer assignment.
    pub target_layer: Option<String>,
    /// Timestamp (seconds since epoch).
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackAction {
    Accept,
    Reject,
    Edit, // accepted after manual edits
}

/// Structured reject reasons — measurable, not free text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    TooSparse,
    TooDense,
    WrongStyle,
    BadEdges,
    PaletteViolation,
    BadComposition,
    Other(String),
}

/// Aggregated feedback statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total_accepts: usize,
    pub total_rejects: usize,
    pub total_edits: usize,
    pub acceptance_rate: f64,
    pub avg_accepted_score: f64,
    pub avg_rejected_score: f64,
    /// Most common reject reasons.
    pub top_reject_reasons: Vec<(String, usize)>,
}

/// Constraints derived from feedback history — used by generate/context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackConstraints {
    /// Style features averaged from accepted tiles.
    pub preferred_style: Option<StyleLatent>,
    /// Measurable constraints from rejections.
    pub avoid: Vec<String>,
    /// Few-shot: PAX grids of recently accepted tiles (max 3).
    pub examples: Vec<FewShotExample>,
    /// Minimum style score to pass pre-display filter.
    pub min_style_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewShotExample {
    pub name: String,
    pub grid: String,
    pub tags: Vec<String>,
}

/// In-memory feedback store with JSON persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStore {
    events: Vec<FeedbackEvent>,
}

impl FeedbackStore {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn record(&mut self, event: FeedbackEvent) {
        self.events.push(event);
    }

    pub fn events(&self) -> &[FeedbackEvent] {
        &self.events
    }

    /// Compute aggregate statistics.
    pub fn stats(&self) -> FeedbackStats {
        let accepts: Vec<_> = self.events.iter()
            .filter(|e| e.action == FeedbackAction::Accept)
            .collect();
        let rejects: Vec<_> = self.events.iter()
            .filter(|e| e.action == FeedbackAction::Reject)
            .collect();
        let edits: Vec<_> = self.events.iter()
            .filter(|e| e.action == FeedbackAction::Edit)
            .collect();

        let total = accepts.len() + rejects.len() + edits.len();
        let acceptance_rate = if total > 0 {
            (accepts.len() + edits.len()) as f64 / total as f64
        } else {
            0.0
        };

        let avg_accepted_score = {
            let scores: Vec<f64> = accepts.iter()
                .filter_map(|e| e.style_score)
                .collect();
            if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 }
        };

        let avg_rejected_score = {
            let scores: Vec<f64> = rejects.iter()
                .filter_map(|e| e.style_score)
                .collect();
            if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 }
        };

        // Count reject reasons
        let mut reason_counts: HashMap<String, usize> = HashMap::new();
        for e in &rejects {
            if let Some(ref reason) = e.reject_reason {
                let key = match reason {
                    RejectReason::TooSparse => "too_sparse",
                    RejectReason::TooDense => "too_dense",
                    RejectReason::WrongStyle => "wrong_style",
                    RejectReason::BadEdges => "bad_edges",
                    RejectReason::PaletteViolation => "palette_violation",
                    RejectReason::BadComposition => "bad_composition",
                    RejectReason::Other(_) => "other",
                };
                *reason_counts.entry(key.to_string()).or_insert(0) += 1;
            }
        }
        let mut top_reject_reasons: Vec<(String, usize)> = reason_counts.into_iter().collect();
        top_reject_reasons.sort_by(|a, b| b.1.cmp(&a.1));

        FeedbackStats {
            total_accepts: accepts.len(),
            total_rejects: rejects.len(),
            total_edits: edits.len(),
            acceptance_rate,
            avg_accepted_score,
            avg_rejected_score,
            top_reject_reasons,
        }
    }

    /// Build generation constraints from feedback history.
    /// This is the structured alternative to prompt injection.
    pub fn constraints(&self) -> FeedbackConstraints {
        let accepts: Vec<_> = self.events.iter()
            .filter(|e| e.action == FeedbackAction::Accept || e.action == FeedbackAction::Edit)
            .collect();
        let rejects: Vec<_> = self.events.iter()
            .filter(|e| e.action == FeedbackAction::Reject)
            .collect();

        // Preferred style: average of accepted tile features
        let preferred_style = {
            let features: Vec<&StyleLatent> = accepts.iter()
                .filter_map(|e| e.tile_features.as_ref())
                .collect();
            if features.is_empty() {
                None
            } else {
                Some(average_latents(&features))
            }
        };

        // Avoid constraints: derived from reject reason patterns
        let mut avoid = Vec::new();
        let stats = self.stats();
        for (reason, count) in &stats.top_reject_reasons {
            if *count >= 2 {
                // Only add constraints that appear multiple times
                avoid.push(match reason.as_str() {
                    "too_sparse" => "Avoid sparse tiles — increase pixel density.".to_string(),
                    "too_dense" => "Avoid overly dense tiles — include some transparency or variation.".to_string(),
                    "wrong_style" => "Match the established style latent closely.".to_string(),
                    "bad_edges" => "Ensure all edge rows/columns are solid for WFC compatibility.".to_string(),
                    "palette_violation" => "Use only symbols from the declared palette.".to_string(),
                    "bad_composition" => "Improve visual balance and element placement.".to_string(),
                    _ => format!("Address repeated issue: {}", reason),
                });
            }
        }

        // Few-shot examples: last 3 accepted tiles with grids
        let examples: Vec<FewShotExample> = accepts.iter()
            .rev()
            .filter_map(|e| {
                e.grid.as_ref().map(|grid| {
                    FewShotExample {
                        name: e.tile_name.clone(),
                        grid: grid.iter()
                            .map(|row| row.iter().collect::<String>())
                            .collect::<Vec<_>>()
                            .join("\n"),
                        tags: e.tags.clone(),
                    }
                })
            })
            .take(3)
            .collect();

        // Minimum style score: adaptive threshold based on acceptance history
        let min_style_score = if stats.avg_accepted_score > 0.0 && stats.total_accepts >= 3 {
            // Set threshold at 80% of average accepted score
            (stats.avg_accepted_score * 0.8).max(0.3)
        } else {
            0.0 // no filtering until we have enough data
        };

        FeedbackConstraints {
            preferred_style,
            avoid,
            examples,
            min_style_score,
        }
    }

    /// Serialize to JSON for persistence.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Load from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Average multiple style latents into a composite preference profile.
fn average_latents(latents: &[&StyleLatent]) -> StyleLatent {
    let n = latents.len() as f64;
    if n == 0.0 {
        return StyleLatent::default();
    }

    StyleLatent {
        light_direction: latents.iter().map(|l| l.light_direction).sum::<f64>() / n,
        run_length_mean: latents.iter().map(|l| l.run_length_mean).sum::<f64>() / n,
        shadow_ratio: latents.iter().map(|l| l.shadow_ratio).sum::<f64>() / n,
        palette_breadth: latents.iter().map(|l| l.palette_breadth).sum::<f64>() / n,
        pixel_density: latents.iter().map(|l| l.pixel_density).sum::<f64>() / n,
        palette_entropy: latents.iter().map(|l| l.palette_entropy).sum::<f64>() / n,
        hue_bias: {
            // Circular mean for hue
            let x: f64 = latents.iter().map(|l| (l.hue_bias * std::f64::consts::PI / 180.0).cos()).sum::<f64>() / n;
            let y: f64 = latents.iter().map(|l| (l.hue_bias * std::f64::consts::PI / 180.0).sin()).sum::<f64>() / n;
            let h = y.atan2(x) * 180.0 / std::f64::consts::PI;
            if h < 0.0 { h + 360.0 } else { h }
        },
        luminance_mean: latents.iter().map(|l| l.luminance_mean).sum::<f64>() / n,
        sample_count: latents.iter().map(|l| l.sample_count).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn now() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    #[test]
    fn record_and_stats() {
        let mut store = FeedbackStore::new();
        store.record(FeedbackEvent {
            tile_name: "wall_01".to_string(),
            action: FeedbackAction::Accept,
            tile_features: None,
            style_score: Some(0.85),
            reject_reason: None,
            grid: None,
            tags: vec!["wall".to_string()],
            target_layer: Some("walls".to_string()),
            timestamp: now(),
        });
        store.record(FeedbackEvent {
            tile_name: "wall_02".to_string(),
            action: FeedbackAction::Reject,
            tile_features: None,
            style_score: Some(0.4),
            reject_reason: Some(RejectReason::BadEdges),
            grid: None,
            tags: vec!["wall".to_string()],
            target_layer: Some("walls".to_string()),
            timestamp: now(),
        });

        let stats = store.stats();
        assert_eq!(stats.total_accepts, 1);
        assert_eq!(stats.total_rejects, 1);
        assert!((stats.acceptance_rate - 0.5).abs() < 0.01);
        assert!((stats.avg_accepted_score - 0.85).abs() < 0.01);
    }

    #[test]
    fn constraints_with_few_shots() {
        let mut store = FeedbackStore::new();
        let grid = vec![
            "####".chars().collect(),
            "#++#".chars().collect(),
            "#++#".chars().collect(),
            "####".chars().collect(),
        ];
        store.record(FeedbackEvent {
            tile_name: "wall_accepted".to_string(),
            action: FeedbackAction::Accept,
            tile_features: None,
            style_score: Some(0.9),
            reject_reason: None,
            grid: Some(grid),
            tags: vec!["wall".to_string()],
            target_layer: None,
            timestamp: now(),
        });

        let constraints = store.constraints();
        assert_eq!(constraints.examples.len(), 1);
        assert_eq!(constraints.examples[0].name, "wall_accepted");
        assert!(constraints.examples[0].grid.contains("####"));
    }

    #[test]
    fn avoid_constraints_from_repeated_rejects() {
        let mut store = FeedbackStore::new();
        for i in 0..3 {
            store.record(FeedbackEvent {
                tile_name: format!("bad_{}", i),
                action: FeedbackAction::Reject,
                tile_features: None,
                style_score: Some(0.3),
                reject_reason: Some(RejectReason::BadEdges),
                grid: None,
                tags: vec![],
                target_layer: None,
                timestamp: now(),
            });
        }

        let constraints = store.constraints();
        assert!(!constraints.avoid.is_empty());
        assert!(constraints.avoid[0].contains("edge"));
    }

    #[test]
    fn json_roundtrip() {
        let mut store = FeedbackStore::new();
        store.record(FeedbackEvent {
            tile_name: "test".to_string(),
            action: FeedbackAction::Accept,
            tile_features: None,
            style_score: Some(0.9),
            reject_reason: None,
            grid: None,
            tags: vec![],
            target_layer: None,
            timestamp: now(),
        });

        let json = store.to_json();
        let restored = FeedbackStore::from_json(&json).unwrap();
        assert_eq!(restored.events().len(), 1);
        assert_eq!(restored.events()[0].tile_name, "test");
    }

    #[test]
    fn adaptive_threshold() {
        let mut store = FeedbackStore::new();
        // Add enough accepts to trigger threshold
        for i in 0..5 {
            store.record(FeedbackEvent {
                tile_name: format!("good_{}", i),
                action: FeedbackAction::Accept,
                tile_features: None,
                style_score: Some(0.8),
                reject_reason: None,
                grid: None,
                tags: vec![],
                target_layer: None,
                timestamp: now(),
            });
        }

        let constraints = store.constraints();
        // min_style_score = 0.8 * 0.8 = 0.64
        assert!(constraints.min_style_score > 0.5);
        assert!(constraints.min_style_score < 0.8);
    }
}
