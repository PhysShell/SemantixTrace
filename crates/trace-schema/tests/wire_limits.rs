//! Wire nesting limits: the writer must never emit a line the readers
//! reject.
//!
//! serde_json refuses to descend into the 128th nested container, so a
//! readable document holds at most 127 container levels. Discovered by
//! the structure-aware `upcaster_v1_to_current` fuzz target: an event
//! whose `args` nest deeper serializes happily and then can never be
//! read back — recorded-then-unreadable is the worst possible failure
//! mode for an evidence store. `write_event` therefore fails closed on
//! events whose serialized form would exceed the reader limit, with
//! bounds chosen as the exact complement of the read path: everything
//! readable stays writable, everything writable stays readable.

use chrono::TimeZone;
use serde_json::json;
use trace_core::{EventSeq, FieldId, ScreenId, SessionId, ValuePolicy};
use trace_schema::v1::TraceEventKind;
use trace_schema::{read_event, write_event, Current, SchemaError};

fn nested(depth: usize) -> serde_json::Value {
    let mut value = json!(0);
    for _ in 0..depth {
        value = json!([value]);
    }
    value
}

fn event(kind: TraceEventKind) -> Current {
    Current {
        seq: EventSeq::new(0),
        session_id: SessionId::new(uuid::Uuid::from_u128(1)),
        ts: chrono::Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
        correlation_id: None,
        domain_entity_id: None,
        kind,
    }
}

fn args_event(depth: usize) -> Current {
    event(TraceEventKind::CommandExecuted {
        command_id: trace_core::CommandId::new("X.Do"),
        args: nested(depth),
        duration_ms: 1,
        outcome: trace_core::Outcome::Success,
    })
}

fn raw_policy_event(depth: usize) -> Current {
    event(TraceEventKind::FieldChanged {
        field_id: FieldId::new("F"),
        old: ValuePolicy::Raw {
            value: nested(depth),
        },
        new: ValuePolicy::Removed,
    })
}

/// The deepest readable `args` nesting (126 containers under the
/// 1-level envelope) round-trips losslessly.
#[test]
fn deepest_readable_args_round_trip() {
    let e = args_event(126);
    let line = write_event(&e).expect("boundary-depth event must serialize");
    let back = read_event(line.trim_end()).expect("own output must read back");
    assert_eq!(back, e);
}

/// One level deeper and every reader rejects the line — so the writer
/// must refuse to produce it instead of recording unreadable evidence.
#[test]
fn write_refuses_args_nesting_readers_reject() {
    let e = args_event(127);
    match write_event(&e) {
        Err(SchemaError::InvalidShape(msg)) => {
            assert!(
                msg.contains("nesting") || msg.contains("deep"),
                "diagnostic must name the nesting limit, got: {msg}"
            );
        }
        Ok(line) => {
            // If this ever starts succeeding, the line must be readable.
            read_event(line.trim_end()).expect("write_event emitted a line no reader accepts");
            panic!("boundary moved: update the wire-limit constants");
        }
        Err(other) => panic!("expected InvalidShape, got {other:?}"),
    }
}

/// `ValuePolicy::Raw` sits two container levels below the document
/// root (envelope -> policy object -> value), so its complement bound
/// is one lower: 125 round-trips, 126 must be refused.
#[test]
fn raw_policy_value_bounds_are_the_exact_complement() {
    let ok = raw_policy_event(125);
    let line = write_event(&ok).expect("boundary-depth event must serialize");
    let back = read_event(line.trim_end()).expect("own output must read back");
    assert_eq!(back, ok);

    let too_deep = raw_policy_event(126);
    match write_event(&too_deep) {
        Err(SchemaError::InvalidShape(_)) => {}
        Ok(line) => {
            read_event(line.trim_end()).expect("write_event emitted a line no reader accepts");
            panic!("boundary moved: update the wire-limit constants");
        }
        Err(other) => panic!("expected InvalidShape, got {other:?}"),
    }
}
