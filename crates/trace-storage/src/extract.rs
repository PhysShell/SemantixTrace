//! Helpers that extract typed index-column values from [`TraceEventKind`].
//!
//! These are pure functions used by both [`crate::sqlite`] (for writing
//! index columns) and [`crate::sqlite::DimensionSet`] (for building
//! candidate-filter inputs).  Kept in a separate module so they can be
//! tested in isolation and reused without pulling in the full backend.

use trace_core::Outcome;
use trace_schema::v1::TraceEventKind;

/// Return the `PascalCase` variant name of the event kind
/// (e.g. `"ScreenOpened"`, `"CommandExecuted"`).
#[must_use]
pub const fn event_kind_name(kind: &TraceEventKind) -> &'static str {
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

/// Return the `command_id` string for kinds that carry one, else `None`.
///
/// Covers `CommandExecuted` and `AsyncOperationCompleted` (which uses
/// `operation_id` as its semantic command identifier).
#[must_use]
pub fn extract_command_id(kind: &TraceEventKind) -> Option<&str> {
    match kind {
        TraceEventKind::CommandExecuted { command_id, .. } => Some(command_id.as_str()),
        TraceEventKind::AsyncOperationCompleted { operation_id, .. } => Some(operation_id),
        _ => None,
    }
}

/// Return the `screen_id` string for kinds that carry one, else `None`.
#[must_use]
pub fn extract_screen_id(kind: &TraceEventKind) -> Option<&str> {
    match kind {
        TraceEventKind::ScreenOpened { screen_id, .. } => Some(screen_id.as_str()),
        _ => None,
    }
}

/// Return the outcome label for kinds that carry an [`Outcome`], else `None`.
#[must_use]
pub fn extract_outcome_str(kind: &TraceEventKind) -> Option<String> {
    match kind {
        TraceEventKind::CommandExecuted { outcome, .. }
        | TraceEventKind::AsyncOperationCompleted { outcome, .. } => {
            Some(outcome_label(outcome).to_owned())
        }
        _ => None,
    }
}

/// Convert an [`Outcome`] to its stable lowercase label used in the index
/// column and in `--by outcome` slice filters.
#[must_use]
pub const fn outcome_label(outcome: &Outcome) -> &'static str {
    match outcome {
        Outcome::Success => "success",
        Outcome::Failure { .. } => "failure",
        Outcome::Cancelled => "cancelled",
        Outcome::TimedOut => "timed_out",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use trace_core::{CommandId, Outcome, ScreenId};
    use trace_schema::v1::TraceEventKind;

    use super::{event_kind_name, extract_command_id, extract_outcome_str, extract_screen_id};

    #[test]
    fn screen_opened_kind_and_screen_id() {
        let kind = TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("Editor"),
            params: serde_json::json!({}),
        };
        assert_eq!(event_kind_name(&kind), "ScreenOpened");
        assert_eq!(extract_screen_id(&kind), Some("Editor"));
        assert!(extract_command_id(&kind).is_none());
        assert!(extract_outcome_str(&kind).is_none());
    }

    #[test]
    fn command_executed_extracts_all() {
        let kind = TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Save"),
            args: serde_json::json!({}),
            duration_ms: 5,
            outcome: Outcome::Success,
        };
        assert_eq!(event_kind_name(&kind), "CommandExecuted");
        assert_eq!(extract_command_id(&kind), Some("Save"));
        assert!(extract_screen_id(&kind).is_none());
        assert_eq!(extract_outcome_str(&kind).as_deref(), Some("success"));
    }
}
