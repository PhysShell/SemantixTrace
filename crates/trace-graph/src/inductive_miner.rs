//! Inductive Miner (IMDF variant) for `trace-graph` (S8).
//!
//! Implements the Inductive Miner – Directly Follows algorithm
//! (Leemans et al., 2013) over [`Scenario`] corpora.  The miner
//! produces a *process tree* internally, then maps it back to an
//! [`ActionGraph`] for CLI compatibility.
//!
//! # Algorithm outline
//!
//! 1. Build a Directly-Follows Graph (DFG) from the activity sequences.
//! 2. Identify start and end activities.
//! 3. Attempt cuts in priority order:
//!    - Exclusive-choice (`×`): no DFG edge crosses partition boundaries.
//!    - Sequence (`→`): a total order exists where all edges are forward.
//!    - Loop (`↺`): body activities alternate with redo activities.
//! 4. If no cut applies: fall through to a *flower model*.
//! 5. Recurse on each sub-log produced by the cut.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use trace_core::scenario::{CanonicalAction, Scenario};
use trace_core::{CommandId, ScreenId};

use crate::builder::get_or_insert;
use crate::graph::{ActionGraph, Transition};

/// A process-tree node produced by the Inductive Miner.
#[allow(missing_docs)]
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ProcessTree {
    /// A single activity.
    Activity(String),
    /// Sequence of children (→).
    Sequence(Vec<ProcessTree>),
    /// Exclusive choice between children (×).
    ExclusiveChoice(Vec<ProcessTree>),
    /// Loop: body followed by optional redo (↺).
    Loop {
        /// The main body (executed at least once).
        body: Box<ProcessTree>,
        /// The redo part (executed zero or more times).
        redo: Box<ProcessTree>,
    },
    /// Flower model: any activity may follow any other (τ*).
    Flower(Vec<String>),
    /// Silent (tau) step — skippable.
    Tau,
}

impl ProcessTree {
    /// Render the process tree as a human-readable expression string.
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Activity(name) => name.clone(),
            Self::Sequence(children) => {
                let parts: Vec<_> = children.iter().map(Self::display).collect();
                format!("→({})", parts.join(", "))
            }
            Self::ExclusiveChoice(children) => {
                let parts: Vec<_> = children.iter().map(Self::display).collect();
                format!("×({})", parts.join(", "))
            }
            Self::Loop { body, redo } => {
                format!("↺({}, {})", body.display(), redo.display())
            }
            Self::Flower(activities) => {
                format!("flower({})", activities.join(", "))
            }
            Self::Tau => "τ".to_owned(),
        }
    }
}

/// Inductive Miner configuration.
#[derive(Clone, Copy, Debug)]
pub struct ImdfConfig {
    /// Minimum directly-follows count to consider an edge significant.
    /// Edges below this threshold are filtered before cut detection.
    /// Default: `1` (all edges kept).
    pub min_df_count: u64,
}

impl Default for ImdfConfig {
    fn default() -> Self {
        Self { min_df_count: 1 }
    }
}

/// The Inductive Miner – Directly Follows (IMDF) variant.
#[derive(Clone, Copy, Debug, Default)]
pub struct InductiveMiner {
    /// Configuration knobs.
    pub config: ImdfConfig,
}

impl InductiveMiner {
    /// Create a new miner with the given configuration.
    #[must_use]
    pub const fn new(config: ImdfConfig) -> Self {
        Self { config }
    }

    /// Discover a process model from `scenarios`.
    ///
    /// Returns a CLI-compatible [`ActionGraph`] and the raw
    /// [`ProcessTree`] for optional `--format process-tree` rendering.
    #[must_use]
    pub fn mine(&self, scenarios: &[Scenario]) -> (ActionGraph, ProcessTree) {
        let traces: Vec<Vec<String>> = scenarios
            .iter()
            .map(|s| {
                s.actions
                    .iter()
                    .map(|a| a.command_id.as_str().to_owned())
                    .collect()
            })
            .collect();

        let all_activities: BTreeSet<String> =
            traces.iter().flat_map(|t| t.iter().cloned()).collect();

        let tree = self.discover(&traces, &all_activities);
        let graph = tree_to_graph(&tree);
        (graph, tree)
    }

    // -----------------------------------------------------------------------
    // Internal recursive discovery
    // -----------------------------------------------------------------------

    fn discover(self, traces: &[Vec<String>], activities: &BTreeSet<String>) -> ProcessTree {
        if activities.is_empty() {
            return ProcessTree::Tau;
        }
        if activities.len() == 1 {
            let name = activities.iter().next().expect("non-empty").clone();
            return ProcessTree::Activity(name);
        }

        let dfg = build_dfg(traces, activities, self.config.min_df_count);
        let starts = start_activities(traces, activities);
        let ends = end_activities(traces, activities);

        // Exclusive-choice cut.
        if let Some(groups) = xor_cut(activities, &dfg) {
            if groups.len() > 1 {
                let children = groups
                    .iter()
                    .map(|g| {
                        let sub = filter_traces(traces, g);
                        self.discover(&sub, g)
                    })
                    .collect();
                return ProcessTree::ExclusiveChoice(children);
            }
        }

        // Sequence cut.
        if let Some(groups) = sequence_cut(activities, &dfg) {
            if groups.len() > 1 {
                let children = groups
                    .iter()
                    .map(|g| {
                        let sub = filter_traces(traces, g);
                        self.discover(&sub, g)
                    })
                    .collect();
                return ProcessTree::Sequence(children);
            }
        }

        // Loop cut.
        if let Some((body, redo)) = loop_cut(activities, &dfg, &starts, &ends) {
            let body_traces = filter_traces(traces, &body);
            let redo_traces = filter_traces(traces, &redo);
            let body_tree = self.discover(&body_traces, &body);
            let redo_tree = if redo.is_empty() {
                ProcessTree::Tau
            } else {
                self.discover(&redo_traces, &redo)
            };
            return ProcessTree::Loop {
                body: Box::new(body_tree),
                redo: Box::new(redo_tree),
            };
        }

        // Flower fallthrough.
        ProcessTree::Flower(activities.iter().cloned().collect())
    }
}

// ---------------------------------------------------------------------------
// DFG helpers
// ---------------------------------------------------------------------------

type Dfg = HashMap<(String, String), u64>;

fn build_dfg(traces: &[Vec<String>], activities: &BTreeSet<String>, min_count: u64) -> Dfg {
    let mut dfg: Dfg = HashMap::new();
    for trace in traces {
        let filtered: Vec<&String> = trace.iter().filter(|a| activities.contains(*a)).collect();
        for w in filtered.windows(2) {
            *dfg.entry((w[0].clone(), w[1].clone())).or_insert(0) += 1;
        }
    }
    dfg.retain(|_, v| *v >= min_count);
    dfg
}

fn start_activities<'a>(
    traces: &'a [Vec<String>],
    activities: &BTreeSet<String>,
) -> BTreeSet<&'a str> {
    traces
        .iter()
        .filter_map(|t| t.iter().find(|a| activities.contains(*a)))
        .map(String::as_str)
        .collect()
}

fn end_activities<'a>(
    traces: &'a [Vec<String>],
    activities: &BTreeSet<String>,
) -> BTreeSet<&'a str> {
    traces
        .iter()
        .filter_map(|t| t.iter().rev().find(|a| activities.contains(*a)))
        .map(String::as_str)
        .collect()
}

// ---------------------------------------------------------------------------
// Cut detection
// ---------------------------------------------------------------------------

/// Exclusive-choice cut: connected components in the undirected DFG.
fn xor_cut(activities: &BTreeSet<String>, dfg: &Dfg) -> Option<Vec<BTreeSet<String>>> {
    fn find<'a>(parent: &mut BTreeMap<&'a str, &'a str>, x: &'a str) -> &'a str {
        if parent[x] == x {
            return x;
        }
        let root = find(parent, parent[x]);
        parent.insert(x, root);
        root
    }

    fn union<'a>(parent: &mut BTreeMap<&'a str, &'a str>, x: &'a str, y: &'a str) {
        let rx = find(parent, x);
        let ry = find(parent, y);
        if rx != ry {
            parent.insert(rx, ry);
        }
    }

    let mut parent: BTreeMap<&str, &str> = activities
        .iter()
        .map(|a| (a.as_str(), a.as_str()))
        .collect();

    for (from, to) in dfg.keys() {
        if activities.contains(from.as_str()) && activities.contains(to.as_str()) {
            union(&mut parent, from, to);
        }
    }

    let act_strs: Vec<&str> = activities.iter().map(String::as_str).collect();
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for a in act_strs {
        let root = find(&mut parent, a).to_owned();
        groups.entry(root).or_default().insert(a.to_owned());
    }

    let result: Vec<BTreeSet<String>> = groups.into_values().collect();
    if result.len() > 1 {
        Some(result)
    } else {
        None
    }
}

/// Sequence cut: topological order of activities in the DFG.
fn sequence_cut(activities: &BTreeSet<String>, dfg: &Dfg) -> Option<Vec<BTreeSet<String>>> {
    let acts: Vec<&str> = activities.iter().map(String::as_str).collect();

    // Build reachability via Floyd-Warshall.
    let mut reachable: HashSet<(&str, &str)> = HashSet::new();
    for (f, t) in dfg.keys() {
        if activities.contains(f.as_str()) && activities.contains(t.as_str()) {
            reachable.insert((f.as_str(), t.as_str()));
        }
    }
    for &k in &acts {
        for &i in &acts {
            if reachable.contains(&(i, k)) {
                for &j in &acts {
                    if reachable.contains(&(k, j)) {
                        reachable.insert((i, j));
                    }
                }
            }
        }
    }

    // Topological sort: repeatedly pick the minimal element.
    let mut ordered: Vec<&str> = Vec::new();
    let mut remaining: Vec<&str> = acts.clone();

    while !remaining.is_empty() {
        let min_idx = remaining.iter().position(|&a| {
            !remaining
                .iter()
                .any(|&b| b != a && reachable.contains(&(b, a)))
        })?;
        ordered.push(remaining.remove(min_idx));
    }

    // Verify total order.
    for i in 0..ordered.len() {
        for j in (i + 1)..ordered.len() {
            if !reachable.contains(&(ordered[i], ordered[j])) {
                return None;
            }
        }
    }

    let groups: Vec<BTreeSet<String>> = ordered
        .into_iter()
        .map(|a| {
            let mut g = BTreeSet::new();
            g.insert(a.to_owned());
            g
        })
        .collect();

    if groups.len() > 1 {
        Some(groups)
    } else {
        None
    }
}

/// Loop cut: separate body activities from redo activities.
fn loop_cut(
    activities: &BTreeSet<String>,
    dfg: &Dfg,
    starts: &BTreeSet<&str>,
    ends: &BTreeSet<&str>,
) -> Option<(BTreeSet<String>, BTreeSet<String>)> {
    let mut redo: BTreeSet<String> = BTreeSet::new();
    for (f, t) in dfg.keys() {
        if ends.contains(f.as_str()) && starts.contains(t.as_str()) {
            redo.insert(f.clone());
        }
    }
    let body: BTreeSet<String> = activities.difference(&redo).cloned().collect();
    if redo.is_empty() || body.is_empty() {
        None
    } else {
        Some((body, redo))
    }
}

// ---------------------------------------------------------------------------
// Sub-log filtering
// ---------------------------------------------------------------------------

fn filter_traces(traces: &[Vec<String>], activities: &BTreeSet<String>) -> Vec<Vec<String>> {
    traces
        .iter()
        .map(|t| {
            t.iter()
                .filter(|a| activities.contains(*a))
                .cloned()
                .collect()
        })
        .filter(|t: &Vec<String>| !t.is_empty())
        .collect()
}

// ---------------------------------------------------------------------------
// Process tree → ActionGraph
// ---------------------------------------------------------------------------

/// Convert a process tree to an [`ActionGraph`] for CLI compatibility.
///
/// Edges are derived from the tree structure so that only semantically
/// implied transitions appear in the graph:
///
/// - **Sequence**: exit leaves of child *i* connect to entry leaves of child
///   *i+1*.
/// - **`ExclusiveChoice`**: branches are independent; no cross-edges.
/// - **Loop**: body-exit → redo-entry and redo-exit → body-entry.
/// - **Flower**: every activity may follow every other (any-to-any edges).
/// - **Activity / Tau**: single node or empty.
///
/// Synthetic `ScreenId("discovered")` is used for all nodes since the miner
/// only sees command names.
#[must_use]
pub fn tree_to_graph(tree: &ProcessTree) -> ActionGraph {
    let mut graph = ActionGraph::new();
    // Insert all leaf nodes.
    for name in &collect_leaf_names(tree) {
        get_or_insert(&mut graph, &synthetic_action(name));
    }
    // Derive edges that the tree structure actually implies.
    let (_, _, edges) = collect_tree_edges(tree);
    for (from, to) in edges {
        let from_idx = get_or_insert(&mut graph, &synthetic_action(&from));
        let to_idx = get_or_insert(&mut graph, &synthetic_action(&to));
        graph.graph.add_edge(
            from_idx,
            to_idx,
            Transition {
                frequency: 1,
                failure_count: 0,
                anomaly_score: 0.0,
            },
        );
    }
    graph
}

fn synthetic_action(command_name: &str) -> CanonicalAction {
    CanonicalAction {
        screen_id: ScreenId::new("discovered"),
        command_id: CommandId::new(command_name),
        abstract_args: serde_json::json!({}),
    }
}

/// Recursively derive the structurally implied control-flow edges.
///
/// Returns `(entry_leaves, exit_leaves, edges)` where:
/// - `entry_leaves`: activities that can execute first in the subtree.
/// - `exit_leaves`: activities that can execute last in the subtree.
/// - `edges`: `(from, to)` pairs of activities that are directly sequenced.
#[allow(clippy::type_complexity)]
fn collect_tree_edges(tree: &ProcessTree) -> (Vec<String>, Vec<String>, Vec<(String, String)>) {
    match tree {
        ProcessTree::Activity(name) => (vec![name.clone()], vec![name.clone()], vec![]),
        ProcessTree::Tau => (vec![], vec![], vec![]),

        ProcessTree::Sequence(children) => {
            let mut entries: Vec<String> = vec![];
            let mut exits: Vec<String> = vec![];
            let mut edges: Vec<(String, String)> = vec![];
            let mut prev_exits: Vec<String> = vec![];

            for (i, child) in children.iter().enumerate() {
                let (child_entries, child_exits, child_edges) = collect_tree_edges(child);
                edges.extend(child_edges);
                if i == 0 {
                    entries.clone_from(&child_entries);
                } else {
                    for prev in &prev_exits {
                        for entry in &child_entries {
                            edges.push((prev.clone(), entry.clone()));
                        }
                    }
                }
                prev_exits.clone_from(&child_exits);
                if i + 1 == children.len() {
                    exits.clone_from(&child_exits);
                }
            }
            (entries, exits, edges)
        }

        ProcessTree::ExclusiveChoice(children) => {
            // Branches are mutually exclusive — no cross-branch edges.
            let mut entries: Vec<String> = vec![];
            let mut exits: Vec<String> = vec![];
            let mut edges: Vec<(String, String)> = vec![];
            for child in children {
                let (child_entries, child_exits, child_edges) = collect_tree_edges(child);
                entries.extend(child_entries);
                exits.extend(child_exits);
                edges.extend(child_edges);
            }
            (entries, exits, edges)
        }

        ProcessTree::Loop { body, redo } => {
            let (body_entries, body_exits, body_edges) = collect_tree_edges(body);
            let (redo_entries, redo_exits, redo_edges) = collect_tree_edges(redo);
            let mut edges = body_edges;
            edges.extend(redo_edges);
            // body-exit → redo-entry (start the redo pass)
            for exit in &body_exits {
                for entry in &redo_entries {
                    edges.push((exit.clone(), entry.clone()));
                }
            }
            // redo-exit → body-entry (loop back)
            for exit in &redo_exits {
                for entry in &body_entries {
                    edges.push((exit.clone(), entry.clone()));
                }
            }
            (body_entries, body_exits, edges)
        }

        ProcessTree::Flower(activities) => {
            // Flower = τ*: any activity may follow any other.
            let mut edges: Vec<(String, String)> = vec![];
            for a in activities {
                for b in activities {
                    if a != b {
                        edges.push((a.clone(), b.clone()));
                    }
                }
            }
            (activities.clone(), activities.clone(), edges)
        }
    }
}

fn collect_leaf_names(tree: &ProcessTree) -> Vec<String> {
    match tree {
        ProcessTree::Activity(name) => vec![name.clone()],
        ProcessTree::Sequence(children) | ProcessTree::ExclusiveChoice(children) => {
            children.iter().flat_map(collect_leaf_names).collect()
        }
        ProcessTree::Loop { body, redo } => {
            let mut names = collect_leaf_names(body);
            names.extend(collect_leaf_names(redo));
            names
        }
        ProcessTree::Flower(activities) => activities.clone(),
        ProcessTree::Tau => vec![],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use trace_core::scenario::{CanonicalAction, Scenario};
    use trace_core::{CommandId, ScreenId};

    use super::InductiveMiner;

    fn make_scenario(actions: &[&str]) -> Scenario {
        let acts: Vec<CanonicalAction> = actions
            .iter()
            .map(|&cmd| CanonicalAction {
                screen_id: ScreenId::new("Screen"),
                command_id: CommandId::new(cmd),
                abstract_args: serde_json::json!({}),
            })
            .collect();
        Scenario::new(acts)
    }

    #[test]
    fn single_activity_returns_leaf() {
        let scenarios = vec![make_scenario(&["A"])];
        let miner = InductiveMiner::default();
        let (graph, tree) = miner.mine(&scenarios);
        assert_eq!(graph.node_count(), 1);
        assert!(matches!(tree, super::ProcessTree::Activity(_)));
    }

    #[test]
    fn sequence_discovered() {
        let scenarios = vec![
            make_scenario(&["A", "B", "C"]),
            make_scenario(&["A", "B", "C"]),
        ];
        let miner = InductiveMiner::default();
        let (_graph, tree) = miner.mine(&scenarios);
        assert!(
            matches!(tree, super::ProcessTree::Sequence(_)),
            "expected Sequence, got: {}",
            tree.display()
        );
    }

    #[test]
    fn xor_discovered_for_disjoint_traces() {
        let scenarios = vec![make_scenario(&["A"]), make_scenario(&["B"])];
        let miner = InductiveMiner::default();
        let (_graph, tree) = miner.mine(&scenarios);
        assert!(
            matches!(tree, super::ProcessTree::ExclusiveChoice(_)),
            "expected XOR, got: {}",
            tree.display()
        );
    }

    #[test]
    fn process_tree_display_smoke() {
        let scenarios = vec![make_scenario(&["A", "B"])];
        let miner = InductiveMiner::default();
        let (_graph, tree) = miner.mine(&scenarios);
        let _ = tree.display();
    }
}
