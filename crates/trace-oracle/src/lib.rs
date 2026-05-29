//! `trace-oracle` — oracle rule engine for `SemantxTrace` (S5).
//!
//! Core types: [`OracleSchedule`], [`Severity`], [`OracleViolation`],
//! [`OracleResult`], [`Rule`] trait, [`OracleEngine`].
//! Built-in rules: [`NoUnhandledException`], [`NoErrorModalAfterCommand`],
//! [`CommandMustSucceed`], [`ScreenMustNavigateForward`],
//! [`ValidationsPassBeforeSubmit`].
//! Composition: [`AndRule`], [`OrRule`], [`WithinWindowRule`].
//! Reporting: [`HtmlReporter`].

#![forbid(unsafe_code)]

pub mod compose;
pub mod engine;
pub mod reporter;
pub mod rule;
pub mod rules;

pub use compose::{AndRule, OrRule, WithinWindowRule};
pub use engine::OracleEngine;
pub use reporter::HtmlReporter;
pub use rule::{OracleResult, OracleSchedule, OracleViolation, Rule, Severity};
pub use rules::{
    CommandMustSucceed, NoErrorModalAfterCommand, NoUnhandledException, ScreenMustNavigateForward,
    ValidationsPassBeforeSubmit,
};
