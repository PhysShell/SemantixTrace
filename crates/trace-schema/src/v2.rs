//! Schema **v2** — adds `domain_entity_id` to the trace event envelope.
//!
//! The event-kind variants are unchanged from v1; [`TraceEventKind`] is
//! re-exported from [`crate::v1`] so downstream code that imports
//! `trace_schema::v1::TraceEventKind` continues to compile and pattern-
//! match without modification.
//!
//! Upcaster: [`V1ToV2`] lifts every v1 event by inserting
//! `domain_entity_id: None`. The upcast is lossless — v1 events simply
//! had no entity-id field, so `None` is the correct representation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use trace_core::{CorrelationId, DomainEntityId, EventSeq, SessionId};

/// Re-export the unchanged kind enum so downstream crates that import
/// `trace_schema::v1::TraceEventKind` see the same type as `v2::TraceEventKind`.
pub use crate::v1::TraceEventKind;

/// JSON envelope for a single v2 trace event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEnvelope {
    /// Always `2` for v2.
    pub schema_version: u32,
    /// The wrapped event.
    #[serde(flatten)]
    pub event: TraceEvent,
}

impl TraceEnvelope {
    /// Wrap an event with the v2 `schema_version` field.
    #[must_use]
    pub const fn from_event(event: TraceEvent) -> Self {
        Self {
            schema_version: 2,
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

/// A single v2 trace event.
///
/// Identical to v1 except for the new optional [`domain_entity_id`]
/// field, which carries the semantic identifier of the primary domain
/// entity the event applies to (e.g. `Declaration:doc-123`).
///
/// [`domain_entity_id`]: TraceEvent::domain_entity_id
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Semantic identifier of the primary domain entity this event
    /// applies to (e.g. `Declaration:doc-123`). `None` when the
    /// current screen has no single entity focus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_entity_id: Option<DomainEntityId>,
    /// The event payload — see [`TraceEventKind`].
    #[serde(flatten)]
    pub kind: TraceEventKind,
}

impl crate::sealed::Sealed for TraceEvent {}

// ---------------------------------------------------------------------------
// Upcaster v1 → v2
// ---------------------------------------------------------------------------

/// Upcaster that lifts a v1 [`crate::v1::TraceEvent`] to a v2
/// [`TraceEvent`] by setting `domain_entity_id = None`.
///
/// The upcast is lossless: v1 events had no entity-id concept, so
/// `None` faithfully represents their absence.
#[derive(Clone, Copy, Debug)]
pub struct V1ToV2;

impl crate::sealed::Sealed for V1ToV2 {}

impl crate::Upcaster for V1ToV2 {
    type From = crate::v1::TraceEvent;
    type To = TraceEvent;

    /// Lossless: v1 events had no `domain_entity_id`.
    const LOSSY: bool = false;

    fn upcast(input: crate::v1::TraceEvent) -> TraceEvent {
        TraceEvent {
            seq: input.seq,
            session_id: input.session_id,
            ts: input.ts,
            correlation_id: input.correlation_id,
            domain_entity_id: None,
            kind: input.kind,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use trace_core::{CommandId, EventSeq, Outcome, ScreenId, SessionId};

    use super::{TraceEnvelope, TraceEvent, TraceEventKind, V1ToV2};
    use crate::Upcaster;

    fn sample_v2() -> TraceEvent {
        TraceEvent {
            seq: EventSeq::new(1),
            session_id: SessionId::random(),
            ts: chrono::Utc.with_ymd_and_hms(2026, 5, 30, 0, 0, 0).unwrap(),
            correlation_id: None,
            domain_entity_id: None,
            kind: TraceEventKind::CommandExecuted {
                command_id: CommandId::new("Graph47.Recalculate"),
                args: serde_json::json!({}),
                duration_ms: 10,
                outcome: Outcome::Success,
            },
        }
    }

    fn sample_v2_with_entity() -> TraceEvent {
        use trace_core::DomainEntityId;
        TraceEvent {
            domain_entity_id: Some(DomainEntityId::new("Declaration:doc-123")),
            ..sample_v2()
        }
    }

    #[test]
    fn envelope_round_trip_no_entity() {
        let env = TraceEnvelope::from_event(sample_v2());
        let json = serde_json::to_string(&env).expect("serialize");
        let back: TraceEnvelope = serde_json::from_str(&json).expect("parse");
        assert_eq!(env, back);
        assert_eq!(back.schema_version, 2);
        assert!(back.event.domain_entity_id.is_none());
    }

    #[test]
    fn envelope_round_trip_with_entity() {
        let env = TraceEnvelope::from_event(sample_v2_with_entity());
        let json = serde_json::to_string(&env).expect("serialize");
        assert!(
            json.contains("Declaration:doc-123"),
            "entity id must appear in JSON"
        );
        let back: TraceEnvelope = serde_json::from_str(&json).expect("parse");
        assert_eq!(env, back);
    }

    #[test]
    fn upcast_preserves_all_v1_fields() {
        let v1_event = crate::v1::TraceEvent {
            seq: EventSeq::new(7),
            session_id: SessionId::random(),
            ts: chrono::Utc.with_ymd_and_hms(2026, 5, 30, 0, 0, 0).unwrap(),
            correlation_id: None,
            kind: TraceEventKind::ScreenOpened {
                screen_id: ScreenId::new("DeclarationEditor"),
                params: serde_json::json!({}),
            },
        };
        let v2_event = V1ToV2::upcast(v1_event.clone());
        assert_eq!(v2_event.seq, v1_event.seq);
        assert_eq!(v2_event.session_id, v1_event.session_id);
        assert_eq!(v2_event.ts, v1_event.ts);
        assert_eq!(v2_event.correlation_id, v1_event.correlation_id);
        assert!(v2_event.domain_entity_id.is_none());
        assert_eq!(v2_event.kind, v1_event.kind);
    }

    #[test]
    fn schema_version_is_2_in_wire_format() {
        let env = TraceEnvelope::from_event(sample_v2());
        let json = serde_json::to_string(&env).expect("serialize");
        assert!(
            json.contains("\"schema_version\":2"),
            "schema_version must be 2"
        );
    }
}
