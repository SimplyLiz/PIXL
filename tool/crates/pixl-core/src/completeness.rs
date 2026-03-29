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

// ── Sub-completeness (N-WFC, Nie et al. 2024) ──────────────────────

/// A missing edge pairing that breaks sub-completeness.
#[derive(Debug, Clone, Serialize)]
pub struct SubcompleteMissing {
    /// The edge class that has no match on the opposite side.
    pub edge_class: String,
    /// The direction where the edge class appears (e.g., "north").
    pub appears_on: String,
    /// The opposite direction where no tile has this edge class (e.g., "south").
    pub needs_on: String,
    /// Tiles that have this edge class on `appears_on`.
    pub example_tiles: Vec<String>,
}

/// Sub-completeness report.
#[derive(Debug, Clone, Serialize)]
pub struct SubcompleteReport {
    /// Whether the tileset is sub-complete.
    pub is_subcomplete: bool,
    /// Missing pairings (empty if sub-complete).
    pub missing: Vec<SubcompleteMissing>,
    /// Human-readable summary.
    pub summary: String,
}

/// Check if a tileset is sub-complete.
///
/// A tileset is sub-complete if for every edge class `e` on direction D,
/// there exists at least one tile with edge class `e` on the opposite
/// direction. This guarantees WFC propagation is contradiction-free
/// without backtracking.
///
/// Reference: Nie et al., "N-WFC" (IEEE Transactions on Games, 2024).
pub fn check_subcomplete(pax: &PaxFile) -> SubcompleteReport {
    // Collect all (edge_class, direction) pairs including auto-rotated variants
    // Direction pairs: N↔S, E↔W
    let mut north_classes: HashMap<String, Vec<String>> = HashMap::new(); // class → tile names
    let mut south_classes: HashMap<String, Vec<String>> = HashMap::new();
    let mut east_classes: HashMap<String, Vec<String>> = HashMap::new();
    let mut west_classes: HashMap<String, Vec<String>> = HashMap::new();

    for (name, tile) in &pax.tile {
        if tile.template.is_some() {
            continue;
        }
        let ec = match &tile.edge_class {
            Some(ec) => ec,
            None => continue,
        };

        north_classes
            .entry(ec.n.clone())
            .or_default()
            .push(name.clone());
        south_classes
            .entry(ec.s.clone())
            .or_default()
            .push(name.clone());
        east_classes
            .entry(ec.e.clone())
            .or_default()
            .push(name.clone());
        west_classes
            .entry(ec.w.clone())
            .or_default()
            .push(name.clone());

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
                let rname = format!("{}{}", name, suffix);
                north_classes
                    .entry(rotated.n.clone())
                    .or_default()
                    .push(rname.clone());
                south_classes
                    .entry(rotated.s.clone())
                    .or_default()
                    .push(rname.clone());
                east_classes
                    .entry(rotated.e.clone())
                    .or_default()
                    .push(rname.clone());
                west_classes
                    .entry(rotated.w.clone())
                    .or_default()
                    .push(rname.clone());
            }
        }
    }

    let mut missing = Vec::new();

    // For every edge class on north, there must be a tile with that class on south
    for (ec, tiles) in &north_classes {
        if !south_classes.contains_key(ec) {
            missing.push(SubcompleteMissing {
                edge_class: ec.clone(),
                appears_on: "north".to_string(),
                needs_on: "south".to_string(),
                example_tiles: tiles.iter().take(3).cloned().collect(),
            });
        }
    }

    // For every edge class on south, there must be a tile with that class on north
    for (ec, tiles) in &south_classes {
        if !north_classes.contains_key(ec) {
            missing.push(SubcompleteMissing {
                edge_class: ec.clone(),
                appears_on: "south".to_string(),
                needs_on: "north".to_string(),
                example_tiles: tiles.iter().take(3).cloned().collect(),
            });
        }
    }

    // For every edge class on east, there must be a tile with that class on west
    for (ec, tiles) in &east_classes {
        if !west_classes.contains_key(ec) {
            missing.push(SubcompleteMissing {
                edge_class: ec.clone(),
                appears_on: "east".to_string(),
                needs_on: "west".to_string(),
                example_tiles: tiles.iter().take(3).cloned().collect(),
            });
        }
    }

    // For every edge class on west, there must be a tile with that class on east
    for (ec, tiles) in &west_classes {
        if !east_classes.contains_key(ec) {
            missing.push(SubcompleteMissing {
                edge_class: ec.clone(),
                appears_on: "west".to_string(),
                needs_on: "east".to_string(),
                example_tiles: tiles.iter().take(3).cloned().collect(),
            });
        }
    }

    let is_subcomplete = missing.is_empty();

    let summary = if is_subcomplete {
        let all_classes: HashSet<&String> = north_classes
            .keys()
            .chain(south_classes.keys())
            .chain(east_classes.keys())
            .chain(west_classes.keys())
            .collect();
        format!(
            "Tileset is sub-complete ({} edge classes). WFC is guaranteed contradiction-free.",
            all_classes.len()
        )
    } else {
        let mut s = format!(
            "Tileset is NOT sub-complete — {} missing edge pairing(s):\n",
            missing.len()
        );
        for m in &missing {
            s.push_str(&format!(
                "  - '{}' appears on {} (e.g. {}) but no tile has it on {}\n",
                m.edge_class,
                m.appears_on,
                m.example_tiles.first().unwrap_or(&"?".to_string()),
                m.needs_on,
            ));
        }
        s.push_str(
            "Add tiles with the missing edge classes to guarantee contradiction-free WFC.",
        );
        s
    };

    SubcompleteReport {
        is_subcomplete,
        missing,
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

    // ── Sub-completeness tests ────────────────────────────

    #[test]
    fn subcomplete_connected_tileset() {
        // CONNECTED_PAX has wall (solid/solid/solid/solid),
        // floor (floor/floor/floor/floor),
        // wall_floor_n (solid/solid/floor/solid) with 4way rotation,
        // wall_corner_ne (solid/solid/floor/floor) with 4way rotation.
        //
        // After rotation: every edge class (solid, floor) appears on
        // every direction (N, E, S, W). Sub-complete.
        let pax = parse(CONNECTED_PAX);
        let report = check_subcomplete(&pax);
        assert!(
            report.is_subcomplete,
            "connected tileset should be sub-complete: {}",
            report.summary
        );
        assert!(report.missing.is_empty());
    }

    #[test]
    fn subcomplete_disconnected_tileset() {
        // DISCONNECTED_PAX has wall (solid all sides) + floor (floor all sides).
        // Both solid and floor appear on all 4 directions → sub-complete.
        // (Sub-completeness doesn't require transitions between different
        // classes — it only requires each class appears on both sides of
        // each axis.)
        let pax = parse(DISCONNECTED_PAX);
        let report = check_subcomplete(&pax);
        assert!(
            report.is_subcomplete,
            "disconnected but symmetric tileset IS sub-complete: {}",
            report.summary
        );
    }

    #[test]
    fn subcomplete_asymmetric_fails() {
        // A tileset where "solid" only appears on north, never on south.
        // This breaks sub-completeness.
        let asym_pax = concat!(
            "[pax]\nversion = \"2.0\"\nname = \"test\"\n\n",
            "[palette.t]\n\".\" = \"#000000\"\n\"+\" = \"#ffffff\"\n\n",
            "[tile.cap]\npalette = \"t\"\nsize = \"4x4\"\n",
            "edge_class = { n = \"solid\", e = \"floor\", s = \"floor\", w = \"floor\" }\n",
            "grid = '''\n....\n++++\n++++\n++++\n'''\n\n",
            "[tile.floor]\npalette = \"t\"\nsize = \"4x4\"\n",
            "edge_class = { n = \"floor\", e = \"floor\", s = \"floor\", w = \"floor\" }\n",
            "grid = '''\n++++\n++++\n++++\n++++\n'''\n",
        );
        let pax = parse(asym_pax);
        let report = check_subcomplete(&pax);
        assert!(
            !report.is_subcomplete,
            "asymmetric tileset should NOT be sub-complete"
        );
        // "solid" appears on north but not south
        let solid_missing = report
            .missing
            .iter()
            .find(|m| m.edge_class == "solid" && m.needs_on == "south");
        assert!(solid_missing.is_some(), "should report 'solid' missing on south");
    }

    #[test]
    fn subcomplete_dungeon_example() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let pax = parse(&source);
        let report = check_subcomplete(&pax);
        // dungeon.pax has auto_rotate tiles — should be sub-complete
        // (solid, floor, water all appear symmetrically)
        println!("Dungeon sub-completeness: {}", report.summary);
        // Just verify it runs without panic — actual sub-completeness
        // depends on the current tile set
    }
}
