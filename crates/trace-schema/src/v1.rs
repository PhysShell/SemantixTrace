//! Schema **v1** — the first wire shape for `SemantxTrace` trace events.
//!
//! Frozen forever after v1.0 ships per ADR-0006. New variants or
//! field changes go into a future `v2` module plus a
//! [`crate::Upcaster`] impl that lifts v1 → v2; the v1 module is never
//! edited after release.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use trace_core::{
    CommandId, CorrelationId, EventSeq, FieldId, Outcome, ScreenId, SessionId, ValuePolicy,
};

/// JSON envelope for a single v1 trace event.
///
/// The envelope carries `schema_version` as a sibling of the event
/// fields (not embedded in `kind`), so [`crate::read_event`] can
/// dispatch on a probe struct (`VersionProbe`) before deserializing
/// the full event. ADR-0006 §"Wire format details" fixes this.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceEnvelope {
    /// Always `1` for v1. Future versions ship as `2`, `3`, ….
    pub schema_version: u32,
    /// The wrapped event.
    #[serde(flatten)]
    pub event: TraceEvent,
}

impl TraceEnvelope {
    /// Wrap an event with the v1 `schema_version` field.
    #[must_use]
    pub const fn from_event(event: TraceEvent) -> Self {
        Self {
            schema_version: 1,
            event,
        }
    }

    /// Discard the envelope, keep the event.
    #[must_use]
    #[allow(
        clippy::missing_const_for_fn,
        reason = "TraceEvent destructor is not const"
    )]
    pub fn into_event(self) -> TraceEvent {
        self.event
    }
}

/// A single v1 trace event.
///
/// The 7 event kinds in [`TraceEventKind`] correspond one-to-one to the
/// list in `docs/glossary.md` §2 ("Trace events"). Domain meaning
/// dominates physical shape: e.g. there is no `ButtonClicked`, only
/// `CommandExecuted { command_id: CommandId, … }` (ADR-0005).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Monotonic per-session counter (gaps indicate dropped events).
    pub seq: EventSeq,
    /// Session this event belongs to.
    pub session_id: SessionId,
    /// UTC wall-clock timestamp.
    pub ts: DateTime<Utc>,
    /// Optional id linking related events (e.g. start / end of an
    /// async operation, a command and its post-modal).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<CorrelationId>,
    /// The event payload — see [`TraceEventKind`].
    #[serde(flatten)]
    pub kind: TraceEventKind,
}

/// Discriminated payload of a v1 [`TraceEvent`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase")]
#[non_exhaustive]
pub enum TraceEventKind {
    /// A screen / view was opened.
    ScreenOpened {
        /// Semantic screen identifier (ADR-0005).
        screen_id: ScreenId,
        /// Open-time parameters (e.g. document id), policy-wrapped.
        #[serde(default)]
        params: serde_json::Value,
    },
    /// A semantic command executed.
    CommandExecuted {
        /// Semantic command identifier (ADR-0005).
        command_id: CommandId,
        /// Command arguments, policy-wrapped if applicable.
        #[serde(default)]
        args: serde_json::Value,
        /// Wall-clock duration in milliseconds.
        duration_ms: u64,
        /// Domain outcome.
        #[serde(flatten)]
        outcome: Outcome,
    },
    /// A bound field's value changed.
    FieldChanged {
        /// Semantic field identifier (ADR-0005).
        field_id: FieldId,
        /// Previous value, privacy-wrapped (ADR-0007).
        old: ValuePolicy,
        /// New value, privacy-wrapped (ADR-0007).
        new: ValuePolicy,
    },
    /// An exception escaped to the trace boundary.
    ExceptionThrown {
        /// Concrete exception / error type name.
        exception_type: String,
        /// Human-readable message.
        message: String,
        /// Stack trace, if available.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stack: Option<String>,
    },
    /// Navigation occurred between two semantic screens.
    NavigationOccurred {
        /// Departing screen.
        from: ScreenId,
        /// Arriving screen.
        to: ScreenId,
    },
    /// A named validator reported a failure on a bound field.
    ValidationFailed {
        /// Validator name (e.g. `Required`, `EmailFormat`).
        validator: String,
        /// Field that failed.
        field_id: FieldId,
        /// Human-readable reason.
        reason: String,
    },
    /// An async operation completed.
    AsyncOperationCompleted {
        /// Application-defined operation identifier.
        operation_id: String,
        /// Wall-clock duration in milliseconds.
        duration_ms: u64,
        /// Domain outcome.
        #[serde(flatten)]
        outcome: Outcome,
    },
}

// ===========================================================================
// Sealing — these per-version types participate in the [`Upcaster`] /
// [`StreamUpcaster`] trait surface (ADR-0012 `C-SEALED`).
// ===========================================================================

impl crate::sealed::Sealed for TraceEvent {}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use trace_core::{CommandId, EventSeq, Outcome, ScreenId, SessionId, ValuePolicy};

    use super::{TraceEnvelope, TraceEvent, TraceEventKind};

    fn sample() -> TraceEvent {
        TraceEvent {
            seq: EventSeq::new(7),
            session_id: SessionId::random(),
            ts: chrono::Utc.with_ymd_and_hms(2026, 5, 27, 12, 0, 0).unwrap(),
            correlation_id: None,
            kind: TraceEventKind::CommandExecuted {
                command_id: CommandId::new("Graph47.Recalculate"),
                args: serde_json::json!({"docId": ValuePolicy::default_masked()}),
                duration_ms: 42,
                outcome: Outcome::Success,
            },
        }
    }

    #[test]
    fn envelope_round_trip() {
        let env = TraceEnvelope::from_event(sample());
        let json = serde_json::to_string(&env).expect("serialize");
        let back: TraceEnvelope = serde_json::from_str(&json).expect("parse");
        assert_eq!(env, back);
        assert_eq!(env.schema_version, 1);
    }

    #[test]
    fn screen_opened_round_trip() {
        let mut e = sample();
        e.kind = TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("DeclarationEditor"),
            params: serde_json::json!({"docId": "***"}),
        };
        let env = TraceEnvelope::from_event(e);
        let json = serde_json::to_string(&env).expect("serialize");
        let back: TraceEnvelope = serde_json::from_str(&json).expect("parse");
        assert_eq!(env, back);
    }

    #[test]
    fn read_event_dispatches_v1() {
        let env = TraceEnvelope::from_event(sample());
        let raw = serde_json::to_string(&env).expect("serialize");
        let event = crate::read_event(&raw).expect("read_event");
        assert_eq!(event, env.into_event());
    }

    #[test]
    fn read_event_rejects_unsupported_version() {
        let raw = r#"{"schema_version":999,"seq":0,"session_id":"00000000-0000-0000-0000-000000000000","ts":"2026-05-27T12:00:00Z","kind":"ScreenOpened","screen_id":"X","params":null}"#;
        let err = crate::read_event(raw).unwrap_err();
        match err {
            crate::SchemaError::UnsupportedSchemaVersion(999) => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
