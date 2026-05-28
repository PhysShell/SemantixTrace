//! Property tests for the v1 wire schema and the (identity) upcaster
//! chain, per acceptance criteria in
//! `docs/stages/S1-trace-core-and-schema-v1.md` and the contract in
//! `docs/upcasters.md`.

use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use trace_core::{
    CommandId, CorrelationId, EventSeq, FieldId, Outcome, ScreenId, SessionId, ValuePolicy,
};
use trace_schema::{read_event, v1, write_event, Current, SchemaError};

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

fn arb_session_id() -> impl Strategy<Value = SessionId> {
    any::<u128>().prop_map(|bits| SessionId::new(uuid::Uuid::from_u128(bits)))
}

fn arb_correlation_id() -> impl Strategy<Value = CorrelationId> {
    any::<u128>().prop_map(|bits| CorrelationId::new(uuid::Uuid::from_u128(bits)))
}

fn arb_outcome() -> impl Strategy<Value = Outcome> {
    prop_oneof![
        Just(Outcome::Success),
        Just(Outcome::Cancelled),
        Just(Outcome::TimedOut),
        ".{0,32}".prop_map(|message| Outcome::Failure { message }),
    ]
}

fn arb_value_policy() -> impl Strategy<Value = ValuePolicy> {
    prop_oneof![
        Just(ValuePolicy::Removed),
        ".{0,16}".prop_map(|display| ValuePolicy::Masked { display }),
        ".{0,16}".prop_map(|bucket| ValuePolicy::Bucketed { bucket }),
        (".{0,16}", ".{0,8}").prop_map(|(hash, algo)| ValuePolicy::Hashed { hash, algo }),
    ]
}

fn arb_kind() -> impl Strategy<Value = v1::TraceEventKind> {
    prop_oneof![
        ("[A-Za-z]{1,12}".prop_map(ScreenId::new)).prop_map(|screen_id| {
            v1::TraceEventKind::ScreenOpened {
                screen_id,
                params: serde_json::json!({}),
            }
        }),
        (
            "[A-Za-z]{1,12}\\.[A-Za-z]{1,12}".prop_map(CommandId::new),
            0u64..1_000_000,
            arb_outcome(),
        )
            .prop_map(|(command_id, duration_ms, outcome)| {
                v1::TraceEventKind::CommandExecuted {
                    command_id,
                    args: serde_json::json!({}),
                    duration_ms,
                    outcome,
                }
            }),
        (
            "[A-Za-z]{1,12}".prop_map(FieldId::new),
            arb_value_policy(),
            arb_value_policy(),
        )
            .prop_map(|(field_id, old, new)| v1::TraceEventKind::FieldChanged {
                field_id,
                old,
                new
            }),
        ("[A-Za-z]{1,32}", "[^\"\\\\]{0,32}").prop_map(|(exception_type, message)| {
            v1::TraceEventKind::ExceptionThrown {
                exception_type,
                message,
                stack: None,
            }
        }),
        (
            "[A-Za-z]{1,12}".prop_map(ScreenId::new),
            "[A-Za-z]{1,12}".prop_map(ScreenId::new),
        )
            .prop_map(|(from, to)| v1::TraceEventKind::NavigationOccurred { from, to }),
        (
            "[A-Za-z]{1,12}",
            "[A-Za-z]{1,12}".prop_map(FieldId::new),
            "[^\"\\\\]{0,32}",
        )
            .prop_map(|(validator, field_id, reason)| {
                v1::TraceEventKind::ValidationFailed {
                    validator,
                    field_id,
                    reason,
                }
            }),
        ("[A-Za-z]{1,12}", 0u64..1_000_000, arb_outcome(),).prop_map(
            |(operation_id, duration_ms, outcome)| {
                v1::TraceEventKind::AsyncOperationCompleted {
                    operation_id,
                    duration_ms,
                    outcome,
                }
            }
        ),
    ]
}

fn arb_event() -> impl Strategy<Value = v1::TraceEvent> {
    (
        any::<u64>(),
        arb_session_id(),
        prop::option::of(arb_correlation_id()),
        arb_kind(),
    )
        .prop_map(|(seq, session_id, correlation_id, kind)| v1::TraceEvent {
            seq: EventSeq::new(seq),
            session_id,
            ts: Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            correlation_id,
            kind,
        })
}

// ---------------------------------------------------------------------------
// Invariants — see docs/stages/S1 §"Acceptance criteria" and
// docs/upcasters.md §"Property tests (mandatory)".
// ---------------------------------------------------------------------------

proptest! {
    /// forall e, parse(serialize(e)) == Ok(e)
    #[test]
    fn serialize_then_read_event_round_trips(event in arb_event()) {
        let raw = write_event(&event).expect("serialize");
        // Strip trailing newline write_event adds.
        let trimmed = raw.trim_end_matches('\n');
        let parsed: Current = read_event(trimmed).expect("read_event");
        prop_assert_eq!(parsed, event);
    }

    /// Identity upcast chain: upcast_to_current(parse(serialize(e))) ==
    /// upcast_to_current(e). At S1 the chain is identity (Current = v1),
    /// so this collapses to the round-trip invariant but the test is
    /// kept under its own name so the v2 bump only needs to extend it.
    #[test]
    fn upcast_then_serialize_matches_direct_upcast(event in arb_event()) {
        let raw = write_event(&event).expect("serialize");
        let parsed: Current = read_event(raw.trim_end_matches('\n')).expect("read_event");
        prop_assert_eq!(parsed, event);
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
