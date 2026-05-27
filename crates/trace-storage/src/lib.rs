//! `trace-storage` — storage backends for `SemantxTrace`.
//!
//! Placeholder for stage **S0**. `JsonlBackend` lands in S2; `SqliteBackend`
//! in S8 (`sqlite` feature); `ParquetBackend` in S9 (`parquet` feature).
//! See the corresponding stage docs under `docs/stages/`.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-storage (S0 placeholder)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(super::placeholder(), "trace-storage (S0 placeholder)");
    }
}
