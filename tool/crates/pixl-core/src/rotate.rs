use crate::types::{AutoRotate, EdgeClass, Tile};

/// Rotate a grid 90 degrees clockwise.
pub fn rotate_grid_cw(grid: &[Vec<char>]) -> Vec<Vec<char>> {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return vec![] };

    let mut out = vec![vec!['.'; h]; w];
    for y in 0..h {
        for x in 0..w {
            out[x][h - 1 - y] = grid[y][x];
        }
    }
    out
}

/// Flip a grid horizontally (mirror left-right).
pub fn flip_grid_h(grid: &[Vec<char>]) -> Vec<Vec<char>> {
    grid.iter()
        .map(|row| row.iter().rev().cloned().collect())
        .collect()
}

/// Rotate edge classes 90 degrees clockwise: N->E, E->S, S->W, W->N.
pub fn rotate_edge_class_cw(ec: &EdgeClass) -> EdgeClass {
    EdgeClass {
        n: ec.w.clone(),
        e: ec.n.clone(),
        s: ec.e.clone(),
        w: ec.s.clone(),
    }
}

/// Flip edge classes horizontally: E<->W, N and S stay.
pub fn flip_edge_class_h(ec: &EdgeClass) -> EdgeClass {
    EdgeClass {
        n: ec.n.clone(),
        e: ec.w.clone(),
        s: ec.s.clone(),
        w: ec.e.clone(),
    }
}

/// Generate all auto-rotated variants from a source tile.
/// Returns: Vec<(suffix, grid, edge_class, weight)>
pub fn generate_variants(
    source: &Tile,
    auto_rotate_weight: Option<&str>,
) -> Vec<(String, Vec<Vec<char>>, EdgeClass, f64)> {
    let num_variants = match source.auto_rotate {
        AutoRotate::None => return vec![],
        AutoRotate::FourWay => 3,
        AutoRotate::Flip => 1,
        AutoRotate::EightWay => 7,
    };

    let variant_weight = match auto_rotate_weight.unwrap_or("source_only") {
        "equal" => source.weight / (num_variants as f64 + 1.0),
        _ => 0.1, // "source_only": original keeps full weight, variants get 0.1
    };

    match source.auto_rotate {
        AutoRotate::None => vec![],

        AutoRotate::FourWay => {
            let mut variants = Vec::new();
            let mut grid = source.grid.clone();
            let mut ec = source.edge_class.clone();

            for suffix in ["_90", "_180", "_270"] {
                grid = rotate_grid_cw(&grid);
                ec = rotate_edge_class_cw(&ec);
                variants.push((
                    suffix.to_string(),
                    grid.clone(),
                    ec.clone(),
                    variant_weight,
                ));
            }
            variants
        }

        AutoRotate::Flip => {
            let flipped = flip_grid_h(&source.grid);
            let ec = flip_edge_class_h(&source.edge_class);
            vec![("_flip".to_string(), flipped, ec, variant_weight)]
        }

        AutoRotate::EightWay => {
            let mut variants = Vec::new();
            let mut grid = source.grid.clone();
            let mut ec = source.edge_class.clone();

            // 4 rotations (90, 180, 270)
            for suffix in ["_90", "_180", "_270"] {
                grid = rotate_grid_cw(&grid);
                ec = rotate_edge_class_cw(&ec);
                variants.push((suffix.to_string(), grid.clone(), ec.clone(), variant_weight));
            }

            // Original flipped
            let flipped = flip_grid_h(&source.grid);
            let flipped_ec = flip_edge_class_h(&source.edge_class);
            variants.push(("_flip".to_string(), flipped.clone(), flipped_ec.clone(), variant_weight));

            // Flipped + 3 rotations
            let mut fgrid = flipped;
            let mut fec = flipped_ec;
            for suffix in ["_90f", "_180f", "_270f"] {
                fgrid = rotate_grid_cw(&fgrid);
                fec = rotate_edge_class_cw(&fec);
                variants.push((suffix.to_string(), fgrid.clone(), fec.clone(), variant_weight));
            }

            variants
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid(rows: &[&str]) -> Vec<Vec<char>> {
        rows.iter().map(|r| r.chars().collect()).collect()
    }

    #[test]
    fn rotate_90_cw() {
        let grid = make_grid(&[
            "12",
            "34",
        ]);
        let rotated = rotate_grid_cw(&grid);
        assert_eq!(rotated, make_grid(&[
            "31",
            "42",
        ]));
    }

    #[test]
    fn rotate_360_identity() {
        let grid = make_grid(&[
            "AB",
            "CD",
        ]);
        let r1 = rotate_grid_cw(&grid);
        let r2 = rotate_grid_cw(&r1);
        let r3 = rotate_grid_cw(&r2);
        let r4 = rotate_grid_cw(&r3);
        assert_eq!(r4, grid);
    }

    #[test]
    fn flip_horizontal() {
        let grid = make_grid(&[
            "123",
            "456",
        ]);
        let flipped = flip_grid_h(&grid);
        assert_eq!(flipped, make_grid(&[
            "321",
            "654",
        ]));
    }

    #[test]
    fn edge_class_rotation() {
        let ec = EdgeClass {
            n: "solid".to_string(),
            e: "floor".to_string(),
            s: "open".to_string(),
            w: "mixed".to_string(),
        };
        let rotated = rotate_edge_class_cw(&ec);
        assert_eq!(rotated.n, "mixed"); // W -> N
        assert_eq!(rotated.e, "solid"); // N -> E
        assert_eq!(rotated.s, "floor"); // E -> S
        assert_eq!(rotated.w, "open");  // S -> W
    }
}
