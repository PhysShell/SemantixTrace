//! `trace-normalizer` — value/temporal abstraction and scenario folding.
//!
//! Stage **S3** turns a raw [`Session`] of [`Current`] events into a
//! normalized [`Scenario`] (a sequence of [`CanonicalAction`]s) plus a
//! [`FoldReport`] describing what was abstracted away. The transform is
//! pure and deterministic.
//!
//! The pipeline:
//!
//! 1. **Value abstraction** ([`abstraction`]) — replace concrete values
//!    in command args / screen params with stable, idempotent buckets
//!    and format classes (`glossary.md` §4).
//! 2. **Folding** ([`fold`]) — track screen context, project
//!    `CommandExecuted` events into canonical `(screen, command,
//!    abstract_args)` triples, and collapse consecutive identical
//!    actions that occur within the configured burst gap.
//!
//! Idempotency (`glossary.md` §3): value abstraction is a fixed point
//! (`abstract_value(abstract_value(v)) == abstract_value(v)`), and
//! [`refold`] is endomorphic and idempotent on [`Scenario`]
//! (`refold(refold(s)) == refold(s)`). The Session→Scenario projection
//! itself is not endomorphic (different types), so the glossary's
//! `normalize(normalize(t)) == normalize(t)` is realized through these
//! two layers (see `docs/decisions.log.md`).
//!
//! [`Session`]: trace_core::Session
//! [`Scenario`]: trace_core::Scenario
//! [`CanonicalAction`]: trace_core::CanonicalAction
//! [`Current`]: trace_schema::Current

#![forbid(unsafe_code)]

pub mod abstraction;
pub mod config;
pub mod fold;

pub use config::NormCfg;
pub use fold::{normalize, refold, FoldReport};
