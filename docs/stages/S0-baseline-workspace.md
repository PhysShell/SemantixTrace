# S0: Baseline workspace, lint policy, CI

Status: planned
Depends on: —
ADRs: ADR-0001, ADR-0002, ADR-0004, ADR-0009, ADR-0010

## Goal

Turn the documentation-only repository into a buildable workspace skeleton.
Establish the lint policy, CI gates, and the ADR / stage / decisions-log
process before any domain code is written. **From the first commit**,
the workspace enforces the Rust API Guidelines lint surface (ADR-0012)
so the discipline does not have to be retrofitted later.

## Inputs / Outputs

- In: `docs/` (this commit's content).
- Out: `Cargo.toml` (workspace), empty member crates with `lib.rs`
  placeholders, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`,
  `deny.toml`, `.github/workflows/ci.yml`, isolated `fuzz/` crate per
  ADR-0010, and the `#![warn(missing_docs, …)]` lint preamble on every
  in-scope crate per ADR-0012.

## Approach

- Create the workspace members listed in [`../glossary.md`](../glossary.md)
  §12; each crate ships only a `lib.rs` with `pub fn _placeholder() {}`
  so the workspace builds.
- Configure `[workspace.lints.rust]` with `unsafe_code = "forbid"`,
  `missing_docs = "warn"`, `missing_debug_implementations = "warn"`,
  `missing_copy_implementations = "warn"`, `rust_2018_idioms = "warn"`,
  `unreachable_pub = "warn"`, `single_use_lifetimes = "warn"`,
  `unused_qualifications = "warn"`. Configure
  `[workspace.lints.clippy]` with `pedantic = "warn"` + selected
  `nursery` lints (`option_if_let_else`, `redundant_pub_crate`,
  `use_self`, `trivially_copy_pass_by_ref`). No blanket exceptions;
  per-item `#[allow(...)]` must be justified by a comment.
- `cargo-deny` config: licenses, advisories, sources, bans. License
  allowlist: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unicode-DFS-2016,
  CC0-1.0; everything else needs an entry in `deny.toml` with a
  reason.
- Workspace `Cargo.toml` metadata defaults (`rust-version`, `edition
  = "2021"`, `repository`, `homepage`, `documentation`, `readme`,
  `license`, `keywords`, `categories`) inherited via
  `[workspace.package]`; per-crate manifests inherit and override
  only `description` (`C-METADATA` from ADR-0012).
- GitHub Actions:
  - `cargo fmt --check`,
  - `cargo clippy --all-targets --all-features -- -D warnings -W
    clippy::pedantic`,
  - `cargo test --workspace`,
  - `cargo doc --all-features --no-deps -D warnings` (catches broken
    intra-doc links and `missing_docs` violations — ADR-0012),
  - `cargo deny check`,
  - bounded fuzz smoke + regression replay run wired up but blank
    (no targets yet).
- Stand up the isolated `fuzz/` crate skeleton with its own
  `rust-toolchain.toml` pinning nightly; no targets land here. The
  `fuzz/` crate is **out of scope** for the API Guidelines lints
  (ADR-0012 §1).

## Acceptance criteria

- `cargo build --workspace` passes on stable.
- `cargo fmt --check` is green workspace-wide.
- `cargo clippy --all-targets --all-features -- -D warnings -W
  clippy::pedantic` is green.
- `cargo doc --all-features --no-deps -D warnings` is green (no
  broken intra-doc links, no `missing_docs` warnings — every `pub`
  placeholder item in the empty crates carries at least a one-line
  doc).
- `cargo deny check` is green.
- `cargo test --workspace` passes (empty test suites are OK).
- Each in-scope crate's `lib.rs` starts with the API-Guidelines lint
  preamble per ADR-0012 §4.
- Each in-scope crate's `Cargo.toml` inherits `[workspace.package]`
  metadata and sets `description` per `C-METADATA`.
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
- [`../adr/0012-follow-rust-api-guidelines-on-public-surfaces.md`](../adr/0012-follow-rust-api-guidelines-on-public-surfaces.md)
- [`../glossary.md`](../glossary.md) §12
