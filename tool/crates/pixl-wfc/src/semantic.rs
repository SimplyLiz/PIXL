/// Semantic constraint filter for WFC.
///
/// Architecture (per spec):
/// - `forbids` rules: HARD constraints applied during AC-3 propagation (prune impossible tiles)
/// - `requires` rules: SOFT weight bias applied at collapse-time only (boost probability)
///
/// Applying `requires` during propagation causes spurious contradictions because
/// early cells are in superposition.

/// A parsed semantic rule.
#[derive(Debug, Clone)]
pub enum SemanticRule {
    /// Hard constraint: tile with affordance X cannot be adjacent to tile with affordance Y.
    /// Applied during propagation.
    Forbids {
        source_affordance: String,
        target_affordance: String,
    },
    /// Soft constraint: tile with affordance X prefers to be adjacent to tile with affordance Y.
    /// Applied as weight bias at collapse time.
    Requires {
        source_affordance: String,
        target_affordance: String,
    },
}

/// Parse a forbids rule string: "affordance:X forbids affordance:Y adjacent"
pub fn parse_forbids(rule: &str) -> Option<SemanticRule> {
    let parts: Vec<&str> = rule.split_whitespace().collect();
    // Expected: ["affordance:X", "forbids", "affordance:Y", "adjacent"]
    if parts.len() >= 3 && parts[1] == "forbids" {
        let source = parts[0].strip_prefix("affordance:")?;
        let target = parts[2].strip_prefix("affordance:")
            .or_else(|| parts[2].strip_prefix("type:"))?;
        Some(SemanticRule::Forbids {
            source_affordance: source.to_string(),
            target_affordance: target.to_string(),
        })
    } else {
        None
    }
}

/// Parse a requires rule string: "affordance:X requires affordance:Y adjacent_any"
pub fn parse_requires(rule: &str) -> Option<SemanticRule> {
    let parts: Vec<&str> = rule.split_whitespace().collect();
    if parts.len() >= 3 && parts[1] == "requires" {
        let source = parts[0].strip_prefix("affordance:")?;
        let target = parts[2].strip_prefix("affordance:")
            .or_else(|| parts[2].strip_prefix("type:"))?;
        Some(SemanticRule::Requires {
            source_affordance: source.to_string(),
            target_affordance: target.to_string(),
        })
    } else {
        None
    }
}

/// Tile affordance metadata for semantic checks.
#[derive(Debug, Clone)]
pub struct TileAffordance {
    pub affordance: Option<String>,
}

/// Check if a candidate tile is forbidden from being placed, given current neighbor
/// possibilities. Returns false if the tile should be pruned.
///
/// Called during AC-3 propagation — only `Forbids` rules apply here.
pub fn check_forbids(
    candidate_affordance: &Option<String>,
    neighbor_affordances: &[&Option<String>],
    forbids_rules: &[SemanticRule],
) -> bool {
    let Some(cand) = candidate_affordance else {
        return true; // no affordance = no rules apply
    };

    for rule in forbids_rules {
        if let SemanticRule::Forbids {
            source_affordance,
            target_affordance,
        } = rule
        {
            if cand == source_affordance {
                // Check if ANY neighbor has the forbidden affordance
                for neighbor in neighbor_affordances {
                    if let Some(naff) = neighbor {
                        if naff == target_affordance {
                            return false; // forbidden!
                        }
                    }
                }
            }
        }
    }

    true // not forbidden
}

/// Compute weight bias for a candidate tile based on `requires` rules.
/// Returns a multiplier (>1.0 = boosted, <1.0 = suppressed).
///
/// Called at collapse time only — NOT during propagation.
pub fn compute_requires_bias(
    candidate_affordance: &Option<String>,
    neighbor_affordances: &[&Option<String>],
    requires_rules: &[SemanticRule],
    require_boost: f64,
) -> f64 {
    let Some(cand) = candidate_affordance else {
        return 1.0;
    };

    let mut bias = 1.0;

    for rule in requires_rules {
        if let SemanticRule::Requires {
            source_affordance,
            target_affordance,
        } = rule
        {
            if cand == source_affordance {
                let has_match = neighbor_affordances.iter().any(|n| {
                    n.as_ref().is_some_and(|naff| naff == target_affordance)
                });
                if has_match {
                    bias *= require_boost;
                } else {
                    bias *= 0.1; // suppress, don't prune
                }
            }
        }
    }

    bias
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_forbids_rule() {
        let rule = parse_forbids("affordance:obstacle forbids affordance:hazard adjacent").unwrap();
        match rule {
            SemanticRule::Forbids { source_affordance, target_affordance } => {
                assert_eq!(source_affordance, "obstacle");
                assert_eq!(target_affordance, "hazard");
            }
            _ => panic!("expected Forbids"),
        }
    }

    #[test]
    fn parse_requires_rule() {
        let rule = parse_requires("affordance:walkable requires affordance:obstacle adjacent_any").unwrap();
        match rule {
            SemanticRule::Requires { source_affordance, target_affordance } => {
                assert_eq!(source_affordance, "walkable");
                assert_eq!(target_affordance, "obstacle");
            }
            _ => panic!("expected Requires"),
        }
    }

    #[test]
    fn forbids_blocks_adjacent() {
        let rules = vec![SemanticRule::Forbids {
            source_affordance: "wall".to_string(),
            target_affordance: "water".to_string(),
        }];

        let candidate = Some("wall".to_string());
        let neighbor = Some("water".to_string());
        assert!(!check_forbids(&candidate, &[&neighbor], &rules));
    }

    #[test]
    fn forbids_allows_non_matching() {
        let rules = vec![SemanticRule::Forbids {
            source_affordance: "wall".to_string(),
            target_affordance: "water".to_string(),
        }];

        let candidate = Some("wall".to_string());
        let neighbor = Some("floor".to_string());
        assert!(check_forbids(&candidate, &[&neighbor], &rules));
    }

    #[test]
    fn requires_boosts_matching() {
        let rules = vec![SemanticRule::Requires {
            source_affordance: "floor".to_string(),
            target_affordance: "wall".to_string(),
        }];

        let candidate = Some("floor".to_string());
        let neighbor = Some("wall".to_string());
        let bias = compute_requires_bias(&candidate, &[&neighbor], &rules, 3.0);
        assert!((bias - 3.0).abs() < 0.001);
    }

    #[test]
    fn requires_suppresses_missing() {
        let rules = vec![SemanticRule::Requires {
            source_affordance: "floor".to_string(),
            target_affordance: "wall".to_string(),
        }];

        let candidate = Some("floor".to_string());
        let neighbor = Some("floor".to_string()); // not wall
        let bias = compute_requires_bias(&candidate, &[&neighbor], &rules, 3.0);
        assert!((bias - 0.1).abs() < 0.001);
    }

    #[test]
    fn no_affordance_means_no_rules() {
        let rules = vec![SemanticRule::Forbids {
            source_affordance: "wall".to_string(),
            target_affordance: "water".to_string(),
        }];

        let candidate: Option<String> = None;
        let neighbor = Some("water".to_string());
        assert!(check_forbids(&candidate, &[&neighbor], &rules));
    }
}
