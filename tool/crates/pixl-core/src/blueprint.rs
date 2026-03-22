/// Blueprint system — anatomy/layout reference data for character sprites.
/// Built-in reference data, NOT part of .pax file format.
/// Queryable by MCP, CLI, PIXL Studio, or any LLM integration.

/// A resolved landmark with pixel coordinates for a specific canvas size.
#[derive(Debug, Clone)]
pub struct ResolvedLandmark {
    pub name: String,
    pub x: u32,
    pub y: u32,
}

/// Size-specific rules for feature rendering.
#[derive(Debug, Clone)]
pub struct SizeRule {
    pub omit: Vec<String>,
    pub eye_size_px: u32,
    pub has_pupil: bool,
    pub has_highlight: bool,
}

/// A resolved blueprint with pixel coordinates for a given canvas size.
#[derive(Debug, Clone)]
pub struct ResolvedBlueprint {
    pub model: String,
    pub canvas_w: u32,
    pub canvas_h: u32,
    pub landmarks: Vec<ResolvedLandmark>,
    pub eye_size: u32,
    pub omitted: Vec<String>,
}

/// A fractional landmark definition (0.0–1.0 of canvas).
#[derive(Debug, Clone)]
struct Landmark {
    name: &'static str,
    x: f32,
    y: f32,
}

/// Built-in anatomy model.
#[derive(Debug, Clone)]
pub struct Blueprint {
    name: &'static str,
    landmarks: &'static [Landmark],
}

// ── Built-in: Humanoid Chibi (6-head proportions) ───────────────────

const CHIBI_LANDMARKS: &[Landmark] = &[
    Landmark { name: "head_top",       x: 0.50, y: 0.00 },
    Landmark { name: "eye_left",       x: 0.35, y: 0.12 },
    Landmark { name: "eye_right",      x: 0.65, y: 0.12 },
    Landmark { name: "nose",           x: 0.50, y: 0.16 },
    Landmark { name: "mouth",          x: 0.50, y: 0.19 },
    Landmark { name: "chin",           x: 0.50, y: 0.22 },
    Landmark { name: "shoulder_left",  x: 0.20, y: 0.28 },
    Landmark { name: "shoulder_right", x: 0.80, y: 0.28 },
    Landmark { name: "elbow_left",     x: 0.12, y: 0.45 },
    Landmark { name: "elbow_right",    x: 0.88, y: 0.45 },
    Landmark { name: "hand_left",      x: 0.10, y: 0.60 },
    Landmark { name: "hand_right",     x: 0.90, y: 0.60 },
    Landmark { name: "waist",          x: 0.50, y: 0.55 },
    Landmark { name: "knee_left",      x: 0.35, y: 0.78 },
    Landmark { name: "knee_right",     x: 0.65, y: 0.78 },
    Landmark { name: "foot_left",      x: 0.35, y: 1.00 },
    Landmark { name: "foot_right",     x: 0.65, y: 1.00 },
];

/// Get the size rule for a given canvas size.
fn size_rule_for(w: u32, h: u32) -> SizeRule {
    match (w, h) {
        (0..=8, _) | (_, 0..=8) => SizeRule {
            omit: vec![
                "eye_left", "eye_right", "nose", "mouth", "chin",
                "elbow_left", "elbow_right", "hand_left", "hand_right",
                "knee_left", "knee_right",
            ].into_iter().map(String::from).collect(),
            eye_size_px: 0,
            has_pupil: false,
            has_highlight: false,
        },
        (w, h) if w <= 16 && h <= 16 => SizeRule {
            omit: vec![
                "eye_left", "eye_right", "nose", "mouth", "chin",
                "elbow_left", "elbow_right", "hand_left", "hand_right",
            ].into_iter().map(String::from).collect(),
            eye_size_px: 0,
            has_pupil: false,
            has_highlight: false,
        },
        (w, h) if w <= 16 && h <= 32 => SizeRule {
            omit: vec!["nose", "mouth"].into_iter().map(String::from).collect(),
            eye_size_px: 2,
            has_pupil: false,
            has_highlight: false,
        },
        (w, h) if w <= 24 && h <= 32 => SizeRule {
            omit: vec!["nose"].into_iter().map(String::from).collect(),
            eye_size_px: 2,
            has_pupil: true,
            has_highlight: false,
        },
        (w, h) if w <= 32 && h <= 48 => SizeRule {
            omit: vec![],
            eye_size_px: 3,
            has_pupil: true,
            has_highlight: true,
        },
        _ => SizeRule {
            omit: vec![],
            eye_size_px: 4,
            has_pupil: true,
            has_highlight: true,
        },
    }
}

/// Resolve a blueprint for a given canvas size.
pub fn resolve(model: &str, width: u32, height: u32) -> Option<ResolvedBlueprint> {
    let landmarks = match model {
        "humanoid_chibi" | "chibi" => CHIBI_LANDMARKS,
        _ => return None,
    };

    let rule = size_rule_for(width, height);

    let resolved_landmarks: Vec<ResolvedLandmark> = landmarks
        .iter()
        .filter(|l| !rule.omit.iter().any(|o| o == l.name))
        .map(|l| ResolvedLandmark {
            name: l.name.to_string(),
            x: (l.x * width as f32).round() as u32,
            y: (l.y * height as f32).round() as u32,
        })
        .collect();

    Some(ResolvedBlueprint {
        model: model.to_string(),
        canvas_w: width,
        canvas_h: height,
        landmarks: resolved_landmarks,
        eye_size: rule.eye_size_px,
        omitted: rule.omit,
    })
}

/// Render a text guide for LLM/tool consumption.
pub fn render_guide(model: &str, width: u32, height: u32) -> Option<String> {
    let bp = resolve(model, width, height)?;

    let mut lines = Vec::new();
    lines.push(format!(
        "Canvas {}x{} ({} model):",
        width, height, model
    ));

    if bp.eye_size == 0 {
        lines.push("  No facial features at this size (color region only).".to_string());
    } else {
        lines.push(format!(
            "  Eye size: {}x{} {}{}",
            bp.eye_size,
            bp.eye_size,
            if bp.eye_size > 1 {
                format!("with {}px pupil", if bp.eye_size >= 4 { 2 } else { 1 })
            } else {
                "no pupil".to_string()
            },
            if size_rule_for(width, height).has_highlight {
                ", 1px highlight"
            } else {
                ""
            }
        ));
    }

    lines.push(String::new());

    // Group landmarks by row for readable output
    let mut by_row: std::collections::BTreeMap<u32, Vec<&ResolvedLandmark>> =
        std::collections::BTreeMap::new();
    for lm in &bp.landmarks {
        by_row.entry(lm.y).or_default().push(lm);
    }

    for (row, lms) in &by_row {
        let parts: Vec<String> = lms
            .iter()
            .map(|lm| format!("{} (col {})", lm.name, lm.x))
            .collect();
        lines.push(format!("  Row {:>3}: {}", row, parts.join(", ")));
    }

    if !bp.omitted.is_empty() {
        lines.push(String::new());
        lines.push(format!("  Omitted at this size: {}", bp.omitted.join(", ")));
    }

    if bp.eye_size > 0 {
        lines.push(String::new());
        lines.push("  Draw eyes first. Everything else is measured from the eyes.".to_string());
    }

    Some(lines.join("\n"))
}

/// List available blueprint models.
pub fn available_models() -> Vec<&'static str> {
    vec!["humanoid_chibi"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_chibi_32x48() {
        let bp = resolve("humanoid_chibi", 32, 48).unwrap();
        assert_eq!(bp.eye_size, 3);
        assert!(bp.omitted.is_empty());
        assert!(bp.landmarks.iter().any(|l| l.name == "eye_left"));
        assert!(bp.landmarks.iter().any(|l| l.name == "nose"));
        assert!(bp.landmarks.iter().any(|l| l.name == "mouth"));
    }

    #[test]
    fn resolve_chibi_16x32_omits_nose_mouth() {
        let bp = resolve("humanoid_chibi", 16, 32).unwrap();
        assert_eq!(bp.eye_size, 2);
        assert!(bp.omitted.contains(&"nose".to_string()));
        assert!(bp.omitted.contains(&"mouth".to_string()));
        assert!(bp.landmarks.iter().any(|l| l.name == "eye_left"));
        assert!(!bp.landmarks.iter().any(|l| l.name == "nose"));
    }

    #[test]
    fn resolve_chibi_16x16_no_face() {
        let bp = resolve("humanoid_chibi", 16, 16).unwrap();
        assert_eq!(bp.eye_size, 0);
        assert!(!bp.landmarks.iter().any(|l| l.name == "eye_left"));
    }

    #[test]
    fn render_guide_32x48() {
        let guide = render_guide("humanoid_chibi", 32, 48).unwrap();
        assert!(guide.contains("Eye size: 3x3"));
        assert!(guide.contains("eye_left"));
        assert!(guide.contains("Draw eyes first"));
    }

    #[test]
    fn render_guide_16x16_no_eyes() {
        let guide = render_guide("humanoid_chibi", 16, 16).unwrap();
        assert!(guide.contains("No facial features"));
        assert!(!guide.contains("Draw eyes first"));
    }

    #[test]
    fn unknown_model_returns_none() {
        assert!(resolve("unknown_model", 32, 48).is_none());
    }

    #[test]
    fn landmarks_are_within_canvas() {
        let bp = resolve("humanoid_chibi", 32, 48).unwrap();
        for lm in &bp.landmarks {
            assert!(lm.x <= 32, "{} x={} > 32", lm.name, lm.x);
            assert!(lm.y <= 48, "{} y={} > 48", lm.name, lm.y);
        }
    }
}
