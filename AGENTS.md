# SemantxTrace — agent guide

## Why

`SemantxTrace` records and analyses **semantic** user actions in desktop apps
(`Graph47.Recalculate`, `Declaration.Validate`) — not physical UI events
(`Button.Click` on `Window > Grid > StackPanel[1]`). Real production sessions
get normalized into scenarios, mined into action graphs, checked by oracles,
and turned into replay plans that survive UI redesigns. The unique angle is
the **semantic action map**, decoupled from the physical UI map (ADR-0005).

## What (project map)

- `crates/trace-core/`            — domain value objects (`TraceEvent`,
  `Session`, `Scenario`). No I/O.
- `crates/trace-schema/`          — versioned JSON Schema and the upcaster
  chain (ADR-0006, [`docs/upcasters.md`](docs/upcasters.md)).
- `crates/trace-storage/`         — `StorageBackend` port + JSONL/SQLite/
  Parquet adapters (feature-gated).
- `crates/trace-normalizer/`      — value/temporal abstraction, equivalence
  classes, scenario folding.
- `crates/trace-graph/`           — `petgraph` 0.8.x wrapper, Heuristics
  miner, Inductive miner (v0.2).
- `crates/trace-oracle/`          — `OracleRule` trait + built-in rules.
- `crates/trace-replay-planner/`  — scenario → `ReplayPlan` JSON (v0.4).
- `crates/trace-cli/`             — `trace` binary.
- `crates/trace-viewer/`          — ratatui TUI (deferred).
- `adapters/trace-wpf/`           — .NET / NuGet (`[TraceCommand]`, ScreenId
  attached behavior, JSONL sink).
- `adapters/trace-avalonia/`      — v0.3.
- `fuzz/`                         — isolated nightly cargo-fuzz crate
  (ADR-0010); not a workspace member.
- `docs/`                         — knowledge base; start at
  [`docs/SPEC.md`](docs/SPEC.md).

## Constitution

[`docs/glossary.md`](docs/glossary.md) is authoritative. On any term
conflict, defer to it; extend it rather than inventing synonyms in code.
The repository starts as **documentation only** (S0 has not happened yet);
nothing in `crates/` or `adapters/` exists at HEAD.

## How (commands — once S0 lands)

- Test:    `cargo test --workspace`
- Lint:    `cargo clippy --all-targets -- -D warnings`
- Format:  `cargo fmt --all` (`--check` in CI)
- Docs:    `cargo doc --no-deps --workspace`
- Fuzz:    `cargo +nightly fuzz run upcaster_v1_to_current` (from repo root;
  see [`docs/fuzzing.md`](docs/fuzzing.md))
- .NET:    `dotnet test adapters/trace-wpf/`

## Routing

- A roadmap stage? → `docs/stages/SN-*.md` (canonical S0…S12, ending at
  v1.0 stable).
- An architectural decision? → new ADR in `docs/adr/` (Nygard, ADR-0009).
- A small decision? → append to `docs/decisions.log.md`.
- Schema-version question? → [`docs/upcasters.md`](docs/upcasters.md) +
  ADR-0006.
- Privacy question? → [`docs/privacy.md`](docs/privacy.md) + ADR-0007.
- Fuzzing question? → [`docs/fuzzing.md`](docs/fuzzing.md) + ADR-0010.
- A term? → `docs/glossary.md`.
- Scope question? → `docs/SPEC.md`.

## TDD / property-test workflow (mandatory for core crates)

Methodology is intentionally lopsided (SPEC hard rule 7):

1. **`trace-core`, `trace-schema`, `trace-normalizer`, `trace-oracle`** —
   strict red→green→refactor with `proptest` invariants. Write failing
   tests first and commit them before any implementation. Never commit a
   new public function in the same commit as its tests.
2. **`trace-graph`, `trace-storage` (parsers)** — characterization tests
   plus property tests; TDD when feasible.
3. **Adapters (`trace-wpf`, `trace-avalonia`), demo apps, CLI ergonomics**
   — integration-test driven. TDD is not required there; the cycle is too
   slow to be honest.

Property invariants that must always hold (more in
[`docs/upcasters.md`](docs/upcasters.md)):

- `upcast(parse(serialize(v_n_event))) ≡ upcast(v_n_event)`
- `normalize(normalize(t)) == normalize(t)`
- Scenarios derived by the normalizer are acyclic when the configured
  policy forbids cycles.

## Hard constraints (full list in docs/SPEC.md)

- `unsafe_code = "forbid"` workspace-wide; no exceptions without an ADR
  (ADR-0004).
- **Semantic action map ≠ physical UI map** (ADR-0005). Trace schema, oracle
  rules, and replay plans operate on semantic IDs (`CommandId`, `ScreenId`).
  Physical selectors live in adapter code only.
- **Schema evolution via upcasters** (ADR-0006). The wire format keeps every
  historical version; domain code only ever sees `Current`. No
  in-place rewrites of stored JSONL.
- **Privacy by default** (ADR-0007). String values masked, numerics
  bucketed, raw export requires explicit opt-in plus an audit-log entry.
- **JSONL is the MVP wire format** (ADR-0003). SQLite (v0.2) and Parquet
  (v0.3) are additional read paths, never replacements.
- **Hexagonal architecture** (ADR-0002). Every cross-boundary type goes
  through a trait in `trace-core`; reach-through is a review-blocker.
- **petgraph pinned to 0.8.x** (ADR-0008). 0.9 trunk is unstable.
- **All repository text is English** (SPEC hard rule 10). Code, comments,
  commit messages, docs, fixtures — English only. Chats and tickets are
  unaffected.
- **Stage numbering follows `docs/glossary.md` §0 only** — never improvise
  labels. The roadmap is S0…S12 and ends at v1.0 stable.
- **Fuzzing is mandatory** for the JSONL parser, the upcaster chain, the
  WPF-adapter event ingest, and selected normalizer transforms (ADR-0010,
  [`docs/fuzzing.md`](docs/fuzzing.md)).

## Anti-patterns (block in review)

1. **Storing physical UI paths in trace events.** XPath, visual-tree paths,
   bounds — these belong in the adapter, never in `trace-core`/`trace-schema`.
2. **Conditional logic on `schema_version` outside upcasters.** Domain code
   reads `Current` only. If you write `match envelope.version { 1 => … }`
   anywhere outside `trace-schema::upcasters`, the design is wrong.
3. **Storing raw PII without explicit opt-in.** Default policy is `Masked`
   for strings, `Bucketed` for numerics ([`docs/privacy.md`](docs/privacy.md)).
4. **"It works fine" without tests, fixtures, or acceptance criteria.**
   See `docs/glossary.md` §19.
5. **Pulling in `rusqlite`, `parquet`, or any non-serde dependency in
   `trace-core`.** That crate stays minimal forever.
