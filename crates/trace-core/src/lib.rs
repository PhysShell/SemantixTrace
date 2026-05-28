//! `trace-core` — domain value objects for `SemantxTrace`.
//!
//! Stage **S1** lands the first real domain surface (see
//! `docs/stages/S1-trace-core-and-schema-v1.md`):
//!
//! - Strongly-typed newtypes for identifiers ([`SessionId`], [`EventSeq`],
//!   [`CommandId`], [`ScreenId`], [`FieldId`], [`CorrelationId`]).
//! - Domain enums ([`Outcome`], [`ValuePolicy`]).
//! - Aggregates ([`Session`], [`Scenario`]).
//! - Port traits ([`ports::StorageBackend`], [`ports::EventSource`],
//!   [`ports::OracleRule`], [`ports::ReplayAdapter`], [`ports::Reporter`]).
//!
//! The crate's full dependency closure is `chrono`, `serde`,
//! `serde_json`, `thiserror`, `uuid` (per ADR-0002).
//!
//! The on-wire envelope and per-version event types live in
//! `trace-schema`. The alias `trace_schema::Current` is re-exported as
//! `TraceEvent` here once a binary depends on both crates; this crate
//! itself does not name `TraceEvent` to keep the dependency direction
//! pure (ADR-0002).

#![forbid(unsafe_code)]

pub mod ids;
pub mod outcome;
pub mod policy;
pub mod ports;
pub mod scenario;
pub mod session;

pub use ids::{CommandId, CorrelationId, EventSeq, FieldId, ScreenId, SessionId};
pub use outcome::Outcome;
pub use policy::ValuePolicy;
pub use scenario::{CanonicalAction, Scenario};
pub use session::{Session, SessionError};
