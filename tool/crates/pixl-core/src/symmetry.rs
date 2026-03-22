use crate::types::Symmetry;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SymmetryError {
    #[error("horizontal symmetry requires grid width = tile_width/2, got {got} (tile_width={tile_width})")]
    HorizontalWidthMismatch { got: usize, tile_width: u32 },

    #[error("vertical symmetry requires grid height = tile_height/2, got {got} (tile_height={tile_height})")]
    VerticalHeightMismatch { got: usize, tile_height: u32 },

    #[error("{axis} symmetry requires even tile dimension, got {dim}")]
    OddDimension { axis: String, dim: u32 },
}

/// Expand a partial grid using the given symmetry mode.
/// Returns the full-size grid.
pub fn expand_symmetry(
    grid: &[Vec<char>],
    tile_width: u32,
    tile_height: u32,
    symmetry: Symmetry,
) -> Result<Vec<Vec<char>>, SymmetryError> {
    match symmetry {
        Symmetry::None => Ok(grid.to_vec()),

        Symmetry::Horizontal => {
            if !tile_width.is_multiple_of(2) {
                return Err(SymmetryError::OddDimension {
                    axis: "horizontal".to_string(),
                    dim: tile_width,
                });
            }
            let half_w = (tile_width / 2) as usize;
            if grid.is_empty() || grid[0].len() != half_w {
                return Err(SymmetryError::HorizontalWidthMismatch {
                    got: grid.first().map_or(0, |r| r.len()),
                    tile_width,
                });
            }
            Ok(grid
                .iter()
                .map(|row| {
                    let mut full = row.clone();
                    let mut mirrored: Vec<char> = row.iter().rev().cloned().collect();
                    full.append(&mut mirrored);
                    full
                })
                .collect())
        }

        Symmetry::Vertical => {
            if !tile_height.is_multiple_of(2) {
                return Err(SymmetryError::OddDimension {
                    axis: "vertical".to_string(),
                    dim: tile_height,
                });
            }
            let half_h = (tile_height / 2) as usize;
            if grid.len() != half_h {
                return Err(SymmetryError::VerticalHeightMismatch {
                    got: grid.len(),
                    tile_height,
                });
            }
            let mut full = grid.to_vec();
            let mut mirrored: Vec<Vec<char>> = grid.iter().rev().cloned().collect();
            full.append(&mut mirrored);
            Ok(full)
        }

        Symmetry::Quad => {
            if !tile_width.is_multiple_of(2) {
                return Err(SymmetryError::OddDimension {
                    axis: "horizontal (quad)".to_string(),
                    dim: tile_width,
                });
            }
            if !tile_height.is_multiple_of(2) {
                return Err(SymmetryError::OddDimension {
                    axis: "vertical (quad)".to_string(),
                    dim: tile_height,
                });
            }
            // Step 1: expand horizontally
            let h_expanded: Vec<Vec<char>> = grid
                .iter()
                .map(|row| {
                    let mut full = row.clone();
                    let mut mirrored: Vec<char> = row.iter().rev().cloned().collect();
                    full.append(&mut mirrored);
                    full
                })
                .collect();
            // Step 2: expand vertically
            let mut full = h_expanded.clone();
            let mut mirrored: Vec<Vec<char>> = h_expanded.iter().rev().cloned().collect();
            full.append(&mut mirrored);
            Ok(full)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_symmetry() {
        let half = vec![
            vec!['#', '+'],
            vec!['+', '#'],
        ];
        let full = expand_symmetry(&half, 4, 2, Symmetry::Horizontal).unwrap();
        assert_eq!(full[0], vec!['#', '+', '+', '#']);
        assert_eq!(full[1], vec!['+', '#', '#', '+']);
    }

    #[test]
    fn vertical_symmetry() {
        let half = vec![
            vec!['#', '#', '#', '#'],
            vec!['+', '+', '+', '+'],
        ];
        let full = expand_symmetry(&half, 4, 4, Symmetry::Vertical).unwrap();
        assert_eq!(full.len(), 4);
        assert_eq!(full[0], vec!['#', '#', '#', '#']);
        assert_eq!(full[1], vec!['+', '+', '+', '+']);
        assert_eq!(full[2], vec!['+', '+', '+', '+']);
        assert_eq!(full[3], vec!['#', '#', '#', '#']);
    }

    #[test]
    fn quad_symmetry() {
        let quarter = vec![
            vec!['#', '+'],
            vec!['+', '.'],
        ];
        let full = expand_symmetry(&quarter, 4, 4, Symmetry::Quad).unwrap();
        assert_eq!(full.len(), 4);
        assert_eq!(full[0], vec!['#', '+', '+', '#']);
        assert_eq!(full[1], vec!['+', '.', '.', '+']);
        assert_eq!(full[2], vec!['+', '.', '.', '+']);
        assert_eq!(full[3], vec!['#', '+', '+', '#']);
    }

    #[test]
    fn odd_dimension_rejected() {
        let half = vec![vec!['#', '+']];
        let err = expand_symmetry(&half, 5, 2, Symmetry::Horizontal).unwrap_err();
        assert!(matches!(err, SymmetryError::OddDimension { .. }));
    }

    #[test]
    fn none_passthrough() {
        let grid = vec![vec!['#', '+', '.', '#']];
        let result = expand_symmetry(&grid, 4, 1, Symmetry::None).unwrap();
        assert_eq!(result, grid);
    }
}
