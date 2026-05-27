//! `trace-core` — domain value objects for `SemantxTrace`.
//!
//! This is a placeholder for stage **S0** (workspace baseline). The domain
//! types (`TraceEvent`, `Session`, `Scenario`, port traits) land in S1; see
//! `docs/stages/S1-trace-core-and-schema-v1.md`.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-core (S0 placeholder)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(super::placeholder(), "trace-core (S0 placeholder)");
    }
}
