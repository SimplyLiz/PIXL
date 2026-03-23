use crate::adjacency::{AdjacencyRules, Direction};
use crate::semantic::{self, SemanticRule, TileAffordance};
use fixedbitset::FixedBitSet;
use rand::prelude::*;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WfcError {
    #[error("contradiction at ({x}, {y}): no valid tiles remaining")]
    Contradiction { x: usize, y: usize },

    #[error("WFC failed after {retries} retries (last contradiction at ({last_x}, {last_y}), {compatible_pairs} compatible tile pairs out of {total_pairs} possible)")]
    ExhaustedRetries {
        retries: u32,
        last_x: usize,
        last_y: usize,
        compatible_pairs: usize,
        total_pairs: usize,
    },

    #[error("empty tileset")]
    EmptyTileset,
}

/// Result of a successful WFC run.
pub struct WfcResult {
    pub grid: Vec<Vec<usize>>, // tile indices
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub retries: u32,
}

/// Configuration for a WFC run.
pub struct WfcConfig {
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub max_retries: u32,
    pub weights: Vec<f64>,
    pub affordances: Vec<TileAffordance>,
    pub forbids_rules: Vec<SemanticRule>,
    pub requires_rules: Vec<SemanticRule>,
    pub require_boost: f64,
}

/// A pre-collapsed cell (pin).
pub struct Pin {
    pub x: usize,
    pub y: usize,
    pub tile_idx: usize,
}

/// Run WFC with optional pre-collapsed pins.
pub fn run_wfc(
    rules: &AdjacencyRules,
    config: &WfcConfig,
    pins: &[Pin],
) -> Result<WfcResult, WfcError> {
    let n = rules.num_tiles();
    if n == 0 {
        return Err(WfcError::EmptyTileset);
    }

    let mut last_contradiction = (0, 0);
    for retry in 0..=config.max_retries {
        let seed = config.seed + retry as u64;
        match attempt_wfc(rules, config, pins, n, seed) {
            Ok(grid) => {
                return Ok(WfcResult {
                    grid,
                    width: config.width,
                    height: config.height,
                    seed,
                    retries: retry,
                });
            }
            Err(WfcError::Contradiction { x, y }) if retry < config.max_retries => {
                last_contradiction = (x, y);
                continue; // retry with next seed
            }
            Err(WfcError::Contradiction { x, y }) => {
                last_contradiction = (x, y);
            }
            Err(e) => return Err(e),
        }
    }

    // Count compatible pairs to help diagnose connectivity issues
    let mut compatible_pairs = 0;
    let total_pairs = n * n * 4;
    for tile_idx in 0..n {
        for dir in Direction::all() {
            compatible_pairs += rules.compatible(tile_idx, dir).count_ones(..);
        }
    }

    Err(WfcError::ExhaustedRetries {
        retries: config.max_retries,
        last_x: last_contradiction.0,
        last_y: last_contradiction.1,
        compatible_pairs,
        total_pairs,
    })
}

fn attempt_wfc(
    rules: &AdjacencyRules,
    config: &WfcConfig,
    pins: &[Pin],
    num_tiles: usize,
    seed: u64,
) -> Result<Vec<Vec<usize>>, WfcError> {
    let w = config.width;
    let h = config.height;
    let mut rng = StdRng::seed_from_u64(seed);

    // Initialize: every cell can be any tile
    let mut cells: Vec<FixedBitSet> = vec![
        {
            let mut bs = FixedBitSet::with_capacity(num_tiles);
            bs.set_range(.., true);
            bs
        };
        w * h
    ];

    let mut prop_stack: Vec<(usize, usize)> = Vec::new();

    // Pre-collapse pinned cells
    for pin in pins {
        let idx = pin.y * w + pin.x;
        if idx < cells.len() {
            cells[idx].clear();
            cells[idx].insert(pin.tile_idx);
            prop_stack.push((pin.x, pin.y));
        }
    }

    // Propagate from pins
    propagate(&mut cells, &prop_stack, rules, config, w, h, num_tiles)?;
    prop_stack.clear();

    // Main loop
    loop {
        // Find lowest entropy uncollapsed cell
        let chosen = find_lowest_entropy(&cells, &config.weights, w, h, &mut rng);
        let Some((cx, cy)) = chosen else {
            break; // all collapsed
        };

        let cell_idx = cy * w + cx;

        // Collapse: weighted random selection with semantic requires bias
        let tile = collapse_cell(
            &cells[cell_idx],
            &config.weights,
            &config.affordances,
            &cells,
            &config.requires_rules,
            config.require_boost,
            cx,
            cy,
            w,
            h,
            &mut rng,
        );

        cells[cell_idx].clear();
        cells[cell_idx].insert(tile);

        prop_stack.push((cx, cy));
        propagate(&mut cells, &prop_stack, rules, config, w, h, num_tiles)?;
        prop_stack.clear();
    }

    // Convert to tile index grid
    let mut grid = vec![vec![0usize; w]; h];
    for y in 0..h {
        for x in 0..w {
            let cell = &cells[y * w + x];
            grid[y][x] = cell.ones().next().unwrap_or(0);
        }
    }

    Ok(grid)
}

/// Find the uncollapsed cell with lowest Shannon entropy.
fn find_lowest_entropy(
    cells: &[FixedBitSet],
    weights: &[f64],
    w: usize,
    h: usize,
    rng: &mut StdRng,
) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    let mut best_entropy = f64::MAX;

    for y in 0..h {
        for x in 0..w {
            let cell = &cells[y * w + x];
            let count = cell.count_ones(..);
            if count <= 1 {
                continue; // already collapsed
            }

            let entropy = shannon_entropy(cell, weights);
            // Add noise to break ties randomly
            let noisy = entropy - rng.random::<f64>() * 0.001;
            if noisy < best_entropy {
                best_entropy = noisy;
                best = Some((x, y));
            }
        }
    }

    best
}

/// Shannon entropy of a cell's possibility set.
fn shannon_entropy(cell: &FixedBitSet, weights: &[f64]) -> f64 {
    let sum: f64 = cell.ones().map(|i| weights[i].max(1e-10)).sum();
    if sum <= 0.0 {
        return 0.0;
    }

    let mut entropy = 0.0;
    for i in cell.ones() {
        let p = weights[i].max(1e-10) / sum;
        entropy -= p * p.ln();
    }
    entropy
}

/// Collapse a cell to one tile via weighted random selection.
/// Applies `requires` semantic rules as weight bias.
fn collapse_cell(
    cell: &FixedBitSet,
    weights: &[f64],
    affordances: &[TileAffordance],
    all_cells: &[FixedBitSet],
    requires_rules: &[SemanticRule],
    require_boost: f64,
    cx: usize,
    cy: usize,
    w: usize,
    h: usize,
    rng: &mut StdRng,
) -> usize {
    // Collect neighbor affordances for requires bias
    let neighbor_affs: Vec<&Option<String>> = Direction::all()
        .iter()
        .filter_map(|dir| {
            let (dx, dy) = dir.delta();
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                let n_idx = ny as usize * w + nx as usize;
                let n_cell = &all_cells[n_idx];
                // Use first possible tile's affordance as representative
                n_cell.ones().next().map(|t| &affordances[t].affordance)
            } else {
                None
            }
        })
        .collect();

    // Compute adjusted weights
    let adjusted: Vec<(usize, f64)> = cell
        .ones()
        .map(|i| {
            let base_w = weights[i].max(1e-10);
            let bias = semantic::compute_requires_bias(
                &affordances[i].affordance,
                &neighbor_affs,
                requires_rules,
                require_boost,
            );
            (i, base_w * bias)
        })
        .collect();

    let total: f64 = adjusted.iter().map(|(_, w)| w).sum();
    if total <= 0.0 {
        return adjusted.first().map(|(i, _)| *i).unwrap_or(0);
    }

    // Weighted random selection
    let mut roll = rng.random::<f64>() * total;
    for (idx, w) in &adjusted {
        roll -= w;
        if roll <= 0.0 {
            return *idx;
        }
    }

    adjusted.last().map(|(i, _)| *i).unwrap_or(0)
}

/// AC-3 constraint propagation.
fn propagate(
    cells: &mut [FixedBitSet],
    stack: &[(usize, usize)],
    rules: &AdjacencyRules,
    config: &WfcConfig,
    w: usize,
    h: usize,
    num_tiles: usize,
) -> Result<(), WfcError> {
    let mut work: Vec<(usize, usize)> = stack.to_vec();

    while let Some((cx, cy)) = work.pop() {
        for dir in Direction::all() {
            let (dx, dy) = dir.delta();
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;

            if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                continue;
            }

            let nx = nx as usize;
            let ny = ny as usize;
            let n_idx = ny * w + nx;
            let c_idx = cy * w + cx;

            // Compute allowed set for neighbor
            let mut allowed = FixedBitSet::with_capacity(num_tiles);
            for t in cells[c_idx].ones() {
                allowed.union_with(rules.compatible(t, dir));
            }

            // Apply forbids semantic rules
            if !config.forbids_rules.is_empty() {
                let mut to_remove = Vec::new();
                for t in cells[n_idx].ones() {
                    if !allowed.contains(t) {
                        continue;
                    }
                    // Check forbids against current cell's possibilities
                    let cell_affs: Vec<&Option<String>> = cells[c_idx]
                        .ones()
                        .map(|i| &config.affordances[i].affordance)
                        .collect();
                    if !semantic::check_forbids(
                        &config.affordances[t].affordance,
                        &cell_affs,
                        &config.forbids_rules,
                    ) {
                        to_remove.push(t);
                    }
                }
                for t in to_remove {
                    allowed.set(t, false);
                }
            }

            let before = cells[n_idx].count_ones(..);
            cells[n_idx].intersect_with(&allowed);
            let after = cells[n_idx].count_ones(..);

            if after == 0 {
                return Err(WfcError::Contradiction { x: nx, y: ny });
            }

            if after < before {
                work.push((nx, ny));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adjacency::TileEdges;
    use std::collections::HashMap;

    fn simple_tileset() -> (Vec<TileEdges>, Vec<f64>, Vec<TileAffordance>) {
        let tiles = vec![
            TileEdges {
                name: "wall".to_string(),
                n: "solid".to_string(),
                e: "solid".to_string(),
                s: "solid".to_string(),
                w: "solid".to_string(),
                weight: 1.0,
            },
            TileEdges {
                name: "floor".to_string(),
                n: "floor".to_string(),
                e: "floor".to_string(),
                s: "floor".to_string(),
                w: "floor".to_string(),
                weight: 2.0,
            },
        ];
        let weights = vec![1.0, 2.0];
        let affordances = vec![
            TileAffordance {
                affordance: Some("obstacle".to_string()),
            },
            TileAffordance {
                affordance: Some("walkable".to_string()),
            },
        ];
        (tiles, weights, affordances)
    }

    #[test]
    fn wfc_produces_valid_grid() {
        let (tiles, weights, affordances) = simple_tileset();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());

        let config = WfcConfig {
            width: 4,
            height: 4,
            seed: 42,
            max_retries: 5,
            weights,
            affordances,
            forbids_rules: vec![],
            requires_rules: vec![],
            require_boost: 3.0,
        };

        let result = run_wfc(&rules, &config, &[]).unwrap();
        assert_eq!(result.grid.len(), 4);
        assert_eq!(result.grid[0].len(), 4);

        // Every cell should be a valid tile index
        for row in &result.grid {
            for &tile in row {
                assert!(tile < 2);
            }
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let (tiles, weights, affordances) = simple_tileset();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());

        let config1 = WfcConfig {
            width: 6,
            height: 6,
            seed: 123,
            max_retries: 5,
            weights: weights.clone(),
            affordances: affordances.clone(),
            forbids_rules: vec![],
            requires_rules: vec![],
            require_boost: 3.0,
        };
        let config2 = WfcConfig {
            width: 6,
            height: 6,
            seed: 123,
            max_retries: 5,
            weights,
            affordances,
            forbids_rules: vec![],
            requires_rules: vec![],
            require_boost: 3.0,
        };

        let r1 = run_wfc(&rules, &config1, &[]).unwrap();
        let r2 = run_wfc(&rules, &config2, &[]).unwrap();
        assert_eq!(r1.grid, r2.grid);
    }

    #[test]
    fn pins_are_respected() {
        let (tiles, weights, affordances) = simple_tileset();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());

        let config = WfcConfig {
            width: 4,
            height: 4,
            seed: 42,
            max_retries: 5,
            weights,
            affordances,
            forbids_rules: vec![],
            requires_rules: vec![],
            require_boost: 3.0,
        };

        // Pin compatible tiles (both wall — same edge class)
        let pins = vec![
            Pin {
                x: 0,
                y: 0,
                tile_idx: 0,
            }, // wall at (0,0)
            Pin {
                x: 3,
                y: 3,
                tile_idx: 0,
            }, // wall at (3,3)
        ];

        let result = run_wfc(&rules, &config, &pins).unwrap();
        assert_eq!(result.grid[0][0], 0); // wall pinned
        assert_eq!(result.grid[3][3], 0); // wall pinned
    }

    #[test]
    fn edge_compatibility_enforced() {
        // Wall and floor have different edge classes — they can't be adjacent
        let (tiles, weights, affordances) = simple_tileset();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());

        let config = WfcConfig {
            width: 8,
            height: 8,
            seed: 42,
            max_retries: 5,
            weights,
            affordances,
            forbids_rules: vec![],
            requires_rules: vec![],
            require_boost: 3.0,
        };

        let result = run_wfc(&rules, &config, &[]).unwrap();

        // Check adjacency: each pair of adjacent tiles must have matching edge classes
        for y in 0..8 {
            for x in 0..8 {
                let t = result.grid[y][x];
                if x + 1 < 8 {
                    let r = result.grid[y][x + 1];
                    assert_eq!(
                        tiles[t].e,
                        tiles[r].w,
                        "edge mismatch at ({},{})-({},{}) east: {} vs {}",
                        x,
                        y,
                        x + 1,
                        y,
                        tiles[t].e,
                        tiles[r].w
                    );
                }
                if y + 1 < 8 {
                    let b = result.grid[y + 1][x];
                    assert_eq!(
                        tiles[t].s,
                        tiles[b].n,
                        "edge mismatch at ({},{})-({},{}) south: {} vs {}",
                        x,
                        y,
                        x,
                        y + 1,
                        tiles[t].s,
                        tiles[b].n
                    );
                }
            }
        }
    }
}
