//! Builder that constructs an [`ActionGraph`] from a slice of normalized
//! [`Scenario`]s.

use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use trace_core::{CanonicalAction, Scenario};

use crate::graph::{ActionGraph, ActionNode, Transition};

/// Build an [`ActionGraph`] from a corpus of normalized [`Scenario`]s.
///
/// The builder:
/// 1. Inserts a node for every distinct [`CanonicalAction`], accumulating
///    `visit_count`.
/// 2. Records every consecutive `(from, to)` pair, accumulating edge
///    frequencies.
/// 3. Computes the Heuristics dependency measure and derives
///    `anomaly_score` for each edge.
///
/// The result is deterministic for a given `scenarios` slice.
#[must_use]
pub fn from_scenarios(scenarios: &[Scenario]) -> ActionGraph {
    let mut graph = ActionGraph::new();
    // raw edge frequencies: (from_index, to_index) → count
    let mut edge_freq: HashMap<(NodeIndex, NodeIndex), u64> = HashMap::new();

    for scenario in scenarios {
        for action in &scenario.actions {
            let idx = get_or_insert(&mut graph, action);
            graph.graph[idx].visit_count += 1;
        }
        for window in scenario.actions.windows(2) {
            let from = get_or_insert(&mut graph, &window[0]);
            let to = get_or_insert(&mut graph, &window[1]);
            *edge_freq.entry((from, to)).or_insert(0) += 1;
        }
    }

    // Add edges with Heuristics-miner dependency measure and anomaly score.
    #[allow(clippy::cast_precision_loss)]
    for ((from, to), freq) in &edge_freq {
        let fwd = *freq as f64;
        let bwd = edge_freq.get(&(*to, *from)).copied().unwrap_or(0) as f64;
        let anomaly_score = if *from == *to {
            // Self-loop: dep = freq / (freq + 1), anomaly = 1 - dep
            let d = fwd / (fwd + 1.0);
            1.0 - d
        } else {
            let d = (fwd - bwd) / (fwd + bwd + 1.0);
            (1.0 - d) / 2.0
        };
        graph.graph.add_edge(
            *from,
            *to,
            Transition {
                frequency: *freq,
                failure_count: 0,
                anomaly_score,
            },
        );
    }

    graph
}

/// Return the [`NodeIndex`] for `action`, inserting a new node with
/// `visit_count = 0` if it does not yet exist.
pub(crate) fn get_or_insert(graph: &mut ActionGraph, action: &CanonicalAction) -> NodeIndex {
    if let Some(&idx) = graph.node_index.get(action) {
        return idx;
    }
    let idx = graph.graph.add_node(ActionNode {
        action: action.clone(),
        visit_count: 0,
    });
    graph.node_index.insert(action.clone(), idx);
    idx
}
