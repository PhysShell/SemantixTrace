//! `trace-oracle` — oracle rule engine for `SemantxTrace`.
//!
//! Placeholder for stage **S0**. The `OracleRule` trait, `OracleSchedule`,
//! composition primitives, and the five built-in rules land in S5; see
//! `docs/stages/S5-oracle-engine-and-builtin-rules.md`.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-oracle (S0 placeholder)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(super::placeholder(), "trace-oracle (S0 placeholder)");
    }
}
