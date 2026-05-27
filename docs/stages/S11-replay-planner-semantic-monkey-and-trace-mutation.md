# S11: Replay planner (strict/relaxed), semantic-monkey, trace mutation (v0.4)

Status: planned
Depends on: S10
ADRs: ADR-0005, ADR-0006, ADR-0011, ADR-0014

## Goal

Close v0.4 by landing the three v0.4-defining capabilities together,
because they share infrastructure and split clumsily otherwise:

1. **Replay planner with two explicit modes.**
   `strict` reproduces the recorded scenario step-by-step (bug
   reproduction, regression assertions, support replay).
   `relaxed` / `normalized` collapses irrelevant orderings according
   to the normalizer's equivalence classes (analytics-driven test
   generation, clustering, scenario-class replay).
2. **Semantic-monkey exploration.** Coverage-guided walks over the
   `ActionGraph` constrained by oracles. Generates *fresh* scenarios
   that touch rare or untested transitions. The name is
   `semantic-monkey`, not `smart-monkey` (glossary §19, decisions.log
   2026-05-27).
3. **Trace mutation.** Domain-aware transformations applied to
   *recorded* scenarios — swap field order, replace a value with a
   boundary, skip an optional step, repeat a command, navigate back,
   replace document type, shuffle independent blocks. Each mutated
   scenario runs through the same oracle engine as the original.
   `cargo-mutants` for scenarios, in spirit.

## Inputs / Outputs

- In: a normalized scenario, an `ActionGraph`, and the corpus produced
  through S8 (SQLite) / S9 (Parquet) for selection.
- Out:
  - `trace-replay-planner` crate exposing:
    - `plan_from(scenario, &PlanCfg) -> ReplayPlan` with
      `PlanCfg.mode: ReplayMode`;
    - `semantic_monkey(graph, &MonkeyCfg) -> impl Iterator<Item =
      ReplayPlan>`;
    - `mutate(scenario, &MutationCfg) -> impl Iterator<Item =
      (ReplayPlan, MutationProvenance)>`.
  - Versioned `ReplayPlan` schema with its own upcaster chain
    (ADR-0006 applies to plans as well as events). `ReplayMode` is a
    `#[serde(tag = "mode")]` discriminant inside the plan.
  - `trace plan generate --mode {strict,relaxed} …`,
    `trace plan explore …` (semantic-monkey),
    `trace plan mutate …` CLI subcommands. All three follow
    the noun-verb grammar from ADR-0014 §3 (`trace plan <verb>`);
    each supports `--output {text,json,wide}`; each ships with
    a versioned JSON schema for its `--output json` payload
    chained through the same upcaster pattern as event data
    (ADR-0014 §11 / ADR-0006); exit codes per ADR-0014 §6 — a
    plan that hits an oracle failure during dry-run exits `1` or
    `2` by severity, sysexits codes for CLI-layer issues.
  - `ReplayAdapter` reference implementation in `Trace.Wpf` consuming
    both modes and exposing the mutation-provenance metadata in HTML
    reports.

## Approach

### Strict vs relaxed planner

- The planner builds the same step list for both modes, then applies
  a mode-specific *reduction*:
  - `strict` — no reduction; the recorded ordering is preserved
    verbatim, and `Wait` steps include the recorded delta.
  - `relaxed` — collapse independent contiguous `SetField` blocks
    into a `SetFields { canonical_order }` step; weaken `Wait` to
    `WaitUntilReady(timeout)`; group commands whose ordering is
    interchangeable per the normalizer's equivalence classes.
- The two reductions live behind a `PlanReducer` trait so a third
  mode (e.g. `loose` for fuzz seeds) can land without churning the
  planner core.

### Semantic-monkey strategies

- `coverage_guided(target: CoverageTarget)` — walks transitions whose
  current visit-count is below the target.
- `weighted_random(edge_weight = inverse_frequency)` — bias toward
  rare-but-reachable transitions.
- `oracle_guarded(strategy, oracle_pack)` — wraps any strategy with a
  pre-step oracle check that rejects walks producing invalid
  intermediate states (e.g. visiting `Export` without going through
  `Validate`).
- `quick_random(max_steps: u32)` — bounded walk for smoke runs.

All strategies are deterministic under `MonkeyCfg.seed`; the same
`(graph, cfg)` always yields the same sequence of plans.

### Trace mutation

- Mutations are typed and named — every mutation impl carries a
  `MutationKind` enum tag persisted in the resulting plan's
  `MutationProvenance` block. Initial catalogue:

  | Mutation | Effect |
  |---|---|
  | `SwapFieldOrder { fields: Vec<FieldId> }` | reorder independent fields |
  | `ReplaceWithBoundary { field, boundary }` | substitute min/max/zero/negative |
  | `SkipOptionalStep { step_index }` | drop a step the normalizer marks optional |
  | `RepeatCommand { step_index, times }` | re-fire an idempotent-by-contract command |
  | `NavigateBackThen { at, then }` | insert a back-navigation and a follow-up |
  | `ReplaceDocumentType { from, to }` | run the same flow against a different document type |
  | `ShuffleIndependentBlocks { blocks }` | permute order-independent groups |

- Mutations declare *applicability predicates*; a `Mutate` operation
  silently skips mutations that do not apply (e.g.
  `RepeatCommand` on a non-idempotent command). Skipping is logged
  in the provenance so a corpus-of-tried-mutations stays auditable.
- Each emitted mutated plan is paired with the *original* plan it
  was derived from and a list of `OracleRule`s the original satisfied;
  the runner reports any oracle that newly fails after the mutation
  as a candidate bug.

### Property tests

- Every emitted plan (strict, relaxed, monkey, mutated) corresponds to
  a real path in the source graph or a typed mutation of one.
- Data dependencies are satisfiable against an in-memory fixture
  pool.
- Strict replay of a recorded plan against the demo app reproduces
  the source scenario modulo declared tolerances; relaxed replay
  reproduces it modulo declared equivalence classes.
- Mutations are *invertible at the metadata level*: given a mutated
  plan plus its `MutationProvenance`, the planner reconstructs the
  original plan byte-identically.

## Acceptance criteria

- `trace plan generate --mode strict --scenario
  Graph47.RecalculationFlow ./norm.jsonl -o plan.json` produces a JSON
  document validated by the published plan schema, with
  `"mode": "strict"`.
- The same command with `--mode relaxed` produces a plan with
  `"mode": "relaxed"` and a smaller `steps` array (independent fields
  collapsed).
- The reference `ReplayAdapter` in `Trace.Wpf` replays the strict
  plan against `DeclarationApp.Demo` and reproduces all three
  intentional bugs.
- The reference adapter replays the relaxed plan against the demo and
  reproduces the same oracle verdicts, even when the field-fill order
  is permuted.
- `semantic_monkey` is deterministic under a fixed seed across
  Windows / Linux / macOS.
- `trace plan mutate --kinds SwapFieldOrder,SkipOptionalStep
  ./scenario.jsonl -o mutants/` produces at least one mutant that
  newly fails an oracle the original passed — proving the mutation
  path catches real bugs in the demo app's three known classes.
- New fuzz targets `replay_plan_parse` and `trace_mutation_apply`
  green on the bounded run; latter is structure-aware via
  `arbitrary` over `(Scenario, MutationKind)`.
- Plan schema upcaster identity chain (`plan_v1 → plan_current`)
  green under the standard upcaster property tests.

## Open questions

- Whether to embed a recording diff in the plan ("the replay diverged
  here"). Working answer: yes, as an optional `--with-diff` flag on
  the adapter side; not part of the plan schema itself.
- Whether to add `quick_random` as a default exploration strategy in
  CI. Working answer: yes, behind a feature flag in the demo's CI
  pipeline.
- Whether mutations should be allowed to *combine* (compose two
  mutations into one plan). Working answer: not in v1.0 —
  combinatorial explosion + provenance complexity outweighs the
  benefit until single mutations are battle-tested.

## See also

- [`../adr/0005-semantic-action-map-not-physical-ui-map.md`](../adr/0005-semantic-action-map-not-physical-ui-map.md)
- [`../adr/0006-upcaster-pattern-for-schema-evolution.md`](../adr/0006-upcaster-pattern-for-schema-evolution.md)
- [`../adr/0011-trace-as-multi-projection-source-of-truth.md`](../adr/0011-trace-as-multi-projection-source-of-truth.md)
- [`../glossary.md`](../glossary.md) §6
