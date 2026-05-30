//! Unit tests for all five built-in oracle rules.

use chrono::DateTime;
use trace_core::{CommandId, EventSeq, FieldId, Outcome, ScreenId, Session, SessionId};
use trace_oracle::{
    CommandMustSucceed, NoErrorModalAfterCommand, NoUnhandledException, ScreenMustNavigateForward,
    Severity, ValidationsPassBeforeSubmit,
};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const fn sid() -> SessionId {
    SessionId::new(Uuid::from_u128(1))
}

#[allow(clippy::cast_possible_wrap, clippy::missing_const_for_fn)]
fn make_event(seq: u64, sid: SessionId, kind: TraceEventKind) -> Current {
    Current {
        seq: EventSeq::new(seq),
        session_id: sid,
        ts: DateTime::from_timestamp(seq as i64, 0).unwrap(),
        correlation_id: None,
        domain_entity_id: None,
        kind,
    }
}

fn session_from(kinds: Vec<TraceEventKind>) -> Session<Current> {
    let s = sid();
    let events: Vec<Current> = kinds
        .into_iter()
        .enumerate()
        .map(|(i, k)| make_event(i as u64, s, k))
        .collect();
    Session::new(s, events).expect("non-empty")
}

// ---------------------------------------------------------------------------
// NoUnhandledException
// ---------------------------------------------------------------------------

use trace_oracle::Rule as OracleRule;

#[test]
fn no_unhandled_exception_pass() {
    let session = session_from(vec![TraceEventKind::ScreenOpened {
        screen_id: ScreenId::new("Home"),
        params: serde_json::json!({}),
    }]);
    let rule = NoUnhandledException;
    let result = rule.evaluate(&session);
    assert!(result.passed);
    assert!(result.violations.is_empty());
}

#[test]
fn no_unhandled_exception_fail() {
    let session = session_from(vec![
        TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("Home"),
            params: serde_json::json!({}),
        },
        TraceEventKind::ExceptionThrown {
            exception_type: "NullReferenceException".into(),
            message: "Object reference not set".into(),
            stack: None,
        },
    ]);
    let rule = NoUnhandledException;
    let result = rule.evaluate(&session);
    assert!(!result.passed);
    assert_eq!(result.violations.len(), 1);
    assert_eq!(result.violations[0].severity, Severity::Critical);
    assert!(result.violations[0]
        .message
        .contains("NullReferenceException"));
}

#[test]
fn no_unhandled_exception_multiple_exceptions() {
    let session = session_from(vec![
        TraceEventKind::ExceptionThrown {
            exception_type: "NullRef".into(),
            message: "first".into(),
            stack: None,
        },
        TraceEventKind::ExceptionThrown {
            exception_type: "IOException".into(),
            message: "second".into(),
            stack: None,
        },
    ]);
    let rule = NoUnhandledException;
    let result = rule.evaluate(&session);
    assert!(!result.passed);
    assert_eq!(result.violations.len(), 2);
}

// ---------------------------------------------------------------------------
// NoErrorModalAfterCommand
// ---------------------------------------------------------------------------

#[test]
fn no_error_modal_pass_no_command() {
    let session = session_from(vec![TraceEventKind::ScreenOpened {
        screen_id: ScreenId::new("ErrorScreen"),
        params: serde_json::json!({}),
    }]);
    let rule = NoErrorModalAfterCommand;
    let result = rule.evaluate(&session);
    // Screen opened without a preceding command — not a violation.
    assert!(result.passed);
}

#[test]
fn no_error_modal_fail() {
    let session = session_from(vec![
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Doc.Load"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
        TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("ErrorDialog"),
            params: serde_json::json!({}),
        },
    ]);
    let rule = NoErrorModalAfterCommand;
    let result = rule.evaluate(&session);
    assert!(!result.passed);
    assert_eq!(result.violations.len(), 1);
    assert_eq!(result.violations[0].severity, Severity::Error);
    assert!(result.violations[0].message.contains("ErrorDialog"));
}

#[test]
fn no_error_modal_pass_normal_screen_after_command() {
    let session = session_from(vec![
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Doc.Load"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
        TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("Editor"),
            params: serde_json::json!({}),
        },
    ]);
    let rule = NoErrorModalAfterCommand;
    let result = rule.evaluate(&session);
    assert!(result.passed);
}

#[test]
fn no_error_modal_detects_modal_suffix() {
    let session = session_from(vec![
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("X.Delete"),
            args: serde_json::json!({}),
            duration_ms: 5,
            outcome: Outcome::Success,
        },
        TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("ConfirmModal"),
            params: serde_json::json!({}),
        },
    ]);
    let rule = NoErrorModalAfterCommand;
    let result = rule.evaluate(&session);
    assert!(!result.passed);
    assert_eq!(result.violations[0].severity, Severity::Error);
}

// ---------------------------------------------------------------------------
// CommandMustSucceed
// ---------------------------------------------------------------------------

#[test]
fn command_must_succeed_pass() {
    let session = session_from(vec![TraceEventKind::CommandExecuted {
        command_id: CommandId::new("Declaration.Save"),
        args: serde_json::json!({}),
        duration_ms: 10,
        outcome: Outcome::Success,
    }]);
    let rule = CommandMustSucceed::new(["Declaration.Save"]);
    let result = rule.evaluate(&session);
    assert!(result.passed);
}

#[test]
fn command_must_succeed_fail_on_failure_outcome() {
    let session = session_from(vec![TraceEventKind::CommandExecuted {
        command_id: CommandId::new("Declaration.Save"),
        args: serde_json::json!({}),
        duration_ms: 10,
        outcome: Outcome::Failure {
            message: "disk full".into(),
        },
    }]);
    let rule = CommandMustSucceed::new(["Declaration.Save"]);
    let result = rule.evaluate(&session);
    assert!(!result.passed);
    assert_eq!(result.violations.len(), 1);
    assert_eq!(result.violations[0].severity, Severity::Error);
}

#[test]
fn command_must_succeed_ignores_unlisted_commands() {
    let session = session_from(vec![TraceEventKind::CommandExecuted {
        command_id: CommandId::new("Other.Op"),
        args: serde_json::json!({}),
        duration_ms: 10,
        outcome: Outcome::Failure {
            message: "oops".into(),
        },
    }]);
    let rule = CommandMustSucceed::new(["Declaration.Save"]);
    let result = rule.evaluate(&session);
    assert!(
        result.passed,
        "unlisted command failure should not be flagged"
    );
}

#[test]
fn command_must_succeed_empty_set_always_passes() {
    let session = session_from(vec![TraceEventKind::CommandExecuted {
        command_id: CommandId::new("Anything"),
        args: serde_json::json!({}),
        duration_ms: 1,
        outcome: Outcome::Failure {
            message: "meh".into(),
        },
    }]);
    let rule = CommandMustSucceed::new(std::iter::empty::<&str>());
    let result = rule.evaluate(&session);
    assert!(result.passed);
}

// ---------------------------------------------------------------------------
// ScreenMustNavigateForward
// ---------------------------------------------------------------------------

#[test]
fn screen_navigate_forward_pass() {
    let session = session_from(vec![
        TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("Home"),
            params: serde_json::json!({}),
        },
        TraceEventKind::NavigationOccurred {
            from: ScreenId::new("Home"),
            to: ScreenId::new("Editor"),
        },
        TraceEventKind::NavigationOccurred {
            from: ScreenId::new("Editor"),
            to: ScreenId::new("Confirm"),
        },
    ]);
    let rule = ScreenMustNavigateForward;
    let result = rule.evaluate(&session);
    assert!(result.passed);
}

#[test]
fn screen_navigate_forward_fail_on_revisit() {
    let session = session_from(vec![
        TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("Home"),
            params: serde_json::json!({}),
        },
        TraceEventKind::NavigationOccurred {
            from: ScreenId::new("Home"),
            to: ScreenId::new("Editor"),
        },
        TraceEventKind::NavigationOccurred {
            from: ScreenId::new("Editor"),
            to: ScreenId::new("Home"),
        },
    ]);
    let rule = ScreenMustNavigateForward;
    let result = rule.evaluate(&session);
    assert!(!result.passed);
    assert_eq!(result.violations.len(), 1);
    assert_eq!(result.violations[0].severity, Severity::Warning);
    assert!(result.violations[0].message.contains("Home"));
}

// ---------------------------------------------------------------------------
// ValidationsPassBeforeSubmit
// ---------------------------------------------------------------------------

#[test]
fn validations_pass_before_submit_pass_no_failures() {
    let session = session_from(vec![TraceEventKind::CommandExecuted {
        command_id: CommandId::new("Form.Submit"),
        args: serde_json::json!({}),
        duration_ms: 1,
        outcome: Outcome::Success,
    }]);
    let rule = ValidationsPassBeforeSubmit::default();
    let result = rule.evaluate(&session);
    assert!(result.passed);
}

#[test]
fn validations_pass_before_submit_fail() {
    let session = session_from(vec![
        TraceEventKind::ValidationFailed {
            validator: "Required".into(),
            field_id: FieldId::new("Customer.Name"),
            reason: "must not be empty".into(),
        },
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Form.Submit"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
    ]);
    let rule = ValidationsPassBeforeSubmit::default();
    let result = rule.evaluate(&session);
    assert!(!result.passed);
    assert_eq!(result.violations.len(), 1);
    assert_eq!(result.violations[0].severity, Severity::Error);
    assert!(result.violations[0].message.contains("1 outstanding"));
}

#[test]
fn validations_pass_before_submit_cleared_by_navigation() {
    let session = session_from(vec![
        TraceEventKind::ValidationFailed {
            validator: "Required".into(),
            field_id: FieldId::new("Customer.Name"),
            reason: "empty".into(),
        },
        TraceEventKind::NavigationOccurred {
            from: ScreenId::new("FormScreen"),
            to: ScreenId::new("HomeScreen"),
        },
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Form.Save"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
    ]);
    let rule = ValidationsPassBeforeSubmit::default();
    let result = rule.evaluate(&session);
    assert!(result.passed, "navigation should clear pending failures");
}

#[test]
fn validations_pass_before_submit_cleared_by_non_submit_command() {
    let session = session_from(vec![
        TraceEventKind::ValidationFailed {
            validator: "Required".into(),
            field_id: FieldId::new("X"),
            reason: "empty".into(),
        },
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Form.Recalculate"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
        TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Form.Save"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        },
    ]);
    let rule = ValidationsPassBeforeSubmit::default();
    let result = rule.evaluate(&session);
    assert!(
        result.passed,
        "non-submit command should clear pending failures"
    );
}

// ---------------------------------------------------------------------------
// Engine integration
// ---------------------------------------------------------------------------

#[test]
fn engine_with_builtin_rules_runs_all_five() {
    use trace_oracle::OracleEngine;
    let engine = OracleEngine::new().with_builtin_rules();
    assert_eq!(engine.rules().len(), 5);

    let session = session_from(vec![TraceEventKind::ScreenOpened {
        screen_id: ScreenId::new("Home"),
        params: serde_json::json!({}),
    }]);
    let report = engine.run(&session);
    // Clean session with no exceptions/errors/modals — all rules should pass.
    assert!(report.all_passed());
    assert!(report.max_severity().is_none());
}

#[test]
fn engine_run_all_aggregates() {
    use trace_oracle::OracleEngine;
    let engine = OracleEngine::new().with_builtin_rules();

    let s1 = session_from(vec![TraceEventKind::ScreenOpened {
        screen_id: ScreenId::new("Home"),
        params: serde_json::json!({}),
    }]);
    let s2 = session_from(vec![TraceEventKind::ExceptionThrown {
        exception_type: "Boom".into(),
        message: "kaboom".into(),
        stack: None,
    }]);
    let reports = engine.run_all(&[s1, s2]);
    assert_eq!(reports.len(), 2);
    assert!(reports[0].all_passed());
    assert!(!reports[1].all_passed());
    assert_eq!(reports[1].max_severity(), Some(Severity::Critical));
}
