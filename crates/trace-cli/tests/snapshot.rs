//! `trycmd`-anchored snapshot tests for the `trace` binary surface.
//!
//! Per ADR-0014 §12, every subcommand carries at least a success-shape
//! and a help snapshot. S0 only ships `version` and `completions <shell>`;
//! more snapshots land alongside each future stage that adds subcommands.

#[test]
fn cli_snapshots() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd");
}
