# upcasters.md — schema evolution by upcaster chains

> Status: authoritative reference for the pattern fixed in ADR-0006.
> Read this before changing anything in `trace-schema/`.

## Why upcasters at all

SemantxTrace persists every recorded session as JSONL on disk and expects
those files to remain readable for the lifetime of the product. Some of
them are evidence in audit-log workflows; rewriting them in-place to "fix"
their schema version is both operationally hostile and a compliance
red flag. JSONL is the canonical artefact (ADR-0003); SQLite and Parquet
are derived read paths; nothing about the wire format may shift the
ground under historical recordings.

A tagged enum over schema versions (`TraceEnvelope::V1 | V2 | V3 | …`)
works for two versions and degrades into a `match` over every version at
every read site by the third. It also forces domain crates to know
which versions exist, which violates the hexagonal boundary (ADR-0002).

The **upcaster pattern** solves both. The idea is from the event-sourcing
world: Axon Framework's `EventUpcaster` /
`IntermediateEventRepresentation`, Marten's `IEventUpcaster`, the
EventStoreDB docs labelled "event versioning through upcasting", and
Greg Young's CQRS essays from the early 2010s. Storage keeps every event
in the version it was written in; on read, a chain
`V_n → V_n+1 → … → V_current` of pure upcasters lifts the event to the
current shape. Domain code only ever sees `Current`.

Rust's `From` trait is, almost literally, the API the pattern wants.

## The shape

### Per-version modules

`trace-schema` carries one module per concrete version, frozen forever
after release:

```rust
pub mod v1 {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct TraceEvent {
        pub seq:            EventSeq,
        pub session_id:     SessionId,
        pub ts:             chrono::DateTime<chrono::Utc>,
        #[serde(flatten)]
        pub kind:           TraceEventKind,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "PascalCase")]
    pub enum TraceEventKind {
        ScreenOpened       { screen_id: ScreenId, params: serde_json::Value },
        CommandExecuted    { command_id: CommandId, args: serde_json::Value,
                             duration_ms: u64, outcome: Outcome },
        FieldChanged       { field_id: FieldId,
                             old: ValuePolicy, new: ValuePolicy },
        ExceptionThrown    { exception_type: String, message: String,
                             stack: Option<String> },
        NavigationOccurred { from: ScreenId, to: ScreenId },
        ValidationFailed   { validator: String, field_id: FieldId,
                             reason: String },
        AsyncOperationCompleted { operation_id: String,
                                  duration_ms: u64, outcome: Outcome },
    }
}

pub mod v2 { /* …, frozen after v2 ships */ }
pub mod v3 { /* … */ }

pub type Current = v3::TraceEvent;
```

The public alias `Current` is the only event type exported from
`trace-schema` to the rest of the workspace. Domain crates import
`trace_schema::Current` (re-exported as `trace_core::TraceEvent`) and
nothing else.

### The chain via `From`

Between adjacent versions, a `From` impl encodes the upcast:

```rust
impl From<v1::TraceEvent> for v2::TraceEvent {
    fn from(old: v1::TraceEvent) -> Self {
        v2::TraceEvent {
            seq:            old.seq,
            session_id:     old.session_id,
            ts:             old.ts,
            correlation_id: None,           // v2 added it; old events had none
            kind:           old.kind.into(), // delegates to TraceEventKind upcaster
        }
    }
}

impl From<v2::TraceEvent> for v3::TraceEvent {
    fn from(old: v2::TraceEvent) -> Self { /* … */ }
}
```

Composition is mechanical: `Current::from(Current::from(v1_event))`-style
nesting works syntactically but is awkward at three or more versions; the
dispatch helper hides it.

### The dispatch helper

```rust
#[derive(Deserialize)]
struct VersionProbe { schema_version: u32 }

pub fn read_event(raw: &str) -> Result<Current, SchemaError> {
    let probe: VersionProbe = serde_json::from_str(raw)?;
    match probe.schema_version {
        1 => {
            let e: v1::TraceEvent = serde_json::from_str(raw)?;
            Ok(v3::TraceEvent::from(v2::TraceEvent::from(e)))
        }
        2 => {
            let e: v2::TraceEvent = serde_json::from_str(raw)?;
            Ok(v3::TraceEvent::from(e))
        }
        3 => Ok(serde_json::from_str(raw)?),
        v => Err(SchemaError::UnsupportedSchemaVersion(v)),
    }
}
```

This is the single place in the codebase that knows the set of versions
exists. Any domain code that pattern-matches on `schema_version` outside
`trace-schema::upcasters` is a review-blocker.

### The `Upcaster` and `StreamUpcaster` traits

For one-to-one event transitions, `From` is enough. For more constrained
behaviour (lossy-marker, schedule, debugging metadata), `trace-schema`
defines:

```rust
pub trait Upcaster {
    type From;
    type To;
    const LOSSY: bool;
    fn upcast(input: Self::From) -> Self::To;
}
```

A blanket impl bridges `From`-backed upcasters into the trait for cases
where the caller wants the `LOSSY` flag without naming the concrete
upcaster type.

For collapsing multiple events into one (or splitting one into many) —
i.e. when `From<v_n::TraceEvent> for v_n+1::TraceEvent` cannot exist
because the input is "two adjacent events" — we use a stream upcaster:

```rust
pub trait StreamUpcaster {
    type From;
    type To;
    const LOSSY: bool;
    fn upcast_stream<I: Iterator<Item = Self::From>>(
        input: I,
    ) -> Box<dyn Iterator<Item = Self::To>>;
}
```

Axon makes the same distinction between `SingleEventUpcaster` and
`EventUpcaster`; the names differ but the contract is the same. We keep
both traits available from v1 so a future stream upcast does not need a
trait introduction.

### Lossy upcasters

Every upcaster carries `const LOSSY: bool`. Lossy upcasts remove
information that cannot be recovered: a v3 envelope produced by a lossy
chain from a v1 event is *not* round-trippable to v1. The runtime never
attempts a downcast; `trace-schema` exposes only `upcast_to_current`.

When at least one step in the configured chain is lossy, the runtime
records that fact in a `ChainInfo` accessor so reporters can flag
"degraded historical events".

## Property tests (mandatory)

Every concrete version `v_n` ships, at minimum, these `proptest` cases
in `trace-schema/tests/`:

1. **Round-trip on the version itself.**
   `forall e: v_n::TraceEvent, parse(serialize(e)) == Ok(e)`.

2. **Upcast determinism through JSON.**
   `forall e: v_n::TraceEvent,
   upcast_to_current(parse(serialize(e))) == upcast_to_current(e)`.

3. **Identity on the current version.**
   `upcast_to_current(serialize(Current)) == Current` byte-identical.

4. **Chain composition equality.**
   For any pair of intermediate versions `v_a, v_b` with `a < b`,
   `upcast_to_current(direct_from(v_a))` agrees with the result of
   going `v_a → v_b → … → Current` step by step.

Stream upcasters add:

5. **Streaming idempotency.**
   `upcast_stream(upcast_stream(s)) == upcast_stream(s)` when both sides
   are applied to a stream already in `To`'s shape (no-op).

6. **Stream determinism.**
   Same input iterator yields the same output sequence across runs.

`proptest` strategies for each version live alongside its module and are
maintained as the version's frozen test surface; they are not edited
after the version is released.

## Fuzz coverage (mandatory)

Per ADR-0010 and [`fuzzing.md`](fuzzing.md):

- `upcaster_v{n}_to_current` per version, structure-aware via
  `arbitrary` over `v_n::TraceEvent`. Oracle: never panics, never
  hangs, returns `Current`.
- `jsonl_parse_v{n}` per version, raw bytes. Oracle: typed-error xor
  success; no panics; no unbounded allocation.
- Regression corpus: every closed schema-bump bug commits its minimised
  crash input under `fuzz/corpus/upcaster_*/`.

## Wire format details

- The envelope keys `schema_version` at the top level (sibling of
  `session_id`, `ts`, etc.), not as a discriminant on the event kind.
  `VersionProbe` reads only that single field.
- `schema_version` is a JSON integer, not a string, so the dispatch can
  use `u32` arithmetic and exhaustive `match` cleanly.
- `kind` remains the inner discriminator (`{ "kind": "CommandExecuted",
  … }`), unchanged across upcasters when possible. When a v_n+1 renames
  an event kind, the upcaster maps it; the old name never appears in
  v_n+1 events.

## Worked example: introducing v2

The story to follow when a real schema bump happens.

1. **Decide the change is necessary.** Additive enum variants and
   optional fields fit in a *minor* bump; renames, type changes, and
   semantic redefinitions force a *major* bump → new module `v2`.
2. **Open an ADR** if the change has architectural implications
   (otherwise a `decisions.log.md` entry suffices).
3. **Copy `v1`'s module to `v2`** and apply the changes inside `v2`
   only. **Never** touch `v1` after release; the property tests for
   `v1` should not need updates.
4. **Implement `From<v1::TraceEvent> for v2::TraceEvent`** in
   `trace-schema/src/upcasters/v1_to_v2.rs` with `const LOSSY: bool`.
   Add a single-event property test and (if applicable) a worked
   round-trip test.
5. **Repoint `Current`** to `v2::TraceEvent`.
6. **Update `read_event`** to add the `2 => …` arm and adjust the
   `1 => …` arm to compose through `v2`.
7. **Add fuzz target `upcaster_v1_to_v2`** and seed it from
   representative v1 envelopes.
8. **Run the full property + fuzz suites.** A green run is the contract
   for the bump.
9. **Note the bump in `decisions.log.md` and in the changelog** with a
   one-line summary of *what changed* and *what historical events look
   like after the upcast*.

A bug fix in `v2` after release does not require introducing `v3` if
the fix is purely additive (e.g. a new optional field on an existing
variant) and is `#[serde(default)]`-handled. A bug fix that changes
already-serialised data **does** require `v3`; this is exactly what the
pattern is for.

## Compaction (optional, never required)

Long chains slow down per-read parsing — five sequential upcasts per
event add up at corpus scale. The CLI provides:

```
trace compact <input.jsonl> -o <output.jsonl>
```

which re-reads the input through `read_event` and rewrites it as
`Current`. This is **never** required for correctness, **never** wired
into CI, and **never** triggered by the recorder. It is an opt-in
storage optimisation an operator can run when batch analytics over old
recordings becomes a hot path. The fact that compaction is optional is
the whole point of the pattern: there is no migration window, no
upgrade step, no flag day.

If `trace compact` ever feels mandatory, the upcaster chain has grown a
real performance problem and the right answer is to profile the
specific upcaster(s), not to retroactively change the pattern.

## Where upcasters apply beyond `TraceEvent`

The `ReplayPlan` document carries its own `schema_version` and its own
upcaster chain (see [`stages/S11-replay-planner-semantic-monkey-and-trace-mutation.md`](stages/S11-replay-planner-semantic-monkey-and-trace-mutation.md)).
The pattern generalises to anything serialised and stored. The rule is
identical: storage keeps every version intact; reads chain `V_n →
Current`; the runtime never rewrites a past artefact.

The `FoldReport` (from `trace-normalizer`) and the `OracleResult`
HTML-report manifest are **not** persisted as canonical artefacts and
do not need upcaster chains; if they did, the pattern would apply.

## Things this pattern does **not** solve

- It does not let you change the *meaning* of historical events: an
  event from v1 still records what the application did at v1's wire
  shape. If the new `Current` adds a field whose absence is significant
  for analysis, the upcaster fills a defensible default and the
  `FoldReport` flags the assumption.
- It does not give you cheap downcasting. Lossy steps make downcasting
  impossible by definition; even lossless chains are not auto-inverted.
- It does not absolve you of the property + fuzz contract for every
  version. The freedom from migrations comes from the discipline of
  upcaster correctness.

## See also

- [`adr/0006-upcaster-pattern-for-schema-evolution.md`](adr/0006-upcaster-pattern-for-schema-evolution.md)
- [`adr/0003-jsonl-as-mvp-wire-format.md`](adr/0003-jsonl-as-mvp-wire-format.md)
- [`adr/0010-fuzz-storage-parsers-and-upcaster-chain.md`](adr/0010-fuzz-storage-parsers-and-upcaster-chain.md)
- [`stages/S1-trace-core-and-schema-v1.md`](stages/S1-trace-core-and-schema-v1.md)
- [`fuzzing.md`](fuzzing.md)
- [`glossary.md`](glossary.md) §2 (events), §13 (testing)
- External: Axon Framework's
  [event upcasting docs](https://docs.axoniq.io/) (`EventUpcaster`,
  `SingleEventUpcaster`, `IntermediateEventRepresentation`);
  Marten's `IEventUpcaster` documentation; EventStoreDB's event-
  versioning guide.
