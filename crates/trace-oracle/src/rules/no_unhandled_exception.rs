//! [`NoUnhandledException`] oracle rule.

use trace_core::Session;
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

use crate::rule::{OracleResult, OracleSchedule, OracleViolation, Rule, Severity};

/// Fails if any `ExceptionThrown` event appears in the session.
///
/// An unhandled exception reaching the trace boundary is a [`Severity::Critical`]
/// violation: the application's error handling did not suppress or wrap the
/// exception before it escaped the command boundary.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoUnhandledException;

impl Rule for NoUnhandledException {
    fn name(&self) -> &'static str {
        "NoUnhandledException"
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::PerScenario
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let violations: Vec<OracleViolation> = session
            .events()
            .iter()
            .filter_map(|ev| {
                if let TraceEventKind::ExceptionThrown {
                    exception_type,
                    message,
                    ..
                } = &ev.kind
                {
                    Some(OracleViolation {
                        rule: self.name().to_owned(),
                        severity: Severity::Critical,
                        message: format!("Unhandled exception: {exception_type}: {message}"),
                        evidence: vec![ev.seq],
                    })
                } else {
                    None
                }
            })
            .collect();
        OracleResult::fail(self.name(), violations)
    }
}
