/// PIXL project files (.pixlproject) for cross-session continuity.
/// A project organizes multiple .pax files (worlds), persists the style
/// latent, and tracks authoring progress.
use crate::style::StyleLatent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct PixlProject {
    pub project: ProjectMeta,
    #[serde(default)]
    pub worlds: HashMap<String, String>, // name -> relative .pax path
    #[serde(default)]
    pub style_latent: Option<StyleLatent>,
    #[serde(default)]
    pub progress: ProjectProgress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProjectProgress {
    #[serde(default)]
    pub tiles_authored: u32,
    #[serde(default)]
    pub total_target: u32,
    #[serde(default)]
    pub worlds_completed: Vec<String>,
}

impl PixlProject {
    /// Create a new project.
    pub fn new(name: &str, theme: Option<&str>) -> Self {
        PixlProject {
            project: ProjectMeta {
                name: name.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                created: chrono_now(),
                theme: theme.map(String::from),
            },
            worlds: HashMap::new(),
            style_latent: None,
            progress: ProjectProgress::default(),
        }
    }

    /// Add a world to the project.
    pub fn add_world(&mut self, name: &str, pax_path: &str) {
        self.worlds.insert(name.to_string(), pax_path.to_string());
    }

    /// Load a project from a .pixlproject file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        toml::from_str(&source).map_err(|e| format!("parse error: {}", e))
    }

    /// Save the project to a .pixlproject file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let source = toml::to_string_pretty(self).map_err(|e| format!("serialize error: {}", e))?;
        std::fs::write(path, source).map_err(|e| format!("cannot write {}: {}", path.display(), e))
    }

    /// Get the style description for prompt injection.
    pub fn style_description(&self) -> String {
        self.style_latent
            .as_ref()
            .map(|l| l.describe())
            .unwrap_or_else(|| "No style latent yet — create tiles first.".to_string())
    }

    /// Summarize the project.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Project: {}", self.project.name));
        if let Some(ref theme) = self.project.theme {
            lines.push(format!("Theme: {}", theme));
        }
        lines.push(format!("Worlds: {}", self.worlds.len()));
        for (name, path) in &self.worlds {
            lines.push(format!("  {} -> {}", name, path));
        }
        lines.push(format!(
            "Progress: {}/{} tiles",
            self.progress.tiles_authored, self.progress.total_target
        ));
        if self.style_latent.is_some() {
            lines.push("Style latent: extracted".to_string());
        }
        lines.join("\n")
    }
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    "2026-03-23".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_summarize() {
        let mut proj = PixlProject::new("dungeon_game", Some("dark_fantasy"));
        proj.add_world("dungeon_stone", "worlds/dungeon_stone.pax");
        proj.add_world("ice_cave", "worlds/ice_cave.pax");
        proj.progress.tiles_authored = 43;
        proj.progress.total_target = 120;

        let summary = proj.summary();
        assert!(summary.contains("dungeon_game"));
        assert!(summary.contains("dark_fantasy"));
        assert!(summary.contains("43/120"));
    }

    #[test]
    fn roundtrip_toml() {
        let mut proj = PixlProject::new("test", None);
        proj.add_world("world1", "w1.pax");

        let toml_str = toml::to_string_pretty(&proj).unwrap();
        let parsed: PixlProject = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.project.name, "test");
        assert!(parsed.worlds.contains_key("world1"));
    }

    #[test]
    fn with_style_latent() {
        let mut proj = PixlProject::new("styled", None);
        proj.style_latent = Some(StyleLatent::default());

        let toml_str = toml::to_string_pretty(&proj).unwrap();
        assert!(toml_str.contains("style_latent"));

        let parsed: PixlProject = toml::from_str(&toml_str).unwrap();
        assert!(parsed.style_latent.is_some());
    }
}
