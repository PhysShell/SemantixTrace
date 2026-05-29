//! [`NoErrorModalAfterCommand`] oracle rule.

use trace_core::Session;
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

use crate::rule::{OracleResult, OracleSchedule, OracleViolation, Rule, Severity};

/// Fails if a screen whose ID contains "Error" or "Modal" is opened
/// immediately after a `CommandExecuted` event.
///
/// "Immediately" means the very next event in stream order.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoErrorModalAfterCommand;

impl Rule for NoErrorModalAfterCommand {
    fn name(&self) -> &'static str {
        "NoErrorModalAfterCommand"
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
            if matches!(&prev.kind, TraceEventKind::CommandExecuted { .. }) {
                if let TraceEventKind::ScreenOpened { screen_id, .. } = &next.kind {
                    let id = screen_id.as_str();
                    if id.contains("Error") || id.contains("Modal") {
                        violations.push(OracleViolation {
                            rule: self.name().to_owned(),
                            severity: Severity::Error,
                            message: format!(
                                "Error/Modal screen '{id}' opened after command (seq {})",
                                prev.seq
                            ),
                            evidence: vec![prev.seq, next.seq],
                        });
                    }
                }
            }
        }
        OracleResult::fail(self.name(), violations)
    }
}
