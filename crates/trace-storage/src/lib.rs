//! `trace-storage` — storage backends for `SemantxTrace`.
//!
//! Stage **S2** ships [`JsonlBackend`] — the canonical, append-only
//! JSON Lines backend (ADR-0003). `SqliteBackend` (S8, `sqlite` feature)
//! and `ParquetBackend` (S9, `parquet` feature) land later behind their
//! own feature flags.
//!
//! All read paths funnel through [`trace_schema::read_event`], so callers
//! only ever observe [`trace_schema::Current`] regardless of the
//! on-disk schema version (ADR-0006).

#![forbid(unsafe_code)]

pub mod jsonl;

pub use jsonl::{JsonlBackend, JsonlError};
