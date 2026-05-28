//! Port traits — the hexagonal boundaries between domain crates and
//! their adapters (ADR-0002).
//!
//! Every cross-crate type goes through one of these traits. Concrete
//! implementations live in `trace-storage`, `trace-oracle`,
//! `trace-replay-planner`, the WPF / Avalonia adapters, etc., and may
//! pull heavy dependencies behind feature flags — but the trait
//! signatures here stay minimal.
//!
//! All trait methods are `Send + Sync`-friendly by signature (no
//! lifetimes that prevent send, no `&dyn` returns) per ADR-0012
//! `C-SEND-SYNC` — most consumers expect to fan oracles / backends out
//! across threads.

use std::error::Error;
use std::fmt::Debug;

use crate::session::Session;

/// Boxed iterator of `Result<Event, Error>` items — the shape every
/// [`StorageBackend::events`] implementation returns.
pub type EventStream<'a, Event, Error> = Box<dyn Iterator<Item = Result<Event, Error>> + 'a>;

/// Append-and-iter storage for trace events.
///
/// The error type is associated so backends can keep their natural
/// shapes (`std::io::Error` for JSONL, a typed `SqliteError`, …)
/// without forcing a top-level `Box<dyn Error>` everywhere.
pub trait StorageBackend {
    /// The event type the backend stores; binary crates instantiate with
    /// `trace_schema::Current`.
    type Event;
    /// The backend's error type.
    type Error: Error + Send + Sync + 'static;

    /// Append a single event to the backend.
    ///
    /// # Errors
    ///
    /// Returns the backend-specific error if the underlying write fails
    /// (file I/O, transaction conflict, validation, etc.).
    fn append(&mut self, event: &Self::Event) -> Result<(), Self::Error>;

    /// Iterate over the stored events, in append order.
    ///
    /// # Errors
    ///
    /// Returns the backend-specific error if reading fails before the
    /// iteration starts (file not found, schema mismatch, etc.). Errors
    /// encountered mid-stream are yielded from the iterator.
    fn events(&self) -> Result<EventStream<'_, Self::Event, Self::Error>, Self::Error>;
}

/// A source of trace events (file, in-memory queue, named pipe, …).
pub trait EventSource {
    /// The event type the source yields.
    type Event;
    /// The source's error type.
    type Error: Error + Send + Sync + 'static;

    /// Pull the next event, or return `Ok(None)` on clean EOF.
    ///
    /// # Errors
    ///
    /// Returns the source's error if the next read fails for any reason
    /// other than EOF.
    fn next_event(&mut self) -> Result<Option<Self::Event>, Self::Error>;
}

/// An oracle rule — a domain invariant that judges whether a session
/// (or a window within it) satisfies a contract.
///
/// Concrete rules live in `trace-oracle` (S5) plus user-defined rules
/// in downstream applications. The trait signature stays minimal so the
/// rule engine can schedule rules without knowing their internals.
pub trait OracleRule: Debug + Send + Sync {
    /// The event type the rule operates on; binary crates instantiate
    /// with `trace_schema::Current`.
    type Event;
    /// The output of an evaluation.
    type Result;

    /// Stable name for the rule, used in reports and config.
    fn name(&self) -> &str;

    /// Evaluate the rule against a [`Session`].
    fn evaluate(&self, session: &Session<Self::Event>) -> Self::Result;
}

/// Adapter that resolves semantic IDs ([`CommandId`], [`ScreenId`], …)
/// to physical interactions during replay.
///
/// Implementations live per UI framework — `Trace.Wpf` ships the first
/// one alongside S6 / S11.
///
/// [`CommandId`]: crate::ids::CommandId
/// [`ScreenId`]: crate::ids::ScreenId
pub trait ReplayAdapter {
    /// The replay-plan type this adapter executes.
    type Plan;
    /// The adapter's error type.
    type Error: Error + Send + Sync + 'static;

    /// Drive the application through `plan`.
    ///
    /// # Errors
    ///
    /// Returns the adapter-specific error if any step fails to resolve
    /// to a physical interaction or the application diverges from the
    /// plan beyond declared tolerances.
    fn replay(&mut self, plan: &Self::Plan) -> Result<(), Self::Error>;
}

/// Emits a human-readable artefact from analysis output (HTML, `JUnit`
/// XML, Markdown, …).
pub trait Reporter {
    /// The input shape the reporter renders.
    type Input;
    /// The reporter's error type.
    type Error: Error + Send + Sync + 'static;

    /// Write a report to the given sink.
    ///
    /// # Errors
    ///
    /// Returns the reporter-specific error if rendering or writing
    /// fails.
    fn write_report<W: std::io::Write>(
        &self,
        input: &Self::Input,
        sink: &mut W,
    ) -> Result<(), Self::Error>;
}
