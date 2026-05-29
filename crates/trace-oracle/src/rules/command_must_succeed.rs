//! [`CommandMustSucceed`] oracle rule.

use std::collections::HashSet;

use trace_core::{CommandId, Session};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

use crate::rule::{OracleResult, OracleSchedule, OracleViolation, Rule, Severity};

/// Fails if any of the tracked `command_ids` produce a non-`Success` outcome.
///
/// Useful for asserting that critical commands (e.g. `Declaration.Save`)
/// always complete successfully in a passing test session.
#[derive(Clone, Debug)]
pub struct CommandMustSucceed {
    /// The commands that must always succeed.
    pub command_ids: HashSet<CommandId>,
}

impl CommandMustSucceed {
    /// Construct from an iterator of command-id strings.
    pub fn new(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            command_ids: ids.into_iter().map(|s| CommandId::new(s.into())).collect(),
        }
    }
}

impl Rule for CommandMustSucceed {
    fn name(&self) -> &'static str {
        "CommandMustSucceed"
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::PerScenario
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let violations: Vec<_> = session
            .events()
            .iter()
            .filter_map(|ev| {
                if let TraceEventKind::CommandExecuted {
                    command_id,
                    outcome,
                    ..
                } = &ev.kind
                {
                    if self.command_ids.contains(command_id) && outcome.is_failure() {
                        return Some(OracleViolation {
                            rule: self.name().to_owned(),
                            severity: Severity::Error,
                            message: format!(
                                "Command '{command_id}' did not succeed (seq {})",
                                ev.seq
                            ),
                            evidence: vec![ev.seq],
                        });
                    }
                }
                None
            })
            .collect();
        OracleResult::fail(self.name(), violations)
    }
}
