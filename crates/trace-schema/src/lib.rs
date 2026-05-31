//! `trace-schema` — versioned wire schema and upcaster chain for
//! `SemantxTrace` (ADR-0006).
//!
//! - Module [`v1`] — the first concrete wire schema. Frozen forever once
//!   v1.0 ships; new fields land in `v2` per `docs/upcasters.md`.
//! - Module [`v2`] — adds [`v2::TraceEvent::domain_entity_id`]. The
//!   [`v2::V1ToV2`] upcaster lifts v1 events by setting the new field to
//!   `None`. [`Current`] now points at `v2::TraceEvent`.
//! - [`Current`] — alias pointing at the latest event type known to
//!   this binary. Every downstream crate imports `trace_schema::Current`
//!   (re-exported as `TraceEvent` in some crates) and never names a
//!   per-version event type directly.
//! - [`Upcaster`] / [`StreamUpcaster`] — closed-set traits (ADR-0012
//!   `C-SEALED`) that describe how an older event becomes the current
//!   one.
//! - [`read_event`] — single-line JSON parser dispatching on
//!   `schema_version`. The only place in the workspace that pattern-
//!   matches on a version number.
//!
//! See `docs/upcasters.md` for the full pattern reference, including
//! the bump procedure when a v3 lands.

#![forbid(unsafe_code)]

use serde::Deserialize;

pub mod v1;
pub mod v2;

mod error;
mod upcaster;

pub use error::SchemaError;
pub use upcaster::{sealed, StreamUpcaster, Upcaster};

/// The latest schema version this binary build understands.
///
/// Bumped in lock-step with a new module (`v2`, `v3`, …) plus a new
/// [`Upcaster`] impl. Domain code that needs to write the "current
/// schema number" reads this constant, never a literal.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Type alias pointing at the highest schema version known to this
/// binary build. Always re-exported as `TraceEvent` by `trace-core`
/// consumers.
pub type Current = v2::TraceEvent;

/// Minimal helper struct used by [`read_event`] to inspect the
/// `schema_version` discriminator before deserializing the full event
/// at the right concrete type.
#[derive(Debug, Deserialize)]
struct VersionProbe {
    schema_version: u32,
}

/// Parse a single JSONL line into the current event shape, applying
/// the upcaster chain as needed.
///
/// This is the single place in the workspace that pattern-matches on
/// `schema_version`. Domain crates (`trace-normalizer`, `trace-graph`,
/// `trace-oracle`) call this and see only [`Current`] (ADR-0006 hard
/// rule, mirrored in `AGENTS.md` anti-patterns).
///
/// # Errors
///
/// Returns:
/// - [`SchemaError::Parse`] if the input is not valid JSON;
/// - [`SchemaError::InvalidShape`] if the JSON parses but does not
///   match the expected event envelope for its declared version;
/// - [`SchemaError::UnsupportedSchemaVersion`] if the
///   `schema_version` is higher than [`CURRENT_SCHEMA_VERSION`] or
///   lower than the oldest known version (`1`).
pub fn read_event(raw: &str) -> Result<Current, SchemaError> {
    let probe: VersionProbe = serde_json::from_str(raw).map_err(classify_json_error)?;
    match probe.schema_version {
        1 => {
            let envelope: v1::TraceEnvelope =
                serde_json::from_str(raw).map_err(classify_json_error)?;
            Ok(v2::V1ToV2::upcast(envelope.into_event()))
        }
        2 => {
            let envelope: v2::TraceEnvelope =
                serde_json::from_str(raw).map_err(classify_json_error)?;
            Ok(envelope.into_event())
        }
        other => Err(SchemaError::UnsupportedSchemaVersion(other)),
    }
}

/// Classify a `serde_json` failure into the right [`SchemaError`].
///
/// A `Data` category failure means the bytes are valid JSON of the
/// wrong shape (missing field, unknown `kind`, bad type) — that is
/// [`SchemaError::InvalidShape`], the schema-specific diagnostic
/// callers use to distinguish corrupt-but-parseable JSONL records from
/// genuinely malformed bytes. Syntax / EOF / IO failures stay
/// [`SchemaError::Parse`].
fn classify_json_error(err: serde_json::Error) -> SchemaError {
    match err.classify() {
        serde_json::error::Category::Data => SchemaError::InvalidShape(err.to_string()),
        _ => SchemaError::Parse(err),
    }
}

/// Serialize a [`Current`] event into a single JSONL line (terminated
/// by `\n`).
///
/// # Errors
///
/// Returns [`SchemaError::Parse`] (wrapping `serde_json::Error`) on the
/// rare case the event contains a value `serde_json` refuses to
/// serialize. Domain-constructed events never trigger that.
pub fn write_event(event: &Current) -> Result<String, SchemaError> {
    let envelope = v2::TraceEnvelope::from_event(event.clone());
    let mut s = serde_json::to_string(&envelope).map_err(SchemaError::Parse)?;
    s.push('\n');
    Ok(s)
}
