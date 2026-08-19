//! Shared `proptest` strategies for the trace-schema integration tests.
//!
//! Extracted from `roundtrip.rs` so every test binary draws from the
//! same generator distribution. Per `docs/upcasters.md` §"Property
//! tests", the strategies for a version become that version's frozen
//! test surface once the version ships in a stable release; until
//! v1.0 they may still grow coverage (wider timestamps, optional
//! fields), never lose it.

#![allow(dead_code)] // each test binary uses a subset of the strategies

use chrono::{DateTime, TimeZone, Utc};
use proptest::prelude::*;
use trace_core::{
    CommandId, CorrelationId, DomainEntityId, EventSeq, FieldId, Outcome, ScreenId, SessionId,
    ValuePolicy,
};
use trace_schema::{v1, v2, Current, Upcaster};

pub(crate) fn arb_session_id() -> impl Strategy<Value = SessionId> {
    any::<u128>().prop_map(|bits| SessionId::new(uuid::Uuid::from_u128(bits)))
}

pub(crate) fn arb_correlation_id() -> impl Strategy<Value = CorrelationId> {
    any::<u128>().prop_map(|bits| CorrelationId::new(uuid::Uuid::from_u128(bits)))
}

pub(crate) fn arb_domain_entity_id() -> impl Strategy<Value = DomainEntityId> {
    "[A-Za-z]{1,10}:[a-z0-9-]{1,12}".prop_map(DomainEntityId::new)
}

/// Timestamps across a ±50-year window with arbitrary sub-second
/// precision, so fractional-second RFC 3339 serialization is exercised
/// (the previous fixed-instant generator never was).
pub(crate) fn arb_ts() -> impl Strategy<Value = DateTime<Utc>> {
    (-1_577_880_000_i64..1_577_880_000, 0_u32..1_000_000_000).prop_map(|(secs, nanos)| {
        Utc.timestamp_opt(1_500_000_000 + secs, nanos)
            .single()
            .expect("generated instant is within chrono's supported range")
    })
}

pub(crate) fn arb_outcome() -> impl Strategy<Value = Outcome> {
    prop_oneof![
        Just(Outcome::Success),
        Just(Outcome::Cancelled),
        Just(Outcome::TimedOut),
        ".{0,32}".prop_map(|message| Outcome::Failure { message }),
    ]
}

pub(crate) fn arb_value_policy() -> impl Strategy<Value = ValuePolicy> {
    prop_oneof![
        Just(ValuePolicy::Removed),
        ".{0,16}".prop_map(|display| ValuePolicy::Masked { display }),
        ".{0,16}".prop_map(|bucket| ValuePolicy::Bucketed { bucket }),
        (".{0,16}", ".{0,8}").prop_map(|(hash, algo)| ValuePolicy::Hashed { hash, algo }),
    ]
}

pub(crate) fn arb_kind() -> impl Strategy<Value = v1::TraceEventKind> {
    prop_oneof![
        ("[A-Za-z]{1,12}".prop_map(ScreenId::new)).prop_map(|screen_id| {
            v1::TraceEventKind::ScreenOpened {
                screen_id,
                params: serde_json::json!({}),
            }
        }),
        (
            "[A-Za-z]{1,12}\\.[A-Za-z]{1,12}".prop_map(CommandId::new),
            0_u64..1_000_000,
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
        (
            "[A-Za-z]{1,32}",
            "[^\"\\\\]{0,32}",
            prop::option::of("[^\"\\\\]{0,64}"),
        )
            .prop_map(|(exception_type, message, stack)| {
                v1::TraceEventKind::ExceptionThrown {
                    exception_type,
                    message,
                    stack,
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
        ("[A-Za-z]{1,12}", 0_u64..1_000_000, arb_outcome(),).prop_map(
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

/// Generate a v1 event (used for upcaster-chain property tests).
pub(crate) fn arb_v1_event() -> impl Strategy<Value = v1::TraceEvent> {
    (
        any::<u64>(),
        arb_session_id(),
        arb_ts(),
        prop::option::of(arb_correlation_id()),
        arb_kind(),
    )
        .prop_map(
            |(seq, session_id, ts, correlation_id, kind)| v1::TraceEvent {
                seq: EventSeq::new(seq),
                session_id,
                ts,
                correlation_id,
                kind,
            },
        )
}

/// Generate a v2 / `Current` event. Unlike the pre-exam generator this
/// produces `Some(domain_entity_id)` roughly half the time, so the one
/// field that distinguishes v2 from v1 sits inside property coverage.
pub(crate) fn arb_current_event() -> impl Strategy<Value = Current> {
    (arb_v1_event(), prop::option::of(arb_domain_entity_id())).prop_map(
        |(event, domain_entity_id)| {
            let mut current = v2::V1ToV2::upcast(event);
            current.domain_entity_id = domain_entity_id;
            current
        },
    )
}
