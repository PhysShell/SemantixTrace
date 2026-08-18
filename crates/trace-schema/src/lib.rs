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
pub use upcaster::{StreamUpcaster, Upcaster};

pub(crate) use upcaster::sealed;

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
///   match the expected event envelope for its declared version, or
///   if the envelope carries a field introduced in a schema version
///   later than the one it declares (a version-confused writer —
///   tolerating it would silently discard data);
/// - [`SchemaError::UnsupportedSchemaVersion`] if the
///   `schema_version` is higher than [`CURRENT_SCHEMA_VERSION`] or
///   lower than the oldest known version (`1`).
pub fn read_event(raw: &str) -> Result<Current, SchemaError> {
    let probe: VersionProbe = serde_json::from_str(raw).map_err(classify_json_error)?;
    if !(1..=CURRENT_SCHEMA_VERSION).contains(&probe.schema_version) {
        return Err(SchemaError::UnsupportedSchemaVersion(probe.schema_version));
    }

    // Parse once to a Value so the guard sees *decoded* object keys
    // (JSON `\u` escapes included); each arm then builds its envelope
    // from the same Value — no third pass over the input.
    //
    // The guard runs HERE, once, before version dispatch: it applies
    // to every declared version, so a future bump extends
    // [`FIELDS_BY_INTRODUCING_VERSION`] and every older version's arm
    // is protected automatically. A per-arm guard would quietly leave
    // the newest frozen arm unguarded at the next bump — the exact
    // shape of bug this crate exists to prevent.
    let value: serde_json::Value = serde_json::from_str(raw).map_err(classify_json_error)?;
    reject_later_version_fields(&value, probe.schema_version)?;

    match probe.schema_version {
        1 => {
            let envelope: v1::TraceEnvelope =
                serde_json::from_value(value).map_err(classify_json_error)?;
            Ok(v2::V1ToV2::upcast(envelope.into_event()))
        }
        2 => {
            let envelope: v2::TraceEnvelope =
                serde_json::from_value(value).map_err(classify_json_error)?;
            Ok(envelope.into_event())
        }
        // Unreachable after the range check above; kept total so a
        // future refactor cannot turn this into a panic path.
        other => Err(SchemaError::UnsupportedSchemaVersion(other)),
    }
}

/// Top-level envelope fields keyed by the schema version that
/// introduced them. An envelope stamped with an *older* version that
/// carries one of these is version-confused input (most likely a newer
/// writer stamping the wrong number); parsing it would silently drop
/// the field's data, so [`read_event`] fails closed instead. Genuinely
/// unknown fields — not claimed by any later version — remain
/// tolerated, because `docs/upcasters.md` sanctions additive
/// `#[serde(default)]` fields within a released version.
///
/// Bump procedure: when a v(n+1) module lands, add an entry mapping
/// `n + 1` to its new top-level field names. The guard is invoked once
/// in [`read_event`], before version dispatch, so every version older
/// than `n + 1` — including the previously-newest one — rejects the
/// new fields automatically; no arm needs to remember anything.
const FIELDS_BY_INTRODUCING_VERSION: [(u32, &[&str]); 1] = [(2, &["domain_entity_id"])];

/// Fail closed when an envelope carries a field introduced in a schema
/// version later than the one it declares.
///
/// Operates on the *parsed* object so JSON `\u`-escaped key spellings
/// cannot bypass the check, while field names occurring inside string
/// values (data, not keys) never trip it.
fn reject_later_version_fields(
    value: &serde_json::Value,
    declared: u32,
) -> Result<(), SchemaError> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    for (introduced_in, fields) in FIELDS_BY_INTRODUCING_VERSION {
        if introduced_in <= declared {
            continue;
        }
        for field in fields {
            if object.contains_key(*field) {
                return Err(SchemaError::InvalidShape(format!(
                    "v{declared} envelope carries `{field}`, a field introduced in \
                     v{introduced_in}; refusing to parse a version-confused line"
                )));
            }
        }
    }
    Ok(())
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
/// The writer never emits a line the readers reject: embedded JSON
/// values are depth-checked against the exact complement of the read
/// path's recursion limit before serialization (see
/// [`MAX_DOCUMENT_CONTAINER_DEPTH`]'s docs), so `write_event(e) = Ok(line)`
/// implies `read_event(line) = Ok(e)`.
///
/// # Errors
///
/// Returns:
/// - [`SchemaError::InvalidShape`] if an embedded value nests so
///   deeply that the serialized document would exceed the readers'
///   recursion limit — recording it would create a line no reader can
///   ever parse back;
/// - [`SchemaError::Parse`] (wrapping `serde_json::Error`) on the
///   rare case the event contains a value `serde_json` refuses to
///   serialize. Domain-constructed events never trigger that.
pub fn write_event(event: &Current) -> Result<String, SchemaError> {
    reject_unreadable_nesting(event)?;
    let envelope = v2::TraceEnvelope::from_event(event.clone());
    let mut s = serde_json::to_string(&envelope).map_err(SchemaError::Parse)?;
    s.push('\n');
    Ok(s)
}

/// The deepest container nesting a serialized document may reach and
/// still be readable: `serde_json` refuses to descend into the 128th
/// nested container, so 127 levels is the empirically calibrated
/// maximum (pinned by the `wire_limits` boundary tests).
const MAX_DOCUMENT_CONTAINER_DEPTH: usize = 127;

/// Fail closed on events whose embedded values would serialize into a
/// document deeper than every reader's recursion limit. The check is
/// the exact complement of the read path: each embedded value's own
/// container depth plus its fixed offset below the document root
/// (`args`/`params` sit under 1 envelope level, a
/// [`ValuePolicy::Raw`] value under 2) must stay within
/// [`MAX_DOCUMENT_CONTAINER_DEPTH`]. Anything `read_event` accepts
/// therefore re-serializes, and anything serialized re-reads.
///
/// The walk is iterative: a hostile many-thousand-level value must not
/// overflow the checker's own stack (serialization would — which is
/// why the check runs first).
fn reject_unreadable_nesting(event: &Current) -> Result<(), SchemaError> {
    use trace_core::ValuePolicy;

    let mut embedded: Vec<(&serde_json::Value, usize)> = Vec::new();
    match &event.kind {
        v1::TraceEventKind::ScreenOpened { params, .. } => embedded.push((params, 1)),
        v1::TraceEventKind::CommandExecuted { args, .. } => embedded.push((args, 1)),
        v1::TraceEventKind::FieldChanged { old, new, .. } => {
            for policy in [old, new] {
                if let ValuePolicy::Raw { value } = policy {
                    embedded.push((value, 2));
                }
            }
        }
        _ => {}
    }

    for (value, offset) in embedded {
        let depth = offset + container_depth(value);
        if depth > MAX_DOCUMENT_CONTAINER_DEPTH {
            return Err(SchemaError::InvalidShape(format!(
                "event value nesting produces a document {depth} containers deep; \
                 readers stop at {MAX_DOCUMENT_CONTAINER_DEPTH} — refusing to \
                 write an unreadable line"
            )));
        }
    }
    Ok(())
}

/// Maximum container (array/object) nesting depth of `value`, computed
/// iteratively so the checker itself cannot overflow on hostile input.
fn container_depth(value: &serde_json::Value) -> usize {
    let mut deepest = 0;
    let mut stack: Vec<(&serde_json::Value, usize)> = vec![(value, 0)];
    while let Some((node, depth)) = stack.pop() {
        match node {
            serde_json::Value::Array(items) => {
                let d = depth + 1;
                deepest = deepest.max(d);
                for item in items {
                    stack.push((item, d));
                }
            }
            serde_json::Value::Object(entries) => {
                let d = depth + 1;
                deepest = deepest.max(d);
                for child in entries.values() {
                    stack.push((child, d));
                }
            }
            _ => {}
        }
    }
    deepest
}
