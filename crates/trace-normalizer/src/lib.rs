//! `trace-normalizer` — value/temporal abstraction and scenario folding.
//!
//! Placeholder for stage **S0**. The `normalize(session, &NormCfg) ->
//! Scenario` implementation lands in S3; see
//! `docs/stages/S3-trace-normalizer.md`.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-normalizer (S0 placeholder)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(super::placeholder(), "trace-normalizer (S0 placeholder)");
    }
}
