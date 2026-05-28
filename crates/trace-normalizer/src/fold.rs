//! Scenario folding: project a [`Session`] of events into a normalized
//! [`Scenario`], plus the [`FoldReport`] describing what was abstracted.

use chrono::{DateTime, Utc};
use serde::Serialize;
use trace_core::{CanonicalAction, Scenario, ScreenId, Session};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

use crate::abstraction::abstract_value;
use crate::config::NormCfg;

/// Sidecar describing what the fold abstracted away (`glossary.md` §3,
/// "scenario folding"). Surfaced by `trace normalize --report`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FoldReport {
    /// Events read from the session.
    pub input_events: usize,
    /// Canonical actions in the resulting scenario.
    pub output_actions: usize,
    /// Consecutive identical actions collapsed within the burst gap.
    pub collapsed_bursts: usize,
    /// Events that produced no canonical action (screen opens,
    /// navigations, field changes, exceptions, validations, async ops).
    pub dropped_noise: usize,
    /// Inter-event idle gaps exceeding `NormCfg::idle_gap_ms`.
    pub session_pauses: usize,
}

const UNKNOWN_SCREEN: &str = "<unknown>";

/// Fold a raw session into a normalized scenario.
///
/// `CommandExecuted` events become canonical `(screen, command,
/// abstract_args)` actions; the current screen is tracked through
/// `ScreenOpened` / `NavigationOccurred`. Consecutive identical actions
/// within `cfg.burst_gap_ms` collapse into one. Deterministic: the same
/// `(session, cfg)` always yields the same output.
#[must_use]
pub fn normalize(session: &Session<Current>, cfg: &NormCfg) -> (Scenario, FoldReport) {
    let mut current_screen = ScreenId::new(UNKNOWN_SCREEN);
    let mut actions: Vec<CanonicalAction> = Vec::new();
    let mut report = FoldReport::default();
    let mut last_ts: Option<DateTime<Utc>> = None;
    let mut last_action_ts: Option<DateTime<Utc>> = None;

    for event in session.events() {
        report.input_events += 1;

        if let Some(prev) = last_ts {
            if (event.ts - prev).num_milliseconds() > cfg.idle_gap_ms {
                report.session_pauses += 1;
            }
        }
        last_ts = Some(event.ts);

        match &event.kind {
            TraceEventKind::ScreenOpened { screen_id, .. } => {
                current_screen = screen_id.clone();
                report.dropped_noise += 1;
            }
            TraceEventKind::NavigationOccurred { to, .. } => {
                current_screen = to.clone();
                report.dropped_noise += 1;
            }
            TraceEventKind::CommandExecuted {
                command_id, args, ..
            } => {
                let action = CanonicalAction {
                    screen_id: current_screen.clone(),
                    command_id: command_id.clone(),
                    abstract_args: abstract_value(args, cfg),
                };
                let is_burst = actions.last() == Some(&action)
                    && last_action_ts.is_some_and(|t| {
                        let delta = (event.ts - t).num_milliseconds();
                        // Negative delta means out-of-order timestamp; treat as non-burst.
                        delta >= 0 && delta < cfg.burst_gap_ms
                    });
                if is_burst {
                    report.collapsed_bursts += 1;
                } else {
                    actions.push(action);
                }
                last_action_ts = Some(event.ts);
            }
            TraceEventKind::FieldChanged { .. }
            | TraceEventKind::ExceptionThrown { .. }
            | TraceEventKind::ValidationFailed { .. }
            | TraceEventKind::AsyncOperationCompleted { .. } => {
                report.dropped_noise += 1;
            }
        }
    }

    report.output_actions = actions.len();
    (Scenario::new(actions), report)
}

/// Re-normalize a scenario: re-abstract each action's args (idempotent)
/// and collapse adjacent identical actions.
///
/// Endomorphic and idempotent on [`Scenario`]: `refold(refold(s)) ==
/// refold(s)`. Unlike [`normalize`], this has no timestamps, so it
/// collapses *all* adjacent duplicates rather than only burst-window
/// ones.
#[must_use]
pub fn refold(scenario: &Scenario, cfg: &NormCfg) -> Scenario {
    let mut out: Vec<CanonicalAction> = Vec::with_capacity(scenario.actions.len());
    for action in &scenario.actions {
        let reabstracted = CanonicalAction {
            screen_id: action.screen_id.clone(),
            command_id: action.command_id.clone(),
            abstract_args: abstract_value(&action.abstract_args, cfg),
        };
        if out.last() != Some(&reabstracted) {
            out.push(reabstracted);
        }
    }
    Scenario::new(out)
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use trace_core::{CommandId, EventSeq, Outcome, ScreenId, Session, SessionId};
    use trace_schema::v1::TraceEventKind;
    use trace_schema::Current;

    use super::{normalize, refold};
    use crate::config::NormCfg;

    fn at(secs: i64, millis: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap()
            + chrono::Duration::seconds(secs)
            + chrono::Duration::milliseconds(i64::from(millis))
    }

    const fn ev(seq: u64, ts: chrono::DateTime<Utc>, kind: TraceEventKind) -> Current {
        Current {
            seq: EventSeq::new(seq),
            session_id: SessionId::new(uuid::Uuid::from_u128(1)),
            ts,
            correlation_id: None,
            kind,
        }
    }

    fn cmd(name: &str) -> TraceEventKind {
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new(name),
            args: serde_json::json!({"qty": 50}),
            duration_ms: 1,
            outcome: Outcome::Success,
        }
    }

    #[test]
    fn tracks_screen_and_projects_commands() {
        let session = Session::new(
            SessionId::new(uuid::Uuid::from_u128(1)),
            vec![
                ev(
                    0,
                    at(0, 0),
                    TraceEventKind::ScreenOpened {
                        screen_id: ScreenId::new("Editor"),
                        params: serde_json::json!({}),
                    },
                ),
                ev(1, at(1, 0), cmd("Graph47.Recalculate")),
            ],
        )
        .unwrap();

        let (scenario, report) = normalize(&session, &NormCfg::default());
        assert_eq!(scenario.actions.len(), 1);
        assert_eq!(scenario.actions[0].screen_id, ScreenId::new("Editor"));
        assert_eq!(
            scenario.actions[0].command_id,
            CommandId::new("Graph47.Recalculate")
        );
        // qty:50 abstracted to a bucket, not the raw value.
        assert_eq!(scenario.actions[0].abstract_args["qty"]["bucket"], "11-100");
        assert_eq!(report.input_events, 2);
        assert_eq!(report.output_actions, 1);
        assert_eq!(report.dropped_noise, 1);
    }

    #[test]
    fn collapses_burst_but_not_spaced_repeats() {
        let cfg = NormCfg::default();
        // Three identical commands: two within 50ms, one 1s later.
        let session = Session::new(
            SessionId::new(uuid::Uuid::from_u128(1)),
            vec![
                ev(0, at(0, 0), cmd("X.Do")),
                ev(1, at(0, 10), cmd("X.Do")), // burst -> collapsed
                ev(2, at(2, 0), cmd("X.Do")),  // spaced -> kept
            ],
        )
        .unwrap();

        let (scenario, report) = normalize(&session, &cfg);
        assert_eq!(scenario.actions.len(), 2);
        assert_eq!(report.collapsed_bursts, 1);
    }

    #[test]
    fn out_of_order_timestamps_not_collapsed_as_burst() {
        // An event with an earlier timestamp than the previous one must NOT
        // be collapsed: negative delta < burst_gap_ms must not trigger burst logic.
        let cfg = NormCfg::default();
        let session = Session::new(
            SessionId::new(uuid::Uuid::from_u128(1)),
            vec![
                ev(0, at(10, 0), cmd("X.Do")),
                ev(1, at(0, 0), cmd("X.Do")), // out-of-order: earlier timestamp
            ],
        )
        .unwrap();

        let (scenario, report) = normalize(&session, &cfg);
        assert_eq!(
            scenario.actions.len(),
            2,
            "out-of-order repeat must not be collapsed"
        );
        assert_eq!(report.collapsed_bursts, 0);
    }

    #[test]
    fn counts_session_pause() {
        let session = Session::new(
            SessionId::new(uuid::Uuid::from_u128(1)),
            vec![
                ev(0, at(0, 0), cmd("A.Do")),
                ev(1, at(10, 0), cmd("B.Do")), // 10s gap > 5s idle
            ],
        )
        .unwrap();
        let (_s, report) = normalize(&session, &NormCfg::default());
        assert_eq!(report.session_pauses, 1);
    }

    #[test]
    fn refold_collapses_adjacent_duplicates_idempotently() {
        let cfg = NormCfg::default();
        let session = Session::new(
            SessionId::new(uuid::Uuid::from_u128(1)),
            vec![
                ev(0, at(0, 0), cmd("X.Do")),
                ev(1, at(2, 0), cmd("X.Do")), // spaced repeat kept by normalize
            ],
        )
        .unwrap();
        let (scenario, _r) = normalize(&session, &cfg);
        assert_eq!(scenario.actions.len(), 2);

        let once = refold(&scenario, &cfg);
        assert_eq!(once.actions.len(), 1); // adjacent dup collapsed
        let twice = refold(&once, &cfg);
        assert_eq!(once, twice); // idempotent
    }
}
