//! Core [`ActionGraph`] type: a directed weighted graph of
//! canonical-action transitions.

use std::collections::HashMap;
use std::fmt;

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use trace_core::{CanonicalAction, CommandId};

/// A node in the action graph, representing one distinct
/// [`CanonicalAction`] and how many times it was visited across all
/// scenarios.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionNode {
    /// The canonical action this node represents.
    pub action: CanonicalAction,
    /// Number of times this action appeared across all input scenarios.
    pub visit_count: u64,
}

/// An edge in the action graph, representing a direct sequential
/// transition from one action to the next.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transition {
    /// How many times this transition was observed across all scenarios.
    pub frequency: u64,
    /// How many times this transition ended in a session that also had
    /// an `ExceptionThrown` event.  Populated by
    /// [`crate::builder`] only when error-session data is provided.
    pub failure_count: u64,
    /// Normalised anomaly score: `0.0` = perfectly normal; `1.0` =
    /// maximally anomalous.
    ///
    /// Derived from the Heuristics dependency measure:
    /// ```text
    /// dep(a,b) = (freq(a,b) − freq(b,a)) / (freq(a,b) + freq(b,a) + 1)
    /// anomaly  = (1.0 − dep(a,b)) / 2.0
    /// ```
    pub anomaly_score: f64,
}

/// Directed weighted graph of canonical-action transitions, built from
/// a corpus of normalized [`Scenario`](trace_core::Scenario)s.
///
/// Internally the graph is a [`petgraph::graph::DiGraph`]; nodes carry
/// [`ActionNode`]s and edges carry [`Transition`]s.  The `node_index`
/// map provides O(1) lookup from a [`CanonicalAction`] to its
/// [`NodeIndex`].
pub struct ActionGraph {
    pub(crate) graph: DiGraph<ActionNode, Transition>,
    pub(crate) node_index: HashMap<CanonicalAction, NodeIndex>,
}

impl fmt::Debug for ActionGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActionGraph")
            .field("nodes", &self.graph.node_count())
            .field("edges", &self.graph.edge_count())
            .finish_non_exhaustive()
    }
}

impl ActionGraph {
    /// Create an empty action graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
        }
    }

    /// Number of nodes (distinct canonical actions) in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Number of directed edges (distinct transitions) in the graph.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Access the underlying petgraph `DiGraph` for traversal and
    /// algorithm use.
    #[must_use]
    pub const fn graph(&self) -> &DiGraph<ActionNode, Transition> {
        &self.graph
    }

    /// Returns the [`NodeIndex`] for `action` if it is present in the
    /// graph, or `None` otherwise.
    #[must_use]
    pub fn node_index(&self, action: &CanonicalAction) -> Option<NodeIndex> {
        self.node_index.get(action).copied()
    }

    /// Returns every [`CommandId`] whose **total** visit count (summed across
    /// all screens it appears on) falls below `floor_fraction` of
    /// `total_visits`.
    ///
    /// Aggregates by `command_id` first so that a command appearing on
    /// multiple screens is evaluated against its combined frequency, not
    /// per-screen slices.
    ///
    /// Used for dead-feature detection in [`crate::report`].
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn commands_below_floor(
        &self,
        total_visits: u64,
        floor_fraction: f64,
    ) -> Vec<(CommandId, u64)> {
        let floor = (total_visits as f64) * floor_fraction;
        let mut by_command: HashMap<CommandId, u64> = HashMap::new();
        for node in self.graph.node_weights() {
            *by_command
                .entry(node.action.command_id.clone())
                .or_insert(0) += node.visit_count;
        }
        by_command
            .into_iter()
            .filter(|(_, count)| (*count as f64) < floor)
            .collect()
    }
}

impl Default for ActionGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use trace_core::{CanonicalAction, CommandId, Scenario, ScreenId};

    use crate::builder::from_scenarios;

    fn action(screen: &str, cmd: &str) -> CanonicalAction {
        CanonicalAction {
            screen_id: ScreenId::new(screen),
            command_id: CommandId::new(cmd),
            abstract_args: json!({}),
        }
    }

    #[test]
    fn commands_below_floor_aggregates_across_screens() {
        // "Export" appears on two screens: 3 visits on Screen1, 4 on Screen2 = 7 total.
        // "Rare" appears only on Screen1: 1 visit.
        // Total visits = 8. Floor at 50% = 4.
        // "Rare" (1 visit) should be flagged; "Export" (7 visits) must NOT.
        let scenarios: Vec<Scenario> = vec![
            // 3 sessions hitting Export on Screen1
            Scenario::new(vec![action("Screen1", "Export")]),
            Scenario::new(vec![action("Screen1", "Export")]),
            Scenario::new(vec![action("Screen1", "Export")]),
            // 4 sessions hitting Export on Screen2
            Scenario::new(vec![action("Screen2", "Export")]),
            Scenario::new(vec![action("Screen2", "Export")]),
            Scenario::new(vec![action("Screen2", "Export")]),
            Scenario::new(vec![action("Screen2", "Export")]),
            // 1 session hitting Rare on Screen1
            Scenario::new(vec![action("Screen1", "Rare")]),
        ];

        let graph = from_scenarios(&scenarios);
        let total_visits: u64 = graph.graph().node_weights().map(|n| n.visit_count).sum();

        // floor at 25% of total (= 2.0): only "Rare" (1 visit) qualifies.
        let below = graph.commands_below_floor(total_visits, 0.25);
        let names: Vec<String> = below
            .iter()
            .map(|(cmd, _)| cmd.as_str().to_owned())
            .collect();
        assert!(
            names.contains(&"Rare".to_owned()),
            "Rare should be a dead-feature candidate"
        );
        assert!(
            !names.contains(&"Export".to_owned()),
            "Export (7 visits, above floor) must not be flagged"
        );
    }
}
