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
    let west: String = grid
        .iter()
        .map(|row| row.first().copied().unwrap_or('.'))
        .collect();
    let east: String = grid
        .iter()
        .map(|row| row.get(w - 1).copied().unwrap_or('.'))
        .collect();

    (north, east, south, west)
}

/// Auto-classify an edge string into a named edge class.
///
/// Classification is majority-based to produce coarse groups that enable
/// WFC tiling. Two edges with the same dominant character are considered
/// compatible, which is visually correct for most pixel art tiles.
///
/// Classes produced:
/// - `"open"` — all transparent (`.`)
/// - `"solid_X"` — 100% character X
/// - `"edge_X"` — majority character X (>50%), mixed with others
/// - `"mixed_<hash>"` — no majority character (perfectly balanced)
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

    // Count character frequencies
    let mut counts: std::collections::HashMap<char, usize> = std::collections::HashMap::new();
    for &c in &chars {
        *counts.entry(c).or_insert(0) += 1;
    }

    // Find the majority character (most frequent)
    let total = chars.len();
    let (majority_char, majority_count) = counts
        .iter()
        .max_by_key(|&(c, count)| (*count, std::cmp::Reverse(*c)))
        .map(|(&c, &count)| (c, count))
        .unwrap();

    // If one character has strict majority (>50%), classify by it
    if majority_count * 2 > total {
        if majority_char == '.' {
            return "open".to_string();
        }
        return format!("edge_{}", majority_char);
    }

    // No majority — use hash for exact matching (rare in practice)
    let h = fnv1a_hash(edge);
    format!("mixed_{:x}", h)
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
    fn classify_majority() {
        // 3 out of 5 chars are '+' -> edge_+
        let class = classify_edge("#+.++");
        assert_eq!(class, "edge_+", "expected edge_+ for majority +, got {}", class);
    }

    #[test]
    fn classify_mixed_balanced() {
        // Exactly 50/50 -> falls to hash
        let class = classify_edge("##++");
        assert!(
            class.starts_with("mixed_"),
            "expected mixed_ prefix for balanced edge, got {}",
            class
        );
    }

    #[test]
    fn classify_edge_majority_dominant() {
        // 10 '+' out of 16 (62.5%) -> edge_+
        let class = classify_edge("++++s+++s++++s++");
        assert_eq!(class, "edge_+", "expected edge_+ for majority +, got {}", class);
    }

    #[test]
    fn deterministic_hashing() {
        let a = classify_edge("#++#++#");
        let b = classify_edge("#++#++#");
        assert_eq!(a, b);
    }

    #[test]
    fn auto_classify_full_grid() {
        let grid = make_grid(&["####", "#++#", "#++#", "++++"]);
        let ec = auto_classify_edges(&grid);
        assert_eq!(ec.n, "solid_#");
        assert_eq!(ec.s, "solid_+");
        // West and east: col 0 = #,#,#,+ (75% #) -> edge_#
        assert_eq!(ec.w, "edge_#");
        assert_eq!(ec.e, "edge_#");
    }
}
