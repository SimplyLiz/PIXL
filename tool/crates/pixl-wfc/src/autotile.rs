/// 47-tile blob autotile bitmask computation.
///
/// 8-bit neighbor bitmask: NW=1, N=2, NE=4, W=8, E=16, SW=32, S=64, SE=128
///
/// Corner cleanup: corner bit only counted if both adjacent cardinal bits are set.
/// After cleanup, 256 raw masks reduce to exactly 47 unique visual cases.

const NW: u8 = 1;
const N: u8 = 2;
const NE: u8 = 4;
const W: u8 = 8;
const E: u8 = 16;
const SW: u8 = 32;
const S: u8 = 64;
const SE: u8 = 128;

/// Apply corner cleanup: corners only count if both adjacent edges are present.
pub fn corner_cleanup(mut mask: u8) -> u8 {
    if mask & NW != 0 && !(mask & N != 0 && mask & W != 0) {
        mask &= !NW;
    }
    if mask & NE != 0 && !(mask & N != 0 && mask & E != 0) {
        mask &= !NE;
    }
    if mask & SW != 0 && !(mask & S != 0 && mask & W != 0) {
        mask &= !SW;
    }
    if mask & SE != 0 && !(mask & S != 0 && mask & E != 0) {
        mask &= !SE;
    }
    mask
}

/// Generate the BITMASK_TO_47 lookup table.
/// Maps all 256 raw masks to one of 47 unique tile indices (0-46).
pub fn generate_bitmask_table() -> [u8; 256] {
    // Apply corner cleanup to all 256 masks, collect unique values
    let cleaned: Vec<u8> = (0..=255u8).map(corner_cleanup).collect();

    let mut unique_masks: Vec<u8> = {
        let mut set = std::collections::BTreeSet::new();
        for &m in &cleaned {
            set.insert(m);
        }
        set.into_iter().collect()
    };

    // Assign canonical indices sorted by mask value
    let mask_to_index: std::collections::HashMap<u8, u8> = unique_masks
        .iter()
        .enumerate()
        .map(|(i, &m)| (m, i as u8))
        .collect();

    let mut table = [0u8; 256];
    for raw in 0..=255u8 {
        let clean = cleaned[raw as usize];
        table[raw as usize] = mask_to_index[&clean];
    }

    table
}

/// Compute the bitmask for a cell in a tilemap.
/// `is_same_type` returns true if the cell at (x+dx, y+dy) is the same terrain type.
pub fn compute_bitmask<F>(x: usize, y: usize, w: usize, h: usize, is_same_type: F) -> u8
where
    F: Fn(i32, i32) -> bool,
{
    let cx = x as i32;
    let cy = y as i32;

    let mut mask = 0u8;

    let neighbors: [(i32, i32, u8); 8] = [
        (-1, -1, NW),
        (0, -1, N),
        (1, -1, NE),
        (-1, 0, W),
        (1, 0, E),
        (-1, 1, SW),
        (0, 1, S),
        (1, 1, SE),
    ];

    for (dx, dy, bit) in &neighbors {
        let nx = cx + dx;
        let ny = cy + dy;
        if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h && is_same_type(nx, ny) {
            mask |= bit;
        }
    }

    corner_cleanup(mask)
}

/// Get the tile index (0-46) for a given cleaned bitmask.
pub fn bitmask_to_tile_index(cleaned_mask: u8) -> u8 {
    // Use lazy static or compute on demand
    let table = generate_bitmask_table();
    table[cleaned_mask as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_47_unique_values() {
        let table = generate_bitmask_table();
        let unique: std::collections::HashSet<u8> = table.iter().copied().collect();
        assert_eq!(unique.len(), 47, "expected 47 unique tile indices, got {}", unique.len());
    }

    #[test]
    fn isolated_tile_is_index_0() {
        let table = generate_bitmask_table();
        assert_eq!(table[0], 0); // no neighbors = isolated
    }

    #[test]
    fn fully_surrounded_is_max_index() {
        let table = generate_bitmask_table();
        assert_eq!(table[255], 46); // all neighbors = fully surrounded
    }

    #[test]
    fn corner_cleanup_removes_invalid_corners() {
        // NW set but neither N nor W → NW should be cleared
        assert_eq!(corner_cleanup(NW), 0);
        // NW set with N and W → NW stays
        assert_eq!(corner_cleanup(NW | N | W), NW | N | W);
        // NE set with N but not E → NE cleared
        assert_eq!(corner_cleanup(NE | N), N);
    }

    #[test]
    fn n_only_and_s_only_are_different_indices() {
        let table = generate_bitmask_table();
        let n_idx = table[N as usize];
        let s_idx = table[S as usize];
        // N-only and S-only should be different visual tiles
        assert_ne!(n_idx, s_idx);
    }

    #[test]
    fn compute_bitmask_center() {
        // 3x3 grid, all same type
        let mask = compute_bitmask(1, 1, 3, 3, |_, _| true);
        assert_eq!(mask, 255); // all 8 neighbors present
    }

    #[test]
    fn compute_bitmask_corner() {
        // Top-left corner, only E and S are same type
        let grid = vec![
            vec![true, true, false],
            vec![true, false, false],
            vec![false, false, false],
        ];
        let mask = compute_bitmask(0, 0, 3, 3, |x, y| {
            grid.get(y as usize).and_then(|row| row.get(x as usize)).copied().unwrap_or(false)
        });
        // At (0,0): E=(1,0)=true, S=(0,1)=true, SE=(1,1)=false
        // After cleanup: E + S (no SE because SE requires both S and E, but SE cell is false)
        assert_eq!(mask, E | S);
    }

    #[test]
    fn deterministic_table() {
        let t1 = generate_bitmask_table();
        let t2 = generate_bitmask_table();
        assert_eq!(t1, t2);
    }
}
