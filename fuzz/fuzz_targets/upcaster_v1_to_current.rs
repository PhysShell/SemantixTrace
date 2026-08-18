//! Fuzz target `upcaster_v1_to_current` — structure-aware `arbitrary`
//! over the v1 event shape, per ADR-0010 / `docs/fuzzing.md` (input
//! shape: "`arbitrary` over `v1::TraceEvent`"; oracle: "result is a
//! valid `Current` (validates against the published schema)").
//!
//! The `Arbitrary` mirrors live here, not on the production types:
//! `trace-schema` stays free of fuzz-only derives, and the mirrors
//! convert into the real frozen v1 types before every invariant runs.

#![no_main]

use std::sync::OnceLock;

use arbitrary::Arbitrary;
use chrono::TimeZone;
use libfuzzer_sys::fuzz_target;
use trace_core::{
    CommandId, CorrelationId, EventSeq, FieldId, Outcome, ScreenId, SessionId, ValuePolicy,
};
use trace_schema::{Upcaster, read_event, v1, v2, write_event};

#[derive(Arbitrary, Debug)]
struct ArbV1Event {
    seq: u64,
    session_bits: u128,
    correlation_bits: Option<u128>,
    ts_millis: i64,
    kind: ArbKind,
}

#[derive(Arbitrary, Debug)]
enum ArbKind {
    ScreenOpened {
        screen: String,
        params: ArbJson,
    },
    CommandExecuted {
        command: String,
        args: ArbJson,
        duration_ms: u64,
        outcome: ArbOutcome,
    },
    FieldChanged {
        field: String,
        old: ArbPolicy,
        new: ArbPolicy,
    },
    ExceptionThrown {
        exception_type: String,
        message: String,
        stack: Option<String>,
    },
    NavigationOccurred {
        from: String,
        to: String,
    },
    ValidationFailed {
        validator: String,
        field: String,
        reason: String,
    },
    AsyncOperationCompleted {
        operation_id: String,
        duration_ms: u64,
        outcome: ArbOutcome,
    },
}

#[derive(Arbitrary, Debug)]
enum ArbOutcome {
    Success,
    Failure { message: String },
    Cancelled,
    TimedOut,
}

#[derive(Arbitrary, Debug)]
enum ArbPolicy {
    Raw { value: ArbJson },
    Masked { display: String },
    Bucketed { bucket: String },
    Hashed { hash: String, algo: String },
    Removed,
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

fn to_json(v: ArbJson) -> serde_json::Value {
    match v {
        ArbJson::Null => serde_json::Value::Null,
        ArbJson::Bool(b) => serde_json::Value::Bool(b),
        ArbJson::Int(n) => serde_json::Value::Number(n.into()),
        ArbJson::Str(s) => serde_json::Value::String(s),
        ArbJson::Arr(items) => serde_json::Value::Array(items.into_iter().map(to_json).collect()),
        ArbJson::Obj(entries) => serde_json::Value::Object(
            entries.into_iter().map(|(k, v)| (k, to_json(v))).collect(),
        ),
    }
}

fn to_outcome(o: ArbOutcome) -> Outcome {
    match o {
        ArbOutcome::Success => Outcome::Success,
        ArbOutcome::Failure { message } => Outcome::Failure { message },
        ArbOutcome::Cancelled => Outcome::Cancelled,
        ArbOutcome::TimedOut => Outcome::TimedOut,
    }
}

fn to_policy(p: ArbPolicy) -> ValuePolicy {
    match p {
        ArbPolicy::Raw { value } => ValuePolicy::Raw {
            value: to_json(value),
        },
        ArbPolicy::Masked { display } => ValuePolicy::Masked { display },
        ArbPolicy::Bucketed { bucket } => ValuePolicy::Bucketed { bucket },
        ArbPolicy::Hashed { hash, algo } => ValuePolicy::Hashed { hash, algo },
        ArbPolicy::Removed => ValuePolicy::Removed,
    }
}

fn to_kind(k: ArbKind) -> v1::TraceEventKind {
    match k {
        ArbKind::ScreenOpened { screen, params } => v1::TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new(screen),
            params: to_json(params),
        },
        ArbKind::CommandExecuted {
            command,
            args,
            duration_ms,
            outcome,
        } => v1::TraceEventKind::CommandExecuted {
            command_id: CommandId::new(command),
            args: to_json(args),
            duration_ms,
            outcome: to_outcome(outcome),
        },
        ArbKind::FieldChanged { field, old, new } => v1::TraceEventKind::FieldChanged {
            field_id: FieldId::new(field),
            old: to_policy(old),
            new: to_policy(new),
        },
        ArbKind::ExceptionThrown {
            exception_type,
            message,
            stack,
        } => v1::TraceEventKind::ExceptionThrown {
            exception_type,
            message,
            stack,
        },
        ArbKind::NavigationOccurred { from, to } => v1::TraceEventKind::NavigationOccurred {
            from: ScreenId::new(from),
            to: ScreenId::new(to),
        },
        ArbKind::ValidationFailed {
            validator,
            field,
            reason,
        } => v1::TraceEventKind::ValidationFailed {
            validator,
            field_id: FieldId::new(field),
            reason,
        },
        ArbKind::AsyncOperationCompleted {
            operation_id,
            duration_ms,
            outcome,
        } => v1::TraceEventKind::AsyncOperationCompleted {
            operation_id,
            duration_ms,
            outcome: to_outcome(outcome),
        },
    }
}

fn to_v1(input: ArbV1Event) -> v1::TraceEvent {
    // Clamp into 1970..≈2100 so chrono accepts every generated value.
    let millis = input.ts_millis.rem_euclid(4_102_444_800_000);
    v1::TraceEvent {
        seq: EventSeq::new(input.seq),
        session_id: SessionId::new(uuid::Uuid::from_u128(input.session_bits)),
        ts: chrono::Utc
            .timestamp_millis_opt(millis)
            .single()
            .expect("clamped range is valid"),
        correlation_id: input
            .correlation_bits
            .map(|bits| CorrelationId::new(uuid::Uuid::from_u128(bits))),
        kind: to_kind(input.kind),
    }
}

fn v1_validator() -> &'static jsonschema::Validator {
    static V: OnceLock<jsonschema::Validator> = OnceLock::new();
    V.get_or_init(|| {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../crates/trace-schema/schema/trace-event-v1.schema.json"
        ))
        .expect("schema file is valid JSON");
        jsonschema::validator_for(&schema).expect("schema file compiles")
    })
}

fn v2_validator() -> &'static jsonschema::Validator {
    static V: OnceLock<jsonschema::Validator> = OnceLock::new();
    V.get_or_init(|| {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../crates/trace-schema/schema/trace-event-v2.schema.json"
        ))
        .expect("schema file is valid JSON");
        jsonschema::validator_for(&schema).expect("schema file compiles")
    })
}

fuzz_target!(|input: ArbV1Event| {
    let event = to_v1(input);
    let current = v2::V1ToV2::upcast(event.clone());

    // Upcast is lossless and fills the v2-only field with None.
    assert_eq!(current.seq, event.seq);
    assert_eq!(current.session_id, event.session_id);
    assert_eq!(current.ts, event.ts);
    assert_eq!(current.correlation_id, event.correlation_id);
    assert!(current.domain_entity_id.is_none());
    assert_eq!(current.kind, event.kind);

    // A serialized v1 envelope validates against the published v1
    // schema and reads back through the chain to the same Current.
    let raw_v1 = serde_json::to_string(&v1::TraceEnvelope::from_event(event))
        .expect("v1 envelope serializes");
    let v1_instance: serde_json::Value =
        serde_json::from_str(&raw_v1).expect("own output is JSON");
    assert!(
        v1_validator().is_valid(&v1_instance),
        "v1 envelope violates the published v1 schema: {raw_v1}"
    );
    let via_chain = read_event(&raw_v1).expect("valid v1 envelope must parse");
    assert_eq!(via_chain, current, "chain result differs from direct upcast");

    // The Current write path re-reads identically and validates
    // against the published v2 schema (fuzzing.md oracle).
    let line = write_event(&current).expect("current serializes");
    let trimmed = line.trim_end_matches('\n');
    let reread = read_event(trimmed).expect("self-written line must parse");
    assert_eq!(reread, current, "write/read round-trip is not stable");
    let v2_instance: serde_json::Value =
        serde_json::from_str(trimmed).expect("own output is JSON");
    assert!(
        v2_validator().is_valid(&v2_instance),
        "Current output violates the published v2 schema: {trimmed}"
    );
});
