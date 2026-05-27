//! Placeholder fuzz target so the isolated `fuzz/` crate has at least one
//! buildable bin during S0. Real targets (`jsonl_parse_v1`,
//! `upcaster_v1_to_current`, …) land per ADR-0010 / `docs/fuzzing.md`.
//!
//! This file is **not** a libFuzzer harness; libFuzzer wiring lands with
//! the first real target.

fn main() {
    // Intentionally empty.
}
