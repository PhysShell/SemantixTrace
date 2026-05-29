//! [`ScreenMustNavigateForward`] oracle rule.

use trace_core::{ScreenId, Session};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

use crate::rule::{OracleResult, OracleSchedule, OracleViolation, Rule, Severity};

/// Fails if navigation returns to a screen that was already visited within
/// the session (i.e. a back-navigation cycle).
///
/// Tracks screen transitions via `ScreenOpened` and `NavigationOccurred`.
/// A revisit is flagged when either event names a screen already in the
/// session history.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenMustNavigateForward;

impl Rule for ScreenMustNavigateForward {
    fn name(&self) -> &'static str {
        "ScreenMustNavigateForward"
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::PerScenario
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let mut history: Vec<ScreenId> = Vec::new();
        let mut violations = Vec::new();

        for ev in session.events() {
            let arrived: Option<&ScreenId> = match &ev.kind {
                TraceEventKind::ScreenOpened { screen_id, .. } => Some(screen_id),
                TraceEventKind::NavigationOccurred { to, .. } => Some(to),
                _ => None,
            };
            if let Some(screen) = arrived {
                // Flag if this screen already appears anywhere earlier in history.
                if history.contains(screen) {
                    violations.push(OracleViolation {
                        rule: self.name().to_owned(),
                        severity: Severity::Warning,
                        message: format!(
                            "Navigation revisited screen '{}' (seq {})",
                            screen.as_str(),
                            ev.seq
                        ),
                        evidence: vec![ev.seq],
                    });
                }
                history.push(screen.clone());
            }
        }
        OracleResult::fail(self.name(), violations)
    }
}
