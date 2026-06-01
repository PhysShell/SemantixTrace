//! `trace-storage` — storage backends for `SemantxTrace`.
//!
//! Stage **S2** ships [`JsonlBackend`] — the canonical, append-only
//! JSON Lines backend (ADR-0003). [`SqliteBackend`] lands in S8 behind
//! the `sqlite` feature; `ParquetBackend` lands in S9 behind `parquet`.
//!
//! All read paths funnel through [`trace_schema::read_event`], so callers
//! only ever observe [`trace_schema::Current`] regardless of the
//! on-disk schema version (ADR-0006).

#![forbid(unsafe_code)]

pub mod extract;
pub mod jsonl;

pub use jsonl::{read_events, JsonlBackend, JsonlError};

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "sqlite")]
pub use sqlite::{DimensionSet, SliceBy, SqliteBackend, SqliteError};
