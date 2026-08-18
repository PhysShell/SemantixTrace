# fuzzing.md — fuzz-testing policy

> Status: authoritative policy for the fuzz layer fixed in ADR-0010.

## What fuzzing is and is not in SemantxTrace

Fuzzing is a **robustness layer**, not a correctness proof. It guarantees
that the codepaths that touch externally produced bytes — JSONL parsing,
the upcaster chain, the WPF adapter's event ingest, and selected
normalizer transforms — never panic, hang, or allocate without bound on
arbitrary input. Functional correctness lives in unit, property, and
golden tests.

## Hard rules (mirroring SPEC §12 and ADR-0010)

1. The targets listed in [Target catalogue](#target-catalogue) are
   **mandatory**. A PR that lands a new wire-touching component without
   its corresponding fuzz target does not merge.
2. The `fuzz/` crate is **isolated** from the main workspace: its own
   `rust-toolchain.toml` pins nightly; `cargo` in the parent workspace
   does not see it.
3. The bounded smoke run plus regression-corpus replay is a
   **blocking** CI gate; deep fuzzing runs scheduled, non-blocking.
4. Every closed crash / hang ships a minimised input committed under
   `fuzz/corpus/<target>/` and stays there forever.
5. Every fuzz oracle asserts at minimum: no panic, no hang
   (`-timeout=10`), no unbounded allocation
   (`-rss_limit_mb=2048 -malloc_limit_mb=1024`), and the typed-error
   contract (`Ok(_)` xor a typed enum).

## Target catalogue

| Target | Priority | Stage | Input shape | Oracle additions |
|---|---|---|---|---|
| `jsonl_parse_v1` | P0 | S1/S2 | raw bytes | parsed event re-serializes to a string `read_event` accepts again |
| `upcaster_v1_to_current` | P0 | S1 | `arbitrary` over `v1::TraceEvent` | result is a valid `Current` (validates against the published schema) |
| `jsonl_parse_v{n}` | P0 | S(n+0) | raw bytes | per-version round-trip |
| `upcaster_v{n}_to_current` | P0 | S(n+0) | `arbitrary` over `v_n::TraceEvent` | result valid against `Current` schema |
| `normalize_fold` | P1 | S3 | `arbitrary` `Session` | idempotency (`normalize(normalize(s)) == normalize(s)`), order preservation modulo collapse |
| `graph_build` | P1 | S4 | `arbitrary` `Vec<Scenario>` | determinism, cyclicity matches `NormCfg::allow_cycles` |
| `oracle_replay` | P1 | S5 | `arbitrary` `Scenario` driven through each built-in rule | result severity falls in the allowed set; commutativity for stateless rules |
| `sqlite_query` | P2 | S8 | `arbitrary` filter combos | never panics; never produces unbounded results |
| `parquet_round_trip` | P2 | S9 | `arbitrary` `Vec<TraceEvent>` | written→read sequence equals input |
| `replay_plan_parse` | P1 | S11 | raw bytes | parsed plan re-serializes to a string the parser accepts again |
| `wpf_ingest_jsonl` | P1 | S6 | raw bytes that the WPF adapter would write | same contract as `jsonl_parse_v_current` plus adapter-specific invariants |

P0 targets ship with their owning stage; P1 follow within the same
stage if at all possible, otherwise within the next stage; P2 ship with
their owning stage and run only on the nightly schedule until v1.0.

## Corpus layout

```
fuzz/
├── Cargo.toml
├── rust-toolchain.toml      # pins nightly
├── fuzz_targets/
│   ├── jsonl_parse_v1.rs
│   ├── upcaster_v1_to_current.rs
│   ├── normalize_fold.rs
│   └── ...
└── corpus/
    ├── jsonl_parse_v1/
    │   ├── seed_minimal.jsonl
    │   ├── seed_all_kinds.jsonl
    │   └── regression_*.jsonl
    ├── upcaster_v1_to_current/
    │   └── seed_all_kinds.bin
    └── ...
```

Seed corpora are hand-picked minimal valid inputs that bootstrap the
target. Regression corpora are minimised crash / hang inputs from
closed findings; they replay on every CI run.

## Oracle templates

A canonical raw-byte target:

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use trace_schema::{read_event, SchemaError};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return; };
    match read_event(s) {
        Ok(_event) => {}
        Err(SchemaError::Parse(_))
        | Err(SchemaError::UnsupportedSchemaVersion(_))
        | Err(SchemaError::InvalidShape(_)) => {}
    }
});
```

A canonical structure-aware target (upcaster v1):

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use trace_schema::{v1, upcast_to_current, Current};
use arbitrary::Arbitrary;

fuzz_target!(|event: v1::TraceEvent| {
    let current: Current = upcast_to_current(event.clone());
    // domain invariant: current's seq / session_id are preserved unchanged
    assert_eq!(current.seq, event.seq);
    assert_eq!(current.session_id, event.session_id);
});
```

## CI policy

- **Blocking gate per PR**:
  - bounded smoke: 60 s per P0 target;
  - regression replay: every committed corpus entry, every P0+P1
    target;
  - oracle invariants asserted in the target body (above).
- **Non-blocking nightly**:
  - deep run: 30 min per target, P0..P2;
  - filing: any new finding opens an issue tagged `fuzz-finding`;
  - the minimised input is committed under `fuzz/corpus/<target>/`
    before the fix lands.

## Known findings

Append-only list in the shape:

> **F-NNN — `<target>` — `<short description>`**
> Seed: `fuzz/corpus/<target>/<file>`.
> Found at: `<date>`. Fixed at: `<commit>` or `<deferred-to-stage>`.
> Notes: …

This mirrors griff's `fuzzing.md` "Known findings" section and serves
the same purpose: a flat, durable record of the bugs the fuzz layer
caught, and what each one cost to fix.

> **F-001 — `upcaster_v1_to_current` — writer emitted
> recorded-but-unreadable evidence**
> Seed: not retained — see Notes.
> Found at: 2026-08-18 (first run of the structure-aware target,
> within seconds). Fixed at: `d14774c` ("fix(S1): refuse to write
> lines no reader can parse back"; red test `bdea067`).
> Notes: `write_event` happily serialized an event whose embedded
> `args` nested deeper than serde_json's recursion limit; the
> resulting line could never be parsed back by any reader —
> recorded-then-unreadable evidence. Root cause: no complement
> between the write path and the read path's recursion limit. Fix:
> `write_event` depth-checks embedded values (iteratively) against
> the exact complement of the reader limit (`args`/`params` 126,
> `ValuePolicy::Raw` 125 under the 127-container document ceiling).
> The original minimized artifact is **not retained**: it was
> discarded before this ledger's discipline was applied, and it
> cannot be re-committed honestly — the fix also depth-bounded the
> target's `Arbitrary` mirror, so no byte string decodes to a
> deep-enough value under the current generator. The deterministic
> regression lives in
> `crates/trace-schema/tests/wire_limits.rs` (both sides of both
> boundaries), which is stronger than a corpus replay for this class.

## See also

- [`adr/0010-fuzz-storage-parsers-and-upcaster-chain.md`](adr/0010-fuzz-storage-parsers-and-upcaster-chain.md)
- [`adr/0006-upcaster-pattern-for-schema-evolution.md`](adr/0006-upcaster-pattern-for-schema-evolution.md)
- [`upcasters.md`](upcasters.md)
- [`SPEC.md`](SPEC.md) §12
