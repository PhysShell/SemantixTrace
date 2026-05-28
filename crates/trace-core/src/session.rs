//! [`Session`] aggregate — an ordered non-empty sequence of events with a
//! single [`SessionId`].

use thiserror::Error;

use crate::ids::SessionId;

/// Errors that can occur when constructing a [`Session`].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum SessionError {
    /// A session must contain at least one event.
    #[error("a session must contain at least one event")]
    Empty,
}

/// A recorded session: an ordered, non-empty sequence of events with a
/// single [`SessionId`].
///
/// Generic over the event type so the trace-core crate does not name
/// `TraceEvent` directly (which lives in `trace-schema`, and which
/// changes across versions through the upcaster chain — ADR-0006).
/// Binary crates typically instantiate `Session<trace_schema::Current>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session<E> {
    id: SessionId,
    events: Vec<E>,
}

impl<E> Session<E> {
    /// Build a session, asserting it is non-empty.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::Empty`] if `events` is empty.
    pub fn new(id: SessionId, events: Vec<E>) -> Result<Self, SessionError> {
        if events.is_empty() {
            return Err(SessionError::Empty);
        }
        Ok(Self { id, events })
    }

    /// The session identifier.
    #[must_use]
    pub const fn id(&self) -> &SessionId {
        &self.id
    }

    /// The recorded events, in order.
    #[must_use]
    pub fn events(&self) -> &[E] {
        &self.events
    }

    /// Number of recorded events; always `>= 1`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Always `false` — a [`Session`] is non-empty by construction. Kept
    /// to satisfy the clippy `len_without_is_empty` lint without
    /// suppressing it.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Decompose into `(id, events)`.
    #[must_use]
    pub fn into_parts(self) -> (SessionId, Vec<E>) {
        (self.id, self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::{Session, SessionError, SessionId};

    #[test]
    fn empty_session_rejected() {
        let id = SessionId::random();
        let result: Result<Session<()>, _> = Session::new(id, Vec::new());
        assert_eq!(result, Err(SessionError::Empty));
    }

    #[test]
    fn non_empty_session_preserves_events_and_id() {
        let id = SessionId::random();
        let session = Session::new(id, vec![1_u32, 2, 3]).expect("non-empty");
        assert_eq!(session.id(), &id);
        assert_eq!(session.events(), &[1, 2, 3]);
        assert_eq!(session.len(), 3);
        assert!(!session.is_empty());
    }
}
