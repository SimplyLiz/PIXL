use thiserror::Error;

#[derive(Debug, Error)]
pub enum PaxlError {
    #[error("serialization: {0}")]
    Serialize(String),

    #[error("line {line}: {message}")]
    Parse { line: usize, message: String },

    #[error("row count mismatch in tile '{tile}': declared [{declared}], actual {actual}")]
    RowCountMismatch {
        tile: String,
        declared: usize,
        actual: usize,
    },

    #[error("unknown directive: @{0}")]
    UnknownDirective(String),

    #[error("stamp not found: @{0}")]
    StampNotFound(String),

    #[error("palette not found: {0}")]
    PaletteNotFound(String),

    #[error("size parse error: {0}")]
    Size(String),
}
