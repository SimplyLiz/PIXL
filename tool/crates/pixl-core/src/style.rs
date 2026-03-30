use crate::types::{Palette, Rgba};
use std::collections::HashMap;

/// Style latent — a statistical fingerprint extracted from reference tiles.
/// Encodes 8 measurable visual properties so that new tiles can be
/// conditioned on the style of existing ones.
///
/// Inspired by NTC's compact learned representation applied to authorship:
/// encode correlated visual information as a compact token, decode on demand.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StyleLatent {
    /// Dominant highlight position bias (0.0=top-left, 1.0=bottom-right)
    pub light_direction: f64,
    /// Average run-length of same-symbol sequences in rows
    pub run_length_mean: f64,
    /// Fraction of pixels using the darkest palette colors (shadow density)
    pub shadow_ratio: f64,
    /// Average distinct symbols per tile (palette usage breadth)
    pub palette_breadth: f64,
    /// Pixel density: fraction of non-void pixels
    pub pixel_density: f64,
    /// Shannon entropy of symbol distribution (0=one color, high=uniform)
    pub palette_entropy: f64,
    /// Dominant hue in degrees (from palette-weighted average)
    pub hue_bias: f64,
    /// Mean luminance of non-void pixels (0.0-1.0)
    pub luminance_mean: f64,
    /// Number of reference tiles used to compute this latent
    pub sample_count: usize,
}

impl StyleLatent {
    /// Extract a style latent from a set of reference tile grids.
    pub fn extract(grids: &[&Vec<Vec<char>>], palette: &Palette, void_sym: char) -> Self {
        if grids.is_empty() {
            return Self::default();
        }

        let mut total_run_length = 0.0;
        let mut total_runs = 0usize;
        let mut total_shadow = 0usize;
        let mut total_pixels = 0usize;
        let mut total_non_void = 0usize;
        let mut symbol_counts: HashMap<char, usize> = HashMap::new();
        let mut light_score_sum = 0.0;
        let mut light_score_count = 0usize;
        let mut luminance_sum = 0.0;
        let mut hue_x_sum = 0.0;
        let mut hue_y_sum = 0.0;
        let mut palette_breadths = Vec::new();

        // Find the darkest non-void palette color for shadow detection
        let shadow_threshold = palette
            .symbols
            .iter()
            .filter(|(s, _)| **s != void_sym)
            .map(|(_, c)| luminance(c))
            .fold(f64::MAX, f64::min)
            * 1.5; // 1.5x the darkest color's luminance

        for grid in grids {
            let h = grid.len();
            let w = if h > 0 { grid[0].len() } else { continue };
            let mut tile_symbols: HashMap<char, usize> = HashMap::new();

            for (y, row) in grid.iter().enumerate() {
                // Run-length analysis
                let mut run_len = 1usize;
                for x in 1..row.len() {
                    if row[x] == row[x - 1] {
                        run_len += 1;
                    } else {
                        total_run_length += run_len as f64;
                        total_runs += 1;
                        run_len = 1;
                    }
                }
                total_run_length += run_len as f64;
                total_runs += 1;

                for (x, &sym) in row.iter().enumerate() {
                    total_pixels += 1;
                    *symbol_counts.entry(sym).or_insert(0) += 1;
                    *tile_symbols.entry(sym).or_insert(0) += 1;

                    if sym == void_sym {
                        continue;
                    }
                    total_non_void += 1;

                    if let Some(color) = palette.symbols.get(&sym) {
                        let lum = luminance(color);
                        luminance_sum += lum;

                        // Shadow detection
                        if lum <= shadow_threshold {
                            total_shadow += 1;
                        }

                        // Light direction: bright pixels in top-left = low score
                        let nx = x as f64 / w.max(1) as f64;
                        let ny = y as f64 / h.max(1) as f64;
                        let position_score = (nx + ny) / 2.0; // 0=top-left, 1=bottom-right
                        if lum > 0.3 {
                            // Only count bright pixels for light direction
                            light_score_sum += position_score;
                            light_score_count += 1;
                        }

                        // Hue accumulation (circular mean via x/y components)
                        let hue = hue_degrees(color);
                        hue_x_sum += (hue * std::f64::consts::PI / 180.0).cos();
                        hue_y_sum += (hue * std::f64::consts::PI / 180.0).sin();
                    }
                }
            }

            palette_breadths.push(tile_symbols.len() as f64);
        }

        // Compute averages
        let run_length_mean = if total_runs > 0 {
            total_run_length / total_runs as f64
        } else {
            1.0
        };

        let shadow_ratio = if total_non_void > 0 {
            total_shadow as f64 / total_non_void as f64
        } else {
            0.0
        };

        let palette_breadth = if !palette_breadths.is_empty() {
            palette_breadths.iter().sum::<f64>() / palette_breadths.len() as f64
        } else {
            0.0
        };

        let pixel_density = if total_pixels > 0 {
            total_non_void as f64 / total_pixels as f64
        } else {
            0.0
        };

        // Shannon entropy of overall symbol distribution
        let palette_entropy = {
            let total = symbol_counts.values().sum::<usize>() as f64;
            if total > 0.0 {
                -symbol_counts
                    .values()
                    .map(|&c| {
                        let p = c as f64 / total;
                        if p > 0.0 { p * p.ln() } else { 0.0 }
                    })
                    .sum::<f64>()
            } else {
                0.0
            }
        };

        let light_direction = if light_score_count > 0 {
            light_score_sum / light_score_count as f64
        } else {
            0.5
        };

        let luminance_mean = if total_non_void > 0 {
            luminance_sum / total_non_void as f64
        } else {
            0.0
        };

        // Circular mean for hue
        let hue_bias = hue_y_sum.atan2(hue_x_sum) * 180.0 / std::f64::consts::PI;
        let hue_bias = if hue_bias < 0.0 {
            hue_bias + 360.0
        } else {
            hue_bias
        };

        StyleLatent {
            light_direction,
            run_length_mean,
            shadow_ratio,
            palette_breadth,
            pixel_density,
            palette_entropy,
            hue_bias,
            luminance_mean,
            sample_count: grids.len(),
        }
    }

    /// Score a tile against this style latent. Returns 0.0-1.0 (1.0 = perfect match).
    pub fn score_tile(&self, grid: &[Vec<char>], palette: &Palette, void_sym: char) -> f64 {
        let tile_latent = Self::extract(&[&grid.to_vec()], palette, void_sym);

        // Weighted distance across 8 dimensions
        let diffs = [
            (self.light_direction - tile_latent.light_direction).abs() * 2.0,
            ((self.run_length_mean - tile_latent.run_length_mean) / self.run_length_mean.max(1.0))
                .abs(),
            (self.shadow_ratio - tile_latent.shadow_ratio).abs() * 3.0,
            ((self.palette_breadth - tile_latent.palette_breadth) / self.palette_breadth.max(1.0))
                .abs(),
            (self.pixel_density - tile_latent.pixel_density).abs() * 2.0,
            ((self.palette_entropy - tile_latent.palette_entropy) / self.palette_entropy.max(0.1))
                .abs(),
            hue_distance_normalized(self.hue_bias, tile_latent.hue_bias),
            (self.luminance_mean - tile_latent.luminance_mean).abs() * 2.0,
        ];

        let total_diff: f64 = diffs.iter().sum::<f64>() / diffs.len() as f64;
        (1.0 - total_diff).max(0.0).min(1.0)
    }

    /// Generate a text description for injection into LLM prompts.
    pub fn describe(&self) -> String {
        let light_dir = if self.light_direction < 0.35 {
            "top-left"
        } else if self.light_direction < 0.65 {
            "center/even"
        } else {
            "bottom-right"
        };

        let density = if self.pixel_density > 0.8 {
            "dense (mostly filled)"
        } else if self.pixel_density > 0.5 {
            "moderate"
        } else {
            "sparse (lots of transparency)"
        };

        let shadow = if self.shadow_ratio > 0.25 {
            "heavy shadows"
        } else if self.shadow_ratio > 0.1 {
            "moderate shadows"
        } else {
            "light/minimal shadows"
        };

        format!(
            "Style reference (from {} tiles):\n\
             \x20 Light: {} ({:.2})\n\
             \x20 Pixel density: {} ({:.0}%)\n\
             \x20 Shadows: {} ({:.0}% of pixels)\n\
             \x20 Avg colors per tile: {:.1}\n\
             \x20 Run length: {:.1} pixels (higher = more uniform areas)\n\
             \x20 Hue bias: {:.0}°\n\
             \x20 Luminance: {:.2}",
            self.sample_count,
            light_dir,
            self.light_direction,
            density,
            self.pixel_density * 100.0,
            shadow,
            self.shadow_ratio * 100.0,
            self.palette_breadth,
            self.run_length_mean,
            self.hue_bias,
            self.luminance_mean,
        )
    }

    /// Blend two style latents. `t` = 0.0 returns self, `t` = 1.0 returns other.
    /// Hue is interpolated on the circular axis to handle wrap-around correctly.
    pub fn blend(&self, other: &StyleLatent, t: f64) -> StyleLatent {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;

        // Circular interpolation for hue
        let mut hue_diff = other.hue_bias - self.hue_bias;
        if hue_diff > 180.0 {
            hue_diff -= 360.0;
        } else if hue_diff < -180.0 {
            hue_diff += 360.0;
        }
        let blended_hue = (self.hue_bias + hue_diff * t + 360.0) % 360.0;

        StyleLatent {
            light_direction: self.light_direction * inv + other.light_direction * t,
            run_length_mean: self.run_length_mean * inv + other.run_length_mean * t,
            shadow_ratio: self.shadow_ratio * inv + other.shadow_ratio * t,
            palette_breadth: self.palette_breadth * inv + other.palette_breadth * t,
            pixel_density: self.pixel_density * inv + other.pixel_density * t,
            palette_entropy: self.palette_entropy * inv + other.palette_entropy * t,
            hue_bias: blended_hue,
            luminance_mean: self.luminance_mean * inv + other.luminance_mean * t,
            sample_count: self.sample_count + other.sample_count,
        }
    }

    /// Euclidean distance between two style latents (normalized to 0.0-1.0).
    pub fn distance(&self, other: &StyleLatent) -> f64 {
        let diffs = [
            self.light_direction - other.light_direction,
            (self.run_length_mean - other.run_length_mean) / self.run_length_mean.max(1.0),
            self.shadow_ratio - other.shadow_ratio,
            (self.palette_breadth - other.palette_breadth) / self.palette_breadth.max(1.0),
            self.pixel_density - other.pixel_density,
            (self.palette_entropy - other.palette_entropy) / self.palette_entropy.max(0.1),
            hue_distance_normalized(self.hue_bias, other.hue_bias),
            self.luminance_mean - other.luminance_mean,
        ];
        let sum_sq: f64 = diffs.iter().map(|d| d * d).sum();
        (sum_sq / diffs.len() as f64).sqrt()
    }
}

impl Default for StyleLatent {
    fn default() -> Self {
        StyleLatent {
            light_direction: 0.5,
            run_length_mean: 1.0,
            shadow_ratio: 0.0,
            palette_breadth: 0.0,
            pixel_density: 0.0,
            palette_entropy: 0.0,
            hue_bias: 0.0,
            luminance_mean: 0.0,
            sample_count: 0,
        }
    }
}

/// Perceptual lightness via OKLab (more accurate than RGB luminance).
fn luminance(c: &Rgba) -> f64 {
    crate::oklab::lightness(c.r, c.g, c.b) as f64
}

/// Perceptual hue angle via OKLab (more uniform than HSV hue).
fn hue_degrees(c: &Rgba) -> f64 {
    crate::oklab::hue(c.r, c.g, c.b) as f64
}

fn hue_distance_normalized(a: f64, b: f64) -> f64 {
    let diff = (a - b).abs();
    let circular = diff.min(360.0 - diff);
    circular / 180.0 // normalize to 0-1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dungeon_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert(
            '.',
            Rgba {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        );
        symbols.insert(
            '#',
            Rgba {
                r: 42,
                g: 31,
                b: 61,
                a: 255,
            },
        );
        symbols.insert(
            '+',
            Rgba {
                r: 74,
                g: 58,
                b: 109,
                a: 255,
            },
        );
        symbols.insert(
            's',
            Rgba {
                r: 26,
                g: 15,
                b: 46,
                a: 255,
            },
        );
        symbols.insert(
            'g',
            Rgba {
                r: 45,
                g: 90,
                b: 39,
                a: 255,
            },
        );
        Palette { symbols }
    }

    fn wall_grid() -> Vec<Vec<char>> {
        vec![
            "################".chars().collect(),
            "##++##++##++####".chars().collect(),
            "#+++++++++++++##".chars().collect(),
            "##++########++##".chars().collect(),
            "################".chars().collect(),
            "##++++++++####++".chars().collect(),
            "#++++##+++++++++".chars().collect(),
            "##++##++##++####".chars().collect(),
            "################".chars().collect(),
            "##++##++########".chars().collect(),
            "#+++++++++++++##".chars().collect(),
            "##++##++##++####".chars().collect(),
            "################".chars().collect(),
            "##++##++##++####".chars().collect(),
            "#+++++++++++++##".chars().collect(),
            "################".chars().collect(),
        ]
    }

    fn floor_grid() -> Vec<Vec<char>> {
        vec![
            "++++++++++++++++".chars().collect(),
            "+++++++++++s++++".chars().collect(),
            "++++s+++++++++++".chars().collect(),
            "++++++++++++++++".chars().collect(),
            "++++++++s+++++++".chars().collect(),
            "++++++++++++++++".chars().collect(),
            "+s++++++++++++++".chars().collect(),
            "+++++++++++++s++".chars().collect(),
            "++++++++++++++++".chars().collect(),
            "++++++s+++++++++".chars().collect(),
            "++++++++++++++++".chars().collect(),
            "++++++++++++++++".chars().collect(),
            "++s+++++++++++++".chars().collect(),
            "++++++++++s+++++".chars().collect(),
            "++++++++++++++++".chars().collect(),
            "++++++++++++++++".chars().collect(),
        ]
    }

    #[test]
    fn extract_from_single_tile() {
        let palette = dungeon_palette();
        let grid = wall_grid();
        let latent = StyleLatent::extract(&[&grid], &palette, '.');
        assert_eq!(latent.sample_count, 1);
        assert!(latent.pixel_density > 0.99); // wall has no void
        assert!(latent.palette_breadth >= 2.0); // # and + at minimum
        assert!(latent.run_length_mean > 1.0); // has runs of ##
    }

    #[test]
    fn extract_from_multiple_tiles() {
        let palette = dungeon_palette();
        let wall = wall_grid();
        let floor = floor_grid();
        let latent = StyleLatent::extract(&[&wall, &floor], &palette, '.');
        assert_eq!(latent.sample_count, 2);
        assert!(latent.pixel_density > 0.95);
    }

    #[test]
    fn similar_tiles_score_high() {
        let palette = dungeon_palette();
        let wall = wall_grid();
        let latent = StyleLatent::extract(&[&wall], &palette, '.');

        // Score the wall against itself — should be ~1.0
        let score = latent.score_tile(&wall, &palette, '.');
        assert!(score > 0.9, "self-score should be >0.9, got {}", score);
    }

    #[test]
    fn different_tiles_score_lower() {
        let palette = dungeon_palette();
        let wall = wall_grid();
        let floor = floor_grid();
        let latent = StyleLatent::extract(&[&wall], &palette, '.');

        let wall_score = latent.score_tile(&wall, &palette, '.');
        let floor_score = latent.score_tile(&floor, &palette, '.');
        // Wall against wall-latent should score higher than floor
        assert!(
            wall_score > floor_score,
            "wall ({:.3}) should score higher than floor ({:.3})",
            wall_score,
            floor_score
        );
    }

    #[test]
    fn describe_produces_readable_output() {
        let palette = dungeon_palette();
        let wall = wall_grid();
        let latent = StyleLatent::extract(&[&wall], &palette, '.');
        let desc = latent.describe();
        assert!(desc.contains("Style reference"));
        assert!(desc.contains("Light:"));
        assert!(desc.contains("Pixel density:"));
        assert!(desc.contains("Hue bias:"));
    }

    #[test]
    fn empty_grids_produce_default() {
        let palette = dungeon_palette();
        let latent = StyleLatent::extract(&[], &palette, '.');
        assert_eq!(latent.sample_count, 0);
    }

    #[test]
    fn serializes_to_toml() {
        let palette = dungeon_palette();
        let wall = wall_grid();
        let latent = StyleLatent::extract(&[&wall], &palette, '.');
        let toml_str = toml::to_string_pretty(&latent).unwrap();
        assert!(toml_str.contains("light_direction"));
        assert!(toml_str.contains("sample_count"));

        // Roundtrip
        let parsed: StyleLatent = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.sample_count, 1);
    }
}
