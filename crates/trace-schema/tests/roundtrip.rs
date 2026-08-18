//! Property tests for the v1 and v2 wire schemas and the v1→v2 upcaster
//! chain, per acceptance criteria in
//! `docs/stages/S1-trace-core-and-schema-v1.md` and the contract in
//! `docs/upcasters.md`.
//!
//! Strategies live in [`common`] and are shared with the fail-closed
//! and schema-parity suites.

mod common;

use proptest::prelude::*;
use trace_schema::{read_event, v1, v2, write_event, Current, SchemaError, Upcaster};

use common::{arb_current_event, arb_v1_event};

// ---------------------------------------------------------------------------
// Invariants — see docs/stages/S1 §"Acceptance criteria" and
// docs/upcasters.md §"Property tests (mandatory)".
// ---------------------------------------------------------------------------

proptest! {
    /// forall e: Current, parse(serialize(e)) == Ok(e)
    #[test]
    fn serialize_then_read_event_round_trips(event in arb_current_event()) {
        let raw = write_event(&event).expect("serialize");
        let trimmed = raw.trim_end_matches('\n');
        let parsed: Current = read_event(trimmed).expect("read_event");
        prop_assert_eq!(parsed, event);
    }

    /// Upcaster chain property: read_event of a v1-serialised event equals
    /// the direct upcast of that event.
    ///
    /// upcast(parse(serialize_v1(e))) == upcast(e)
    #[test]
    fn upcast_chain_v1_to_current(event in arb_v1_event()) {
        let raw = serde_json::to_string(&v1::TraceEnvelope::from_event(event.clone()))
            .expect("serialize v1");
        let parsed: Current = read_event(&raw).expect("read_event");
        let expected = v2::V1ToV2::upcast(event);
        prop_assert_eq!(parsed, expected);
    }
}

#[test]
fn unsupported_schema_version_rejected() {
    let raw = r#"{"schema_version":999,"seq":0,"session_id":"00000000-0000-0000-0000-000000000000","ts":"2026-05-27T12:00:00Z","kind":"ScreenOpened","screen_id":"X","params":{}}"#;
    match read_event(raw) {
        Err(SchemaError::UnsupportedSchemaVersion(999)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn malformed_json_rejected() {
    match read_event("{not json") {
        Err(SchemaError::Parse(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn valid_json_wrong_shape_is_invalid_shape() {
    // Syntactically valid JSON with schema_version=1 but a malformed
    // envelope (missing `seq`, missing `kind`). Must surface as
    // InvalidShape, not Parse — see Codex review on PR #3.
    let raw = r#"{"schema_version":1,"session_id":"00000000-0000-0000-0000-000000000000"}"#;
    match read_event(raw) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn valid_json_missing_version_is_invalid_shape() {
    // Valid JSON, but no schema_version field at all → InvalidShape
    // (the probe deserialize fails with a Data-category error).
    let raw = r#"{"hello":"world"}"#;
    match read_event(raw) {
        Err(SchemaError::InvalidShape(_)) => {}
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn v1_event_upcasted_to_v2_on_read() {
    // A v1 JSONL line (no domain_entity_id) must be readable as Current
    // (v2) with domain_entity_id = None.
    let raw = r#"{"schema_version":1,"seq":0,"session_id":"00000000-0000-0000-0000-000000000001","ts":"2026-05-30T00:00:00Z","kind":"ScreenOpened","screen_id":"Editor","params":{}}"#;
    let event = read_event(raw).expect("read_event");
    assert!(event.domain_entity_id.is_none());
}

#[test]
fn v2_event_with_entity_id_round_trips() {
    use trace_core::DomainEntityId;
    let raw = r#"{"schema_version":2,"seq":0,"session_id":"00000000-0000-0000-0000-000000000001","ts":"2026-05-30T00:00:00Z","domain_entity_id":"Declaration:doc-123","kind":"ScreenOpened","screen_id":"Editor","params":{}}"#;
    let event = read_event(raw).expect("read_event");
    assert_eq!(
        event.domain_entity_id,
        Some(DomainEntityId::new("Declaration:doc-123"))
    );
}
