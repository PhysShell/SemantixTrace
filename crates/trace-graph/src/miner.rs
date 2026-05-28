//! Heuristics miner: build a dependency graph from frequency data.
//!
//! The miner filters edges by the Heuristics dependency measure
//! (`dep(a,b) = (freq(a,b) − freq(b,a)) / (freq(a,b) + freq(b,a) + 1)`)
//! and frequency floor, retaining only sufficiently strong causal links.

use petgraph::visit::EdgeRef;
use trace_core::CanonicalAction;

use crate::graph::{ActionGraph, ActionNode};

/// Configuration for the Heuristics miner.
#[derive(Clone, Copy, Debug)]
pub struct MinerConfig {
    /// Minimum dependency measure to include an edge (default `0.5`).
    ///
    /// An edge `(a, b)` passes when `dep(a,b) >= dependency_threshold`,
    /// which is equivalent to `anomaly_score <= (1 − threshold) / 2`.
    pub dependency_threshold: f64,
    /// Minimum absolute frequency required for an edge to be included
    /// at all (default `1`).
    pub min_frequency: u64,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            dependency_threshold: 0.5,
            min_frequency: 1,
        }
    }
}

/// Heuristics process-discovery miner (ported from `PM4Py` / `ProM`
/// literature).
///
/// Operates on a pre-built [`ActionGraph`] to produce a filtered
/// dependency graph or identify frequent / anomalous paths.
#[derive(Clone, Copy, Debug)]
pub struct HeuristicsMiner;

impl HeuristicsMiner {
    /// Produce a dependency graph: keep only edges where the dependency
    /// measure `dep(a,b) >= cfg.dependency_threshold` **and**
    /// `frequency >= cfg.min_frequency`.
    ///
    /// All nodes are preserved; only edges that don't meet the thresholds
    /// are removed.
    ///
    /// # Equivalence
    ///
    /// `dep >= d  ⟺  anomaly_score <= (1 − d) / 2` for non-self-loops.
    #[must_use]
    pub fn mine(graph: &ActionGraph, cfg: &MinerConfig) -> ActionGraph {
        let anomaly_threshold = (1.0 - cfg.dependency_threshold) / 2.0;
        let mut out = ActionGraph::new();

        // Copy all nodes first so they always appear in output.
        for node_idx in graph.graph.node_indices() {
            let node = &graph.graph[node_idx];
            let new_idx = out.graph.add_node(ActionNode {
                action: node.action.clone(),
                visit_count: node.visit_count,
            });
            out.node_index.insert(node.action.clone(), new_idx);
        }

        // Copy only edges that pass both thresholds.
        for edge in graph.graph.edge_references() {
            let t = edge.weight();
            if t.frequency < cfg.min_frequency || t.anomaly_score > anomaly_threshold {
                continue;
            }
            let src_action = &graph.graph[edge.source()].action;
            let dst_action = &graph.graph[edge.target()].action;
            let src_new = out.node_index[src_action];
            let dst_new = out.node_index[dst_action];
            out.graph.add_edge(src_new, dst_new, *t);
        }

        out
    }

    /// Find the most frequent path through the graph.
    ///
    /// Uses a greedy approach: start from the node with the highest
    /// `visit_count` that has no predecessors (or, if all nodes have
    /// predecessors, the node with the highest `visit_count`), then at
    /// each step follow the outgoing edge with the highest `frequency`
    /// until no unvisited edge can be taken.
    ///
    /// Returns the sequence of [`CanonicalAction`]s along the path.
    #[must_use]
    pub fn most_frequent_path(graph: &ActionGraph) -> Vec<CanonicalAction> {
        if graph.graph.node_count() == 0 {
            return Vec::new();
        }

        // Find root: node with no incoming edges among all nodes.
        // Fall back to the node with the highest visit_count.
        let start = graph
            .graph
            .node_indices()
            .filter(|&n| {
                graph
                    .graph
                    .edges_directed(n, petgraph::Direction::Incoming)
                    .next()
                    .is_none()
            })
            .max_by_key(|&n| graph.graph[n].visit_count)
            .or_else(|| {
                graph
                    .graph
                    .node_indices()
                    .max_by_key(|&n| graph.graph[n].visit_count)
            });

        let Some(start_idx) = start else {
            return Vec::new();
        };

        let mut path = Vec::new();
        let mut visited_edges = std::collections::HashSet::new();
        let mut current = start_idx;

        path.push(graph.graph[current].action.clone());

        loop {
            // Pick the outgoing edge with the highest frequency not yet visited.
            let next = graph
                .graph
                .edges_directed(current, petgraph::Direction::Outgoing)
                .filter(|e| !visited_edges.contains(&e.id()))
                .max_by_key(|e| e.weight().frequency);

            let Some(edge) = next else {
                break;
            };

            visited_edges.insert(edge.id());
            current = edge.target();
            path.push(graph.graph[current].action.clone());

            // Avoid infinite loops in cyclic graphs — stop if we reach
            // a node we have already added to the path and there is no
            // fresh unvisited outgoing edge to continue with.
            if path.len() > graph.graph.node_count() + 1 {
                break;
            }
        }

        path
    }

    /// Return all edges whose `anomaly_score` exceeds `threshold`.
    ///
    /// Each entry is `(from_action, to_action, anomaly_score)`.
    #[must_use]
    pub fn anomaly_transitions(
        graph: &ActionGraph,
        threshold: f64,
    ) -> Vec<(CanonicalAction, CanonicalAction, f64)> {
        graph
            .graph
            .edge_references()
            .filter(|e| e.weight().anomaly_score > threshold)
            .map(|e| {
                let from = graph.graph[e.source()].action.clone();
                let to = graph.graph[e.target()].action.clone();
                (from, to, e.weight().anomaly_score)
            })
            .collect()
    }
}
