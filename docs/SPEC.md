# SPEC.md — SemantxTrace

One page: what `SemantxTrace` is, what it is not, and the rules that do not
bend. Term definitions live in [`glossary.md`](glossary.md). Decisions live
in [`adr/`](adr/). Stage detail lives in [`stages/`](stages/).

## What SemantxTrace is

`SemantxTrace` is a **behavioral observability platform for desktop UI
applications**, built around one canonical artefact — the **semantic
trace** — and a fan-out of projections on top of it (ADR-0011). Each
projection serves a real consumer:

| Projection | Consumer question |
|---|---|
| Analytics | "Which scenarios actually run in production?" |
| Diagnostic | "What did the user do before this crash?" |
| Regression test | "Can we replay this scenario and assert what should happen?" |
| UX | "Where do users get stuck, retry, or back out?" |
| Product | "What features are dead? Which got worse after the redesign?" |
| Support replay | "Reproduce the customer's reported issue without a phone call." |
| Exploration | "What neighbouring scenarios might also be broken?" |

The unique angle: **the trace lives at the semantic action layer, not at
the physical UI layer** (ADR-0005). A trace event is
`Graph47.Recalculate` with domain context, never
`Window.Grid.Button[3].Click` with bounds. Tests, oracles, replay plans,
and behavioral metrics built on those events survive UI redesigns,
theme changes, and visual-tree rearrangements — the changes that break
selector-based testing tools and contextless click counters in the same
stroke.

The Rust core is **framework-agnostic**. The first adapter (`trace-wpf`)
targets WPF via `[TraceCommand]`-annotated `ICommand`s and an
`AutoAutomationId` attached behavior. Avalonia, MAUI, and Web adapters
are on the v0.3+ roadmap; their addition must not pollute the core
schema.

The phrase **"semantic metrics, not contextless counters"** is the
project's elevator pitch: counting "Export pressed 1200 times" without
the scenario context is bookkeeping; recording the scenario it sits in
(open declaration → edit goods → recalculate → export) makes the same
event answer product, UX, support, and test questions at once.

## What SemantxTrace is not

- **Not a SaaS dashboard product** at v1.0. Self-hosted, file-based,
  CLI + HTML reports + mdBook docs. A hosted analytics frontend is
  conceivable post-v1.0 but is not a delivery commitment.
- **Not a marketing-funnel / heatmap tool.** Heatmaps, scroll-depth
  visualisations, page-view A/B-attribution, and ad-conversion
  pipelines are out of scope. The model is semantic actions, not
  pointer pixels.
- **Not a pixel-perfect deterministic replay.** Meticulous-style
  Chromium determinism is impossible against the .NET runtime. We
  replay at the **semantic command** layer; timing is bounded by
  tolerances, not frozen. Strict vs relaxed replay modes (glossary §6)
  make the trade-off explicit instead of pretending it does not exist.
- **Not an RPA tool.** No bot-orchestration, no scheduling, no
  "automate this workflow at 6 AM" surface.
- **Not a generic event-log library.** The schema is opinionated around
  desktop UI interactions; bending it for backend tracing is out of
  scope (use OpenTelemetry).
- **Not a neural-anomaly detector.** ML-based PII detection and
  LLM-driven oracle generation are explicit non-goals through v1.0
  (glossary §17.4). Anomaly detection at v1.0 is frequency-threshold
  based and explainable.
- **Not a cloud product** at v1.0. Self-hosted, file-based, local-buffer-
  then-flush. Real-time live monitoring is out of scope; the analysis
  layer is batch.

## Delivery shape

Strict staged delivery, **S0 … S12**, defined canonically in
[`glossary.md`](glossary.md) §0 and detailed in [`stages/`](stages/). Each
stage is a vertical slice with a measurable acceptance criterion. Stages
are implemented in order; library groundwork may land earlier but a later
stage does not "close" until its acceptance criterion is met and documented.

Major milestones:

- **v0.1 / MVP / PH-launch** — closes at S7. JSONL storage, Heuristics
  miner, 5 built-in oracles, WPF adapter, WPF demo (`DeclarationApp.Demo`),
  CLI, mdBook docs.
- **v0.2** — closes at S8. SQLite analysis backend, Inductive miner (IMDF),
  improved scenario folding.
- **v0.3** — closes at S10. Parquet archive tier, Avalonia adapter.
- **v0.4** — closes at S11. Replay-planner emits portable JSON plans
  with explicit `strict` / `relaxed` modes; `semantic-monkey`
  exploration over the action graph; first-class trace-mutation
  generators.
- **v1.0 stable** — closes at S12. API freeze on `trace-core`/`trace-schema`,
  upcaster chain proven across at least two real schema bumps, fuzz corpora
  green for 30+ consecutive nightlies, semver guarantees published.

## Hard rules (do not violate without a superseding ADR)

1. **Semantic action map ≠ physical UI map.** Trace schema, oracle rules,
   and replay plans operate on `CommandId`/`ScreenId`/`FieldId` — never on
   `AutomationId`, XPath, or visual-tree paths (ADR-0005).
2. **`unsafe_code = "forbid"`** workspace-wide (ADR-0004). The isolated
   `fuzz/` crate is the only exception, by libFuzzer harness necessity.
3. **Hexagonal boundaries.** Every cross-crate type goes through a trait
   defined in `trace-core` or its nearest domain crate. Storage backends,
   event sources, oracle rules, replay adapters, and reporters are all
   ports (ADR-0002).
4. **Schema evolution via upcasters.** Storage keeps every historical
   schema version untouched; reads chain `V_n → V_n+1 → … → Current`
   pure-function upcasters. Domain code only ever sees `Current`
   (ADR-0006, [`upcasters.md`](upcasters.md)).
5. **Privacy by default.** `ValuePolicy::Masked` for strings,
   `ValuePolicy::Bucketed` for numerics. Raw values require explicit
   opt-in and an audit-log entry (ADR-0007, [`privacy.md`](privacy.md)).
6. **JSONL is the MVP wire format** (ADR-0003). SQLite (v0.2) and Parquet
   (v0.3) are additional analysis paths read through the same upcaster
   chain — never the source of truth, never a one-way migration target.
7. **Methodology is layered.** Strict red→green→refactor with `proptest`
   for `trace-core` / `trace-schema` / `trace-normalizer` / `trace-oracle`.
   Characterization + property tests for `trace-graph` / `trace-storage`
   parsers. Integration-test driven for adapters, demos, CLI ergonomics.
   See [`AGENTS.md`](../AGENTS.md) and `glossary.md` §13.
8. **Deterministic core.** Given an input session and a fixed
   normalization config, the normalizer, graph builder, and replay-planner
   produce byte-identical outputs across runs and platforms.
9. **`trace-core` has minimal dependencies.** `serde`, `serde_json`,
   `chrono`, `uuid` only. No `rusqlite`, no `parquet`, no UI crates.
   Violation breaks hexagonal boundaries.
10. **All repository text is English.** Glossary, specs, ADRs, stage docs,
    code, comments, commit messages, fixture filenames — English only.
11. **Strict lint policy.** `cargo fmt`, `cargo clippy --all-targets -D
    warnings`, and the full test suite must be green at every stage
    boundary.
12. **Fuzzing is a mandatory robustness layer.** The JSONL parser, the
    upcaster chain, the WPF adapter's event ingest, and selected
    normalizer transforms must have fuzz targets per ADR-0010 and
    [`fuzzing.md`](fuzzing.md). Bounded smoke fuzzing plus the regression
    corpus is a blocking CI gate; deep fuzzing runs scheduled,
    non-blocking.
13. **`petgraph` pinned to 0.8.x** until 0.9 releases stable (ADR-0008).
14. **Stage numbering follows `glossary.md` §0 only** — never improvise
    labels like "S2.5" or "S6b".
15. **Trace is the single source of truth; projections fan out from it**
    (ADR-0011). The seven projections (analytics, diagnostic,
    regression test, UX, product, support replay, exploration) read the
    same canonical artefact and the same `Current` schema. A second
    "lightweight analytics event log" — separate format, separate sink,
    separate schema — is **explicitly forbidden**: every consumer
    shares one wire format and one upcaster chain, or the architecture
    has lost its central promise.
16. **Replay has two distinct modes** (glossary §6, S11):
    `strict` (reproduce the recorded scenario step-by-step for bug
    reproduction and regression) and `relaxed` / `normalized` (collapse
    irrelevant orderings for analytics, clustering, and test-candidate
    selection). They are **not** the same code path with a flag flipped;
    they are two named operations with different determinism contracts.
17. **Trace mutation is a first-class generation path**, not a side
    effect of "monkey testing". Mutations are domain-aware (swap field
    order, replace a value with a boundary value, skip an optional
    step, repeat a command, navigate back) and every mutated scenario
    runs through the same oracle engine the recorded scenario does
    (S11).

## Current state

The repository is **documentation only**. No crates, no `Cargo.toml`, no
CI. The first code lands in S0 (workspace skeleton, lint policy, CI).
Until then, everything in `docs/` is the binding contract for what gets
built.

## See also

- [`glossary.md`](glossary.md) — the constitution.
- [`upcasters.md`](upcasters.md) — schema-evolution pattern.
- [`privacy.md`](privacy.md) — mask-by-default policy.
- [`fuzzing.md`](fuzzing.md) — fuzz-testing policy (ADR-0010).
- [`adr/README.md`](adr/README.md) — decision index, including
  ADR-0011 ("trace is the single source of truth; projections fan
  out").
- [`stages/`](stages/) — S0 … S12.
- [`decisions.log.md`](decisions.log.md) — small decisions, append-only.
