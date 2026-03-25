/// Narrative-to-map pipeline.
///
/// Transforms spatial descriptions into WFC constraint configurations,
/// then generates a tilemap. The LLM (or a rule parser) extracts
/// predicates from natural language; the WFC engine assembles the map.

use crate::adjacency::{AdjacencyRules, TileEdges};
use crate::semantic::{self, SemanticRule, TileAffordance};
use crate::wfc::{self, Pin, WfcConfig, WfcError, WfcResult};
use std::collections::HashMap;

/// A spatial predicate extracted from a narrative prompt.
#[derive(Debug, Clone)]
pub enum Predicate {
    /// Place a specific region at a biased position.
    Region {
        name: String,
        tile_type: String,       // affordance or tile name pattern
        min_size: (usize, usize), // minimum width x height in tiles
        position: Position,
    },
    /// Border the map with a specific tile type.
    Border {
        tile_type: String,
    },
    /// Ensure a path exists between two points.
    PathRequired {
        from: (usize, usize),
        to: (usize, usize),
    },
}

/// Approximate position bias within the map.
#[derive(Debug, Clone)]
pub enum Position {
    North,
    South,
    East,
    West,
    Northeast,
    Northwest,
    Southeast,
    Southwest,
    Center,
    Anywhere,
}

impl Position {
    /// Parse a position string.
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "north" | "n" | "top" => Position::North,
            "south" | "s" | "bottom" => Position::South,
            "east" | "e" | "right" => Position::East,
            "west" | "w" | "left" => Position::West,
            "northeast" | "ne" | "top-right" => Position::Northeast,
            "northwest" | "nw" | "top-left" => Position::Northwest,
            "southeast" | "se" | "bottom-right" => Position::Southeast,
            "southwest" | "sw" | "bottom-left" => Position::Southwest,
            "center" | "middle" => Position::Center,
            _ => Position::Anywhere,
        }
    }

    /// Get the bias center as (x_fraction, y_fraction) of the map.
    fn bias_center(&self) -> (f64, f64) {
        match self {
            Position::North => (0.5, 0.2),
            Position::South => (0.5, 0.8),
            Position::East => (0.8, 0.5),
            Position::West => (0.2, 0.5),
            Position::Northeast => (0.8, 0.2),
            Position::Northwest => (0.2, 0.2),
            Position::Southeast => (0.8, 0.8),
            Position::Southwest => (0.2, 0.8),
            Position::Center => (0.5, 0.5),
            Position::Anywhere => (0.5, 0.5),
        }
    }
}

/// Configuration for narrate_map.
pub struct NarrateConfig {
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub max_retries: u32,
    pub predicates: Vec<Predicate>,
}

/// Result of a narrate_map run.
pub struct NarrateResult {
    pub grid: Vec<Vec<usize>>,
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub retries: u32,
    pub pins_applied: usize,
}

/// Run the narrate-to-map pipeline.
///
/// 1. Convert predicates to WFC pins and constraints
/// 2. Run WFC with pins pre-collapsed
/// 3. Validate path requirements
/// 4. Retry on contradiction or blocked paths
pub fn narrate_map(
    tiles: &[TileEdges],
    affordances: &[TileAffordance],
    rules: &AdjacencyRules,
    forbids: &[SemanticRule],
    requires: &[SemanticRule],
    require_boost: f64,
    config: &NarrateConfig,
) -> Result<NarrateResult, WfcError> {
    let weights: Vec<f64> = tiles.iter().map(|t| t.weight).collect();

    // Build tile lookup by name
    let name_to_idx: HashMap<&str, usize> = tiles
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.as_str(), i))
        .collect();

    // Build affordance-to-tile-indices lookup
    let affordance_to_tiles: HashMap<&str, Vec<usize>> = {
        let mut map: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, aff) in affordances.iter().enumerate() {
            if let Some(ref a) = aff.affordance {
                map.entry(a.as_str()).or_default().push(i);
            }
        }
        map
    };

    let mut last_contradiction = (0usize, 0usize);
    for retry in 0..=config.max_retries {
        let seed = config.seed + retry as u64;
        let pins = build_pins_from_predicates(
            &config.predicates,
            &name_to_idx,
            &affordance_to_tiles,
            config.width,
            config.height,
        );
        let pins_count = pins.len();

        // Run WFC
        let wfc_config = WfcConfig {
            width: config.width,
            height: config.height,
            seed,
            max_retries: 0, // we handle retries ourselves
            weights: weights.clone(),
            tile_names: tiles.iter().map(|t| t.name.clone()).collect(),
            affordances: affordances.to_vec(),
            forbids_rules: forbids.to_vec(),
            requires_rules: requires.to_vec(),
            require_boost,
        };

        match wfc::run_wfc(rules, &wfc_config, &pins) {
            Ok(result) => {
                // Validate path requirements
                let mut paths_ok = true;
                for pred in &config.predicates {
                    if let Predicate::PathRequired { from, to } = pred {
                        if !check_path(&result.grid, affordances, *from, *to) {
                            paths_ok = false;
                            break;
                        }
                    }
                }

                if paths_ok {
                    return Ok(NarrateResult {
                        grid: result.grid,
                        width: config.width,
                        height: config.height,
                        seed,
                        retries: retry,
                        pins_applied: pins_count,
                    });
                }
                // Path blocked — retry
            }
            Err(WfcError::Contradiction { x, y }) => {
                last_contradiction = (x, y);
                // Retry with next seed
            }
            Err(WfcError::ExhaustedRetries { last_x, last_y, .. }) => {
                last_contradiction = (last_x, last_y);
                // Inner WFC exhausted (max_retries=0), but narrate has its own retry loop
            }
            Err(e) => return Err(e),
        }
    }

    // Count compatible pairs for diagnostic
    let mut compatible_pairs = 0;
    let total_pairs = tiles.len() * tiles.len() * 4;
    for tile_idx in 0..tiles.len() {
        for dir in crate::adjacency::Direction::all() {
            compatible_pairs += rules.compatible(tile_idx, dir).count_ones(..);
        }
    }

    // Build diagnostic pins from the last retry's predicates
    let tile_names: Vec<String> = tiles.iter().map(|t| t.name.clone()).collect();
    let last_pins = build_pins_from_predicates(
        &config.predicates,
        &name_to_idx,
        &affordance_to_tiles,
        config.width,
        config.height,
    );
    let diagnostics = wfc::diagnose_wfc_failure(rules, &tile_names, &last_pins, config.width, config.height);

    Err(WfcError::ExhaustedRetries {
        retries: config.max_retries,
        last_x: last_contradiction.0,
        last_y: last_contradiction.1,
        compatible_pairs,
        total_pairs,
        diagnostics,
    })
}

/// Build pins from predicates (extracted for reuse in diagnostics).
fn build_pins_from_predicates(
    predicates: &[Predicate],
    name_to_idx: &HashMap<&str, usize>,
    affordance_to_tiles: &HashMap<&str, Vec<usize>>,
    width: usize,
    height: usize,
) -> Vec<Pin> {
    let mut pins = Vec::new();
    for pred in predicates {
        match pred {
            Predicate::Border { tile_type } => {
                if let Some(idx) = find_tile_for_type(tile_type, name_to_idx, affordance_to_tiles) {
                    for x in 0..width {
                        pins.push(Pin { x, y: 0, tile_idx: idx });
                        pins.push(Pin { x, y: height - 1, tile_idx: idx });
                    }
                    for y in 1..height - 1 {
                        pins.push(Pin { x: 0, y, tile_idx: idx });
                        pins.push(Pin { x: width - 1, y, tile_idx: idx });
                    }
                }
            }
            Predicate::Region { tile_type, min_size, position, .. } => {
                if let Some(idx) = find_tile_for_type(tile_type, name_to_idx, affordance_to_tiles) {
                    let (bx, by) = position.bias_center();
                    let cx = (bx * width as f64) as usize;
                    let cy = (by * height as f64) as usize;
                    let half_w = min_size.0 / 2;
                    let half_h = min_size.1 / 2;
                    for dy in 0..min_size.1.min(3) {
                        for dx in 0..min_size.0.min(3) {
                            let px = (cx + dx).saturating_sub(half_w).min(width - 1);
                            let py = (cy + dy).saturating_sub(half_h).min(height - 1);
                            pins.push(Pin { x: px, y: py, tile_idx: idx });
                        }
                    }
                }
            }
            Predicate::PathRequired { .. } => {}
        }
    }
    pins
}

/// Find a tile index matching a type string (tile name or affordance).
fn find_tile_for_type(
    tile_type: &str,
    name_to_idx: &HashMap<&str, usize>,
    affordance_to_tiles: &HashMap<&str, Vec<usize>>,
) -> Option<usize> {
    // Try exact tile name first
    if let Some(&idx) = name_to_idx.get(tile_type) {
        return Some(idx);
    }
    // Try as affordance — return first matching tile
    if let Some(tiles) = affordance_to_tiles.get(tile_type) {
        return tiles.first().copied();
    }
    None
}

/// BFS pathfinding on the collapsed grid. Returns true if a passable path exists.
fn check_path(
    grid: &[Vec<usize>],
    affordances: &[TileAffordance],
    from: (usize, usize),
    to: (usize, usize),
) -> bool {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return false };

    let mut visited = vec![vec![false; w]; h];
    let mut queue = std::collections::VecDeque::new();

    queue.push_back(from);
    visited[from.1][from.0] = true;

    while let Some((x, y)) = queue.pop_front() {
        if (x, y) == to {
            return true;
        }

        for (dx, dy) in &[(0i32, -1i32), (0, 1), (-1, 0), (1, 0)] {
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            if visited[ny][nx] {
                continue;
            }

            // Check if tile is passable
            let tile_idx = grid[ny][nx];
            let passable = affordances
                .get(tile_idx)
                .and_then(|a| a.affordance.as_ref())
                .map(|a| a == "walkable" || a == "interactive")
                .unwrap_or(false);

            if passable {
                visited[ny][nx] = true;
                queue.push_back((nx, ny));
            }
        }
    }

    false
}

/// Parse a simple narrative rule string into predicates.
/// Format examples:
///   "border:wall"
///   "region:boss_room:obstacle:3x3:southeast"
///   "path:0,5:19,5"
pub fn parse_predicate(rule: &str) -> Option<Predicate> {
    let parts: Vec<&str> = rule.split(':').collect();
    match parts.first()? {
        &"border" => Some(Predicate::Border {
            tile_type: parts.get(1)?.to_string(),
        }),
        &"region" => {
            let name = parts.get(1)?.to_string();
            let tile_type = parts.get(2)?.to_string();
            let size_str = parts.get(3).unwrap_or(&"3x3");
            let pos_str = parts.get(4).unwrap_or(&"anywhere");
            let (sw, sh) = size_str
                .split_once('x')
                .map(|(w, h)| (w.parse().unwrap_or(3), h.parse().unwrap_or(3)))
                .unwrap_or((3, 3));
            Some(Predicate::Region {
                name,
                tile_type,
                min_size: (sw, sh),
                position: Position::parse(pos_str),
            })
        }
        &"path" => {
            let from_str = parts.get(1)?;
            let to_str = parts.get(2)?;
            let (fx, fy) = from_str
                .split_once(',')
                .map(|(x, y)| (x.parse().unwrap_or(0), y.parse().unwrap_or(0)))?;
            let (tx, ty) = to_str
                .split_once(',')
                .map(|(x, y)| (x.parse().unwrap_or(0), y.parse().unwrap_or(0)))?;
            Some(Predicate::PathRequired {
                from: (fx, fy),
                to: (tx, ty),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_tiles() -> (Vec<TileEdges>, Vec<TileAffordance>) {
        let tiles = vec![
            TileEdges {
                name: "wall".to_string(),
                n: "solid".to_string(), e: "solid".to_string(),
                s: "solid".to_string(), w: "solid".to_string(),
                weight: 1.0,
            },
            TileEdges {
                name: "floor".to_string(),
                n: "floor".to_string(), e: "floor".to_string(),
                s: "floor".to_string(), w: "floor".to_string(),
                weight: 2.0,
            },
        ];
        let affordances = vec![
            TileAffordance { affordance: Some("obstacle".to_string()) },
            TileAffordance { affordance: Some("walkable".to_string()) },
        ];
        (tiles, affordances)
    }

    #[test]
    fn parse_border_predicate() {
        let pred = parse_predicate("border:wall").unwrap();
        match pred {
            Predicate::Border { tile_type } => assert_eq!(tile_type, "wall"),
            _ => panic!("expected Border"),
        }
    }

    #[test]
    fn parse_region_predicate() {
        let pred = parse_predicate("region:boss:obstacle:4x4:southeast").unwrap();
        match pred {
            Predicate::Region { name, tile_type, min_size, .. } => {
                assert_eq!(name, "boss");
                assert_eq!(tile_type, "obstacle");
                assert_eq!(min_size, (4, 4));
            }
            _ => panic!("expected Region"),
        }
    }

    #[test]
    fn parse_path_predicate() {
        let pred = parse_predicate("path:0,5:19,5").unwrap();
        match pred {
            Predicate::PathRequired { from, to } => {
                assert_eq!(from, (0, 5));
                assert_eq!(to, (19, 5));
            }
            _ => panic!("expected PathRequired"),
        }
    }

    #[test]
    fn narrate_with_border() {
        let (tiles, affordances) = simple_tiles();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());

        let config = NarrateConfig {
            width: 6,
            height: 6,
            seed: 42,
            max_retries: 5,
            predicates: vec![
                Predicate::Border {
                    tile_type: "wall".to_string(),
                },
            ],
        };

        let result = narrate_map(
            &tiles, &affordances, &rules,
            &[], &[], 3.0,
            &config,
        ).unwrap();

        // Border should be walls (index 0)
        for x in 0..6 {
            assert_eq!(result.grid[0][x], 0, "top border at x={}", x);
            assert_eq!(result.grid[5][x], 0, "bottom border at x={}", x);
        }
        for y in 0..6 {
            assert_eq!(result.grid[y][0], 0, "left border at y={}", y);
            assert_eq!(result.grid[y][5], 0, "right border at y={}", y);
        }
    }

    #[test]
    fn narrate_with_region_bias() {
        let (tiles, affordances) = simple_tiles();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());

        let config = NarrateConfig {
            width: 8,
            height: 8,
            seed: 42,
            max_retries: 5,
            predicates: vec![
                Predicate::Region {
                    name: "boss".to_string(),
                    tile_type: "obstacle".to_string(),
                    min_size: (2, 2),
                    position: Position::Southeast,
                },
            ],
        };

        let result = narrate_map(
            &tiles, &affordances, &rules,
            &[], &[], 3.0,
            &config,
        ).unwrap();

        // Southeast quadrant should have obstacle tiles (index 0)
        let grid = &result.grid;
        let se_obstacles: usize = (4..8)
            .flat_map(|y| (4..8).map(move |x| grid[y][x]))
            .filter(|&t| t == 0)
            .count();
        assert!(se_obstacles > 0, "southeast should have obstacles from region bias");
    }

    #[test]
    fn path_check_finds_route() {
        let grid = vec![
            vec![0, 0, 0, 0, 0],
            vec![0, 1, 1, 1, 0],
            vec![0, 1, 0, 1, 0],
            vec![0, 1, 1, 1, 0],
            vec![0, 0, 0, 0, 0],
        ];
        let affordances = vec![
            TileAffordance { affordance: Some("obstacle".to_string()) },
            TileAffordance { affordance: Some("walkable".to_string()) },
        ];

        assert!(check_path(&grid, &affordances, (1, 1), (3, 3)));
        assert!(!check_path(&grid, &affordances, (0, 0), (4, 4))); // walls block
    }
}
