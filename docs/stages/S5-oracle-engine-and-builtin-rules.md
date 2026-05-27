# S5: Oracle engine and five built-in rules

Status: planned
Depends on: S4
ADRs: ADR-0002, ADR-0014

## Goal

Implement the `OracleRule` trait, the `OracleContext`, scheduling, and
the five built-in oracles. Ship the `trace oracle run` CLI subcommand
producing an HTML report.

## Inputs / Outputs

- In: a session (raw or normalized).
- Out:
  - `trace-oracle` crate with `OracleRule`, `OracleSchedule`,
    `OracleContext`, `OracleResult`.
  - Built-in rules:
    - `NoUnhandledException`,
    - `NoErrorModalAfterCommand`,
    - `CommandMustSucceed`,
    - `ScreenMustNavigateForward`,
    - `ValidationsPassBeforeSubmit`.
  - Composition primitives: `AndRule`, `OrRule`, `WithinWindowRule`.
  - HTML reporter implementing the `Reporter` port from S1.

## Approach

- Each oracle is a small struct implementing `OracleRule`. The trait is
  `Send + Sync + 'static` so the engine can fan out across cores when
  needed.
- The engine schedules oracles per their `OracleSchedule`:
  - `PerEvent` — called for every event in order.
  - `PerScenario` — called once per scenario.
  - `EndOfSession` — called once after the last event.
  - `WindowBased { window_ms }` — given a sliding window of events.
- Composition: `AndRule(rules)` succeeds iff every inner rule succeeds;
  `OrRule(rules)` succeeds iff any does; `WithinWindowRule(inner, ms)`
  applies an inner rule only to events within the rolling window.
- The HTML reporter renders an executive summary, per-oracle pass/fail
  rows, evidence links jumping to the offending events in a JSONL
  pretty-printer view.
- Property: for stateless oracles, evaluation order does not change the
  set of results (oracle commutativity).
- New fuzz target `oracle_replay`: drive the engine with arbitrary
  scenarios under each built-in rule; no panics, no hangs.

## Acceptance criteria

- All five built-in oracles have unit tests on hand-crafted positive /
  negative scenarios.
- `trace oracle run --rules builtin ./fixture.jsonl` emits the blessed
  HTML snapshot (with stable timestamps stripped).
- Commutativity property green across 10 000 generated scenarios.
- `oracle_replay` fuzz target green for the bounded run; runs nightly
  for 30 min non-blocking.
- Memory: oracle engine for a 100k-event session stays under 100 MB
  resident on the reference machine.

## Open questions

- Whether to expose a YAML / TOML rule-config format now or only when
  domain oracles ship (post-v1.0). Working answer: YAML only for
  parameterising built-ins (e.g. choosing which commands the
  `CommandMustSucceed` rule applies to).
- Per-severity exit codes for `trace oracle run`. Resolved per
  ADR-0014 §6: `0` for all-pass, `1` for any `Warning` /
  `Error` severity, `2` for any `Critical`; sysexits-style codes
  (`64`, `65`, `66`, `70`) reserved for CLI-layer failures (bad
  args / bad input / I/O / internal bug). Documented in
  `trace oracle run --help` under `EXIT CODES:`.

## See also

- [`../glossary.md`](../glossary.md) §5
