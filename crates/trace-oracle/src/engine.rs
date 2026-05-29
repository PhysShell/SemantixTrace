//! [`OracleEngine`] — runs oracle rules against sessions.

use trace_core::{Session, SessionId};
use trace_schema::Current;

use crate::rule::{OracleResult, Rule, Severity};

/// Runs a set of oracle rules against one or more sessions.
#[derive(Debug)]
pub struct OracleEngine {
    rules: Vec<Box<dyn Rule>>,
}

/// The combined results for one session.
#[derive(Debug, serde::Serialize)]
pub struct SessionReport {
    /// The session that was evaluated.
    pub session_id: SessionId,
    /// One result per rule.
    pub results: Vec<OracleResult>,
}

impl SessionReport {
    /// Highest severity across all violations in this session.
    #[must_use]
    pub fn max_severity(&self) -> Option<Severity> {
        self.results
            .iter()
            .flat_map(|r| r.violations.iter().map(|v| v.severity))
            .max()
    }

    /// True iff all rules passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }
}

impl OracleEngine {
    /// Create an empty engine with no rules registered.
    #[must_use]
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Register an additional rule.
    pub fn add_rule(&mut self, rule: Box<dyn Rule>) {
        self.rules.push(rule);
    }

    /// Add all five built-in rules with default configuration.
    #[must_use]
    pub fn with_builtin_rules(mut self) -> Self {
        use crate::rules::{
            CommandMustSucceed, NoErrorModalAfterCommand, NoUnhandledException,
            ScreenMustNavigateForward, ValidationsPassBeforeSubmit,
        };
        self.rules.push(Box::new(NoUnhandledException));
        self.rules.push(Box::new(NoErrorModalAfterCommand));
        self.rules.push(Box::new(
            CommandMustSucceed::new(std::iter::empty::<&str>()),
        ));
        self.rules.push(Box::new(ScreenMustNavigateForward));
        self.rules
            .push(Box::new(ValidationsPassBeforeSubmit::default()));
        self
    }

    /// All registered rules.
    #[must_use]
    pub fn rules(&self) -> &[Box<dyn Rule>] {
        &self.rules
    }

    /// Evaluate all rules against `session` and return a [`SessionReport`].
    #[must_use]
    pub fn run(&self, session: &Session<Current>) -> SessionReport {
        let results = self.rules.iter().map(|r| r.evaluate(session)).collect();
        SessionReport {
            session_id: *session.id(),
            results,
        }
    }

    /// Evaluate all rules against every session in `sessions`.
    #[must_use]
    pub fn run_all(&self, sessions: &[Session<Current>]) -> Vec<SessionReport> {
        sessions.iter().map(|s| self.run(s)).collect()
    }
}

impl Default for OracleEngine {
    fn default() -> Self {
        Self::new()
    }
}
