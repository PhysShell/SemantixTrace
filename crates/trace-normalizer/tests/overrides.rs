//! Per-field bucket overrides and accounted refolding (S3 residuals).
//!
//! The S3 stage doc lists "per-field policy overrides" among the
//! `NormCfg` knobs; the open question deferred them to S6, S6 shipped
//! without them, and the repo is past S8 — so they land here, red
//! first. Second residual: `refold` collapses adjacent duplicates and
//! reports the loss nowhere, which violates the loss-accounting
//! posture the FoldReport exists for; `refold_with_report` closes
//! that hole.

use chrono::{Duration, TimeZone, Utc};
use proptest::prelude::*;
use trace_core::{CanonicalAction, CommandId, EventSeq, Outcome, ScreenId, Session, SessionId};
use trace_normalizer::{normalize, refold, refold_with_report, NormCfg};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

fn cmd_event(seq: u64, ms: i64, args: serde_json::Value) -> Current {
    Current {
        seq: EventSeq::new(seq),
        session_id: SessionId::new(uuid::Uuid::from_u128(9)),
        ts: Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap() + Duration::milliseconds(ms),
        correlation_id: None,
        domain_entity_id: None,
        kind: TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Graph47.Recalculate"),
            args,
            duration_ms: 1,
            outcome: Outcome::Success,
        },
    }
}

fn session_of(events: Vec<Current>) -> Session<Current> {
    Session::new(SessionId::new(uuid::Uuid::from_u128(9)), events).expect("non-empty")
}

// ---------------------------------------------------------------------------
// Per-field bucket overrides.
// ---------------------------------------------------------------------------

/// An override for field `qty` applies that field's own bucket table…
#[test]
fn override_applies_to_the_named_field() {
    let mut cfg = NormCfg::default();
    cfg.per_field_bucket_bounds
        .insert("qty".to_owned(), vec![5]);

    let session = session_of(vec![cmd_event(
        0,
        0,
        serde_json::json!({"qty": 7, "other": 7}),
    )]);
    let (scenario, _report) = normalize(&session, &cfg);

    let args = &scenario.actions[0].abstract_args;
    // qty falls under the override table (bounds [5] → "6+" for 7)…
    assert_eq!(args["qty"]["bucket"], serde_json::json!("6+"));
    // …while a sibling field keeps the global table ("2-10" for 7).
    assert_eq!(args["other"]["bucket"], serde_json::json!("2-10"));
}

/// …and the override reaches numeric leaves nested under the named
/// field (arrays and sub-objects), because the field is the unit the
/// policy is declared for.
#[test]
fn override_propagates_into_nested_values_under_the_field() {
    let mut cfg = NormCfg::default();
    cfg.per_field_bucket_bounds
        .insert("qty".to_owned(), vec![5]);

    let session = session_of(vec![cmd_event(
        0,
        0,
        serde_json::json!({"qty": {"amount": 7, "batch": [7]}}),
    )]);
    let (scenario, _report) = normalize(&session, &cfg);

    let qty = &scenario.actions[0].abstract_args["qty"];
    assert_eq!(qty["amount"]["bucket"], serde_json::json!("6+"));
    assert_eq!(qty["batch"][0]["bucket"], serde_json::json!("6+"));
}

proptest! {
    /// Locality law: an override for a key that does not occur in the
    /// arguments changes nothing at all.
    #[test]
    fn override_for_absent_field_is_a_no_op(qty in 0_i64..10_000, other in 0_i64..10_000) {
        let baseline = NormCfg::default();
        let mut cfg = NormCfg::default();
        cfg.per_field_bucket_bounds.insert("absent".to_owned(), vec![1, 2, 3]);

        let session = session_of(vec![cmd_event(0, 0, serde_json::json!({"qty": qty, "other": other}))]);
        prop_assert_eq!(normalize(&session, &baseline), normalize(&session, &cfg));
    }

    /// Locality law: overriding field F changes at most F's subtree —
    /// every sibling key's abstraction is byte-identical to the
    /// baseline.
    #[test]
    fn override_touches_only_the_named_field(qty in 0_i64..10_000, other in 0_i64..10_000) {
        let baseline = NormCfg::default();
        let mut cfg = NormCfg::default();
        cfg.per_field_bucket_bounds.insert("qty".to_owned(), vec![0, 50, 5_000]);

        let session = session_of(vec![cmd_event(0, 0, serde_json::json!({"qty": qty, "other": other}))]);
        let (base_scenario, _) = normalize(&session, &baseline);
        let (over_scenario, _) = normalize(&session, &cfg);

        prop_assert_eq!(
            &base_scenario.actions[0].abstract_args["other"],
            &over_scenario.actions[0].abstract_args["other"]
        );
    }

    /// Overridden abstraction stays idempotent: re-normalizing the
    /// produced scenario's args is a no-op under the same cfg.
    #[test]
    fn override_output_remains_a_fixed_point(qty in 0_i64..10_000) {
        let mut cfg = NormCfg::default();
        cfg.per_field_bucket_bounds.insert("qty".to_owned(), vec![5]);

        let session = session_of(vec![cmd_event(0, 0, serde_json::json!({"qty": qty}))]);
        let (scenario, _) = normalize(&session, &cfg);
        for action in &scenario.actions {
            let re = trace_normalizer::abstraction::abstract_value(&action.abstract_args, &cfg);
            prop_assert_eq!(&re, &action.abstract_args);
        }
    }
}

// ---------------------------------------------------------------------------
// Accounted refolding.
// ---------------------------------------------------------------------------

fn action(name: &str) -> CanonicalAction {
    CanonicalAction {
        screen_id: ScreenId::new("Editor"),
        command_id: CommandId::new(name),
        abstract_args: serde_json::json!({}),
    }
}

/// `refold_with_report` names its loss: input minus output equals the
/// collapsed count, and the scenario matches the unaccounted `refold`.
#[test]
fn refold_with_report_accounts_for_every_collapsed_action() {
    let cfg = NormCfg::default();
    let scenario = trace_core::Scenario::new(vec![
        action("X.Do"),
        action("X.Do"),
        action("Y.Do"),
        action("Y.Do"),
        action("Y.Do"),
        action("X.Do"),
    ]);

    let (folded, report) = refold_with_report(&scenario, &cfg);
    assert_eq!(folded, refold(&scenario, &cfg));
    assert_eq!(report.input_actions, 6);
    assert_eq!(report.output_actions, 3);
    assert_eq!(report.collapsed_adjacent, 3);
}

proptest! {
    /// Conservation law for refolding, over arbitrary scenarios
    /// produced by the real pipeline.
    #[test]
    fn refold_report_conserves_actions(names in prop::collection::vec("[A-C]", 1..20)) {
        let cfg = NormCfg::default();
        let scenario = trace_core::Scenario::new(
            names.iter().map(|n| action(n)).collect::<Vec<_>>(),
        );
        let (folded, report) = refold_with_report(&scenario, &cfg);
        prop_assert_eq!(report.input_actions, scenario.actions.len());
        prop_assert_eq!(report.output_actions, folded.actions.len());
        prop_assert_eq!(
            report.input_actions,
            report.output_actions + report.collapsed_adjacent,
            "refold loss must be fully accounted"
        );
        prop_assert_eq!(folded, refold(&scenario, &cfg));
    }
}
