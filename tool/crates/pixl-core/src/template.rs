use crate::types::TileRaw;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("tile '{child}': template '{template}' not found")]
    NotFound { child: String, template: String },

    #[error("tile '{child}': template '{template}' is itself a template (chains forbidden)")]
    Chain { child: String, template: String },

    #[error(
        "tile '{child}': has both 'template' and 'grid'/'rle'/'layout' — template tiles must not define pixel data"
    )]
    HasPixelData { child: String },

    #[error("tile '{child}': template '{template}' has no grid data to inherit")]
    NoGridData { child: String, template: String },
}

/// Validate template references across all tiles.
/// Returns errors for: missing templates, template chains, grid+template conflicts.
pub fn validate_templates(tiles: &HashMap<String, TileRaw>) -> Vec<TemplateError> {
    let mut errors = Vec::new();

    for (name, tile) in tiles {
        let Some(ref template_name) = tile.template else {
            continue;
        };

        // Template must exist
        let Some(base) = tiles.get(template_name.as_str()) else {
            errors.push(TemplateError::NotFound {
                child: name.clone(),
                template: template_name.clone(),
            });
            continue;
        };

        // No template chains
        if base.template.is_some() {
            errors.push(TemplateError::Chain {
                child: name.clone(),
                template: template_name.clone(),
            });
        }

        // Template tile must not have its own grid
        if tile.grid.is_some() || tile.rle.is_some() || tile.layout.is_some() {
            errors.push(TemplateError::HasPixelData {
                child: name.clone(),
            });
        }

        // Base must have grid data
        if base.grid.is_none() && base.rle.is_none() && base.layout.is_none() {
            errors.push(TemplateError::NoGridData {
                child: name.clone(),
                template: template_name.clone(),
            });
        }
    }

    errors
}

/// Resolve a template tile's size from its base tile.
/// Returns the base tile's size if the child doesn't have one.
pub fn resolve_template_size(child: &TileRaw, tiles: &HashMap<String, TileRaw>) -> Option<String> {
    if child.size.is_some() {
        return child.size.clone();
    }
    child
        .template
        .as_ref()
        .and_then(|t| tiles.get(t.as_str()))
        .and_then(|base| base.size.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tile(
        palette: &str,
        size: Option<&str>,
        template: Option<&str>,
        grid: Option<&str>,
    ) -> TileRaw {
        TileRaw {
            palette: palette.to_string(),
            size: size.map(|s| s.to_string()),
            encoding: None,
            symmetry: None,
            auto_rotate: None,
            auto_rotate_weight: None,
            template: template.map(|s| s.to_string()),
            edge_class: None,
            corner_class: None,
            tags: vec![],
            target_layer: None,
            weight: 1.0,
            palette_swaps: vec![],
            cycles: vec![],
            nine_slice: None,
            visual_height_extra: None,
            semantic: None,
            grid: grid.map(|s| s.to_string()),
            rle: None,
            layout: None,
        }
    }

    #[test]
    fn valid_template() {
        let mut tiles = HashMap::new();
        tiles.insert(
            "base".to_string(),
            make_tile("p", Some("16x16"), None, Some("####")),
        );
        tiles.insert(
            "child".to_string(),
            make_tile("p2", None, Some("base"), None),
        );

        let errors = validate_templates(&tiles);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn missing_template() {
        let mut tiles = HashMap::new();
        tiles.insert(
            "child".to_string(),
            make_tile("p", None, Some("nonexistent"), None),
        );

        let errors = validate_templates(&tiles);
        assert_eq!(errors.len(), 1);
        assert!(matches!(&errors[0], TemplateError::NotFound { .. }));
    }

    #[test]
    fn template_chain_rejected() {
        let mut tiles = HashMap::new();
        tiles.insert(
            "base".to_string(),
            make_tile("p", Some("16x16"), Some("other"), Some("####")),
        );
        tiles.insert(
            "child".to_string(),
            make_tile("p2", None, Some("base"), None),
        );

        let errors = validate_templates(&tiles);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, TemplateError::Chain { .. }))
        );
    }

    #[test]
    fn template_with_grid_rejected() {
        let mut tiles = HashMap::new();
        tiles.insert(
            "base".to_string(),
            make_tile("p", Some("16x16"), None, Some("####")),
        );
        tiles.insert(
            "child".to_string(),
            make_tile("p2", None, Some("base"), Some("++++")),
        );

        let errors = validate_templates(&tiles);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, TemplateError::HasPixelData { .. }))
        );
    }

    #[test]
    fn resolve_size_from_base() {
        let mut tiles = HashMap::new();
        tiles.insert(
            "base".to_string(),
            make_tile("p", Some("16x16"), None, Some("####")),
        );
        let child = make_tile("p2", None, Some("base"), None);

        let size = resolve_template_size(&child, &tiles);
        assert_eq!(size, Some("16x16".to_string()));
    }
}
