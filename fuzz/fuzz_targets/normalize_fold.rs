//! Fuzz target `normalize_fold` (ADR-0010 P1, S3).
//!
//! Drives the normalizer with arbitrary JSONL: parse whatever events
//! decode, group by session, and fold each. Oracle:
//!
//! - no panic / no hang / no unbounded allocation;
//! - `normalize` is deterministic (same session → same output);
//! - every produced action's `abstract_args` is a fixed point of value
//!   abstraction (re-abstraction is a no-op).

#![no_main]

use std::collections::HashMap;

use libfuzzer_sys::fuzz_target;
use trace_core::Session;
use trace_normalizer::abstraction::abstract_value;
use trace_normalizer::{NormCfg, normalize};
use trace_schema::read_event;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    let mut groups: HashMap<_, Vec<_>> = HashMap::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(event) = read_event(line) {
            groups.entry(event.session_id).or_default().push(event);
        }
    }

    let cfg = NormCfg::default();
    for (sid, events) in groups {
        let Ok(session) = Session::new(sid, events) else {
            continue;
        };
        let first = normalize(&session, &cfg);
        let second = normalize(&session, &cfg);
        assert_eq!(first, second, "normalize is not deterministic");

        for action in &first.0.actions {
            let re = abstract_value(&action.abstract_args, &cfg);
            assert_eq!(
                re, action.abstract_args,
                "normalized args are not a fixed point of abstraction",
            );
        }
    }
});
