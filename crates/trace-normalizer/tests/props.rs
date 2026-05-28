//! Property tests for the normalizer (S3 acceptance criteria;
//! `docs/upcasters.md`-style invariant suite).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "test-only index/timestamp arithmetic stays well within range"
)]

use chrono::{Duration, TimeZone, Utc};
use proptest::prelude::*;
use serde_json::Value;
use trace_core::{
    CanonicalAction, CommandId, EventSeq, Outcome, Scenario, ScreenId, Session, SessionId,
};
use trace_normalizer::abstraction::abstract_value;
use trace_normalizer::{normalize, refold, NormCfg};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

// --- strategies -----------------------------------------------------------

fn arb_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| Value::Number(n.into())),
        ".{0,16}".prop_map(Value::String),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            prop::collection::vec(("[a-z]{1,5}", inner), 0..4)
                .prop_map(|kvs| Value::Object(kvs.into_iter().collect())),
        ]
    })
}

fn arb_action() -> impl Strategy<Value = CanonicalAction> {
    (
        "[A-Z][a-z]{0,5}",
        "[A-Z][a-z]{0,5}\\.[A-Z][a-z]{0,5}",
        arb_json(),
    )
        .prop_map(|(screen, command, args)| CanonicalAction {
            screen_id: ScreenId::new(screen),
            command_id: CommandId::new(command),
            abstract_args: args,
        })
}

fn arb_scenario() -> impl Strategy<Value = Scenario> {
    prop::collection::vec(arb_action(), 0..12).prop_map(Scenario::new)
}

fn arb_event() -> impl Strategy<Value = (u64, TraceEventKind)> {
    let kind = prop_oneof![
        "[A-Z][a-z]{0,5}".prop_map(|s| TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new(s),
            params: serde_json::json!({}),
        }),
        ("[A-Z][a-z]{0,5}\\.[A-Z][a-z]{0,5}", arb_json()).prop_map(|(c, args)| {
            TraceEventKind::CommandExecuted {
                command_id: CommandId::new(c),
                args,
                duration_ms: 1,
                outcome: Outcome::Success,
            }
        }),
        ("[A-Z][a-z]{0,5}", "[A-Z][a-z]{0,5}").prop_map(|(a, b)| {
            TraceEventKind::NavigationOccurred {
                from: ScreenId::new(a),
                to: ScreenId::new(b),
            }
        }),
    ];
    (0u64..1000, kind)
}

fn arb_session() -> impl Strategy<Value = Session<Current>> {
    prop::collection::vec(arb_event(), 1..20).prop_map(|events| {
        let sid = SessionId::new(uuid::Uuid::from_u128(1));
        let base = Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap();
        let events = events
            .into_iter()
            .enumerate()
            .map(|(i, (ms, kind))| Current {
                seq: EventSeq::new(i as u64),
                session_id: sid,
                // Deterministic but varied spacing so bursts / pauses occur.
                ts: base + Duration::milliseconds(i as i64 * 30 + ms as i64),
                correlation_id: None,
                kind,
            })
            .collect();
        Session::new(sid, events).expect("non-empty")
    })
}

// --- invariants -----------------------------------------------------------

proptest! {
    /// abstract_value(abstract_value(v)) == abstract_value(v)
    #[test]
    fn value_abstraction_is_idempotent(v in arb_json()) {
        let cfg = NormCfg::default();
        let once = abstract_value(&v, &cfg);
        let twice = abstract_value(&once, &cfg);
        prop_assert_eq!(once, twice);
    }

    /// refold(refold(s)) == refold(s)
    #[test]
    fn refold_is_idempotent(s in arb_scenario()) {
        let cfg = NormCfg::default();
        let once = refold(&s, &cfg);
        let twice = refold(&once, &cfg);
        prop_assert_eq!(once, twice);
    }

    /// normalize is deterministic: same (session, cfg) -> same output.
    #[test]
    fn normalize_is_deterministic(s in arb_session()) {
        let cfg = NormCfg::default();
        let a = normalize(&s, &cfg);
        let b = normalize(&s, &cfg);
        prop_assert_eq!(a, b);
    }

    /// Every action a normalize produces has fully-abstracted args
    /// (re-abstracting them is a no-op), so the scenario is stable under
    /// re-abstraction.
    #[test]
    fn normalized_args_are_fully_abstracted(s in arb_session()) {
        let cfg = NormCfg::default();
        let (scenario, _report) = normalize(&s, &cfg);
        for action in &scenario.actions {
            let re = abstract_value(&action.abstract_args, &cfg);
            prop_assert_eq!(&re, &action.abstract_args);
        }
    }
}
