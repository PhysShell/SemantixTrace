# ADR 0001: Use a Rust workspace with trace-core / trace-schema / … / trace-cli crates

Date: 2026-05-27
Status: Accepted

## Context

SemantxTrace ships a domain kernel, a versioned wire schema, multiple
storage backends, a normalizer, a graph layer, an oracle engine, a
replay-planner, a CLI binary, and several optional adapters. These share
musical-grade value-object types and depend on each other in a strict
inward-facing chain (hexagonal, ADR-0002). They also need independent
feature flags (`sqlite`, `parquet`, `avalonia`) and independent release
cadences.

## Decision

We use a Cargo workspace with the members listed in
[`AGENTS.md`](../../AGENTS.md): `trace-core`, `trace-schema`,
`trace-storage`, `trace-normalizer`, `trace-graph`, `trace-oracle`,
`trace-replay-planner`, `trace-cli`, `trace-viewer`. Common dependencies
(`serde`, `serde_json`, `chrono`, `uuid`, `thiserror`, `clap`,
`petgraph`) are pinned in `[workspace.dependencies]`. Lints are inherited
via `[workspace.lints]`. Resolver 2. The isolated `fuzz/` crate is
**excluded** from the workspace (ADR-0010).

The .NET adapters (`trace-wpf`, `trace-avalonia`, `trace-maui`) live
under `adapters/` and are out of the Cargo workspace; they have their
own `.csproj` and CI lane.

## Consequences

- One `cargo test --workspace` and one `cargo clippy --all-targets -- -D
  warnings`; shared, strict lint config; per-crate features without
  manifest duplication.
- More boilerplate per new crate; a workspace-wide MSRV bump affects all
  crates.
- The .NET adapters must reach Rust artefacts via packaged dylibs / NuGet
  pre-built binaries; cross-language workspace tooling is not attempted.
