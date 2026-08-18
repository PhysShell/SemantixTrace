//! Metamorphic and conservation laws for the normalizer (S3 exam).
//!
//! The existing property suite pins idempotence and structural
//! determinism; this suite pins the laws that make normalization
//! *trustworthy*:
//!
//! - **Event conservation**: every input event is accounted for —
//!   `input_events == output_actions + collapsed_bursts +
//!   dropped_noise`. This is cardinality conservation over events,
//!   not information conservation: the canonical action deliberately
//!   projects away `outcome` and `duration_ms` (uniform and
//!   documented on `FoldReport`; machine-readable accounting for that
//!   projection is tracked as issue #16, pre-v1.0).
//! - **Byte determinism**: same `(session, cfg)` produces the same
//!   serialized bytes, not merely `PartialEq`-equal structures.
//! - **Constant time-shift invariance**: temporal abstraction is
//!   delta-based, so shifting every timestamp by the same Δ changes
//!   nothing.
//! - **Order preservation**: canonical actions are a subsequence of
//!   the naive projection, in source order; with the burst window
//!   disabled the output *is* the projection — out-of-order
//!   timestamps never smuggle in a sort.
//! - **Equivalence-class invariance**: mutating a raw value within its
//!   abstraction class (same string class + length bucket, same
//!   numeric bucket) leaves the canonical scenario byte-identical —
//!   the positive proof that raw values do not leak into the output.

use chrono::{DateTime, Duration, TimeZone, Utc};
use proptest::prelude::*;
use serde_json::Value;
use trace_core::FieldId;
use trace_core::{
    CanonicalAction, CommandId, EventSeq, Outcome, ScreenId, Session, SessionId, ValuePolicy,
};
use trace_normalizer::abstraction::abstract_value;
use trace_normalizer::{normalize, NormCfg};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

// ---------------------------------------------------------------------------
// Strategies: richer than props.rs — all 7 kinds (noise included),
// class-stable string/numeric leaves, and non-monotonic timestamps.
// ---------------------------------------------------------------------------

/// Argument leaves whose abstraction class is easy to mutate within:
/// lowercase words (class `free`), digit strings (class `numeric`),
/// e-mail addresses (class `email`), and integers.
fn arb_leaf() -> impl Strategy<Value = Value> {
    prop_oneof![
        "[a-z]{1,20}".prop_map(Value::String),
        "[0-9]{1,12}".prop_map(Value::String),
        ("[a-z]{1,8}", "[a-z]{1,8}").prop_map(|(l, d)| Value::String(format!("{l}@{d}.io"))),
        any::<i32>().prop_map(|n| Value::Number(i64::from(n).into())),
        any::<bool>().prop_map(Value::Bool),
        Just(Value::Null),
    ]
}

fn arb_args() -> impl Strategy<Value = Value> {
    prop::collection::btree_map("[a-z]{1,6}", arb_leaf(), 0..4)
        .prop_map(|m| Value::Object(m.into_iter().collect()))
}

fn arb_kind() -> impl Strategy<Value = TraceEventKind> {
    prop_oneof![
        3 => ("[A-Z][a-z]{0,6}\\.[A-Z][a-z]{0,6}", arb_args()).prop_map(|(c, args)| {
            TraceEventKind::CommandExecuted {
                command_id: CommandId::new(c),
                args,
                duration_ms: 1,
                outcome: Outcome::Success,
            }
        }),
        1 => "[A-Z][a-z]{0,6}".prop_map(|s| TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new(s),
            params: serde_json::json!({}),
        }),
        1 => ("[A-Z][a-z]{0,6}", "[A-Z][a-z]{0,6}").prop_map(|(a, b)| {
            TraceEventKind::NavigationOccurred {
                from: ScreenId::new(a),
                to: ScreenId::new(b),
            }
        }),
        1 => "[A-Z][a-z]{0,6}".prop_map(|f| TraceEventKind::FieldChanged {
            field_id: FieldId::new(f),
            old: ValuePolicy::default_masked(),
            new: ValuePolicy::Removed,
        }),
        1 => ("[A-Z][a-z]{0,8}", "[a-z ]{0,16}").prop_map(|(t, m)| {
            TraceEventKind::ExceptionThrown {
                exception_type: t,
                message: m,
                stack: None,
            }
        }),
        1 => ("[A-Z][a-z]{0,6}", "[A-Z][a-z]{0,6}", "[a-z ]{0,12}").prop_map(|(v, f, r)| {
            TraceEventKind::ValidationFailed {
                validator: v,
                field_id: FieldId::new(f),
                reason: r,
            }
        }),
        1 => ("[a-z0-9-]{1,10}", 0_u64..10_000).prop_map(|(op, d)| {
            TraceEventKind::AsyncOperationCompleted {
                operation_id: op,
                duration_ms: d,
                outcome: Outcome::TimedOut,
            }
        }),
    ]
}

fn base_ts() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap()
}

/// Sessions with deliberately non-monotonic timestamps: each event gets
/// an independent offset in a window that produces bursts (< 50 ms),
/// idle gaps (> 5 s), and out-of-order neighbours.
fn arb_session() -> impl Strategy<Value = Session<Current>> {
    prop::collection::vec((arb_kind(), 0_i64..20_000), 1..24).prop_map(|events| {
        let sid = SessionId::new(uuid::Uuid::from_u128(3));
        let events = events
            .into_iter()
            .enumerate()
            .map(|(i, (kind, offset_ms))| Current {
                seq: EventSeq::new(u64::try_from(i).expect("small index")),
                session_id: sid,
                ts: base_ts() + Duration::milliseconds(offset_ms),
                correlation_id: None,
                domain_entity_id: None,
                kind,
            })
            .collect();
        Session::new(sid, events).expect("non-empty")
    })
}

fn shift_session(session: &Session<Current>, delta_ms: i64) -> Session<Current> {
    let events = session
        .events()
        .iter()
        .cloned()
        .map(|mut e| {
            e.ts += Duration::milliseconds(delta_ms);
            e
        })
        .collect();
    Session::new(*session.id(), events).expect("non-empty")
}

// ---------------------------------------------------------------------------
// Class-stable mutation: rewrites every raw leaf to a *different* value
// in the *same* abstraction class (same string class, same length
// bucket, same numeric bucket).
// ---------------------------------------------------------------------------

fn rotate_char(c: char) -> char {
    match c {
        'a'..='y' | '0'..='8' => char::from(c as u8 + 1),
        'z' => 'a',
        '9' => '0',
        other => other,
    }
}

fn mutate_leaf(value: &Value, cfg: &NormCfg) -> Value {
    match value {
        Value::String(s) => Value::String(s.chars().map(rotate_char).collect()),
        Value::Number(n) => n.as_i64().map_or_else(
            || value.clone(),
            |v| {
                let candidate = v.saturating_add(1);
                if cfg.bucket_label(candidate) == cfg.bucket_label(v) {
                    Value::Number(candidate.into())
                } else {
                    value.clone()
                }
            },
        ),
        Value::Array(items) => Value::Array(items.iter().map(|v| mutate_leaf(v, cfg)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), mutate_leaf(v, cfg)))
                .collect(),
        ),
        Value::Bool(_) | Value::Null => value.clone(),
    }
}

fn mutate_session_values(session: &Session<Current>, cfg: &NormCfg) -> Session<Current> {
    let events = session
        .events()
        .iter()
        .cloned()
        .map(|mut e| {
            if let TraceEventKind::CommandExecuted { args, .. } = &mut e.kind {
                *args = mutate_leaf(args, cfg);
            }
            e
        })
        .collect();
    Session::new(*session.id(), events).expect("non-empty")
}

/// Naive projection oracle: what the fold would produce with no burst
/// collapsing at all (screen tracking included).
fn full_projection(session: &Session<Current>, cfg: &NormCfg) -> Vec<CanonicalAction> {
    let mut screen = ScreenId::new("<unknown>");
    let mut out = Vec::new();
    for event in session.events() {
        match &event.kind {
            TraceEventKind::ScreenOpened { screen_id, .. } => screen = screen_id.clone(),
            TraceEventKind::NavigationOccurred { to, .. } => screen = to.clone(),
            TraceEventKind::CommandExecuted {
                command_id, args, ..
            } => out.push(CanonicalAction {
                screen_id: screen.clone(),
                command_id: command_id.clone(),
                abstract_args: abstract_value(args, cfg),
            }),
            _ => {}
        }
    }
    out
}

fn is_subsequence(needle: &[CanonicalAction], haystack: &[CanonicalAction]) -> bool {
    let mut it = haystack.iter();
    needle.iter().all(|a| it.any(|b| b == a))
}

// ---------------------------------------------------------------------------
// Laws.
// ---------------------------------------------------------------------------

proptest! {
    /// Event conservation: every input event is either a canonical
    /// action, a collapsed burst, or named noise. No fourth bucket —
    /// no event vanishes uncounted.
    #[test]
    fn every_input_event_is_accounted_for(s in arb_session()) {
        let (_scenario, report) = normalize(&s, &NormCfg::default());
        prop_assert_eq!(report.input_events, s.len());
        prop_assert_eq!(
            report.input_events,
            report.output_actions + report.collapsed_bursts + report.dropped_noise,
            "conservation violated: {:?}", report
        );
    }

    /// Byte determinism: the *serialized* scenario and report are
    /// identical across runs, not merely structurally equal.
    #[test]
    fn normalization_is_byte_deterministic(s in arb_session()) {
        let cfg = NormCfg::default();
        let a = normalize(&s, &cfg);
        let b = normalize(&s, &cfg);
        let bytes_a = serde_json::to_string(&a).expect("serialize");
        let bytes_b = serde_json::to_string(&b).expect("serialize");
        prop_assert_eq!(bytes_a, bytes_b);
    }

    /// Shifting every timestamp by the same Δ (here ±10 years) changes
    /// neither the scenario nor the fold report: temporal abstraction
    /// is purely delta-based.
    #[test]
    fn constant_time_shift_is_invariant(s in arb_session(), delta_ms in -315_360_000_000_i64..315_360_000_000) {
        let cfg = NormCfg::default();
        let original = normalize(&s, &cfg);
        let shifted = normalize(&shift_session(&s, delta_ms), &cfg);
        prop_assert_eq!(original, shifted);
    }

    /// With the burst window disabled the fold is exactly the naive
    /// projection — same actions, same source order, regardless of
    /// timestamp disorder. A sort anywhere in the pipeline fails this.
    #[test]
    fn zero_burst_gap_yields_full_projection_in_source_order(s in arb_session()) {
        let cfg = NormCfg { burst_gap_ms: 0, ..NormCfg::default() };
        let (scenario, report) = normalize(&s, &cfg);
        let expected = full_projection(&s, &cfg);
        prop_assert_eq!(report.collapsed_bursts, 0);
        prop_assert_eq!(scenario.actions, expected);
    }

    /// Under any burst window the output is a subsequence of the naive
    /// projection: collapse may only *remove* actions, never reorder,
    /// rewrite, or invent them.
    #[test]
    fn collapse_only_removes_never_reorders(s in arb_session()) {
        let cfg = NormCfg::default();
        let (scenario, _report) = normalize(&s, &cfg);
        let projection = full_projection(&s, &cfg);
        prop_assert!(
            is_subsequence(&scenario.actions, &projection),
            "output is not an in-order subsequence of the projection"
        );
    }

    /// Equivalence-class invariance (and the no-raw-leak proof):
    /// rewriting every raw argument leaf to a different value in the
    /// same abstraction class leaves the canonical output
    /// byte-identical.
    #[test]
    fn class_stable_value_mutation_is_invariant(s in arb_session()) {
        let cfg = NormCfg::default();
        let mutated = mutate_session_values(&s, &cfg);
        let original = normalize(&s, &cfg);
        let after = normalize(&mutated, &cfg);
        let bytes_original = serde_json::to_string(&original).expect("serialize");
        let bytes_after = serde_json::to_string(&after).expect("serialize");
        prop_assert_eq!(bytes_original, bytes_after);
    }

    /// `bucket_label` is total for any configured bounds table —
    /// custom, unsorted, or empty — and for the full i64 domain.
    #[test]
    fn bucket_label_is_total_for_any_bounds(
        bounds in prop::collection::vec(-1_000_i64..1_000, 0..6),
        value in any::<i64>(),
    ) {
        let cfg = NormCfg { numeric_bucket_bounds: bounds, ..NormCfg::default() };
        let label = cfg.bucket_label(value);
        prop_assert!(!label.is_empty());
        // Deterministic for the same inputs.
        prop_assert_eq!(label, cfg.bucket_label(value));
    }
}

/// Characterization: burst adjacency is read in the *canonical action
/// stream*, not the raw event stream — noise events (an exception,
/// say) between two identical commands inside the burst window do NOT
/// break the burst. Both commands still land inside the window and
/// the intervening event is separately accounted as dropped noise, so
/// conservation holds. Pinned so a change to this semantic is a
/// conscious decision with a failing test, not a drive-by.
#[test]
fn burst_collapse_spans_intervening_noise_events() {
    let sid = SessionId::new(uuid::Uuid::from_u128(5));
    let cmd = |seq: u64, ms: i64| Current {
        seq: EventSeq::new(seq),
        session_id: sid,
        ts: base_ts() + Duration::milliseconds(ms),
        correlation_id: None,
        domain_entity_id: None,
        kind: TraceEventKind::CommandExecuted {
            command_id: CommandId::new("X.Do"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
    };
    let noise = Current {
        seq: EventSeq::new(1),
        session_id: sid,
        ts: base_ts() + Duration::milliseconds(10),
        correlation_id: None,
        domain_entity_id: None,
        kind: TraceEventKind::ExceptionThrown {
            exception_type: "Interrupt".into(),
            message: "between the burst".into(),
            stack: None,
        },
    };
    let session = Session::new(sid, vec![cmd(0, 0), noise, cmd(2, 20)]).expect("non-empty");

    let (scenario, report) = normalize(&session, &NormCfg::default());
    assert_eq!(scenario.actions.len(), 1, "noise does not break a burst");
    assert_eq!(report.collapsed_bursts, 1);
    assert_eq!(report.dropped_noise, 1);
    assert_eq!(
        report.input_events,
        report.output_actions + report.collapsed_bursts + report.dropped_noise
    );
}

/// S3 acceptance criterion: "All property invariants hold across
/// 10 000 generated sessions." Ignored by default (the nightly
/// workflow and release gates run it): execute with
/// `cargo test -p trace-normalizer --test metamorphic -- --ignored`.
#[test]
#[ignore = "10k-case acceptance run; executed nightly and on demand"]
fn ten_thousand_sessions_uphold_core_invariants() {
    use proptest::test_runner::{Config, TestRunner};

    let mut runner = TestRunner::new(Config {
        cases: 10_000,
        ..Config::default()
    });
    let cfg = NormCfg::default();
    runner
        .run(&arb_session(), |s| {
            let (scenario, report) = normalize(&s, &cfg);
            prop_assert_eq!(report.input_events, s.len());
            prop_assert_eq!(
                report.input_events,
                report.output_actions + report.collapsed_bursts + report.dropped_noise
            );
            prop_assert_eq!(&(scenario.clone(), report), &normalize(&s, &cfg));
            for action in &scenario.actions {
                prop_assert_eq!(
                    &abstract_value(&action.abstract_args, &cfg),
                    &action.abstract_args
                );
            }
            let projection = full_projection(&s, &cfg);
            prop_assert!(is_subsequence(&scenario.actions, &projection));
            Ok(())
        })
        .expect("10k-session acceptance run");
}
