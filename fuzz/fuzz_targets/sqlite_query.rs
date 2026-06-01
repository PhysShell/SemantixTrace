//! Fuzz target: `sqlite_query` (S8).
//!
//! Structure-aware fuzzer over SQLite filter combinations.
//! Invariants: never panics, always produces a bounded result set.

#![no_main]

use libfuzzer_sys::fuzz_target;
use trace_storage::sqlite::{SliceBy, SqliteBackend};

fuzz_target!(|data: &[u8]| {
    // Need at least 1 byte for the filter discriminant.
    if data.is_empty() {
        return;
    }

    let discriminant = data[0] % 5;
    let rest = &data[1..];
    let Ok(value) = std::str::from_utf8(rest) else {
        return;
    };

    // In-memory SQLite: no filesystem side-effects.
    let Ok(backend) = SqliteBackend::open(std::path::Path::new(":memory:")) else {
        return;
    };

    let filter = match discriminant {
        0 => SliceBy::SessionId(value.to_owned()),
        1 => SliceBy::CommandId(value.to_owned()),
        2 => SliceBy::ScreenId(value.to_owned()),
        3 => SliceBy::Outcome(value.to_owned()),
        _ => SliceBy::DomainEntityId(value.to_owned()),
    };

    // Must never panic; result set is always bounded (empty DB).
    let _ = backend.slice_events(&filter);
});
