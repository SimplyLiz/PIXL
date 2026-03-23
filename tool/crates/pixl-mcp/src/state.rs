use pixl_core::parser::{parse_pax, resolve_all_palettes};
use pixl_core::style::StyleLatent;
use pixl_core::types::{Palette, PaxFile};
use std::collections::HashMap;

/// In-memory PAX file state for an MCP session.
/// Tiles are built up incrementally via tool calls.
pub struct McpState {
    pub file: PaxFile,
    pub palettes: HashMap<String, Palette>,
    pub refinement_count: HashMap<String, u32>,
    pub style_latent: Option<StyleLatent>,
}

impl McpState {
    /// Create a new empty session state.
    pub fn new() -> Self {
        let source = concat!("[pax]\n", "version = \"2.0\"\n", "name = \"session\"\n",);
        let file = parse_pax(source).expect("default pax should parse");
        McpState {
            file,
            palettes: HashMap::new(),
            refinement_count: HashMap::new(),
            style_latent: None,
        }
    }

    /// Load state from a .pax file.
    pub fn from_source(source: &str) -> Result<Self, String> {
        let file = parse_pax(source).map_err(|e| format!("{}", e))?;
        let palettes = resolve_all_palettes(&file).map_err(|e| format!("{}", e))?;
        Ok(McpState {
            file,
            palettes,
            refinement_count: HashMap::new(),
            style_latent: None,
        })
    }

    /// Re-resolve all palettes from current file state.
    pub fn refresh_palettes(&mut self) -> Result<(), String> {
        self.palettes = resolve_all_palettes(&self.file).map_err(|e| format!("{}", e))?;
        Ok(())
    }

    /// Get the active theme name.
    pub fn active_theme(&self) -> Option<&str> {
        self.file.pax.theme.as_deref()
    }

    /// Get the max palette size from the active theme.
    pub fn max_palette_size(&self) -> Option<u32> {
        self.active_theme()
            .and_then(|t| self.file.theme.get(t))
            .and_then(|t| t.max_palette_size)
    }

    /// Get light source from the active theme.
    pub fn light_source(&self) -> Option<&str> {
        self.active_theme()
            .and_then(|t| self.file.theme.get(t))
            .and_then(|t| t.light_source.as_deref())
    }

    /// Track a refinement iteration for a tile.
    /// Returns the new count.
    pub fn record_refinement(&mut self, tile_name: &str) -> u32 {
        let count = self
            .refinement_count
            .entry(tile_name.to_string())
            .or_insert(0);
        *count += 1;
        *count
    }

    /// Get refinement count for a tile.
    pub fn get_refinement_count(&self, tile_name: &str) -> u32 {
        self.refinement_count.get(tile_name).copied().unwrap_or(0)
    }

    /// Serialize the current state back to .pax TOML source.
    pub fn to_pax_source(&self) -> Result<String, String> {
        toml::to_string_pretty(&self.file).map_err(|e| format!("{}", e))
    }

    /// List all tile names.
    pub fn tile_names(&self) -> Vec<&str> {
        self.file.tile.keys().map(|s| s.as_str()).collect()
    }

    /// List all stamp names.
    pub fn stamp_names(&self) -> Vec<&str> {
        self.file.stamp.keys().map(|s| s.as_str()).collect()
    }

    /// Delete a tile by name.
    pub fn delete_tile(&mut self, name: &str) -> bool {
        self.file.tile.remove(name).is_some()
    }
}

impl Default for McpState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let state = McpState::new();
        assert!(state.tile_names().is_empty());
        assert!(state.stamp_names().is_empty());
    }

    #[test]
    fn refinement_tracking() {
        let mut state = McpState::new();
        assert_eq!(state.get_refinement_count("wall"), 0);
        assert_eq!(state.record_refinement("wall"), 1);
        assert_eq!(state.record_refinement("wall"), 2);
        assert_eq!(state.record_refinement("wall"), 3);
        assert_eq!(state.get_refinement_count("wall"), 3);
    }

    #[test]
    fn load_from_dungeon() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let state = McpState::from_source(&source).unwrap();
        assert!(!state.tile_names().is_empty());
        assert!(!state.palettes.is_empty());
        assert_eq!(state.active_theme(), Some("dark_fantasy"));
        assert_eq!(state.max_palette_size(), Some(16));
        assert_eq!(state.light_source(), Some("top-left"));
    }

    #[test]
    fn delete_tile() {
        let source = std::fs::read_to_string("../../examples/dungeon.pax")
            .expect("dungeon.pax should exist");
        let mut state = McpState::from_source(&source).unwrap();
        let before = state.tile_names().len();
        assert!(state.delete_tile("wall_solid"));
        assert_eq!(state.tile_names().len(), before - 1);
        assert!(!state.delete_tile("nonexistent"));
    }
}
