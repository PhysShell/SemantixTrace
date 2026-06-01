//! Nearest-scenario search over a corpus of trace event sequences.
//!
//! [`nearest`] computes the alignment similarity between a probe sequence
//! and every session in a corpus, returning the top-N most similar sessions
//! sorted by similarity descending.

#![forbid(unsafe_code)]

use trace_core::SessionId;
use trace_schema::Current;

use crate::align::align;
use crate::cost::CostFn;

/// A single match result from [`nearest`].
#[derive(Clone, Copy, Debug)]
pub struct NearestMatch {
    /// The session that matched.
    pub session_id: SessionId,
    /// Index of the scenario within the session (0-based).
    ///
    /// Currently always `0` because no internal segmentation is performed
    /// at this layer; reserved for future use.
    pub scenario_idx: usize,
    /// Alignment similarity score (`0.0`–`1.0`; higher is more similar).
    pub similarity: f64,
    /// Raw alignment cost (lower is more similar).
    pub cost: f64,
}

/// Find the `top_n` most similar event sequences to `probe` in `corpus`.
///
/// `corpus` is a slice of `(session_id, events)` pairs.  Each session is
/// treated as a single scenario for comparison purposes (no internal
/// segmentation at this layer).
///
/// Returns up to `top_n` results sorted by similarity descending.
/// The probe session itself is excluded when `probe_session_id` is `Some`.
#[must_use]
pub fn nearest(
    probe: &[Current],
    corpus: &[(SessionId, Vec<Current>)],
    cost: &impl CostFn,
    top_n: usize,
    probe_session_id: Option<SessionId>,
) -> Vec<NearestMatch> {
    let mut results: Vec<NearestMatch> = corpus
        .iter()
        .filter_map(|(session_id, events)| {
            // Exclude the probe session itself if requested.
            if probe_session_id.as_ref() == Some(session_id) {
                return None;
            }
            let alignment = align(probe, events, cost);
            Some(NearestMatch {
                session_id: *session_id,
                scenario_idx: 0,
                similarity: alignment.similarity(),
                cost: alignment.total_cost,
            })
        })
        .collect();

    // Sort by similarity descending (higher similarity first).
    results.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results.truncate(top_n);
    results
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use trace_core::{EventSeq, ScreenId, SessionId};
    use trace_schema::v1::{TraceEvent as V1Event, TraceEventKind};
    use trace_schema::v2::V1ToV2;
    use trace_schema::{Current, Upcaster};
    use uuid::Uuid;

    use crate::cost::StructuralCost;

    use super::nearest;

    fn make_screen(id: &str) -> Current {
        V1ToV2::upcast(V1Event {
            seq: EventSeq::new(0),
            session_id: SessionId::new(Uuid::nil()),
            ts: chrono::Utc::now(),
            correlation_id: None,
            kind: TraceEventKind::ScreenOpened {
                screen_id: ScreenId::new(id),
                params: serde_json::json!({}),
            },
        })
    }

    const fn session(id: u128) -> SessionId {
        SessionId::new(Uuid::from_u128(id))
    }

    #[test]
    fn nearest_returns_best_match() {
        let cost = StructuralCost::default();
        let probe = vec![make_screen("Home"), make_screen("Detail")];
        let corpus = vec![
            (session(1), vec![make_screen("Home"), make_screen("Detail")]),
            (session(2), vec![make_screen("Other")]),
        ];
        let results = nearest(&probe, &corpus, &cost, 2, None);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].session_id, session(1));
        assert!((results[0].similarity - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn nearest_excludes_probe_session() {
        let cost = StructuralCost::default();
        let probe_id = session(1);
        let probe = vec![make_screen("Home")];
        let corpus = vec![
            (probe_id, vec![make_screen("Home")]),
            (session(2), vec![make_screen("Home")]),
        ];
        let results = nearest(&probe, &corpus, &cost, 10, Some(probe_id));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session(2));
    }

    #[test]
    fn nearest_top_n_limits_results() {
        let cost = StructuralCost::default();
        let probe = vec![make_screen("Home")];
        let corpus: Vec<_> = (1_u128..=5)
            .map(|i| (session(i), vec![make_screen("Home")]))
            .collect();
        let results = nearest(&probe, &corpus, &cost, 3, None);
        assert_eq!(results.len(), 3);
    }
}
