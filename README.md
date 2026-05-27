# SemantxTrace

> Production-informed UI regression testing for desktop apps.
> Rust core. Framework-agnostic adapters. Semantic action model, not pixel diff.

SemantxTrace records what *actually* happens in a running desktop app at the
level of named domain actions (`Graph47.Recalculate`, not
`Window.Grid.Button[3].Click`), mines workflows from those traces, evaluates
oracles against them, and turns them into replay plans that survive UI
redesigns.

This repository is currently **documentation only**. Code lands stage by stage
starting at S0 (see [`docs/stages/`](docs/stages/)).

## Status

Pre-S0. The workspace, lint policy, CI, and crates listed below do not exist
yet — they are scoped in stage S0 / S1. The documentation in `docs/` is the
binding contract for what gets built.

## Planned workspace

- `crates/trace-core/` — `TraceEvent`, `Session`, `Scenario` value objects;
  no I/O, no UI, no third-party storage.
- `crates/trace-schema/` — versioned JSON schema, serde models, the upcaster
  chain (ADR-0006).
- `crates/trace-storage/` — `StorageBackend` port plus JSONL (MVP), SQLite
  (v0.2), Parquet (v0.3) adapters behind feature flags.
- `crates/trace-normalizer/` — value/temporal abstraction, equivalence
  classes, scenario folding.
- `crates/trace-graph/` — `petgraph` wrapper, Heuristics miner (MVP),
  Inductive miner (v0.2).
- `crates/trace-oracle/` — `OracleRule` trait, built-in rules, composition.
- `crates/trace-replay-planner/` — scenario → `ReplayPlan` JSON (v0.4).
- `crates/trace-cli/` — `trace` binary (`analyze` / `normalize` / `graph` /
  `oracle run` / `plan generate` / `report`).
- `crates/trace-viewer/` — ratatui TUI, deferred to post-v1.0.
- `adapters/trace-wpf/` — .NET / NuGet adapter (`[TraceCommand]`,
  `[ScreenId]`, `AutoAutomationId` behavior, JSONL sink).
- `adapters/trace-avalonia/` — v0.3.

## Documentation

- [`docs/SPEC.md`](docs/SPEC.md) — what SemantxTrace is, is not, and the
  hard rules that do not bend.
- [`docs/glossary.md`](docs/glossary.md) — the constitution: every term used
  in specs, ADRs, stage docs, and code.
- [`docs/stages/`](docs/stages/) — canonical roadmap **S0 … S12** leading to
  v1.0 stable.
- [`docs/adr/`](docs/adr/README.md) — architecture decisions, Nygard format.
- [`docs/upcasters.md`](docs/upcasters.md) — schema-evolution pattern
  (ADR-0006); how `V_n → V_n+1 → … → Current` chains keep historical traces
  readable without migrations.
- [`docs/privacy.md`](docs/privacy.md) — mask-by-default policy.
- [`docs/fuzzing.md`](docs/fuzzing.md) — fuzz-testing policy for storage
  parsers and upcaster chains.
- [`AGENTS.md`](AGENTS.md) — guide for AI agents and human contributors.

## Build

Nothing to build yet. After S0:

```
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

## License

To be decided in ADR-0011 (planned). Working assumption: **MIT** for Rust
crates and the WPF adapter; commercial dual-license for enterprise add-ons.
See SPEC §6.

## Naming

The project work title is **SemantxTrace**. The GitHub repository is
`PhysShell/SemantixTrace` for legacy reasons; do not rename the binary,
crates, or schema namespace based on the repo slug.
