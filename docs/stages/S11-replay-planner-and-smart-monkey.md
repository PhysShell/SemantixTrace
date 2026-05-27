# S11: Replay planner and smart-monkey exploration (v0.4)

Status: planned
Depends on: S10
ADRs: ADR-0005, ADR-0006

## Goal

Close v0.4. Ship `trace-replay-planner`: turn a normalized scenario into
a portable `ReplayPlan` JSON document and provide smart-monkey
exploration over the action graph for coverage-guided test generation.

## Inputs / Outputs

- In: a normalized scenario and an `ActionGraph`.
- Out:
  - `trace-replay-planner` crate exposing `plan_from(scenario,
    &PlanCfg) -> ReplayPlan` and `smart_monkey(graph, &MonkeyCfg) ->
    impl Iterator<Item = ReplayPlan>`.
  - Versioned `ReplayPlan` schema with its own upcaster chain (ADR-0006
    applies to plans as well as events).
  - `trace plan generate` and `trace plan explore` CLI subcommands.
  - `ReplayAdapter` reference implementation in `Trace.Wpf` consuming
    the JSON plans.

## Approach

- `ReplayPlan` is the structure described in
  [`../glossary.md`](../glossary.md) §6. Its JSON schema is published
  under `trace-replay-planner/schema/`.
- Steps reference semantic IDs only; the adapter resolves them at
  replay time.
- Smart-monkey strategies:
  - `random(coverage_target: f32)` — random walk until coverage hits
    the target;
  - `weighted_random(edge_weight = inverse_frequency)` — bias toward
    rare transitions;
  - `quick_random(max_steps: u32)` — bounded random walk for smoke runs.
- Determinism: `MonkeyCfg` carries a seed; same `(graph, cfg)` produces
  the same sequence of plans.
- Property tests:
  - every emitted plan corresponds to a real path in the source graph;
  - data dependencies are satisfiable against an in-memory fixture pool;
  - replay of a plan against the demo app reproduces the source scenario
    (modulo the declared tolerances).

## Acceptance criteria

- `trace plan generate --scenario Graph47.RecalculationFlow ./norm.jsonl
  -o plan.json` produces a JSON document validated by the published plan
  schema.
- The reference `ReplayAdapter` in `Trace.Wpf` replays the generated
  plan against `DeclarationApp.Demo` and reproduces all three
  intentional bugs.
- `smart_monkey` deterministic under a fixed seed.
- New fuzz target `replay_plan_parse` green on the bounded run.
- Plan schema upcaster identity chain (`plan_v1 → plan_current`) green
  under the standard upcaster property tests.

## Open questions

- Whether to embed a recording diff in the plan ("the replay diverged
  here"). Working answer: yes, as an optional `--with-diff` flag on
  the adapter side; not part of the plan schema itself.
- Whether to add `quick_random` as a default exploration strategy in
  CI. Working answer: yes, behind a feature flag in the demo's CI
  pipeline.

## See also

- [`../adr/0006-upcaster-pattern-for-schema-evolution.md`](../adr/0006-upcaster-pattern-for-schema-evolution.md)
- [`../glossary.md`](../glossary.md) §6
