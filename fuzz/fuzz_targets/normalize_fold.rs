//! Fuzz target `normalize_fold` (ADR-0010 P1, S3) — structure-aware
//! `arbitrary` over a session, per `docs/fuzzing.md` (input shape:
//! "`arbitrary` `Session`"). Oracle:
//!
//! - no panic / no hang / no unbounded allocation;
//! - `normalize` is deterministic (same session → same output);
//! - event conservation: `input_events == output_actions +
//!   collapsed_bursts + dropped_noise`;
//! - every produced action's `abstract_args` is a fixed point of value
//!   abstraction (re-abstraction is a no-op);
//! - order preservation modulo collapse: the default-window fold is an
//!   in-order subsequence of the zero-window fold (which itself
//!   collapses nothing).
//!
//! The `Arbitrary` mirrors live here, not on the production types.

#![no_main]

use arbitrary::Arbitrary;
use chrono::TimeZone;
use libfuzzer_sys::fuzz_target;
use trace_core::{
    CanonicalAction, CommandId, EventSeq, FieldId, Outcome, ScreenId, Session, SessionId,
    ValuePolicy,
};
use trace_normalizer::abstraction::abstract_value;
use trace_normalizer::{NormCfg, normalize};
use trace_schema::Current;
use trace_schema::v1::TraceEventKind;

#[derive(Arbitrary, Debug)]
struct ArbSession {
    session_bits: u128,
    events: Vec<ArbEvent>,
}

#[derive(Arbitrary, Debug)]
struct ArbEvent {
    /// Independent absolute offset: adjacent deltas span bursts
    /// (< 50 ms), idle gaps (> 5 s), and out-of-order neighbours.
    offset_ms: u16,
    kind: ArbKind,
}

#[derive(Arbitrary, Debug)]
enum ArbKind {
    ScreenOpened { screen: String },
    CommandExecuted { command: String, args: ArbJson },
    FieldChanged { field: String },
    ExceptionThrown { message: String },
    NavigationOccurred { from: String, to: String },
    ValidationFailed { field: String, reason: String },
    AsyncOperationCompleted { operation_id: String },
}

#[derive(Arbitrary, Debug)]
enum ArbJson {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<ArbJson>),
    Obj(Vec<(String, ArbJson)>),
}

const MAX_MIRROR_JSON_DEPTH: usize = 24;

fn to_json(v: ArbJson) -> serde_json::Value {
    to_json_bounded(v, 0)
}

/// Depth-bounded conversion: the wire's exact nesting boundary is
/// pinned deterministically by trace-schema's `wire_limits` tests;
/// this target stays comfortably inside it so every generated event
/// is writable and the oracle exercises the chain laws, not the
/// depth guard.
fn to_json_bounded(v: ArbJson, depth: usize) -> serde_json::Value {
    if depth >= MAX_MIRROR_JSON_DEPTH {
        return serde_json::Value::Null;
    }
    match v {
        ArbJson::Null => serde_json::Value::Null,
        ArbJson::Bool(b) => serde_json::Value::Bool(b),
        ArbJson::Int(n) => serde_json::Value::Number(n.into()),
        ArbJson::Str(s) => serde_json::Value::String(s),
        ArbJson::Arr(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(|item| to_json_bounded(item, depth + 1))
                .collect(),
        ),
        ArbJson::Obj(entries) => serde_json::Value::Object(
            entries
                .into_iter()
                .map(|(k, item)| (k, to_json_bounded(item, depth + 1)))
                .collect(),
        ),
    }
}

fn to_kind(k: ArbKind) -> TraceEventKind {
    match k {
        ArbKind::ScreenOpened { screen } => TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new(screen),
            params: serde_json::json!({}),
        },
        ArbKind::CommandExecuted { command, args } => TraceEventKind::CommandExecuted {
            command_id: CommandId::new(command),
            args: to_json(args),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
        ArbKind::FieldChanged { field } => TraceEventKind::FieldChanged {
            field_id: FieldId::new(field),
            old: ValuePolicy::default_masked(),
            new: ValuePolicy::Removed,
        },
        ArbKind::ExceptionThrown { message } => TraceEventKind::ExceptionThrown {
            exception_type: "Fuzz".to_owned(),
            message,
            stack: None,
        },
        ArbKind::NavigationOccurred { from, to } => TraceEventKind::NavigationOccurred {
            from: ScreenId::new(from),
            to: ScreenId::new(to),
        },
        ArbKind::ValidationFailed { field, reason } => TraceEventKind::ValidationFailed {
            validator: "Fuzz".to_owned(),
            field_id: FieldId::new(field),
            reason,
        },
        ArbKind::AsyncOperationCompleted { operation_id } => {
            TraceEventKind::AsyncOperationCompleted {
                operation_id,
                duration_ms: 1,
                outcome: Outcome::TimedOut,
            }
        }
    }
}

fn to_session(input: ArbSession) -> Option<Session<Current>> {
    if input.events.is_empty() {
        return None;
    }
    let sid = SessionId::new(uuid::Uuid::from_u128(input.session_bits));
    let base = chrono::Utc
        .with_ymd_and_hms(2026, 5, 27, 12, 0, 0)
        .single()
        .expect("fixed instant is valid");
    let events = input
        .events
        .into_iter()
        .enumerate()
        .map(|(i, e)| Current {
            seq: EventSeq::new(i as u64),
            session_id: sid,
            ts: base + chrono::Duration::milliseconds(i64::from(e.offset_ms)),
            correlation_id: None,
            domain_entity_id: None,
            kind: to_kind(e.kind),
        })
        .collect();
    Session::new(sid, events).ok()
}

fn is_subsequence(needle: &[CanonicalAction], haystack: &[CanonicalAction]) -> bool {
    let mut it = haystack.iter();
    needle.iter().all(|a| it.any(|b| b == a))
}

fuzz_target!(|input: ArbSession| {
    let Some(session) = to_session(input) else {
        return;
    };

    let cfg = NormCfg::default();
    let first = normalize(&session, &cfg);
    let second = normalize(&session, &cfg);
    assert_eq!(first, second, "normalize is not deterministic");

    let (scenario, report) = &first;
    assert_eq!(report.input_events, session.len());
    assert_eq!(
        report.input_events,
        report.output_actions + report.collapsed_bursts + report.dropped_noise,
        "event conservation violated: {report:?}"
    );

    for action in &scenario.actions {
        let re = abstract_value(&action.abstract_args, &cfg);
        assert_eq!(
            re, action.abstract_args,
            "normalized args are not a fixed point of abstraction"
        );
    }

    // Order preservation modulo collapse: the zero-window fold is the
    // full projection; the default-window fold may only remove.
    let zero_gap = NormCfg {
        burst_gap_ms: 0,
        ..NormCfg::default()
    };
    let (full, full_report) = normalize(&session, &zero_gap);
    assert_eq!(full_report.collapsed_bursts, 0);
    assert!(
        is_subsequence(&scenario.actions, &full.actions),
        "collapse reordered or invented actions"
    );
});
