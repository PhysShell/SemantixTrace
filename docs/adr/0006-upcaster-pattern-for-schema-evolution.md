# ADR 0006: Upcaster pattern for schema evolution (no in-place rewrites)

Date: 2026-05-27
Status: Accepted

## Context

`SemantxTrace` stores recorded sessions as append-only JSONL on disk and
expects to read traces recorded against earlier versions of the wire
schema for the lifetime of the product. Every realistic deployment will
keep months or years of historical recordings; rewriting them on every
schema bump is operationally hostile, legally fragile (audit-log
mutation), and corrupts the property that on-disk JSONL is the
canonical source of truth (ADR-0003).

The naive alternative — a tagged enum `TraceEnvelope::V1 | V2 | V3` —
works for two versions and rots into a `match` over every version at
every read site by the third. It also forces domain code to know which
versions exist, which contradicts the hexagonal boundary (ADR-0002).

The pattern is well-established in event-sourced systems. Axon
Framework (Java) calls it `EventUpcaster` /
`IntermediateEventRepresentation`. Marten (.NET) ships `IEventUpcaster`.
EventStoreDB documents the same idea as "event versioning through
upcasting". Greg Young wrote the original CQRS essays around it.
Storage keeps every event in the version it was written in. On read, a
chain `V_n → V_n+1 → … → V_current` of pure-function upcasters lifts the
event to the current shape, and only the current shape ever escapes the
schema crate. Rust's `From` trait was, almost literally, designed for
this.

## Decision

`trace-schema` owns one module per concrete schema version:
`trace_schema::v1`, `v2`, …, each with its own `TraceEvent`,
`TraceEventKind`, and supporting types. The public-facing alias is

```rust
pub type Current = v_n::TraceEvent;
```

pointing at the highest schema version this binary knows.

Between adjacent versions, a `From<v_k::TraceEvent> for v_k+1::TraceEvent`
impl encodes the upcast. The chain is composed at read time:

```rust
pub fn read_event(raw: &str) -> Result<Current, Error> {
    let probe: VersionProbe = serde_json::from_str(raw)?;
    match probe.schema_version {
        1 => upcast_to_current(serde_json::from_str::<v1::TraceEvent>(raw)?),
        2 => upcast_to_current(serde_json::from_str::<v2::TraceEvent>(raw)?),
        // ...
        v => Err(Error::UnsupportedSchemaVersion(v)),
    }
}
```

For events that cannot be expressed as a single-event upcast (e.g. two
v3 events collapse into one v4 event), a separate `StreamUpcaster`
trait operates over an event stream; Axon makes the same distinction
between `EventUpcaster` and `SingleEventUpcaster` and we mirror it.

Every upcaster is annotated with a `lossy: bool` marker. Lossy upcasts
remove information that cannot be recovered; downcasting is not
supported, and `trace-schema` rejects any attempt to round-trip from
`Current` back to a historical version when the chain contains a lossy
step.

Each schema version, including `v1`, ships JSON Schema files under
`trace-schema/schema/` and has a property test asserting the round-trip
invariant:

> for every `event: v_n::TraceEvent`,
> `upcast_to_current(parse(serialize(event)))` succeeds and equals
> `upcast_to_current(event)`.

Long chains can become slow for batch analytics. An optional CLI
command, `trace compact`, re-reads JSONL and writes it back in the
current version. It is **never** required for correctness; it is a
storage optimisation a user may choose to run. The fact that compaction
is optional is the whole point.

The full pattern, including stream upcasters, lossy markers, the
property-test contract, and worked examples, lives in
[`../upcasters.md`](../upcasters.md). ADR-0006 fixes the architectural
commitment; `upcasters.md` is the working reference.

## Consequences

- Domain code in `trace-normalizer`, `trace-graph`, `trace-oracle`,
  `trace-replay-planner` never observes a historical schema version. A
  schema bump means: add `v_n+1`, add the `From` impl, repoint
  `Current`, update one entry in `read_event`. The rest of the
  codebase is untouched.
- Stored JSONL is never rewritten. Audit logs stay byte-stable; legal /
  compliance reviews are unblocked.
- Rollback is cheap. Reverting `Current = v_n+1` to `Current = v_n`
  works as long as the new code only added the inverse `From` impl in
  the same release.
- Lossy changes constrain future design: once a field is dropped, it
  cannot be recovered. The `lossy: bool` marker forces this trade-off
  to be visible at the upcaster definition rather than discovered at
  the read site.
- Per-read cost grows linearly with the number of versions traversed.
  Acceptable for batch analytics; `trace compact` exists as the escape
  hatch for hot-path readers.
- The upcaster chain is a mandatory fuzz target (ADR-0010): any panic
  in `read_event` against arbitrary bytes is a wire-exploitable bug.

## See also

- [`../upcasters.md`](../upcasters.md) — pattern reference and worked
  examples.
- [`../stages/S1-trace-core-and-schema-v1.md`](../stages/S1-trace-core-and-schema-v1.md)
  — first realisation (identity chain `V1 → Current`).
- ADR-0003 (JSONL canonicality), ADR-0002 (hexagonal boundaries).
