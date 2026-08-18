# S1: trace-core value objects and trace-schema v1 with upcaster machinery

Status: landed (acceptance hardening pass 2026-08-18)
Depends on: S0
ADRs: ADR-0002, ADR-0006, ADR-0010

## Goal

Define the domain value objects in `trace-core` and the first wire
schema version (`v1`) in `trace-schema`. Crucially, wire up the
**upcaster machinery from day one** so the second schema version is a
mechanical addition, not an architectural surprise.

## Inputs / Outputs

- In: S0 workspace.
- Out:
  - `trace-core`: `TraceEvent` (alias for `trace_schema::Current`),
    `Session`, `Scenario`, newtypes (`SessionId`, `EventSeq`,
    `CommandId`, `ScreenId`, `FieldId`, `CorrelationId`),
    `Outcome`, `ValuePolicy`, port traits (`StorageBackend`,
    `EventSource`, `OracleRule`, `ReplayAdapter`, `Reporter`).
  - `trace-schema`:
    - module `v1` with `TraceEvent`, `TraceEventKind`, `TraceEnvelope`,
      `VersionProbe`;
    - `Upcaster` and `StreamUpcaster` traits;
    - `pub type Current = v1::TraceEvent` (identity chain for now);
    - `read_event(raw: &str) -> Result<Current, Error>` dispatching on
      `schema_version`;
    - JSON Schema file `schema/trace-event-v1.schema.json` published
      alongside the crate;
    - property tests for serialize / parse round-trip.
  - First fuzz targets per ADR-0010: `jsonl_parse_v1`,
    `upcaster_v1_to_current` (identity for now), structure-aware via
    `arbitrary`.

## Approach

- TDD strictly. Write the failing property tests for round-trip and
  identity upcasting first; commit them red; then implement.
- Newtypes use `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash,
  Serialize, Deserialize)]` where the inner type permits; otherwise drop
  `Copy`.
- `Upcaster` is a small trait per ADR-0006; concrete `From<v_k> for
  v_k+1` impls satisfy a blanket impl converting them to `Upcaster`
  trait objects, with a `lossy: bool` constant.
- `StreamUpcaster` is defined now even though v1 needs no stream
  upcasts; the trait alone is the API guarantee for v2+.
- Publish the JSON Schema both in-repo and in the crate's docs.rs
  artefacts; the WPF adapter validates against it in CI.

## Acceptance criteria

- All v1 event kinds (from [`../glossary.md`](../glossary.md) §2) parse
  and serialize losslessly under property tests
  (`forall e, parse(serialize(e)) == Ok(e)`).
- `upcast_to_current(parse(serialize(e))) == upcast_to_current(e)` for
  every v1 event kind.
- The JSON Schema validates a hand-crafted seed corpus of representative
  v1 envelopes.
- `jsonl_parse_v1` and `upcaster_v1_to_current` fuzz targets build under
  `cargo +nightly fuzz build` and produce no findings in a 5-minute
  bounded run on the developer's machine.
- `trace-core` has the exact dependency set
  `serde, serde_json, chrono, uuid` (+ `thiserror` for error types).

## Open questions

- Whether `TraceEnvelope` should be a struct wrapping the event (with
  `schema_version` as a sibling field) or a tagged enum keyed by
  version. Decided in S1, documented inline. Working assumption:
  struct wrapper with `schema_version` field, because upcasters key off
  `VersionProbe` which only reads that one field.
- Whether `Outcome::Failure(String)` carries an error code as well.
  Deferred to v1.0 if the WPF demo produces a use case.

## See also

- [`../adr/0006-upcaster-pattern-for-schema-evolution.md`](../adr/0006-upcaster-pattern-for-schema-evolution.md)
- [`../upcasters.md`](../upcasters.md)
- [`../glossary.md`](../glossary.md) §1, §2
