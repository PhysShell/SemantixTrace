//! Summary statistics for `trace analyze` (S2).
//!
//! Counting only — the real workflow mining lands in `trace-graph` (S4).
//! This is the first concrete read-side of the analytics projection
//! (ADR-0011): given a stream of [`Current`] events, report session
//! count, per-kind event counts, and an error rate.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use serde::Serialize;
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

/// Summary report emitted by `trace analyze`.
///
/// The JSON form (`--output json`) is documented by
/// `crates/trace-cli/schema/trace-analyze-v1.schema.json`.
#[derive(Debug, Default, PartialEq, Serialize)]
pub(crate) struct AnalyzeReport {
    /// Distinct session ids seen.
    pub(crate) sessions: usize,
    /// Total events observed.
    pub(crate) events: usize,
    /// Events classified as errors (see [`is_error`]).
    pub(crate) errors: usize,
    /// `errors / events`, rounded to 3 decimals (`0.0` when empty).
    pub(crate) error_rate: f64,
    /// Count per event kind, in stable (sorted) order.
    pub(crate) per_kind: BTreeMap<String, usize>,
}

/// Compute the summary over a stream of events.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "event / error counts do not approach f64's 2^52 mantissa limit"
)]
pub(crate) fn analyze_events<I>(events: I) -> AnalyzeReport
where
    I: IntoIterator<Item = Current>,
{
    let mut sessions: HashSet<_> = HashSet::new();
    let mut report = AnalyzeReport::default();

    for event in events {
        report.events += 1;
        sessions.insert(event.session_id);
        *report
            .per_kind
            .entry(kind_name(&event.kind).to_owned())
            .or_default() += 1;
        if is_error(&event.kind) {
            report.errors += 1;
        }
    }

    report.sessions = sessions.len();
    report.error_rate = if report.events == 0 {
        0.0
    } else {
        // Round to 3 decimals for stable snapshots / JSON output.
        let raw = report.errors as f64 / report.events as f64;
        (raw * 1000.0).round() / 1000.0
    };
    report
}

/// Stable label for an event kind. Exhaustive over the frozen v1
/// variants — a future `v2` that adds a kind will break this match at
/// compile time, which is the intended signal to update the CLI.
const fn kind_name(kind: &TraceEventKind) -> &'static str {
    match kind {
        TraceEventKind::ScreenOpened { .. } => "ScreenOpened",
        TraceEventKind::CommandExecuted { .. } => "CommandExecuted",
        TraceEventKind::FieldChanged { .. } => "FieldChanged",
        TraceEventKind::ExceptionThrown { .. } => "ExceptionThrown",
        TraceEventKind::NavigationOccurred { .. } => "NavigationOccurred",
        TraceEventKind::ValidationFailed { .. } => "ValidationFailed",
        TraceEventKind::AsyncOperationCompleted { .. } => "AsyncOperationCompleted",
    }
}

/// Whether an event counts toward the error rate: exceptions, validation
/// failures, and commands / async ops with a non-success outcome.
const fn is_error(kind: &TraceEventKind) -> bool {
    match kind {
        TraceEventKind::ExceptionThrown { .. } | TraceEventKind::ValidationFailed { .. } => true,
        TraceEventKind::CommandExecuted { outcome, .. }
        | TraceEventKind::AsyncOperationCompleted { outcome, .. } => outcome.is_failure(),
        TraceEventKind::ScreenOpened { .. }
        | TraceEventKind::FieldChanged { .. }
        | TraceEventKind::NavigationOccurred { .. } => false,
    }
}

impl fmt::Display for AnalyzeReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "sessions:   {}", self.sessions)?;
        writeln!(f, "events:     {}", self.events)?;
        writeln!(f, "errors:     {}", self.errors)?;
        writeln!(f, "error_rate: {:.3}", self.error_rate)?;
        writeln!(f, "per kind:")?;
        for (kind, count) in &self.per_kind {
            writeln!(f, "  {kind:<24} {count}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use trace_core::{CommandId, EventSeq, Outcome, ScreenId, SessionId};
    use trace_schema::v1::TraceEventKind;
    use trace_schema::Current;

    use super::analyze_events;

    fn ev(seq: u64, sid: u128, kind: TraceEventKind) -> Current {
        Current {
            seq: EventSeq::new(seq),
            session_id: SessionId::new(uuid::Uuid::from_u128(sid)),
            ts: Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            correlation_id: None,
            kind,
        }
    }

    #[test]
    fn empty_stream() {
        let report = analyze_events(std::iter::empty());
        assert_eq!(report.sessions, 0);
        assert_eq!(report.events, 0);
        assert_eq!(report.errors, 0);
        assert!((report.error_rate - 0.0).abs() < f64::EPSILON);
        assert!(report.per_kind.is_empty());
    }

    #[test]
    fn counts_sessions_kinds_and_errors() {
        let events = vec![
            ev(
                0,
                1,
                TraceEventKind::ScreenOpened {
                    screen_id: ScreenId::new("Editor"),
                    params: serde_json::json!({}),
                },
            ),
            ev(
                1,
                1,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Graph47.Recalculate"),
                    args: serde_json::json!({}),
                    duration_ms: 42,
                    outcome: Outcome::Success,
                },
            ),
            ev(
                2,
                1,
                TraceEventKind::ExceptionThrown {
                    exception_type: "X".into(),
                    message: "boom".into(),
                    stack: None,
                },
            ),
            ev(
                0,
                2,
                TraceEventKind::CommandExecuted {
                    command_id: CommandId::new("Declaration.Save"),
                    args: serde_json::json!({}),
                    duration_ms: 10,
                    outcome: Outcome::Failure {
                        message: "denied".into(),
                    },
                },
            ),
        ];
        let report = analyze_events(events);
        assert_eq!(report.sessions, 2);
        assert_eq!(report.events, 4);
        assert_eq!(report.errors, 2); // exception + failed command
        assert!((report.error_rate - 0.5).abs() < f64::EPSILON);
        assert_eq!(report.per_kind.get("CommandExecuted"), Some(&2));
        assert_eq!(report.per_kind.get("ScreenOpened"), Some(&1));
        assert_eq!(report.per_kind.get("ExceptionThrown"), Some(&1));
    }

    #[test]
    fn error_rate_rounded_to_three_decimals() {
        // 1 error out of 3 → 0.333
        let events = vec![
            ev(
                0,
                1,
                TraceEventKind::ExceptionThrown {
                    exception_type: "X".into(),
                    message: "m".into(),
                    stack: None,
                },
            ),
            ev(
                1,
                1,
                TraceEventKind::NavigationOccurred {
                    from: ScreenId::new("A"),
                    to: ScreenId::new("B"),
                },
            ),
            ev(
                2,
                1,
                TraceEventKind::NavigationOccurred {
                    from: ScreenId::new("B"),
                    to: ScreenId::new("C"),
                },
            ),
        ];
        let report = analyze_events(events);
        assert!((report.error_rate - 0.333).abs() < f64::EPSILON);
    }
}
