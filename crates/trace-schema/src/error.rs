//! Errors emitted by [`crate::read_event`] and the per-version parsers.

use thiserror::Error;

/// All ways a schema operation can fail.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SchemaError {
    /// The bytes are not valid JSON (or are valid JSON but of the wrong
    /// top-level shape).
    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// JSON parsed but did not match the expected event envelope for
    /// its declared version (e.g. missing required field, unknown
    /// `kind` discriminator).
    #[error("invalid event shape: {0}")]
    InvalidShape(String),

    /// `schema_version` is outside the range `[1, CURRENT]`.
    #[error("unsupported schema version: {0}")]
    UnsupportedSchemaVersion(u32),
}
