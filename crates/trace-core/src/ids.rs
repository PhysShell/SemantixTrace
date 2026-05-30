//! Identifier newtypes for the `SemantxTrace` domain.
//!
//! Per ADR-0005 the schema speaks in semantic identifiers
//! (`CommandId`, `ScreenId`, `FieldId`) — never in physical UI
//! selectors. Per ADR-0012 every
//! identifier is a [`C-NEWTYPE`] wrapper so a `CommandId` cannot be
//! mistaken for a `ScreenId` at compile time.
//!
//! [`C-NEWTYPE`]: https://rust-lang.github.io/api-guidelines/type-safety.html#c-newtype

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A recorded session.
///
/// Identifies a single, ordered, non-empty sequence of [`TraceEvent`]s.
/// Aggregate-root key for the recording bounded context.
///
/// [`TraceEvent`]: ../../../trace_schema/type.Current.html
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Construct from a [`Uuid`].
    #[must_use]
    pub const fn new(id: Uuid) -> Self {
        Self(id)
    }

    /// Generate a fresh random session id (v4).
    #[must_use]
    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }

    /// The underlying [`Uuid`].
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Monotonic per-session event counter.
///
/// Gaps indicate dropped events; the sequence is per-[`SessionId`], not
/// global.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventSeq(pub u64);

impl EventSeq {
    /// Construct from a `u64`.
    #[must_use]
    pub const fn new(seq: u64) -> Self {
        Self(seq)
    }

    /// The first event in a session has seq `0`.
    pub const ZERO: Self = Self(0);

    /// Numeric value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Next sequence number; saturating.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

impl fmt::Display for EventSeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Semantic name of a command, e.g. `Graph47.Recalculate`.
///
/// Domain-meaningful identifier, never a physical control name (ADR-0005).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CommandId(pub String);

impl CommandId {
    /// Wrap a string.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CommandId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Semantic name of a screen, e.g. `DeclarationEditor`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScreenId(pub String);

impl ScreenId {
    /// Wrap a string.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ScreenId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Semantic name of a field, e.g. `Customer.Iin`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FieldId(pub String);

impl FieldId {
    /// Wrap a string.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Optional identifier linking related events (start / end of an async
/// operation, a command and its post-modal, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub Uuid);

impl CorrelationId {
    /// Construct from a [`Uuid`].
    #[must_use]
    pub const fn new(id: Uuid) -> Self {
        Self(id)
    }

    /// Generate a fresh random correlation id (v4).
    #[must_use]
    pub fn random() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for CorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Semantic identifier for the primary domain entity a trace event
/// applies to, e.g. `Declaration:doc-123` or `GoodsItem:47`.
///
/// Set by the adapter from the `ViewModel`'s ambient context;
/// `None` when the current screen has no single entity focus
/// (e.g. a list view with no row selected). Not masked by default:
/// entity ids are internal references, not user-supplied text.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DomainEntityId(pub String);

impl DomainEntityId {
    /// Wrap a string.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DomainEntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
