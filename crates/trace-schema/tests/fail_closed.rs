//! Fail-closed dispatch contract for [`trace_schema::read_event`].
//!
//! Three properties the version gate must satisfy, none of which the
//! round-trip suite can pin:
//!
//! 1. **Unknown versions fail closed.** Any `schema_version` outside
//!    `[1, CURRENT_SCHEMA_VERSION]` — including `0` and the entire
//!    future space — yields `UnsupportedSchemaVersion`, never a
//!    best-effort parse, never a silent default.
//! 2. **The probe reads only the version field.** Version dispatch
//!    happens *before* event-shape validation, so a future-versioned
//!    line is rejected by version even when its body is garbage the
//!    current shapes would reject for other reasons.
//! 3. **Non-integer versions are typed errors.** A negative, string,
//!    fractional, boolean, or null `schema_version` is an
//!    `InvalidShape`, not a panic and not a fallback to some default
//!    version.

mod common;

use proptest::prelude::*;
use trace_schema::{read_event, write_event, SchemaError, CURRENT_SCHEMA_VERSION};

use common::arb_current_event;

/// A syntactically valid v1 event body (no `schema_version` key), used
/// to build dispatch-table lines where only the version varies.
const VALID_V1_BODY: &str = r#""seq":0,"session_id":"00000000-0000-0000-0000-000000000001","ts":"2026-05-27T12:00:00Z","kind":"ScreenOpened","screen_id":"X","params":{}"#;

fn line_with_version(version: &str) -> String {
    format!("{{\"schema_version\":{version},{VALID_V1_BODY}}}")
}

#[test]
fn version_zero_fails_closed() {
    match read_event(&line_with_version("0")) {
        Err(SchemaError::UnsupportedSchemaVersion(0)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn next_unshipped_version_fails_closed() {
    let next = CURRENT_SCHEMA_VERSION + 1;
    match read_event(&line_with_version(&next.to_string())) {
        Err(SchemaError::UnsupportedSchemaVersion(v)) if v == next => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn negative_version_is_invalid_shape() {
    match read_event(&line_with_version("-1")) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn string_version_is_invalid_shape() {
    match read_event(&line_with_version("\"1\"")) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn fractional_version_is_invalid_shape() {
    match read_event(&line_with_version("1.5")) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn boolean_version_is_invalid_shape() {
    match read_event(&line_with_version("true")) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn null_version_is_invalid_shape() {
    match read_event(&line_with_version("null")) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

/// The version gate fires before shape validation: a future-versioned
/// line whose body is complete garbage is rejected *by version*, which
/// also proves the probe does not deserialize the full payload (the
/// garbage body would fail any full parse).
#[test]
fn future_version_rejected_before_shape_validation() {
    let raw = r#"{"schema_version":47,"complete":"garbage","not":["an","event"]}"#;
    match read_event(raw) {
        Err(SchemaError::UnsupportedSchemaVersion(47)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

/// Counterpart to the probe test above: once the declared version *is*
/// supported, body validation runs at that version's concrete shape and
/// surfaces `InvalidShape`.
#[test]
fn supported_version_with_bad_body_is_invalid_shape() {
    let raw = format!(
        "{{\"schema_version\":{CURRENT_SCHEMA_VERSION},\"seq\":\"not-a-number\",\"kind\":\"ScreenOpened\"}}"
    );
    match read_event(&raw) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

/// `write_event` stamps exactly [`CURRENT_SCHEMA_VERSION`] on the wire,
/// so the constant and the envelope literal cannot drift apart
/// unnoticed.
#[test]
fn write_event_stamps_current_schema_version() {
    let raw = write_event(&sample_current()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(raw.trim_end()).expect("valid JSON");
    assert_eq!(
        value["schema_version"],
        serde_json::json!(CURRENT_SCHEMA_VERSION)
    );
}

fn sample_current() -> trace_schema::Current {
    use chrono::TimeZone;
    use trace_core::{EventSeq, ScreenId, SessionId};
    trace_schema::Current {
        seq: EventSeq::new(0),
        session_id: SessionId::new(uuid::Uuid::from_u128(1)),
        ts: chrono::Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
        correlation_id: None,
        domain_entity_id: None,
        kind: trace_schema::v1::TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("Editor"),
            params: serde_json::json!({}),
        },
    }
}

proptest! {
    /// The entire future version space fails closed, with the declared
    /// version echoed in the error (never truncated or defaulted).
    #[test]
    fn every_future_version_fails_closed(version in (CURRENT_SCHEMA_VERSION + 1)..=u32::MAX) {
        match read_event(&line_with_version(&version.to_string())) {
            Err(SchemaError::UnsupportedSchemaVersion(v)) => prop_assert_eq!(v, version),
            other => prop_assert!(false, "unexpected: {:?}", other),
        }
    }

    /// docs/upcasters.md mandatory property 3 — identity on the current
    /// version, *byte-identical*: re-serializing a parsed self-written
    /// line reproduces the exact bytes.
    #[test]
    fn reserialization_is_byte_identical(event in arb_current_event()) {
        let first = write_event(&event).expect("serialize");
        let reread = read_event(first.trim_end_matches('\n')).expect("read_event");
        let second = write_event(&reread).expect("re-serialize");
        prop_assert_eq!(first, second);
    }
}
