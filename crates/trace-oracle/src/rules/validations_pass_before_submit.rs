//! [`ValidationsPassBeforeSubmit`] oracle rule.

use trace_core::{CommandId, Session};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

use crate::rule::{OracleResult, OracleSchedule, OracleViolation, Rule, Severity};

/// Fails if a `ValidationFailed` event is immediately followed (as the
/// next event) by a submit command.
///
/// "Submit" commands are configurable; defaults to any command whose ID
/// ends with `.Submit` or `.Save`.
#[derive(Clone, Debug)]
pub struct ValidationsPassBeforeSubmit {
    /// Command-ID suffixes that identify submit commands.
    pub submit_suffixes: Vec<String>,
}

impl Default for ValidationsPassBeforeSubmit {
    fn default() -> Self {
        Self {
            submit_suffixes: vec![".Submit".to_owned(), ".Save".to_owned()],
        }
    }
}

impl ValidationsPassBeforeSubmit {
    fn is_submit(&self, command_id: &CommandId) -> bool {
        self.submit_suffixes
            .iter()
            .any(|s| command_id.as_str().ends_with(s.as_str()))
    }
}

impl Rule for ValidationsPassBeforeSubmit {
    fn name(&self) -> &'static str {
        "ValidationsPassBeforeSubmit"
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::PerScenario
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let events = session.events();
        let mut violations = Vec::new();
        // Track outstanding validation failures (cleared by any CommandExecuted
        // that isn't a submit, or by a successful NavigationOccurred).
        let mut pending_failures: Vec<usize> = Vec::new(); // indices into events

        for (i, ev) in events.iter().enumerate() {
            match &ev.kind {
                TraceEventKind::ValidationFailed { .. } => {
                    pending_failures.push(i);
                }
                TraceEventKind::CommandExecuted { command_id, .. } => {
                    if self.is_submit(command_id) && !pending_failures.is_empty() {
                        let mut evidence: Vec<_> =
                            pending_failures.iter().map(|&j| events[j].seq).collect();
                        evidence.push(ev.seq);
                        violations.push(OracleViolation {
                            rule: self.name().to_owned(),
                            severity: Severity::Error,
                            message: format!(
                                "Submit command '{}' fired with {} outstanding validation failure(s)",
                                command_id.as_str(),
                                pending_failures.len()
                            ),
                            evidence,
                        });
                    }
                    // Clear pending failures after any command (submit or not).
                    pending_failures.clear();
                }
                TraceEventKind::NavigationOccurred { .. } => {
                    pending_failures.clear();
                }
                _ => {}
            }
        }
        OracleResult::fail(self.name(), violations)
    }
}
