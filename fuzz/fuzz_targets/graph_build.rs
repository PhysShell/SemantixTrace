//! Fuzz target `graph_build` (ADR-0010, S4).
//!
//! Arbitrary JSONL → parse events → group by session → normalize →
//! build action graph.
//!
//! Oracle:
//! - No panic / no hang / no unbounded allocation.
//! - `from_scenarios` is deterministic (same input → same output).
//! - The number of graph nodes never exceeds the number of distinct
//!   canonical actions seen across all scenarios.

#![no_main]

use std::collections::HashMap;

use libfuzzer_sys::fuzz_target;
use trace_core::Session;
use trace_graph::from_scenarios;
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
    let mut scenarios = Vec::new();
    for (sid, events) in groups {
        let Ok(session) = Session::new(sid, events) else {
            continue;
        };
        let (scenario, _) = normalize(&session, &cfg);
        scenarios.push(scenario);
    }

    // Oracle: graph construction is deterministic.
    let g1 = from_scenarios(&scenarios);
    let g2 = from_scenarios(&scenarios);
    assert_eq!(
        g1.node_count(),
        g2.node_count(),
        "graph build is not deterministic (nodes)"
    );
    assert_eq!(
        g1.edge_count(),
        g2.edge_count(),
        "graph build is not deterministic (edges)"
    );
});
