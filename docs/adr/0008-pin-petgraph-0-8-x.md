# ADR 0008: Pin petgraph to 0.8.x until 0.9 stabilises

Date: 2026-05-27
Status: Accepted

## Context

`trace-graph` is built on `petgraph`. The 0.8.x line is stable and
widely deployed (hundreds of millions of downloads on crates.io). The
0.9 work in trunk reorganises the crate into a multi-crate layout and
is not API-stable as of this ADR. SemantxTrace cannot accept silent
breaking changes on a load-bearing dependency.

## Decision

We pin `petgraph = "0.8"` in `[workspace.dependencies]` with no caret
upgrade past the 0.8 minor line. Bumping to 0.9 requires a follow-up
ADR documenting the API migration and an explicit acceptance run of all
`trace-graph` property tests and snapshot fixtures.

## Consequences

- Predictable behaviour and stable benchmarks for the v1.0 cycle.
- We accept slower access to 0.9 features (multi-graph types, new
  algorithms) until the migration ADR lands.
- Any third-party crate we adopt that demands `petgraph = "0.9"` is
  blocked until the migration completes.
