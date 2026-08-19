# S3: Trace normalizer (value + temporal abstraction, scenario folding)

Status: landed (acceptance hardening pass 2026-08-18)
Depends on: S2
ADRs: ADR-0002

## Goal

Turn raw `Session`s into normalized `Scenario`s through value abstraction
(buckets and classes), temporal abstraction (`BurstAction`,
`SessionPause`), equivalence classes, and scenario folding.

## Inputs / Outputs

- In: a `Session` of `TraceEvent`s read from any backend.
- Out:
  - `trace-normalizer` crate exposing `normalize(session, &NormCfg) ->
    Scenario`.
  - `NormCfg` knobs: numeric bucket table, burst gap (default 50 ms),
    idle gap (default 5 s), per-field policy overrides.
  - A `trace normalize <file> -o <file>` CLI command emitting normalized
    JSONL.

## Approach

- TDD with `proptest`. Required invariants:
  - **Idempotency**: `normalize(normalize(s)) == normalize(s)`.
  - **Determinism**: same `(session, cfg)` always produces the same
    bytes.
  - **Order preservation modulo collapse**: a `BurstAction` covers a
    contiguous prefix of the input it replaces.
- Value abstraction is per the table in [`../glossary.md`](../glossary.md)
  §4. The implementation lives behind a `ValueAbstractor` trait so a
  domain can swap in a different bucket policy without forking the
  crate.
- Temporal abstraction reads `ts` (UTC) and groups by elapsed delta;
  out-of-order events trigger a warning and are normalized in
  source-order rather than time-order.
- Scenario folding emits a `Scenario` with a unique canonical action
  sequence and a sidecar `FoldReport` enumerating losses (events
  collapsed, fields bucketed, dates relativised).
- New fuzz target `normalize_fold` per ADR-0010, structure-aware via
  `arbitrary`.

## Acceptance criteria

- All property invariants hold across 10 000 generated sessions.
- The `trace normalize` CLI passes a snapshot test on a fixture of
  representative sessions.
- `normalize_fold` fuzz target green for a 60s bounded run on PR; runs
  nightly for 30 min non-blocking.
- `FoldReport` is reachable from the CLI (`trace normalize --report
  <file>`).

## Open questions

- Whether `BurstAction` carries the collapsed inner events as metadata
  (yes for diagnostic packages, no for normal use). Tentative: yes,
  behind a feature on the `Scenario` builder.
- Whether numeric buckets should be configurable per-field at the
  adapter level (planned, but defaulted in S3; per-field overrides
  land alongside the WPF adapter at S6).

## See also

- [`../glossary.md`](../glossary.md) §3, §4
- [`../adr/0007-privacy-by-default-mask-and-bucket.md`](../adr/0007-privacy-by-default-mask-and-bucket.md)
