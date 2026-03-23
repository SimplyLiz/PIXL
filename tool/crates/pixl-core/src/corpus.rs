/// GameTileNet corpus conversion pipeline.
/// Converts PNG tile images to indexed .pax stamp format for:
/// 1. Built-in stamp library (ship real art from day one)
/// 2. Training data for fine-tuned PAX LoRA model
///
/// Pipeline: PNG -> palette quantize -> symbol assign -> TOML generate
/// -> affordance tag mapping -> batch validation

use crate::types::{Palette, Rgba};
use std::collections::HashMap;

/// A converted corpus entry — a tile image quantized into PAX format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CorpusEntry {
    pub name: String,
    pub source_file: String,
    pub width: u32,
    pub height: u32,
    pub grid: Vec<Vec<char>>,
    pub palette_name: String,
    pub affordance: Option<String>,
    pub tags: Vec<String>,
    pub color_accuracy: f64,
}

/// Result of a corpus conversion batch.
#[derive(Debug)]
pub struct CorpusBatch {
    pub entries: Vec<CorpusEntry>,
    pub failed: Vec<(String, String)>, // (filename, error)
    pub palette: Palette,
    pub palette_name: String,
}

/// Quantize a single RGBA pixel buffer to a PAX grid using the nearest palette color.
pub fn quantize_pixels(
    pixels: &[(u8, u8, u8, u8)], // RGBA tuples
    width: u32,
    height: u32,
    palette: &Palette,
    void_sym: char,
) -> (Vec<Vec<char>>, f64) {
    let entries: Vec<(char, &Rgba)> = palette.symbols.iter()
        .map(|(&c, rgba)| (c, rgba))
        .collect();

    let mut grid = Vec::with_capacity(height as usize);
    let mut total_dist = 0.0;

    for y in 0..height as usize {
        let mut row = Vec::with_capacity(width as usize);
        for x in 0..width as usize {
            let idx = y * width as usize + x;
            if idx >= pixels.len() {
                row.push(void_sym);
                continue;
            }
            let (r, g, b, a) = pixels[idx];

            // Transparent pixels -> void
            if a < 128 {
                row.push(void_sym);
                continue;
            }

            // Find nearest palette color
            let mut best_sym = void_sym;
            let mut best_dist = f64::MAX;
            for &(sym, rgba) in &entries {
                if sym == void_sym { continue; }
                let dr = r as f64 - rgba.r as f64;
                let dg = g as f64 - rgba.g as f64;
                let db = b as f64 - rgba.b as f64;
                let d = (dr * dr * 0.30 + dg * dg * 0.59 + db * db * 0.11).sqrt();
                if d < best_dist {
                    best_dist = d;
                    best_sym = sym;
                }
            }
            let (sym, dist) = (best_sym, best_dist);

            row.push(sym);
            total_dist += dist;
        }
        grid.push(row);
    }

    let total_pixels = (width * height) as f64;
    let accuracy = 1.0 - (total_dist / total_pixels / 255.0).min(1.0);

    (grid, accuracy)
}

/// Map a GameTileNet affordance label to PAX affordance.
pub fn map_affordance(label: &str) -> Option<String> {
    let normalized = label.to_lowercase();
    match normalized.as_str() {
        "walkable" | "ground" | "floor" | "path" | "road" => Some("walkable".to_string()),
        "obstacle" | "wall" | "rock" | "tree" | "building" => Some("obstacle".to_string()),
        "hazard" | "lava" | "spike" | "acid" | "fire" => Some("hazard".to_string()),
        "collectible" | "coin" | "gem" | "key" | "heart" => Some("collectible".to_string()),
        "character" | "npc" | "player" | "enemy" | "hero" => Some("character".to_string()),
        "decoration" | "prop" | "furniture" | "sign" => Some("decoration".to_string()),
        "interactive" | "door" | "chest" | "lever" | "switch" => Some("interactive".to_string()),
        "water" | "liquid" | "pond" | "river" => Some("hazard".to_string()),
        "background" | "sky" | "void" => Some("walkable".to_string()),
        _ => None,
    }
}

/// Generate a .pax TOML string from a batch of corpus entries.
pub fn generate_pax_stamps(batch: &CorpusBatch) -> String {
    let mut lines = Vec::new();

    lines.push("[pax]".to_string());
    lines.push("version = \"2.0\"".to_string());
    lines.push(format!("name = \"corpus_{}\"", batch.palette_name));
    lines.push(String::new());

    // Palette
    lines.push(format!("[palette.{}]", batch.palette_name));
    let mut sorted_syms: Vec<(&char, &Rgba)> = batch.palette.symbols.iter().collect();
    sorted_syms.sort_by_key(|(c, _)| **c);
    for (sym, rgba) in &sorted_syms {
        lines.push(format!(
            "\"{}\" = \"#{:02x}{:02x}{:02x}{:02x}\"",
            sym, rgba.r, rgba.g, rgba.b, rgba.a
        ));
    }
    lines.push(String::new());

    // Stamps
    for entry in &batch.entries {
        lines.push(format!("[stamp.{}]", entry.name));
        lines.push(format!("palette = \"{}\"", entry.palette_name));
        lines.push(format!("size = \"{}x{}\"", entry.width, entry.height));
        if let Some(ref aff) = entry.affordance {
            lines.push(format!("# affordance: {}", aff));
        }
        if !entry.tags.is_empty() {
            lines.push(format!("# tags: {}", entry.tags.join(", ")));
        }
        lines.push(format!("# accuracy: {:.1}%", entry.color_accuracy * 100.0));
        lines.push("grid = '''".to_string());
        for row in &entry.grid {
            lines.push(row.iter().collect::<String>());
        }
        lines.push("'''".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Generate training data entries for LoRA fine-tuning.
/// Each entry is a (description, pax_grid) pair.
pub fn generate_training_pairs(batch: &CorpusBatch) -> Vec<(String, String)> {
    batch.entries.iter().map(|entry| {
        let mut desc_parts = Vec::new();
        desc_parts.push(format!("a {}x{} pixel art tile", entry.width, entry.height));
        if let Some(ref aff) = entry.affordance {
            desc_parts.push(format!("({} type)", aff));
        }
        if !entry.tags.is_empty() {
            desc_parts.push(format!("tagged: {}", entry.tags.join(", ")));
        }
        let description = desc_parts.join(" ");

        let grid_str: String = entry.grid.iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        (description, grid_str)
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert('.', Rgba { r: 0, g: 0, b: 0, a: 0 });
        symbols.insert('#', Rgba { r: 42, g: 31, b: 61, a: 255 });
        symbols.insert('+', Rgba { r: 74, g: 58, b: 109, a: 255 });
        Palette { symbols }
    }

    #[test]
    fn quantize_solid_color() {
        let palette = test_palette();
        let pixels = vec![(42, 31, 61, 255); 4]; // all #2a1f3d
        let (grid, accuracy) = quantize_pixels(&pixels, 2, 2, &palette, '.');
        assert_eq!(grid[0][0], '#');
        assert!(accuracy > 0.99);
    }

    #[test]
    fn quantize_transparent() {
        let palette = test_palette();
        let pixels = vec![(0, 0, 0, 0); 4]; // all transparent
        let (grid, _) = quantize_pixels(&pixels, 2, 2, &palette, '.');
        assert_eq!(grid[0][0], '.');
    }

    #[test]
    fn affordance_mapping() {
        assert_eq!(map_affordance("walkable"), Some("walkable".to_string()));
        assert_eq!(map_affordance("Wall"), Some("obstacle".to_string()));
        assert_eq!(map_affordance("lava"), Some("hazard".to_string()));
        assert_eq!(map_affordance("coin"), Some("collectible".to_string()));
        assert_eq!(map_affordance("door"), Some("interactive".to_string()));
        assert_eq!(map_affordance("unknown_thing"), None);
    }

    #[test]
    fn generate_pax_output() {
        let batch = CorpusBatch {
            entries: vec![CorpusEntry {
                name: "test_tile".to_string(),
                source_file: "test.png".to_string(),
                width: 2,
                height: 2,
                grid: vec![vec!['#', '+'], vec!['+', '#']],
                palette_name: "test".to_string(),
                affordance: Some("obstacle".to_string()),
                tags: vec!["wall".to_string()],
                color_accuracy: 0.95,
            }],
            failed: vec![],
            palette: test_palette(),
            palette_name: "test".to_string(),
        };

        let pax = generate_pax_stamps(&batch);
        assert!(pax.contains("[stamp.test_tile]"));
        assert!(pax.contains("palette = \"test\""));
        assert!(pax.contains("#+"));
    }

    #[test]
    fn training_pairs_format() {
        let batch = CorpusBatch {
            entries: vec![CorpusEntry {
                name: "wall".to_string(),
                source_file: "wall.png".to_string(),
                width: 4,
                height: 4,
                grid: vec![vec!['#'; 4]; 4],
                palette_name: "test".to_string(),
                affordance: Some("obstacle".to_string()),
                tags: vec!["stone".to_string()],
                color_accuracy: 0.9,
            }],
            failed: vec![],
            palette: test_palette(),
            palette_name: "test".to_string(),
        };

        let pairs = generate_training_pairs(&batch);
        assert_eq!(pairs.len(), 1);
        assert!(pairs[0].0.contains("obstacle"));
        assert!(pairs[0].1.contains("####"));
    }
}
