//! `trace-graph` — action-graph mining for `SemantxTrace`.
//!
//! Placeholder for stage **S0**. The `ActionGraph` type, Heuristics miner,
//! `PrefixSpan`, Mermaid / DOT exporters land in S4; Inductive miner in S8
//! (`inductive-miner` feature). See `docs/stages/S4-action-graph-and-
//! heuristics-miner.md` and `docs/stages/S8-sqlite-backend-and-inductive-
//! miner.md`.

/// Placeholder symbol so the crate builds during S0.
#[must_use]
pub const fn placeholder() -> &'static str {
    "trace-graph (S0 placeholder)"
}

#[cfg(test)]
mod tests {
    #[test]
    fn returns_placeholder_string() {
        assert_eq!(super::placeholder(), "trace-graph (S0 placeholder)");
    }
}
