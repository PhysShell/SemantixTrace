# ADR 0002: Adopt hexagonal architecture with Rust traits as ports

Date: 2026-05-27
Status: Accepted

## Context

The system has well-defined boundaries between (a) the domain kernel
(events, sessions, scenarios, oracles), (b) external I/O (file backends,
named pipes, IPC, OTLP exporters), and (c) UI-framework adapters (WPF,
Avalonia, MAUI). Coupling them yields a project that can never swap a
storage backend without rewriting normalizers, or add a new UI adapter
without touching the schema. Rust's trait system gives us hexagonal
boundaries without an enterprise DI container.

## Decision

We organise the codebase as a hexagonal architecture (Ports & Adapters,
Cockburn). Every cross-crate type goes through a trait declared in
`trace-core` or its nearest domain crate. The known ports at v0.1 are:

- `StorageBackend` — `append(&mut self, e: &TraceEvent)`, `iter(&self) ->
  …`.
- `EventSource` — pluggable input (file, named pipe, in-memory).
- `OracleRule` — `evaluate(&self, ctx, event) -> OracleResult`.
- `ReplayAdapter` — resolves semantic IDs to physical interactions.
- `Reporter` — emits human-readable output (HTML, Markdown, JUnit XML).

Domain crates (`trace-core`, `trace-normalizer`, `trace-graph`,
`trace-oracle`) depend only on these traits. Concrete implementations
live in `trace-storage`, `trace-wpf`, etc., and may bring heavy
dependencies behind feature flags.

DDD-lite is used pragmatically: value objects (`TraceEvent`,
`ActionNode`, `OracleResult`), aggregates (`Session`, `Scenario`),
ubiquitous language (`docs/glossary.md`). No CQRS, no event-sourcing-as-DB,
no enterprise saga orchestration.

## Consequences

- Swapping or adding a backend is a new crate or a new feature flag, not
  a rewrite of domain logic.
- Tests for normalizer / graph / oracle can use in-memory or stub
  backends without ceremony.
- Premature port creation is a known anti-pattern: only crystallise a
  port when there are two real consumers or a real adapter, never on
  speculation.
- New contributors must learn the inward dependency rule. Reach-through
  imports (e.g. `trace-normalizer` depending on `trace-storage`
  directly) are review-blockers.
