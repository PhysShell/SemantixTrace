# SemantxTrace

> Behavioral observability for desktop UI applications.
> One canonical semantic trace, seven projections: analytics, UX, product,
> support, regression tests, replay, exploration.
> Rust core. Framework-agnostic adapters. Semantic action model, not pixel
> diff and not contextless click counters.

SemantxTrace records what *actually* happens in a running desktop app at
the level of named domain actions (`Graph47.Recalculate`, not
`Window.Grid.Button[3].Click`) **with the scenario context attached**
(which document, which screen, what came before, what came after). The
same trace then fans out into analytics over real user workflows,
diagnostic packages for support, regression test candidates, replay
plans that survive UI redesigns, and domain-aware mutations for guided
exploration — see ADR-0011.

Elevator pitch: **semantic metrics, not contextless counters.** Counting
"Export clicked 1200 times" is bookkeeping; recording the scenario
("open declaration → edit goods → recalculate → export, ending in
success or `ErrorModal`") is the same event answering product, UX,
support, and test questions at once.

Code lands stage by stage; the per-stage `Status:` lines in
[`docs/stages/`](docs/stages/) are the single source of truth for what
has shipped — this README intentionally repeats only the headline.

## Status

**S0–S8 landed.** The workspace below is real and CI-gated: schema
v1+v2 with the upcaster chain, JSONL and SQLite backends, the
normalizer (value/temporal abstraction, scenario folding), Heuristics
and Inductive miners, the oracle engine with built-in rules, the
`trace` CLI, the WPF adapter packages, and the DeclarationApp demo.
S9 (Parquet archive tier) through S12 (v1.0 stable) remain planned.

## Workspace

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

```
# Rust workspace
cargo build --workspace
cargo test  --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
cargo doc --workspace --all-features --no-deps   # RUSTDOCFLAGS=-D warnings

# Try the CLI
cargo run -p trace-cli -- version --output json
cargo run -p trace-cli -- analyze crates/trace-cli/tests/fixtures/multi_session.jsonl
cargo run -p trace-cli -- normalize crates/trace-cli/tests/fixtures/multi_session.jsonl --report

# .NET adapters (requires .NET 8.0 SDK)
dotnet build adapters/Trace.Abstractions/Trace.Abstractions.csproj -c Release -warnaserror
dotnet build adapters/Trace.Avalonia/Trace.Avalonia.csproj         -c Release -warnaserror
dotnet build adapters/Trace.Maui/Trace.Maui.csproj                 -c Release -warnaserror
# Trace.Wpf is Windows-only:
# dotnet build adapters/Trace.Wpf/Trace.Wpf.csproj                 -c Release -warnaserror

# Isolated fuzz/ crate (nightly)
cargo build --manifest-path fuzz/Cargo.toml
```

## License

The Rust workspace and adapters currently declare **MIT** in their
manifests. A dedicated licensing ADR (including the dual-license
question for enterprise add-ons) is still pending; ADR-0011 turned out
to be taken by the multi-projection decision. See SPEC §6.

## Naming

The project work title is **SemantxTrace**. The GitHub repository is
`PhysShell/SemantixTrace` for legacy reasons; do not rename the binary,
crates, or schema namespace based on the repo slug.
