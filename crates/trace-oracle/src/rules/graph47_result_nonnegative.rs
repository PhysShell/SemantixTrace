//! [`Graph47ResultMustBeNonNegative`] — example domain oracle rule.
//!
//! A user-defined rule, included here as a reference implementation that
//! demonstrates how to write application-specific oracle rules beyond the
//! five built-in ones.  Ships as part of the S7 demo pipeline.
//!
//! **Bug it catches**: `PaymentsCalculator` rounds quantities > 1 000 down
//! by one unit due to integer division, producing a negative duty base.
//! The resulting `NegativePaymentException` appears in the trace immediately
//! after a *successful* `Graph47.Recalculate` — a contradiction the oracle
//! exploits: the command reported success, but a domain invariant was
//! violated.

use trace_core::Session;
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

use crate::rule::{OracleResult, OracleSchedule, OracleViolation, Rule, Severity};

/// Fails if `Graph47.Recalculate` completes successfully but is immediately
/// followed by an `ExceptionThrown` whose `exception_type` contains
/// `"Negative"` or `"NonNegative"`.
///
/// The rule operates on consecutive event pairs so it is cheap and needs no
/// per-session state beyond the window iteration.
#[derive(Clone, Copy, Debug, Default)]
pub struct Graph47ResultMustBeNonNegative;

impl Rule for Graph47ResultMustBeNonNegative {
    fn name(&self) -> &'static str {
        "Graph47.ResultMustBeNonNegative"
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::PerScenario
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let events = session.events();
        let mut violations = Vec::new();
        for window in events.windows(2) {
            let prev = &window[0];
            let next = &window[1];
            if let TraceEventKind::CommandExecuted {
                command_id,
                outcome,
                ..
            } = &prev.kind
            {
                if command_id.as_str() == "Graph47.Recalculate" && outcome.is_success() {
                    if let TraceEventKind::ExceptionThrown { exception_type, .. } = &next.kind {
                        if exception_type.contains("Negative")
                            || exception_type.contains("NonNegative")
                        {
                            violations.push(OracleViolation {
                                rule: self.name().to_owned(),
                                severity: Severity::Error,
                                message: format!(
                                    "Graph47.Recalculate succeeded (seq {}) but produced \
                                     {exception_type} (seq {}) — off-by-one in quantity \
                                     truncation when total > 1 000",
                                    prev.seq, next.seq
                                ),
                                evidence: vec![prev.seq, next.seq],
                            });
                        }
                    }
                }
            }
        }
        OracleResult::fail(self.name(), violations)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use trace_core::{CommandId, EventSeq, Session, SessionId};
    use trace_schema::v1::TraceEventKind;
    use trace_schema::v2::V1ToV2;
    use trace_schema::Current;
    use trace_schema::Upcaster;

    use super::Graph47ResultMustBeNonNegative;
    use crate::rule::{Rule, Severity};

    fn make_event(seq: u64, kind: TraceEventKind) -> Current {
        V1ToV2::upcast(trace_schema::v1::TraceEvent {
            seq: EventSeq::new(seq),
            session_id: SessionId::new(uuid::Uuid::nil()),
            ts: chrono::Utc.with_ymd_and_hms(2026, 5, 31, 9, 0, 0).unwrap(),
            correlation_id: None,
            kind,
        })
    }

    #[test]
    fn fires_when_recalculate_followed_by_negative_exception() {
        use trace_core::Outcome;
        let events = vec![
            make_event(
                0,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Graph47.Recalculate"),
                    args: serde_json::json!({}),
                    duration_ms: 200,
                    outcome: Outcome::Success,
                },
            ),
            make_event(
                1,
                TraceEventKind::ExceptionThrown {
                    exception_type: "NegativePaymentException".into(),
                    message: "***".into(),
                    stack: None,
                },
            ),
        ];
        let session = Session::new(SessionId::new(uuid::Uuid::nil()), events).unwrap();
        let result = Graph47ResultMustBeNonNegative.evaluate(&session);
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].severity, Severity::Error);
    }

    #[test]
    fn silent_when_recalculate_followed_by_unrelated_event() {
        use trace_core::{Outcome, ScreenId};
        let events = vec![
            make_event(
                0,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Graph47.Recalculate"),
                    args: serde_json::json!({}),
                    duration_ms: 200,
                    outcome: Outcome::Success,
                },
            ),
            make_event(
                1,
                TraceEventKind::ScreenOpened {
                    screen_id: ScreenId::new("ExportDialog"),
                    params: serde_json::json!({}),
                },
            ),
        ];
        let session = Session::new(SessionId::new(uuid::Uuid::nil()), events).unwrap();
        let result = Graph47ResultMustBeNonNegative.evaluate(&session);
        assert!(result.passed);
    }

    #[test]
    fn silent_when_recalculate_fails() {
        use trace_core::Outcome;
        let events = vec![
            make_event(
                0,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Graph47.Recalculate"),
                    args: serde_json::json!({}),
                    duration_ms: 200,
                    outcome: Outcome::Failure {
                        message: "timeout".into(),
                    },
                },
            ),
            make_event(
                1,
                TraceEventKind::ExceptionThrown {
                    exception_type: "NegativePaymentException".into(),
                    message: "***".into(),
                    stack: None,
                },
            ),
        ];
        let session = Session::new(SessionId::new(uuid::Uuid::nil()), events).unwrap();
        let result = Graph47ResultMustBeNonNegative.evaluate(&session);
        assert!(
            result.passed,
            "rule must not fire when command already failed"
        );
    }
}
