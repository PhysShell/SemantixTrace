//! Property tests for `trace-graph` (S4).

use proptest::prelude::*;
use serde_json::json;
use trace_core::{CanonicalAction, CommandId, Scenario, ScreenId};
use trace_graph::{from_scenarios, prefixspan};

fn arb_action() -> impl Strategy<Value = CanonicalAction> {
    (0usize..5, 0usize..5).prop_map(|(s, c)| CanonicalAction {
        screen_id: ScreenId::new(format!("S{s}")),
        command_id: CommandId::new(format!("C{c}")),
        abstract_args: json!({}),
    })
}

fn arb_scenario() -> impl Strategy<Value = Scenario> {
    proptest::collection::vec(arb_action(), 0..10).prop_map(Scenario::new)
}

proptest! {
    #[test]
    fn graph_build_is_deterministic(
        scenarios in proptest::collection::vec(arb_scenario(), 0..20)
    ) {
        let g1 = from_scenarios(&scenarios);
        let g2 = from_scenarios(&scenarios);
        prop_assert_eq!(g1.node_count(), g2.node_count());
        prop_assert_eq!(g1.edge_count(), g2.edge_count());
    }

    #[test]
    fn single_scenario_has_correct_node_count(
        actions in proptest::collection::vec(arb_action(), 1..10)
    ) {
        // Remove consecutive duplicates so each action is unique in sequence.
        let mut deduped: Vec<CanonicalAction> = Vec::new();
        for a in &actions {
            if deduped.last() != Some(a) {
                deduped.push(a.clone());
            }
        }
        if deduped.is_empty() {
            return Ok(());
        }
        let scenario = Scenario::new(deduped.clone());
        let graph = from_scenarios(&[scenario]);
        // Unique nodes equals distinct canonical actions.
        let unique_actions: std::collections::HashSet<_> = deduped.iter().collect();
        prop_assert_eq!(graph.node_count(), unique_actions.len());
    }

    #[test]
    fn prefixspan_patterns_are_subsequences(
        sequences in proptest::collection::vec(
            proptest::collection::vec(arb_action(), 0..8),
            0..20
        )
    ) {
        if sequences.is_empty() {
            return Ok(());
        }
        let min_support = 1;
        let patterns = prefixspan(&sequences, min_support);
        for pattern in &patterns {
            let count = sequences.iter().filter(|seq| {
                contains_subseq(seq, &pattern.sequence)
            }).count();
            prop_assert!(
                count >= min_support,
                "pattern {:?} has support {} but appears in {} sequences",
                pattern.sequence, pattern.support, count
            );
        }
    }

    #[test]
    fn prefixspan_support_count_is_correct(
        sequences in proptest::collection::vec(
            proptest::collection::vec(arb_action(), 0..6),
            0..15
        )
    ) {
        let min_support = 1;
        let patterns = prefixspan(&sequences, min_support);
        for pattern in &patterns {
            let actual_support = sequences.iter().filter(|seq| {
                contains_subseq(seq, &pattern.sequence)
            }).count();
            prop_assert_eq!(
                actual_support,
                pattern.support,
                "wrong support for {:?}",
                pattern.sequence
            );
        }
    }
}

fn contains_subseq(seq: &[CanonicalAction], sub: &[CanonicalAction]) -> bool {
    if sub.is_empty() {
        return true;
    }
    let mut j = 0;
    for item in seq {
        if item == &sub[j] {
            j += 1;
            if j == sub.len() {
                return true;
            }
        }
    }
    false
}
