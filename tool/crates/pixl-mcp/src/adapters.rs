//! Adapter registry — scans directories for LoRA adapter bundles.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Metadata about a discovered LoRA adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterInfo {
    pub name: String,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub train_samples: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epochs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
}

/// Scan `base_dir` for subdirectories that contain adapter artifacts
/// (`adapters.safetensors` or `adapter_config.json`).
/// Returns an `AdapterInfo` for each discovered adapter, enriched with
/// metadata from `style_adapter.json` when present.
pub fn list_adapters(base_dir: &Path) -> Vec<AdapterInfo> {
    let entries = match std::fs::read_dir(base_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut adapters = Vec::new();

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }

        let has_safetensors = dir.join("adapters.safetensors").exists();
        let has_config = dir.join("adapter_config.json").exists();

        if !has_safetensors && !has_config {
            continue;
        }

        let name = dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut info = AdapterInfo {
            name,
            path: dir.clone(),
            model: None,
            train_samples: None,
            epochs: None,
            created: None,
        };

        // Try to load optional style_adapter.json metadata
        let meta_path = dir.join("style_adapter.json");
        if meta_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&meta_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                    info.model = val.get("model").and_then(|v| v.as_str()).map(String::from);
                    info.train_samples =
                        val.get("train_samples").and_then(|v| v.as_u64());
                    info.epochs = val.get("epochs").and_then(|v| v.as_u64());
                    info.created =
                        val.get("created").and_then(|v| v.as_str()).map(String::from);
                }
            }
        }

        adapters.push(info);
    }

    adapters.sort_by(|a, b| a.name.cmp(&b.name));
    adapters
}
