//! PAX-L — LLM-optimized compact wire format for PAX files.
//!
//! PAX-L is a runtime representation, not a file format. It provides ~40-60%
//! token reduction vs TOML while remaining fully lossless and round-trippable.
//!
//! See `docs/specs/pax-llm.md` for the full specification.

pub mod bpe;
pub mod deserialize;
pub mod encoding;
pub mod error;
pub mod serialize;

pub use error::PaxlError;

use crate::types::PaxFile;

/// Configuration for PAX-L serialization.
pub struct PaxlConfig {
    /// Enable =N row references for duplicate rows (default: true)
    pub row_refs: bool,
    /// Enable @fill pattern detection (default: true)
    pub fill_detect: bool,
    /// Enable @delta encoding for similar tiles (default: true)
    pub delta_detect: bool,
    /// Max patch count for delta to be cheaper than full grid (default: 12)
    pub delta_threshold: usize,
}

impl Default for PaxlConfig {
    fn default() -> Self {
        Self {
            row_refs: true,
            fill_detect: true,
            delta_detect: true,
            delta_threshold: 12,
        }
    }
}

/// Convert a PaxFile to PAX-L compact representation.
pub fn to_paxl(file: &PaxFile, config: &PaxlConfig) -> Result<String, PaxlError> {
    serialize::serialize(file, config)
}

/// Parse PAX-L text back into a PaxFile.
///
/// In lenient mode (strict=false), common LLM mistakes are auto-fixed with warnings.
/// In strict mode (strict=true), any structural error is fatal.
pub fn from_paxl(source: &str, strict: bool) -> Result<(PaxFile, Vec<String>), PaxlError> {
    deserialize::from_paxl(source, strict)
}
