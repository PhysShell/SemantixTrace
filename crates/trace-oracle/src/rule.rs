//! Core oracle rule types: [`OracleSchedule`], [`Severity`],
//! [`OracleViolation`], [`OracleResult`], and the object-safe [`Rule`] trait.

use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use trace_core::Session;
use trace_schema::Current;

/// When the engine calls a rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OracleSchedule {
    /// Called once for every event in stream order.
    PerEvent,
    /// Called once after all events for a session.
    PerScenario,
    /// Called once after the very last event of the session (synonym for
    /// `PerScenario`; useful for stateful streaming rules).
    EndOfSession,
    /// Called with a rolling window of events within `window_ms`.
    WindowBased {
        /// Window width in milliseconds.
        window_ms: u64,
    },
}

/// Severity of an oracle violation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Potential issue; investigation recommended.
    Warning,
    /// Definite violation that blocks a scenario from being replayed.
    Error,
    /// Blocker: the session is unsafe to process further.
    Critical,
}

/// A single rule violation, carrying the evidence.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleViolation {
    /// Name of the rule that produced this violation.
    pub rule: String,
    /// Severity level.
    pub severity: Severity,
    /// Human-readable description.
    pub message: String,
    /// Sequence numbers of offending events (for report linking).
    pub evidence: Vec<trace_core::EventSeq>,
}

/// The output of evaluating one rule against one session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleResult {
    /// Name of the evaluated rule.
    pub rule: String,
    /// `true` iff no violations were found.
    pub passed: bool,
    /// All violations found (empty when `passed = true`).
    pub violations: Vec<OracleViolation>,
}

impl OracleResult {
    /// Construct a passing result for the given rule.
    #[must_use]
    pub fn pass(rule: &str) -> Self {
        Self {
            rule: rule.to_owned(),
            passed: true,
            violations: vec![],
        }
    }

    /// Construct a result from a (possibly empty) violation list.
    ///
    /// Sets `passed` to `true` iff `violations` is empty.
    #[must_use]
    pub fn fail(rule: &str, violations: Vec<OracleViolation>) -> Self {
        let passed = violations.is_empty();
        Self {
            rule: rule.to_owned(),
            passed,
            violations,
        }
    }
}

/// Object-safe oracle rule trait used by the engine.
///
/// Distinct from the generic [`trace_core::ports::OracleRule`] port (which
/// uses associated types and cannot be made into a trait object); this
/// concrete trait is what [`crate::OracleEngine`] stores as `Box<dyn Rule>`.
pub trait Rule: Debug + Send + Sync {
    /// Stable identifier, used in reports and configuration.
    fn name(&self) -> &str;
    /// Scheduling hint for the engine.
    fn schedule(&self) -> OracleSchedule;
    /// Evaluate this rule against a full session.
    fn evaluate(&self, session: &Session<Current>) -> OracleResult;
}
