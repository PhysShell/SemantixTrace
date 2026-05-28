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

    /// Returns every [`CommandId`] whose total visit count falls below
    /// `floor_fraction` of `total_visits`.
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
        self.graph
            .node_weights()
            .filter(|n| (n.visit_count as f64) < floor)
            .map(|n| (n.action.command_id.clone(), n.visit_count))
            .collect()
    }
}

impl Default for ActionGraph {
    fn default() -> Self {
        Self::new()
    }
}
