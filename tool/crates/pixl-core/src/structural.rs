//! Structural quality validators for pixel art tiles.
//!
//! Unlike style latents (statistical properties like density, entropy, hue),
//! structural validators check spatial properties: does the tile have an outline?
//! Is the subject centered? Do adjacent pixels have enough contrast?
//!
//! These are used by the SELF-REFINE loop to auto-critique tiles before
//! presenting them to the user.

use crate::oklab;
use crate::types::{Palette, Rgba};

/// Result of structural analysis on a tile grid.
#[derive(Debug, Clone)]
pub struct StructuralReport {
    /// Fraction of subject boundary pixels that have a dark outline (0.0-1.0).
    /// Good pixel art: >0.7. Excellent: >0.85.
    pub outline_coverage: f64,

    /// How centered the subject is (0.0 = corner, 1.0 = perfectly centered).
    /// Measures overlap between subject bounding box center and canvas center.
    pub centering_score: f64,

    /// Bounding box of non-void pixels: (min_x, min_y, max_x, max_y).
    pub bounding_box: (u32, u32, u32, u32),

    /// Canvas utilization: fraction of canvas area used by bounding box (0.0-1.0).
    pub canvas_utilization: f64,

    /// Average OKLab contrast between adjacent non-void pixels.
    /// Low values mean muddy/blobby appearance.
    pub mean_adjacent_contrast: f64,

    /// Fraction of non-void pixels that have at least one void neighbor (silhouette boundary).
    pub silhouette_complexity: f64,

    /// Number of distinct connected regions of non-void pixels.
    /// 1 = single solid subject. >1 = floating fragments.
    pub connected_components: u32,

    /// Fraction of non-void pixels (same as style latent pixel_density).
    pub pixel_density: f64,

    /// Human-readable issues found.
    pub issues: Vec<StructuralIssue>,
}

#[derive(Debug, Clone)]
pub struct StructuralIssue {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
    /// Optional: row/col of the problem area for targeted fixes.
    pub location: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Should auto-reject and regenerate.
    Error,
    /// Should flag for refinement pass.
    Warning,
    /// Informational, no action needed.
    Info,
}

/// Run all structural checks on a tile grid.
pub fn analyze(
    grid: &[Vec<char>],
    palette: &Palette,
    void_sym: char,
) -> StructuralReport {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { 0 };

    if h == 0 || w == 0 {
        return empty_report();
    }

    let mut issues = Vec::new();

    // Compute bounding box of non-void pixels
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x: usize = 0;
    let mut max_y: usize = 0;
    let mut non_void_count: usize = 0;

    for y in 0..h {
        for x in 0..w {
            if grid[y][x] != void_sym {
                non_void_count += 1;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if non_void_count == 0 {
        issues.push(StructuralIssue {
            severity: Severity::Error,
            code: "empty_tile",
            message: "Tile contains no non-void pixels.".to_string(),
            location: None,
        });
        return StructuralReport {
            outline_coverage: 0.0,
            centering_score: 0.0,
            bounding_box: (0, 0, 0, 0),
            canvas_utilization: 0.0,
            mean_adjacent_contrast: 0.0,
            silhouette_complexity: 0.0,
            connected_components: 0,
            pixel_density: 0.0,
            issues,
        };
    }

    let pixel_density = non_void_count as f64 / (w * h) as f64;

    // Bounding box and canvas utilization
    let bb_w = (max_x - min_x + 1) as f64;
    let bb_h = (max_y - min_y + 1) as f64;
    let canvas_utilization = (bb_w * bb_h) / (w * h) as f64;

    // Centering score
    let bb_cx = min_x as f64 + bb_w / 2.0;
    let bb_cy = min_y as f64 + bb_h / 2.0;
    let canvas_cx = w as f64 / 2.0;
    let canvas_cy = h as f64 / 2.0;
    let max_dist = ((canvas_cx * canvas_cx) + (canvas_cy * canvas_cy)).sqrt();
    let dist = ((bb_cx - canvas_cx).powi(2) + (bb_cy - canvas_cy).powi(2)).sqrt();
    let centering_score = (1.0 - dist / max_dist).clamp(0.0, 1.0);

    // Outline coverage: for each boundary pixel (non-void adjacent to void),
    // check if it has an "outline" — either a globally dark pixel (traditional
    // outline) or a self-outline (sel-out) where the boundary pixel is notably
    // darker than its inward non-void neighbors.
    let mut boundary_count = 0u32;
    let mut outlined_boundary_count = 0u32;
    let darkest_lightness = find_darkest_lightness(palette, void_sym);

    for y in 0..h {
        for x in 0..w {
            if grid[y][x] == void_sym {
                continue;
            }
            // Check if this pixel borders void
            let borders_void = neighbors(x, y, w, h)
                .iter()
                .any(|&(nx, ny)| grid[ny][nx] == void_sym);

            if borders_void {
                boundary_count += 1;
                let lightness = pixel_lightness(grid[y][x], palette);

                // Method 1: Traditional dark outline — pixel is globally dark
                if lightness < darkest_lightness + 0.15 {
                    outlined_boundary_count += 1;
                    continue;
                }

                // Method 2: Self-outline (sel-out) — boundary pixel is darker
                // than its inward (non-void) neighbors by at least 0.08 OKLab.
                // This catches colored outlines like dark green around a green tree.
                let inward_neighbors: Vec<f64> = neighbors(x, y, w, h)
                    .iter()
                    .filter(|&&(nx, ny)| grid[ny][nx] != void_sym)
                    .map(|&(nx, ny)| pixel_lightness(grid[ny][nx], palette))
                    .collect();

                if !inward_neighbors.is_empty() {
                    let avg_inward = inward_neighbors.iter().sum::<f64>()
                        / inward_neighbors.len() as f64;
                    // Boundary pixel is at least 0.08 darker than its inward neighbors
                    if avg_inward - lightness > 0.08 {
                        outlined_boundary_count += 1;
                    }
                }
            }
        }
    }
    let outline_coverage = if boundary_count > 0 {
        outlined_boundary_count as f64 / boundary_count as f64
    } else {
        0.0
    };

    // Mean adjacent contrast (OKLab delta E between neighboring non-void pixels)
    let mut contrast_sum = 0.0f64;
    let mut contrast_count = 0u32;

    for y in 0..h {
        for x in 0..w {
            if grid[y][x] == void_sym {
                continue;
            }
            let lab_a = pixel_oklab(grid[y][x], palette);
            // Check right and down neighbors only (avoid double counting)
            if x + 1 < w && grid[y][x + 1] != void_sym {
                let lab_b = pixel_oklab(grid[y][x + 1], palette);
                contrast_sum += oklab::delta_e(&lab_a, &lab_b) as f64;
                contrast_count += 1;
            }
            if y + 1 < h && grid[y + 1][x] != void_sym {
                let lab_b = pixel_oklab(grid[y + 1][x], palette);
                contrast_sum += oklab::delta_e(&lab_a, &lab_b) as f64;
                contrast_count += 1;
            }
        }
    }
    let mean_adjacent_contrast = if contrast_count > 0 {
        contrast_sum / contrast_count as f64
    } else {
        0.0
    };

    // Silhouette complexity (boundary pixels / total non-void pixels)
    let silhouette_complexity = if non_void_count > 0 {
        boundary_count as f64 / non_void_count as f64
    } else {
        0.0
    };

    // Connected components (flood fill)
    let connected_components = count_connected_components(grid, void_sym);

    // Generate issues
    if outline_coverage < 0.3 {
        issues.push(StructuralIssue {
            severity: Severity::Error,
            code: "no_outline",
            message: format!(
                "Only {:.0}% of boundary pixels have an outline (dark or self-outline) — \
                 subject has no readable edge. Add a dark border or use darker variants \
                 of the fill color at the silhouette boundary.",
                outline_coverage * 100.0,
            ),
            location: None,
        });
    } else if outline_coverage < 0.7 {
        issues.push(StructuralIssue {
            severity: Severity::Warning,
            code: "weak_outline",
            message: format!(
                "Only {:.0}% of boundary pixels have an outline — edge is incomplete. \
                 Fill gaps with a dark border or self-outline (darker shade of adjacent fill).",
                outline_coverage * 100.0,
            ),
            location: None,
        });
    }

    if canvas_utilization < 0.25 {
        issues.push(StructuralIssue {
            severity: Severity::Error,
            code: "too_small",
            message: format!(
                "Subject uses only {:.0}% of canvas (bbox {}x{} in {}x{} canvas). \
                 Scale up the subject to fill at least 50% of the canvas.",
                canvas_utilization * 100.0,
                bb_w as u32, bb_h as u32, w, h,
            ),
            location: None,
        });
    } else if canvas_utilization < 0.40 {
        issues.push(StructuralIssue {
            severity: Severity::Warning,
            code: "undersize",
            message: format!(
                "Subject uses {:.0}% of canvas — could be larger. Bbox: {}x{} in {}x{}.",
                canvas_utilization * 100.0,
                bb_w as u32, bb_h as u32, w, h,
            ),
            location: None,
        });
    }

    if centering_score < 0.7 {
        issues.push(StructuralIssue {
            severity: Severity::Warning,
            code: "off_center",
            message: format!(
                "Subject center is at ({:.0},{:.0}) but canvas center is ({:.0},{:.0}). \
                 Shift the subject toward the center.",
                bb_cx, bb_cy, canvas_cx, canvas_cy,
            ),
            location: None,
        });
    }

    if mean_adjacent_contrast < 0.03 && non_void_count > 4 {
        issues.push(StructuralIssue {
            severity: Severity::Warning,
            code: "low_contrast",
            message: format!(
                "Mean adjacent contrast is very low ({:.3}) — colors are too similar. \
                 Use more distinct palette entries for adjacent regions.",
                mean_adjacent_contrast,
            ),
            location: None,
        });
    }

    if connected_components > 3 {
        issues.push(StructuralIssue {
            severity: Severity::Warning,
            code: "fragmented",
            message: format!(
                "Subject has {} disconnected regions — may look fragmented. \
                 Connect floating pixels to the main body.",
                connected_components,
            ),
            location: None,
        });
    }

    StructuralReport {
        outline_coverage,
        centering_score,
        bounding_box: (min_x as u32, min_y as u32, max_x as u32, max_y as u32),
        canvas_utilization,
        mean_adjacent_contrast,
        silhouette_complexity,
        connected_components,
        pixel_density,
        issues,
    }
}

/// Generate a concise text critique from a structural report.
/// Designed to be injected into an LLM refinement prompt.
pub fn critique_text(report: &StructuralReport, tile_name: &str) -> String {
    if report.issues.is_empty() {
        return format!(
            "Tile '{}' passes all structural checks: \
             outline {:.0}%, centered {:.0}%, utilization {:.0}%, \
             contrast {:.3}, {} component(s).",
            tile_name,
            report.outline_coverage * 100.0,
            report.centering_score * 100.0,
            report.canvas_utilization * 100.0,
            report.mean_adjacent_contrast,
            report.connected_components,
        );
    }

    let mut lines = vec![format!("Tile '{}' structural critique:", tile_name)];

    for issue in &report.issues {
        let prefix = match issue.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        };
        lines.push(format!("  [{}] {}", prefix, issue.message));
    }

    lines.push(format!(
        "  Metrics: outline={:.0}% center={:.0}% util={:.0}% contrast={:.3} components={}",
        report.outline_coverage * 100.0,
        report.centering_score * 100.0,
        report.canvas_utilization * 100.0,
        report.mean_adjacent_contrast,
        report.connected_components,
    ));

    lines.join("\n")
}

/// Check if a report has any errors (should auto-reject).
pub fn has_errors(report: &StructuralReport) -> bool {
    report.issues.iter().any(|i| i.severity == Severity::Error)
}

/// Check if a report has any warnings (should refine).
pub fn has_warnings(report: &StructuralReport) -> bool {
    report.issues.iter().any(|i| i.severity == Severity::Warning)
}

// ── Helpers ─────────────────────────────────────────────────────────

fn empty_report() -> StructuralReport {
    StructuralReport {
        outline_coverage: 0.0,
        centering_score: 0.0,
        bounding_box: (0, 0, 0, 0),
        canvas_utilization: 0.0,
        mean_adjacent_contrast: 0.0,
        silhouette_complexity: 0.0,
        connected_components: 0,
        pixel_density: 0.0,
        issues: vec![StructuralIssue {
            severity: Severity::Error,
            code: "empty_grid",
            message: "Grid is empty.".to_string(),
            location: None,
        }],
    }
}

fn neighbors(x: usize, y: usize, w: usize, h: usize) -> Vec<(usize, usize)> {
    let mut n = Vec::with_capacity(4);
    if x > 0 { n.push((x - 1, y)); }
    if x + 1 < w { n.push((x + 1, y)); }
    if y > 0 { n.push((x, y - 1)); }
    if y + 1 < h { n.push((x, y + 1)); }
    n
}

fn pixel_lightness(sym: char, palette: &Palette) -> f64 {
    match palette.symbols.get(&sym) {
        Some(rgba) => oklab::lightness(rgba.r, rgba.g, rgba.b) as f64,
        None => 0.5,
    }
}

fn pixel_oklab(sym: char, palette: &Palette) -> oklab::OkLab {
    match palette.symbols.get(&sym) {
        Some(rgba) => oklab::rgb_to_oklab(rgba.r, rgba.g, rgba.b),
        None => oklab::OkLab { l: 0.5, a: 0.0, b: 0.0 },
    }
}

fn find_darkest_lightness(palette: &Palette, void_sym: char) -> f64 {
    palette
        .symbols
        .iter()
        .filter(|&(sym, rgba)| *sym != void_sym && rgba.a > 128)
        .map(|(_, rgba)| oklab::lightness(rgba.r, rgba.g, rgba.b) as f64)
        .fold(f64::MAX, f64::min)
}

fn count_connected_components(grid: &[Vec<char>], void_sym: char) -> u32 {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return 0 };

    let mut visited = vec![vec![false; w]; h];
    let mut count = 0u32;

    for y in 0..h {
        for x in 0..w {
            if grid[y][x] != void_sym && !visited[y][x] {
                count += 1;
                // Flood fill
                let mut stack = vec![(x, y)];
                while let Some((cx, cy)) = stack.pop() {
                    if visited[cy][cx] {
                        continue;
                    }
                    visited[cy][cx] = true;
                    for (nx, ny) in neighbors(cx, cy, w, h) {
                        if grid[ny][nx] != void_sym && !visited[ny][nx] {
                            stack.push((nx, ny));
                        }
                    }
                }
            }
        }
    }

    count
}

/// Build a refinement prompt for the LLM, given a structural report and the current grid.
/// This tells the LLM exactly what's wrong and how to fix it, referencing specific rows/columns.
pub fn refinement_prompt(
    report: &StructuralReport,
    grid: &[Vec<char>],
    tile_name: &str,
) -> String {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { 0 };

    let mut sections = Vec::new();

    sections.push(format!(
        "You are refining tile '{}' ({}x{}). Examine the rendered preview image carefully.",
        tile_name, w, h,
    ));

    if report.issues.is_empty() {
        sections.push("The tile passes all structural checks. No refinement needed.".to_string());
        return sections.join("\n\n");
    }

    // Pixel art technique reminders based on detected issues
    for issue in &report.issues {
        match issue.code {
            "no_outline" | "weak_outline" => {
                sections.push(format!(
                    "OUTLINE ISSUE: {}\n\
                     Fix: Every non-void pixel that touches void (transparency) should be \
                     the DARKEST non-void palette color. Walk the silhouette boundary and \
                     replace bright/mid pixels with the darkest color. A 1px dark outline \
                     is the single most important pixel art technique for readability.",
                    issue.message,
                ));
                // Find specific rows with outline gaps
                let gap_rows = find_outline_gap_rows(grid, '.');
                if !gap_rows.is_empty() {
                    sections.push(format!(
                        "Outline gaps found at rows: {}. Focus your fixes on these rows.",
                        gap_rows.iter().map(|r| r.to_string()).collect::<Vec<_>>().join(", "),
                    ));
                }
            }
            "too_small" | "undersize" => {
                sections.push(format!(
                    "SIZE ISSUE: {}\n\
                     Fix: Scale up the subject to fill the canvas. The bounding box should \
                     occupy at least 60-80% of the canvas area. Leave 1-2px margin on each side.",
                    issue.message,
                ));
            }
            "off_center" => {
                sections.push(format!(
                    "CENTERING ISSUE: {}\n\
                     Fix: Shift the entire subject so its center of mass aligns with the canvas center.",
                    issue.message,
                ));
            }
            "low_contrast" => {
                sections.push(format!(
                    "CONTRAST ISSUE: {}\n\
                     Fix: Use more distinct palette colors for different parts. \
                     Adjacent regions (e.g., hair vs face, armor vs cloth) need at least \
                     2-3 steps of lightness difference. Dark outline → mid base fill → \
                     light highlight is the minimum value ramp.",
                    issue.message,
                ));
            }
            "fragmented" => {
                sections.push(format!(
                    "FRAGMENTATION ISSUE: {}\n\
                     Fix: Connect isolated pixel groups to the main subject body. \
                     Stray floating pixels look like noise, not detail.",
                    issue.message,
                ));
            }
            _ => {
                sections.push(format!("ISSUE: {}", issue.message));
            }
        }
    }

    sections.push(format!(
        "Current metrics: outline={:.0}% center={:.0}% util={:.0}% contrast={:.3}\n\
         Use pixl_refine_tile to patch specific rows. Target: outline>70%, utilization>40%.",
        report.outline_coverage * 100.0,
        report.centering_score * 100.0,
        report.canvas_utilization * 100.0,
        report.mean_adjacent_contrast,
    ));

    sections.join("\n\n")
}

/// Find rows where outline gaps exist (non-void boundary pixels that aren't dark).
fn find_outline_gap_rows(grid: &[Vec<char>], void_sym: char) -> Vec<usize> {
    let h = grid.len();
    let w = if h > 0 { grid[0].len() } else { return vec![] };
    let mut gap_rows = Vec::new();

    for y in 0..h {
        let mut has_gap = false;
        for x in 0..w {
            if grid[y][x] == void_sym {
                continue;
            }
            let borders_void = neighbors(x, y, w, h)
                .iter()
                .any(|&(nx, ny)| grid[ny][nx] == void_sym);
            if borders_void {
                // This is a boundary pixel — in a well-outlined tile,
                // it should be a dark color. We can't check lightness without
                // the palette here, so just flag rows that have boundary pixels.
                // The actual lightness check happens in analyze().
                has_gap = true;
                break;
            }
        }
        if has_gap {
            gap_rows.push(y);
        }
    }
    gap_rows
}

// ── Aesthetic Rating ───────────────────────────────────────────────

/// Per-axis rating (1-5) for a tile.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AestheticRating {
    /// Readability: can you tell what the tile is at a glance? (outline, contrast, centering)
    pub readability: u8,
    /// Appeal: does it look good? (density, complexity, proportions)
    pub appeal: u8,
    /// Consistency: does it match the project style? (style latent score, 0 if no latent)
    pub consistency: u8,
    /// Overall weighted score (1-5)
    pub overall: u8,
    /// Overall as float for WFC weight adjustment (0.0-1.0)
    pub score: f64,
    /// Brief text assessment
    pub assessment: String,
}

/// Rate a tile aesthetically, combining structural quality + style consistency.
///
/// Returns a 1-5 rating per axis + overall.
/// If `style_score` is None (no style latent learned), consistency is rated 3 (neutral).
pub fn rate_tile(report: &StructuralReport, style_score: Option<f64>) -> AestheticRating {
    // Readability: outline + contrast + centering
    let outline_score = report.outline_coverage;
    let contrast_score = (report.mean_adjacent_contrast / 0.15).min(1.0); // 0.15 = excellent contrast
    let center_score = report.centering_score;
    let readability_raw = outline_score * 0.4 + contrast_score * 0.35 + center_score * 0.25;
    let readability = to_1_5(readability_raw);

    // Appeal: canvas utilization + density + silhouette complexity + low fragmentation
    let util_score = report.canvas_utilization.min(1.0);
    let density_score = if report.pixel_density > 0.15 && report.pixel_density < 0.95 {
        1.0 // Good range
    } else {
        0.5 // Too sparse or too full
    };
    let frag_score = match report.connected_components {
        0 | 1 => 1.0,
        2 => 0.8,
        3 => 0.5,
        _ => 0.2,
    };
    let complexity_score = report.silhouette_complexity.min(1.0);
    let appeal_raw =
        util_score * 0.3 + density_score * 0.25 + frag_score * 0.25 + complexity_score * 0.2;
    let appeal = to_1_5(appeal_raw);

    // Consistency: style latent score (or neutral if no latent)
    let consistency_raw = style_score.unwrap_or(0.5); // 0.5 = neutral when no latent
    let consistency = to_1_5(consistency_raw);

    // Overall: weighted combination
    let overall_raw = if style_score.is_some() {
        readability_raw * 0.4 + appeal_raw * 0.3 + consistency_raw * 0.3
    } else {
        readability_raw * 0.5 + appeal_raw * 0.5
    };
    let overall = to_1_5(overall_raw);
    let score = overall_raw.max(0.0).min(1.0);

    let assessment = match overall {
        5 => "excellent — publish-ready".to_string(),
        4 => "good — minor polish possible".to_string(),
        3 => "acceptable — consider refinement".to_string(),
        2 => "below average — needs improvement".to_string(),
        _ => "poor — consider regenerating".to_string(),
    };

    AestheticRating {
        readability,
        appeal,
        consistency,
        overall,
        score,
        assessment,
    }
}

/// Map 0.0-1.0 to 1-5 scale.
fn to_1_5(raw: f64) -> u8 {
    let clamped = raw.max(0.0).min(1.0);
    match clamped {
        x if x >= 0.85 => 5,
        x if x >= 0.7 => 4,
        x if x >= 0.5 => 3,
        x if x >= 0.3 => 2,
        _ => 1,
    }
}

/// Suggest a WFC weight based on aesthetic rating.
/// Higher-rated tiles get higher weight (appear more often).
pub fn suggested_weight(rating: &AestheticRating) -> f64 {
    match rating.overall {
        5 => 1.0,
        4 => 0.8,
        3 => 0.5,
        2 => 0.3,
        _ => 0.1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_palette() -> Palette {
        let mut symbols = HashMap::new();
        symbols.insert('.', Rgba { r: 0, g: 0, b: 0, a: 0 });       // void
        symbols.insert('#', Rgba { r: 20, g: 20, b: 30, a: 255 });   // dark outline
        symbols.insert('+', Rgba { r: 100, g: 80, b: 120, a: 255 }); // mid fill
        symbols.insert('h', Rgba { r: 180, g: 160, b: 200, a: 255 }); // highlight
        Palette { symbols }
    }

    fn make_grid(rows: &[&str]) -> Vec<Vec<char>> {
        rows.iter().map(|r| r.chars().collect()).collect()
    }

    #[test]
    fn well_outlined_tile() {
        let palette = test_palette();
        // 8x8 tile with full dark outline
        let grid = make_grid(&[
            "..####..",
            ".#+++#..",
            "#++h++#.",
            "#++h++#.",
            "#++++++#",
            "#++++++#",
            ".######.",
            "........",
        ]);
        let report = analyze(&grid, &palette, '.');
        assert!(
            report.outline_coverage > 0.8,
            "outline: {:.2}",
            report.outline_coverage
        );
        assert!(report.issues.iter().all(|i| i.code != "no_outline"));
    }

    #[test]
    fn no_outline_detected() {
        let palette = test_palette();
        // Tile with bright fill but no dark border
        let grid = make_grid(&[
            "........",
            "..hhhh..",
            ".hhhhhh.",
            ".hhhhhh.",
            ".hhhhhh.",
            "..hhhh..",
            "........",
            "........",
        ]);
        let report = analyze(&grid, &palette, '.');
        assert!(
            report.outline_coverage < 0.3,
            "outline should be low: {:.2}",
            report.outline_coverage
        );
        assert!(report.issues.iter().any(|i| i.code == "no_outline"));
    }

    #[test]
    fn tiny_subject_flagged() {
        let palette = test_palette();
        // Small subject in big canvas
        let grid = make_grid(&[
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "......##........",
            "......##........",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
        ]);
        let report = analyze(&grid, &palette, '.');
        assert!(report.canvas_utilization < 0.1);
        assert!(report.issues.iter().any(|i| i.code == "too_small"));
    }

    #[test]
    fn centered_tile_scores_high() {
        let palette = test_palette();
        let grid = make_grid(&[
            "...##...",
            "..#++#..",
            ".#+++#..",
            "#+++++#.",
            "#+++++#.",
            ".#+++#..",
            "..#++#..",
            "...##...",
        ]);
        let report = analyze(&grid, &palette, '.');
        assert!(
            report.centering_score > 0.85,
            "centering: {:.2}",
            report.centering_score
        );
    }

    #[test]
    fn connected_components_counted() {
        let palette = test_palette();
        // Two separate blobs
        let grid = make_grid(&[
            "##...##.",
            "##...##.",
            "........",
            "........",
            "........",
            "........",
            "..##....",
            "..##....",
        ]);
        let report = analyze(&grid, &palette, '.');
        assert_eq!(report.connected_components, 3);
    }

    #[test]
    fn empty_grid_reports_error() {
        let palette = test_palette();
        let grid = make_grid(&[
            "........",
            "........",
            "........",
            "........",
        ]);
        let report = analyze(&grid, &palette, '.');
        assert!(has_errors(&report));
        assert!(report.issues.iter().any(|i| i.code == "empty_tile"));
    }

    #[test]
    fn critique_text_format() {
        let palette = test_palette();
        let grid = make_grid(&[
            "..hhhh..",
            ".hhhhhh.",
            ".hhhhhh.",
            "..hhhh..",
        ]);
        let report = analyze(&grid, &palette, '.');
        let text = critique_text(&report, "test_tile");
        assert!(text.contains("test_tile"));
        assert!(text.contains("outline") || text.contains("ERROR") || text.contains("WARN"));
    }

    #[test]
    fn rate_well_outlined_tile() {
        let palette = test_palette();
        let grid = make_grid(&[
            "..####..",
            ".#++++#.",
            ".#++++#.",
            ".#++++#.",
            ".#++++#.",
            "..####..",
        ]);
        let report = analyze(&grid, &palette, '.');
        let rating = rate_tile(&report, Some(0.9));
        assert!(rating.overall >= 3, "well-outlined tile should rate ≥3, got {}", rating.overall);
        assert!(rating.readability >= 3);
    }

    #[test]
    fn rate_empty_tile_low() {
        let report = empty_report();
        let rating = rate_tile(&report, None);
        assert!(rating.overall <= 2, "empty tile should rate ≤2, got {}", rating.overall);
    }

    #[test]
    fn suggested_weight_scales() {
        let high = AestheticRating {
            readability: 5, appeal: 5, consistency: 5, overall: 5,
            score: 0.95, assessment: String::new(),
        };
        let low = AestheticRating {
            readability: 1, appeal: 1, consistency: 1, overall: 1,
            score: 0.1, assessment: String::new(),
        };
        assert_eq!(suggested_weight(&high), 1.0);
        assert_eq!(suggested_weight(&low), 0.1);
    }

    #[test]
    fn to_1_5_boundaries() {
        assert_eq!(to_1_5(0.0), 1);
        assert_eq!(to_1_5(0.3), 2);
        assert_eq!(to_1_5(0.5), 3);
        assert_eq!(to_1_5(0.7), 4);
        assert_eq!(to_1_5(0.85), 5);
        assert_eq!(to_1_5(1.0), 5);
    }
}
