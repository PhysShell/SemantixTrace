//! `trace-schema` — versioned wire schema and upcaster chain for `SemantxTrace`.
//!
//! This is a placeholder for stage **S0** (workspace baseline). The `v1`
//! event module, the `Upcaster` trait, and the `read_event` dispatch land
//! in S1; see `docs/stages/S1-trace-core-and-schema-v1.md` and
//! `docs/upcasters.md`.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-schema (S0 placeholder)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(super::placeholder(), "trace-schema (S0 placeholder)");
    }
}
