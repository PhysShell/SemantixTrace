//! Composition primitives: [`AndRule`], [`OrRule`], [`WithinWindowRule`].

use trace_core::Session;
use trace_schema::Current;

use crate::rule::{OracleResult, OracleSchedule, OracleViolation, Rule};

/// Passes iff ALL inner rules pass.
#[derive(Debug)]
pub struct AndRule {
    /// Inner rules — all must pass.
    pub rules: Vec<Box<dyn Rule>>,
    /// Stable name for this composite rule.
    pub name: String,
}

impl AndRule {
    /// Construct a named AND composition from a vec of rules.
    pub fn new(name: impl Into<String>, rules: Vec<Box<dyn Rule>>) -> Self {
        Self {
            name: name.into(),
            rules,
        }
    }
}

impl Rule for AndRule {
    fn name(&self) -> &str {
        &self.name
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::PerScenario
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let all_violations: Vec<OracleViolation> = self
            .rules
            .iter()
            .flat_map(|r| r.evaluate(session).violations)
            .collect();
        OracleResult::fail(self.name(), all_violations)
    }
}

/// Passes iff ANY inner rule passes.
#[derive(Debug)]
pub struct OrRule {
    /// Inner rules — at least one must pass.
    pub rules: Vec<Box<dyn Rule>>,
    /// Stable name for this composite rule.
    pub name: String,
}

impl OrRule {
    /// Construct a named OR composition from a vec of rules.
    pub fn new(name: impl Into<String>, rules: Vec<Box<dyn Rule>>) -> Self {
        Self {
            name: name.into(),
            rules,
        }
    }
}

impl Rule for OrRule {
    fn name(&self) -> &str {
        &self.name
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::PerScenario
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let results: Vec<OracleResult> = self.rules.iter().map(|r| r.evaluate(session)).collect();
        if results.iter().any(|r| r.passed) {
            OracleResult::pass(self.name())
        } else {
            let violations = results.into_iter().flat_map(|r| r.violations).collect();
            OracleResult::fail(self.name(), violations)
        }
    }
}

/// Applies an inner rule only to events within a rolling `window_ms` window.
///
/// Splits the session's events into windows by timestamp, evaluates the
/// inner rule on each window, and merges the results.  Window filtering is a
/// scheduling concern handled by the engine; for direct [`Rule::evaluate`]
/// calls the inner rule is invoked on the full session unchanged.
#[derive(Debug)]
pub struct WithinWindowRule {
    /// The rule to apply within each window.
    pub inner: Box<dyn Rule>,
    /// Window width in milliseconds.
    pub window_ms: u64,
}

impl WithinWindowRule {
    /// Construct a window-based wrapper around `inner`.
    #[must_use]
    pub fn new(inner: Box<dyn Rule>, window_ms: u64) -> Self {
        Self { inner, window_ms }
    }
}

impl Rule for WithinWindowRule {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn schedule(&self) -> OracleSchedule {
        OracleSchedule::WindowBased {
            window_ms: self.window_ms,
        }
    }

    fn evaluate(&self, session: &Session<Current>) -> OracleResult {
        let events = session.events();
        let Some(first) = events.first() else {
            return OracleResult::pass(self.name());
        };
        let t0 = first.ts;
        let window_ms = i64::try_from(self.window_ms).unwrap_or(i64::MAX);
        let windowed: Vec<Current> = events
            .iter()
            .filter(|e| {
                let delta = (e.ts - t0).num_milliseconds();
                // Only include events within [0, window_ms]; negative deltas
                // (out-of-order timestamps) are excluded.
                delta >= 0 && delta <= window_ms
            })
            .cloned()
            .collect();
        Session::new(*session.id(), windowed)
            .map_or_else(|_| OracleResult::pass(self.name()), |sub| self.inner.evaluate(&sub))
    }
}
