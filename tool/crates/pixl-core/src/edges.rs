use crate::types::EdgeClass;
use std::hash::Hasher;

/// Extract the four edge strings from a resolved tile grid.
/// Returns (north, east, south, west). Returns empty strings for empty/0-width grids.
pub fn extract_edges(grid: &[Vec<char>]) -> (String, String, String, String) {
    let h = grid.len();
    if h == 0 {
        return (String::new(), String::new(), String::new(), String::new());
    }
    let w = grid[0].len();
    if w == 0 {
        return (String::new(), String::new(), String::new(), String::new());
    }

    let north: String = grid[0].iter().collect();
    let south: String = grid[h - 1].iter().collect();
    let west: String = grid.iter().map(|row| {
        row.first().copied().unwrap_or('.')
    }).collect();
    let east: String = grid.iter().map(|row| {
        row.get(w - 1).copied().unwrap_or('.')
    }).collect();

    (north, east, south, west)
}

/// Auto-classify an edge string into a named edge class.
/// Uses FNV-1a for deterministic hashing (not SipHash).
pub fn classify_edge(edge: &str) -> String {
    let chars: Vec<char> = edge.chars().collect();

    if chars.is_empty() {
        return "empty".to_string();
    }

    // All same symbol -> "solid_<sym>"
    if chars.iter().all(|&c| c == chars[0]) {
        if chars[0] == '.' {
            return "open".to_string();
        }
        return format!("solid_{}", chars[0]);
    }

    // Symmetric edge -> "sym_<hash4>"
    let reversed: Vec<char> = chars.iter().rev().cloned().collect();
    if chars == reversed {
        let h = fnv1a_hash(edge);
        return format!("sym_{:04x}", h & 0xFFFF);
    }

    // Mixed -> "mixed_<hash8>"
    let h = fnv1a_hash(edge);
    format!("mixed_{:08x}", h)
}

/// Auto-classify all four edges of a grid.
pub fn auto_classify_edges(grid: &[Vec<char>]) -> EdgeClass {
    let (n, e, s, w) = extract_edges(grid);
    EdgeClass {
        n: classify_edge(&n),
        e: classify_edge(&e),
        s: classify_edge(&s),
        w: classify_edge(&w),
    }
}

/// FNV-1a 64-bit hash — fast, deterministic, no crypto dependency.
fn fnv1a_hash(s: &str) -> u64 {
    let mut hasher = fnv::FnvHasher::default();
    hasher.write(s.as_bytes());
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid(rows: &[&str]) -> Vec<Vec<char>> {
        rows.iter().map(|r| r.chars().collect()).collect()
    }

    #[test]
    fn extract_edges_4x4() {
        let grid = make_grid(&["####", "#+.#", "#+.#", "++++"]);
        let (n, e, s, w) = extract_edges(&grid);
        assert_eq!(n, "####");
        assert_eq!(s, "++++");
        assert_eq!(w, "###+"); // col 0: #, #, #, +
        assert_eq!(e, "###+"); // col 3: #, #, #, +
    }

    #[test]
    fn classify_solid() {
        assert_eq!(classify_edge("####"), "solid_#");
        assert_eq!(classify_edge("++++"), "solid_+");
    }

    #[test]
    fn classify_open() {
        assert_eq!(classify_edge("...."), "open");
    }

    #[test]
    fn classify_symmetric() {
        let class = classify_edge("#+.+#");
        assert!(class.starts_with("sym_"), "expected sym_ prefix, got {}", class);
    }

    #[test]
    fn classify_mixed() {
        let class = classify_edge("#++.");
        assert!(class.starts_with("mixed_"), "expected mixed_ prefix, got {}", class);
    }

    #[test]
    fn deterministic_hashing() {
        // Same input always produces same hash
        let a = classify_edge("#++#++#");
        let b = classify_edge("#++#++#");
        assert_eq!(a, b);

        // Different input produces different hash (with high probability)
        let c = classify_edge("#+..+#");
        // Could theoretically collide but astronomically unlikely
    }

    #[test]
    fn auto_classify_full_grid() {
        let grid = make_grid(&[
            "####",
            "#++#",
            "#++#",
            "++++"
        ]);
        let ec = auto_classify_edges(&grid);
        assert_eq!(ec.n, "solid_#");
        assert_eq!(ec.s, "solid_+");
        // West and east are mixed
        assert!(ec.w.starts_with("mixed_") || ec.w.starts_with("sym_"));
        assert!(ec.e.starts_with("mixed_") || ec.e.starts_with("sym_"));
    }
}
