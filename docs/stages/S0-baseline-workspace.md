# S0: Baseline workspace, lint policy, CI

Status: planned
Depends on: —
ADRs: ADR-0001, ADR-0002, ADR-0004, ADR-0009, ADR-0010

## Goal

Turn the documentation-only repository into a buildable workspace skeleton.
Establish the lint policy, CI gates, and the ADR / stage / decisions-log
process before any domain code is written.

## Inputs / Outputs

- In: `docs/` (this commit's content).
- Out: `Cargo.toml` (workspace), empty member crates with `lib.rs`
  placeholders, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`,
  `deny.toml`, `.github/workflows/ci.yml`, isolated `fuzz/` crate per
  ADR-0010.

## Approach

- Create the workspace members listed in [`../glossary.md`](../glossary.md)
  §12; each crate ships only a `lib.rs` with `pub fn _placeholder() {}`
  so the workspace builds.
- Configure `[workspace.lints.rust]` with `unsafe_code = "forbid"` and
  `[workspace.lints.clippy]` with `pedantic = "warn"` + selected
  `nursery` lints. No exceptions.
- `cargo-deny` config for license + supply-chain hygiene.
- GitHub Actions: `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings`, `cargo test --workspace`, `cargo deny check`. Bounded fuzz
  smoke + regression replay run is wired up but blank (no targets yet).
- Stand up the isolated `fuzz/` crate skeleton with its own
  `rust-toolchain.toml` pinning nightly; no targets land here.

## Acceptance criteria

- `cargo build --workspace` passes on stable.
- `cargo fmt --check` is green workspace-wide.
- `cargo clippy --all-targets -- -D warnings` is green.
- `cargo deny check` is green.
- `cargo test --workspace` passes (empty test suites are OK).
- CI runs all the above on every PR.
- The `fuzz/` crate builds on `cargo +nightly fuzz build`; no targets
  defined yet.

## Open questions

- MSRV: pin at 1.80 (current stable as of the project start) or hold
  1.74 as in griff? Decided in S0 itself, recorded in
  `decisions.log.md`.
- License: confirmed MIT for the Rust workspace in a follow-up ADR; not
  blocking S0.

## See also

- [`../adr/0001-use-rust-workspace.md`](../adr/0001-use-rust-workspace.md)
- [`../adr/0004-forbid-unsafe-code.md`](../adr/0004-forbid-unsafe-code.md)
- [`../adr/0010-fuzz-storage-parsers-and-upcaster-chain.md`](../adr/0010-fuzz-storage-parsers-and-upcaster-chain.md)
- [`../glossary.md`](../glossary.md) §12
