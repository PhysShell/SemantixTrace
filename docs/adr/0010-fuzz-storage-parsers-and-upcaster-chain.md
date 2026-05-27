# ADR 0010: Fuzz-test storage parsers, the upcaster chain, and selected normalizer transforms

Date: 2026-05-27
Status: Accepted

## Context

SemantxTrace ingests externally produced JSONL (recorded by the WPF
adapter or a future Avalonia / MAUI adapter, or hand-edited by an
operator), passes it through the upcaster chain (ADR-0006), normalizes
it, and builds graphs / runs oracles on the result. The wire boundary
is exactly the place where a `parse → upcast` pipeline turns "the file
is fine" into "the parser died on one byte". Unit and property tests
based on hand-picked inputs will not surface the inputs no one thought
to write.

Two architectural constraints shape this decision:

- The workspace forbids `unsafe_code` (ADR-0004); `cargo-fuzz` /
  libFuzzer emit an `extern "C"` harness that cannot satisfy that lint
  policy.
- The MSRV path stays stable; `cargo-fuzz` requires nightly.

## Decision

1. **Mandatory layer.** The JSONL parser, the upcaster chain (every
   `From` impl and the dispatching `read_event`), the WPF adapter's
   event ingest, and selected normalizer transforms MUST have fuzz
   targets. The current target list, oracles, corpus layout, CI policy,
   priorities, and per-stage mapping are specified in
   [`../fuzzing.md`](../fuzzing.md) and summarised as SPEC hard rule 12.

2. **Tooling.** `cargo-fuzz` + `libfuzzer-sys`, with `arbitrary` for
   structure-aware targets (upcaster inputs, normalizer transforms).
   `proptest` remains the tool for readable invariant specs.

3. **Isolation.** `fuzz/` is its own workspace root and is **excluded**
   from the parent workspace. It carries a local `rust-toolchain.toml`
   pinning nightly. The stable `crates/*` workspace, its pinned
   toolchain, and its lint policy are untouched.

4. **Oracle enforcement.** "No hang" and "no uncontrolled allocation"
   are enforced operationally by libFuzzer `-timeout`, `-rss_limit_mb`,
   and `-malloc_limit_mb`; "no panic" is a libFuzzer crash; the typed-
   error contract (`Ok(_)` xor a typed error enum) is asserted in the
   target body.

5. **CI policy.** Bounded smoke fuzzing (~60 s/target) plus replay of
   the committed regression corpus is a **blocking** PR gate. Deep
   fuzzing runs scheduled / nightly, **non-blocking**, files issues.
   Deep fuzzing is non-deterministic and is deliberately kept off the
   blocking path.

6. **No new stage label.** Targets are scheduled by a P0/P1/P2 priority
   table mapped onto canonical stages (`jsonl_parse` and
   `upcaster_v1_to_current` at S1/S2, `normalize_fold` at S3,
   `graph_build` at S4, `oracle_replay` at S5, additional
   per-adapter targets at S6/S10). See
   [`../fuzzing.md`](../fuzzing.md).

## Consequences

- Wire-format bugs are caught before they reach production traces.
- A nightly dependency exists, but only inside `fuzz/`; the main
  workspace stays stable and reproducible.
- The `fuzz/` crate is outside `unsafe_code = "forbid"` and the
  workspace lint policy by necessity (libFuzzer harness). The blast
  radius is one excluded crate that ships no library code.
- Fuzzing proves robustness, not correctness; it complements, never
  replaces, unit / property / golden tests.
- Deep fuzzing is non-deterministic, so only the bounded smoke +
  regression replay is a blocking gate.
- Migrating to `afl.rs` (stable) later does not invalidate the targets;
  only the harness crate would change.
