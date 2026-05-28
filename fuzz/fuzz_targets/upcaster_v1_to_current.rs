//! Fuzz target `upcaster_v1_to_current` — round-trip of a v1 envelope
//! through write → read, asserting per-event invariants survive.
//!
//! At S1 the chain is identity (Current = v1), so this target's
//! invariant currently collapses to "serialize then read_event yields
//! the same event". The target exists from S1 so the v2 bump only adds
//! mutations to its `arbitrary` inputs without touching the harness
//! plumbing (per ADR-0010 and `docs/fuzzing.md` P0 catalogue).

#![no_main]

use libfuzzer_sys::fuzz_target;
use trace_schema::{read_event, write_event};

fuzz_target!(|data: &[u8]| {
    // Phase 1 — drive `read_event` with the raw input. Same contract
    // as `jsonl_parse_v1` (no panic, no hang, typed error).
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    let event = match read_event(s) {
        Ok(e) => e,
        Err(_) => return,
    };

    // Phase 2 — re-serialize and re-parse; assert the upcast result is
    // stable across the round trip.
    let raw = match write_event(&event) {
        Ok(r) => r,
        Err(_) => return,
    };
    let trimmed = raw.trim_end_matches('\n');
    let event2 = read_event(trimmed).expect("re-read of self-written event must succeed");
    assert_eq!(
        event, event2,
        "upcast chain is not idempotent across write/read",
    );
});
