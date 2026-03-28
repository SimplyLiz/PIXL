use crate::rotate;
/// Tileset completeness analyzer.
///
/// Examines a tileset's edge classes and reports which transition tiles
/// are missing for WFC to generate connected maps. Runs proactively
/// before narrate/WFC to prevent contradictions.
use crate::types::{PaxFile, TileRaw};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};

/// A missing transition tile that the tileset needs.
#[derive(Debug, Clone, Serialize)]
pub struct MissingTile {
    /// Suggested tile name.
    pub name: String,
    /// Required edge classes (n, e, s, w).
    pub edge_class: EdgeSpec,
    /// Why this tile is needed.
    pub reason: String,
    /// Whether auto_rotate = "4way" is recommended.
    pub auto_rotate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EdgeSpec {
    pub n: String,
    pub e: String,
    pub s: String,
    pub w: String,
}

/// Full completeness report.
#[derive(Debug, Clone, Serialize)]
pub struct CompletenessReport {
    /// All unique edge classes found in the tileset.
    pub edge_classes: Vec<String>,
    /// Pairs of edge classes that can be adjacent (matching edges exist).
    pub connected_pairs: Vec<(String, String)>,
    /// Pairs of edge classes with no transition tile between them.
    pub disconnected_pairs: Vec<(String, String)>,
    /// Specific tiles that should be added.
    pub missing_tiles: Vec<MissingTile>,
    /// Overall completeness score (0.0 = no connectivity, 1.0 = fully connected).
    pub score: f32,
    /// Human-readable summary.
    pub summary: String,
}

/// Analyze a tileset for completeness.
///
/// Collects all edge classes, determines which pairs can be adjacent
/// via existing tiles, and recommends specific transition tiles to fill gaps.
pub fn analyze(pax: &PaxFile) -> CompletenessReport {
    // Step 1: Collect all edge classes from tiles (including auto-rotated variants)
    let mut all_edges: Vec<(String, String, String, String, String)> = Vec::new(); // (name, n, e, s, w)

    for (name, tile) in &pax.tile {
        if tile.template.is_some() {
            continue; // skip template tiles, they inherit from base
        }
        let ec = match &tile.edge_class {
            Some(ec) => ec,
            None => continue,
        };

        all_edges.push((
            name.clone(),
            ec.n.clone(),
            ec.e.clone(),
            ec.s.clone(),
            ec.w.clone(),
        ));

        // Include auto-rotated variants
        let rotate_mode = tile.auto_rotate.as_deref().unwrap_or("none");
        if rotate_mode == "4way" || rotate_mode == "8way" {
            let ec_struct = crate::types::EdgeClass {
                n: ec.n.clone(),
                e: ec.e.clone(),
                s: ec.s.clone(),
                w: ec.w.clone(),
            };
            let mut rotated = ec_struct.clone();
            for suffix in ["_90", "_180", "_270"] {
                rotated = rotate::rotate_edge_class_cw(&rotated);
                all_edges.push((
                    format!("{}{}", name, suffix),
                    rotated.n.clone(),
                    rotated.e.clone(),
                    rotated.s.clone(),
                    rotated.w.clone(),
                ));
            }
        }
    }

    // Step 2: Collect unique edge class names
    let mut edge_classes: BTreeSet<String> = BTreeSet::new();
    for (_, n, e, s, w) in &all_edges {
        edge_classes.insert(n.clone());
        edge_classes.insert(e.clone());
        edge_classes.insert(s.clone());
        edge_classes.insert(w.clone());
    }
    let edge_classes: Vec<String> = edge_classes.into_iter().collect();

    // Step 3: Build connectivity — which edge class pairs can be adjacent?
    // Two edge classes are "connected" if there exists a tile where one class
    // appears on the north edge and another on the south edge (or east/west).
    let mut connected: HashSet<(String, String)> = HashSet::new();

    for (_, n, e, s, w) in &all_edges {
        // N-S connections: this tile connects its N edge class to its S edge class
        connected.insert((n.clone(), s.clone()));
        connected.insert((s.clone(), n.clone()));
        // E-W connections
        connected.insert((e.clone(), w.clone()));
        connected.insert((w.clone(), e.clone()));
        // Self-connections (same class on opposite sides)
        connected.insert((n.clone(), n.clone()));
        connected.insert((s.clone(), s.clone()));
        connected.insert((e.clone(), e.clone()));
        connected.insert((w.clone(), w.clone()));
    }

    // Also: two edge classes are adjacent-compatible if a tile has class A on one
    // edge and the same tile or another tile has class B, and they can be placed
    // side by side (matching edges). The real test: class A on south matches class A
    // on north of an adjacent tile.
    // Simplification: we check if for each pair (A, B), there exists a tile path
    // A → ... → B through intermediate tiles.

    // Step 4: Find transitive closure (which pairs are reachable through chains)
    let mut reachable: HashSet<(String, String)> = connected.clone();
    // Floyd-Warshall on edge classes
    for k in &edge_classes {
        for i in &edge_classes {
            for j in &edge_classes {
                if reachable.contains(&(i.clone(), k.clone()))
                    && reachable.contains(&(k.clone(), j.clone()))
                {
                    reachable.insert((i.clone(), j.clone()));
                }
            }
        }
    }

    // Step 5: Find disconnected pairs
    let mut connected_pairs: Vec<(String, String)> = Vec::new();
    let mut disconnected_pairs: Vec<(String, String)> = Vec::new();

    for (i, a) in edge_classes.iter().enumerate() {
        for b in edge_classes.iter().skip(i + 1) {
            if reachable.contains(&(a.clone(), b.clone())) {
                connected_pairs.push((a.clone(), b.clone()));
            } else {
                disconnected_pairs.push((a.clone(), b.clone()));
            }
        }
    }

    // Step 6: Generate missing tile recommendations
    let mut missing_tiles: Vec<MissingTile> = Vec::new();

    for (a, b) in &disconnected_pairs {
        // Need a transition tile with edge A on one side and edge B on the other
        let name = format!("transition_{}_{}", a, b);
        missing_tiles.push(MissingTile {
            name: name.clone(),
            edge_class: EdgeSpec {
                n: a.clone(),
                e: a.clone(),
                s: b.clone(),
                w: a.clone(),
            },
            reason: format!(
                "No tile connects '{}' edges to '{}' edges. WFC cannot transition between them.",
                a, b
            ),
            auto_rotate: true, // 4way rotation gives all cardinal transitions
        });
    }

    // Step 7: Check for one-directional transitions (A→B exists but not directly)
    // Find pairs where connection requires intermediate tiles
    let mut direct_connections: HashSet<(String, String)> = HashSet::new();
    for (_, n, e, s, w) in &all_edges {
        if n != s {
            direct_connections.insert((n.clone(), s.clone()));
            direct_connections.insert((s.clone(), n.clone()));
        }
        if e != w {
            direct_connections.insert((e.clone(), w.clone()));
            direct_connections.insert((w.clone(), e.clone()));
        }
    }

    // Step 8: Compute score
    let total_pairs = if edge_classes.len() > 1 {
        edge_classes.len() * (edge_classes.len() - 1) / 2
    } else {
        1
    };
    let score = if total_pairs > 0 {
        connected_pairs.len() as f32 / total_pairs as f32
    } else {
        1.0
    };

    // Step 9: Build summary
    let summary = if missing_tiles.is_empty() {
        format!(
            "Tileset is fully connected. {} edge classes, all pairs reachable.",
            edge_classes.len()
        )
    } else {
        let mut s = format!(
            "Tileset has {} edge class(es) but {} disconnected pair(s). ",
            edge_classes.len(),
            disconnected_pairs.len()
        );
        s.push_str("Missing transition tiles:\n");
        for mt in &missing_tiles {
            s.push_str(&format!(
                "  - {} ({}→{}, auto_rotate=4way recommended)\n",
                mt.name, mt.edge_class.n, mt.edge_class.s,
            ));
        }
        s.push_str(&format!(
            "Add these tiles to enable WFC map generation across all terrain types. Score: {:.0}%",
            score * 100.0
        ));
        s
    };

    CompletenessReport {
        edge_classes,
        connected_pairs,
        disconnected_pairs,
        missing_tiles,
        score,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn parse(source: &str) -> PaxFile {
        parser::parse_pax(source).expect("test PAX should parse")
    }

    const CONNECTED_PAX: &str = concat!(
        "[pax]\nversion = \"2.0\"\nname = \"test\"\n\n",
        "[palette.t]\n\".\" = \"#000000\"\n\"+\" = \"#ffffff\"\n\n",
        "[tile.wall]\npalette = \"t\"\nsize = \"4x4\"\n",
        "edge_class = { n = \"solid\", e = \"solid\", s = \"solid\", w = \"solid\" }\n",
        "grid = '''\n....\n....\n....\n....\n'''\n\n",
        "[tile.floor]\npalette = \"t\"\nsize = \"4x4\"\n",
        "edge_class = { n = \"floor\", e = \"floor\", s = \"floor\", w = \"floor\" }\n",
        "grid = '''\n++++\n++++\n++++\n++++\n'''\n\n",
        "[tile.wall_floor_n]\npalette = \"t\"\nsize = \"4x4\"\nauto_rotate = \"4way\"\n",
        "edge_class = { n = \"solid\", e = \"solid\", s = \"floor\", w = \"solid\" }\n",
        "grid = '''\n....\n....\n++++\n++++\n'''\n\n",
        "[tile.wall_corner_ne]\npalette = \"t\"\nsize = \"4x4\"\nauto_rotate = \"4way\"\n",
        "edge_class = { n = \"solid\", e = \"solid\", s = \"floor\", w = \"floor\" }\n",
        "grid = '''\n....\n....\n++++\n++++\n'''\n",
    );

    const DISCONNECTED_PAX: &str = concat!(
        "[pax]\nversion = \"2.0\"\nname = \"test\"\n\n",
        "[palette.t]\n\".\" = \"#000000\"\n\"+\" = \"#ffffff\"\n\n",
        "[tile.wall]\npalette = \"t\"\nsize = \"4x4\"\n",
        "edge_class = { n = \"solid\", e = \"solid\", s = \"solid\", w = \"solid\" }\n",
        "grid = '''\n....\n....\n....\n....\n'''\n\n",
        "[tile.floor]\npalette = \"t\"\nsize = \"4x4\"\n",
        "edge_class = { n = \"floor\", e = \"floor\", s = \"floor\", w = \"floor\" }\n",
        "grid = '''\n++++\n++++\n++++\n++++\n'''\n",
    );

    const THREE_CLASS_PAX: &str = concat!(
        "[pax]\nversion = \"2.0\"\nname = \"test\"\n\n",
        "[palette.t]\n\".\" = \"#000000\"\n\"+\" = \"#ffffff\"\n\"~\" = \"#0000ff\"\n\n",
        "[tile.wall]\npalette = \"t\"\nsize = \"4x4\"\n",
        "edge_class = { n = \"solid\", e = \"solid\", s = \"solid\", w = \"solid\" }\n",
        "grid = '''\n....\n....\n....\n....\n'''\n\n",
        "[tile.floor]\npalette = \"t\"\nsize = \"4x4\"\n",
        "edge_class = { n = \"floor\", e = \"floor\", s = \"floor\", w = \"floor\" }\n",
        "grid = '''\n++++\n++++\n++++\n++++\n'''\n\n",
        "[tile.water]\npalette = \"t\"\nsize = \"4x4\"\n",
        "edge_class = { n = \"water\", e = \"water\", s = \"water\", w = \"water\" }\n",
        "grid = '''\n~~~~\n~~~~\n~~~~\n~~~~\n'''\n\n",
        "[tile.wall_floor]\npalette = \"t\"\nsize = \"4x4\"\nauto_rotate = \"4way\"\n",
        "edge_class = { n = \"solid\", e = \"solid\", s = \"floor\", w = \"solid\" }\n",
        "grid = '''\n....\n....\n++++\n++++\n'''\n",
    );

    #[test]
    fn fully_connected_tileset() {
        let pax = parse(CONNECTED_PAX);
        let report = analyze(&pax);
        assert!(
            report.disconnected_pairs.is_empty(),
            "should be fully connected: {:?}",
            report.disconnected_pairs
        );
        assert_eq!(report.score, 1.0);
        assert!(report.missing_tiles.is_empty());
    }

    #[test]
    fn missing_transition() {
        let pax = parse(DISCONNECTED_PAX);
        let report = analyze(&pax);
        assert!(
            !report.disconnected_pairs.is_empty(),
            "should detect disconnection"
        );
        assert!(
            !report.missing_tiles.is_empty(),
            "should recommend missing tiles"
        );
        assert!(report.score < 1.0);
    }

    #[test]
    fn three_edge_classes_partial() {
        let pax = parse(THREE_CLASS_PAX);
        let report = analyze(&pax);
        assert!(!report.disconnected_pairs.is_empty());
        let water_missing: Vec<_> = report
            .missing_tiles
            .iter()
            .filter(|m| m.name.contains("water"))
            .collect();
        assert!(
            !water_missing.is_empty(),
            "should recommend water transition tiles"
        );
    }
}
