# SemantxTrace — agent guide

## Why

`SemantxTrace` is a **behavioral observability platform** for desktop UI
apps. It records **semantic** user actions (`Graph47.Recalculate`,
`Declaration.Validate`) with scenario context — not physical UI events
(`Button.Click` on `Window > Grid > StackPanel[1]`) and not contextless
counters ("button pressed 123 times"). One canonical trace fans out
into seven projections (ADR-0011): **analytics** (top-N workflows,
rare-but-failing scenarios), **diagnostic** (support packages),
**regression test** (replay plans), **UX** (where users back out /
retry), **product** (dead features, redesign deltas), **support
replay** (reproduce a user's bug step-by-step), and **exploration**
(domain-aware trace mutation). The unique architectural angle is the
**semantic action map**, decoupled from the physical UI map (ADR-0005);
the unique product angle is **semantic metrics, not contextless
counters**.

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
Stages S0–S8 have landed; `crates/` and `adapters/` are real and
CI-gated (see `docs/stages/` status lines). S9–S12 remain planned.

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
- **One trace, seven projections** (ADR-0011). Analytics, diagnostic,
  regression test, UX, product, support replay, and exploration are
  consumers of the same canonical artefact. **Never** introduce a
  parallel "lightweight analytics event log" with its own format —
  every consumer reads the same wire format through the same upcaster
  chain.
- **Strict vs relaxed replay are distinct operations** (glossary §6).
  Bug reproduction and regression assertions use strict; analytics,
  clustering, and test-candidate selection use relaxed / normalized.
  Do not collapse them into one flag-controlled code path.
- **`semantic-monkey`, not `smart-monkey`.** The v0.4 exploration
  feature is *domain-aware* mutation guided by oracles and the action
  graph, not a random clicker. The word `smart` is banned per
  glossary §19; the working name is `semantic-monkey`.
- **Public Rust surfaces follow the Rust API Guidelines** (ADR-0012).
  Every `pub` item in a published crate obeys `C-CONV`, `C-GETTER`,
  `C-ITER`, `C-COMMON-TRAITS`, `C-SEND-SYNC`, `C-DEBUG`,
  `C-GOOD-ERR`, `C-SEALED` (mandatory for `Upcaster`),
  `C-NON-EXHAUSTIVE` (on growable enums; **not** on per-version event
  enums frozen by ADR-0006), `C-VALIDATE`, `C-DOC`, `C-EXAMPLE`.
  CI gates: `cargo clippy -- -D warnings -W clippy::pedantic`,
  `cargo doc --no-deps -D warnings`, `missing_docs`, `cargo
  public-api` diff against the previous tag. See the checklist at
  <https://rust-lang.github.io/api-guidelines/checklist.html> when
  reviewing public-API changes.
- **Published .NET adapters follow the Microsoft Framework Design
  Guidelines + .NET Library Guidance** (ADR-0013). Every NuGet
  package under `adapters/Trace.*/` enables
  `AnalysisMode=AllEnabledByDefault` + `TreatWarningsAsErrors=true`
  + `EnablePackageValidation=true` +
  `Microsoft.CodeAnalysis.PublicApiAnalyzers` with committed
  `PublicAPI.Shipped.txt`. Package layout: `Trace.Abstractions`
  (contract, zero third-party deps, ABI-frozen) +
  per-framework adapters. Multi-target per ADR-0013 §6. See
  <https://learn.microsoft.com/en-us/dotnet/standard/design-guidelines/>
  and <https://learn.microsoft.com/en-us/dotnet/standard/library-guidance/>.
- **`trace-cli` binary surface follows clig.dev + POSIX + GNU +
  sysexits.h + Vector precedent** (ADR-0014). Noun-verb subcommands;
  `-o {text,json,wide}`; data→stdout / diag→stderr; `--no-color` +
  `NO_COLOR`; `-q` / `-v...`; sysexits.h exit codes (notably `78`
  `EX_CONFIG` for config-validation failure, matching Vector);
  versioned `--output json` schemas through the same upcaster
  chain as event data (ADR-0006); help text snapshot-tested via
  `trycmd`. See <https://clig.dev/> and
  <https://rust-cli-recommendations.sunshowers.io/>.

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
6. **Recording a contextless click counter.** "Button pressed 123
   times" is the anti-pattern the project exists to replace.
   `CommandExecuted { command_id, screen, args, outcome, previous_step
   (implicit via session order) }` is the right shape. If the event
   has no domain meaning, it has no place in the trace.
7. **Splitting analytics into a separate event format.** Every
   projection reads the same `Current` schema (ADR-0011). A "lighter"
   second wire format is not an optimisation; it is a parallel source
   of truth, and parallel sources of truth diverge.
8. **A `pub` item without a doc comment in a published crate.**
   Blocked by the `missing_docs` lint (ADR-0012). If a type is
   genuinely internal, mark it `pub(crate)`; do not silence the lint.
9. **Opening a previously sealed trait or removing
   `#[non_exhaustive]`** on a published type without an ADR. Both are
   semver-relevant (`C-SEALED`, `C-NON-EXHAUSTIVE`) and need explicit
   reasoning.
10. **`unwrap()` / `expect()` on user-controlled data in a public
    library function.** Replace with typed errors (`C-GOOD-ERR`).
    Internal invariants where the panic is genuinely impossible may
    use `expect("invariant: …")` with a `# Panics` doc section
    explaining why.
11. **A NuGet `.csproj` under `adapters/Trace.*/` without the
    `Directory.Build.props` properties from ADR-0013 §3** —
    `EnableNETAnalyzers`, `AnalysisMode`, `TreatWarningsAsErrors`,
    `EnablePackageValidation`. The lint discipline is opt-out by
    omission, which makes "I forgot" a class of bug.
12. **Adding a third-party dependency to `Trace.Abstractions`.**
    The contract package stays dependency-free post-v1.0 (the .NET
    analog of `trace-core` discipline). If a downstream feature
    genuinely needs the dep, it goes into the implementation
    package, never the contract.
13. **A `trace` subcommand that prints data on `stderr` or
    diagnostics on `stdout`.** Breaks every `|` pipeline. clig.dev
    §output rule, ADR-0014 §5.
14. **An exit code outside the `sysexits.h` table** (or `0`/`1`/`2`
    per ADR-0014 §6). Scripts depend on these; do not improvise.
15. **A `--output json` payload without a published versioned
    schema.** ADR-0014 §11 requires every machine-output shape to
    travel through the same upcaster chain as event data (ADR-0006).
