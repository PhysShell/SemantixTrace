//! `trace-replay-planner` — replay-plan generation, semantic-monkey,
//! and trace mutation for `SemantxTrace`.
//!
//! Placeholder for stage **S0**. The `ReplayPlan` schema, `ReplayMode`
//! (`strict` / `relaxed`), `semantic_monkey(...)`, and the
//! `MutationKind` catalogue land in S11; see
//! `docs/stages/S11-replay-planner-semantic-monkey-and-trace-mutation.md`.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-replay-planner (S0 placeholder)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(
            super::placeholder(),
            "trace-replay-planner (S0 placeholder)"
        );
    }
}
