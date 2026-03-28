use fixedbitset::FixedBitSet;
use std::collections::HashMap;

/// Direction for adjacency rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn opposite(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    pub fn all() -> [Direction; 4] {
        [
            Direction::North,
            Direction::East,
            Direction::South,
            Direction::West,
        ]
    }

    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }
}

/// Edge class for a single tile.
#[derive(Debug, Clone)]
pub struct TileEdges {
    pub name: String,
    pub n: String,
    pub e: String,
    pub s: String,
    pub w: String,
    pub weight: f64,
    /// Optional 8-neighbor corner classes (Godot terrain-style).
    pub ne: Option<String>,
    pub se: Option<String>,
    pub sw: Option<String>,
    pub nw: Option<String>,
}

impl TileEdges {
    /// Create a TileEdges with just the 4 cardinal edges (no corners).
    pub fn new(name: &str, n: &str, e: &str, s: &str, w: &str, weight: f64) -> Self {
        TileEdges {
            name: name.to_string(),
            n: n.to_string(),
            e: e.to_string(),
            s: s.to_string(),
            w: w.to_string(),
            weight,
            ne: None, se: None, sw: None, nw: None,
        }
    }

    pub fn edge_in(&self, dir: Direction) -> &str {
        match dir {
            Direction::North => &self.n,
            Direction::East => &self.e,
            Direction::South => &self.s,
            Direction::West => &self.w,
        }
    }

    /// Get the corner class for a diagonal direction.
    /// Returns None if corner classes aren't defined on this tile.
    pub fn corner_in(&self, dir: Direction, clockwise: bool) -> Option<&str> {
        // For Direction::North + clockwise=true → NE corner
        // For Direction::North + clockwise=false → NW corner
        match (dir, clockwise) {
            (Direction::North, true) => self.ne.as_deref(),
            (Direction::East, true) => self.se.as_deref(),
            (Direction::South, true) => self.sw.as_deref(),
            (Direction::West, true) => self.nw.as_deref(),
            (Direction::North, false) => self.nw.as_deref(),
            (Direction::East, false) => self.ne.as_deref(),
            (Direction::South, false) => self.se.as_deref(),
            (Direction::West, false) => self.sw.as_deref(),
        }
    }
}

/// Check whether two tiles have compatible corner classes for a given adjacency.
/// If either tile has no corner classes defined, corners are compatible (permissive).
fn corners_compatible(a: &TileEdges, b: &TileEdges, dir: Direction) -> bool {
    // When A is placed in `dir` relative to B:
    // - A's clockwise corner in `dir` must match B's counter-clockwise corner in `dir`
    // Example: A is East of B → A.NW must match B.NE, and A.SW must match B.SE
    let a_ccw = a.corner_in(dir, false);  // counter-clockwise corner of A facing dir
    let b_cw = b.corner_in(dir, true);    // clockwise corner of B facing dir

    // Only check if BOTH tiles define the relevant corners
    match (a_ccw, b_cw) {
        (Some(ac), Some(bc)) => ac == bc,
        _ => true, // permissive if either is undefined
    }
}

/// Adjacency rules: for each (tile_index, direction), which tile indices are compatible.
pub struct AdjacencyRules {
    num_tiles: usize,
    /// rules[(tile_idx * 4 + dir_idx)] = FixedBitSet of compatible tile indices
    rules: Vec<FixedBitSet>,
}

impl AdjacencyRules {
    /// Build adjacency rules from tile edge classes.
    /// Two tiles can be adjacent if their touching edge classes match.
    /// Variant groups: all members of a group share edge compatibility.
    pub fn build(tiles: &[TileEdges], variant_groups: &HashMap<String, Vec<String>>) -> Self {
        let n = tiles.len();
        let mut rules = vec![FixedBitSet::with_capacity(n); n * 4];

        // Build group membership: tile_name -> group members (indices)
        let name_to_idx: HashMap<&str, usize> = tiles
            .iter()
            .enumerate()
            .map(|(i, t)| (t.name.as_str(), i))
            .collect();

        let _group_members: HashMap<&str, Vec<usize>> = variant_groups
            .values()
            .map(|members| {
                let indices: Vec<usize> = members
                    .iter()
                    .filter_map(|m| name_to_idx.get(m.as_str()).copied())
                    .collect();
                (members[0].as_str(), indices)
            })
            .collect();

        let tile_to_group: HashMap<usize, Vec<usize>> = variant_groups
            .values()
            .flat_map(|members| {
                let indices: Vec<usize> = members
                    .iter()
                    .filter_map(|m| name_to_idx.get(m.as_str()).copied())
                    .collect();
                indices
                    .iter()
                    .map(|&i| (i, indices.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();

        for dir in Direction::all() {
            let opp = dir.opposite();
            let dir_idx = dir_to_idx(dir);

            for (a_idx, a) in tiles.iter().enumerate() {
                for (b_idx, b) in tiles.iter().enumerate() {
                    if a.edge_in(dir) == b.edge_in(opp) && corners_compatible(a, b, dir) {
                        // Direct compatibility (edges match + corners match if defined)
                        rules[a_idx * 4 + dir_idx].insert(b_idx);

                        // Expand variant groups: if b is in a group, all members compatible
                        if let Some(group) = tile_to_group.get(&b_idx) {
                            for &member_idx in group {
                                // Only add if the member also has a matching edge
                                // (group members might have different edges)
                                if tiles[member_idx].edge_in(opp) == a.edge_in(dir) {
                                    rules[a_idx * 4 + dir_idx].insert(member_idx);
                                }
                            }
                        }
                    }
                }
            }
        }

        AdjacencyRules {
            num_tiles: n,
            rules,
        }
    }

    /// Get the set of tiles compatible with `tile_idx` in `direction`.
    pub fn compatible(&self, tile_idx: usize, dir: Direction) -> &FixedBitSet {
        &self.rules[tile_idx * 4 + dir_to_idx(dir)]
    }

    pub fn num_tiles(&self) -> usize {
        self.num_tiles
    }
}

fn dir_to_idx(dir: Direction) -> usize {
    match dir {
        Direction::North => 0,
        Direction::East => 1,
        Direction::South => 2,
        Direction::West => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tiles() -> Vec<TileEdges> {
        vec![
            TileEdges::new("wall", "solid", "solid", "solid", "solid", 1.0),
            TileEdges::new("floor", "floor", "floor", "floor", "floor", 2.0),
            TileEdges::new("transition", "solid", "solid", "floor", "solid", 1.0),
        ]
    }

    #[test]
    fn wall_compatible_with_wall() {
        let tiles = make_tiles();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());
        let compat = rules.compatible(0, Direction::East);
        assert!(compat.contains(0)); // wall east -> wall west (solid == solid)
        assert!(!compat.contains(1)); // wall east -> floor west (solid != floor)
    }

    #[test]
    fn floor_compatible_with_floor() {
        let tiles = make_tiles();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());
        let compat = rules.compatible(1, Direction::North);
        assert!(compat.contains(1)); // floor north -> floor south (floor == floor)
    }

    #[test]
    fn transition_connects_wall_to_floor() {
        let tiles = make_tiles();
        let rules = AdjacencyRules::build(&tiles, &HashMap::new());
        // transition south edge = "floor", floor north edge = "floor" -> compatible
        let compat_s = rules.compatible(2, Direction::South);
        assert!(compat_s.contains(1)); // transition south -> floor north
        // transition north edge = "solid", wall south edge = "solid" -> compatible
        let compat_n = rules.compatible(2, Direction::North);
        assert!(compat_n.contains(0)); // transition north -> wall south
    }

    #[test]
    fn variant_groups_expand_compatibility() {
        let tiles = vec![
            TileEdges::new("grass_a", "grass", "grass", "grass", "grass", 2.0),
            TileEdges::new("grass_b", "grass", "grass", "grass", "grass", 1.0),
            TileEdges::new("wall", "solid", "solid", "solid", "solid", 1.0),
        ];
        let mut groups = HashMap::new();
        groups.insert(
            "grass".to_string(),
            vec!["grass_a".to_string(), "grass_b".to_string()],
        );

        let rules = AdjacencyRules::build(&tiles, &groups);
        let compat = rules.compatible(0, Direction::East);
        assert!(compat.contains(0)); // grass_a -> grass_a
        assert!(compat.contains(1)); // grass_a -> grass_b (same group, same edges)
        assert!(!compat.contains(2)); // grass_a -> wall (different edge class)
    }
}
