//! Fuzz target `jsonl_parse_v1` — raw-bytes fuzz of `trace_schema::read_event`.
//!
//! Per ADR-0010 / `docs/fuzzing.md`:
//!
//! - **No panics.** A panic = libFuzzer crash = bug.
//! - **No hangs.** libFuzzer `-timeout=10` enforces operationally.
//! - **No unbounded allocation.** libFuzzer `-rss_limit_mb` enforces.
//! - **Typed-error contract.** Every error must be one of the
//!   `SchemaError::*` variants — any non-typed panic escapes through
//!   the harness as a crash.

#![no_main]

use libfuzzer_sys::fuzz_target;
use trace_schema::{SchemaError, read_event, write_event};

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else {
        return;
    };
    match read_event(s) {
        // fuzzing.md oracle: a parsed event re-serializes to a line
        // `read_event` accepts again, and the result is stable.
        Ok(event) => {
            let raw = write_event(&event).expect("self-written event must serialize");
            let reread = read_event(raw.trim_end_matches('\n'))
                .expect("re-read of self-written event must succeed");
            assert_eq!(event, reread, "write/read round-trip must be stable");
        }
        Err(
            SchemaError::Parse(_)
            | SchemaError::InvalidShape(_)
            | SchemaError::UnsupportedSchemaVersion(_),
        ) => {}
        // `SchemaError` is `#[non_exhaustive]` (ADR-0012 C-NON-EXHAUSTIVE);
        // future variants are still typed errors, not panics.
        Err(_) => {}
    }
});
