//! Graph exporters: Mermaid flowchart and Graphviz DOT.

use petgraph::visit::EdgeRef;

use crate::graph::ActionGraph;

/// Render `graph` as a [Mermaid](https://mermaid.js.org/) flowchart.
///
/// Nodes are shown as `N{index}["ScreenId:CommandId"]`. Edges above the
/// 80th-percentile frequency are annotated with `freq=N`; edges below
/// that threshold get a plain `-->`.
#[must_use]
pub fn to_mermaid(graph: &ActionGraph) -> String {
    let mut out = String::from("flowchart TD\n");

    // Collect node labels.
    for idx in graph.graph.node_indices() {
        let node = &graph.graph[idx];
        let label = format!("{}:{}", node.action.screen_id, node.action.command_id);
        out.push_str(&format!("    N{}[\"{}\"]\n", idx.index(), label));
    }

    // Compute p80 threshold for edge annotation.
    let p80 = p80_frequency(graph);

    for edge in graph.graph.edge_references() {
        let freq = edge.weight().frequency;
        let src = edge.source().index();
        let dst = edge.target().index();
        if freq >= p80 {
            out.push_str(&format!("    N{src} -->|\"freq={freq}\"| N{dst}\n"));
        } else {
            out.push_str(&format!("    N{src} --> N{dst}\n"));
        }
    }

    out
}

/// Render `graph` as a [Graphviz](https://graphviz.org/) DOT digraph.
///
/// All edges carry a `label` attribute showing `freq=N`.
#[must_use]
pub fn to_dot(graph: &ActionGraph) -> String {
    let mut out = String::from("digraph ActionGraph {\n    node [shape=box];\n");

    for idx in graph.graph.node_indices() {
        let node = &graph.graph[idx];
        let label = format!("{}:{}", node.action.screen_id, node.action.command_id);
        out.push_str(&format!("    N{} [label=\"{}\"];\n", idx.index(), label));
    }

    for edge in graph.graph.edge_references() {
        let freq = edge.weight().frequency;
        let src = edge.source().index();
        let dst = edge.target().index();
        out.push_str(&format!("    N{src} -> N{dst} [label=\"freq={freq}\"];\n"));
    }

    out.push('}');
    out.push('\n');
    out
}

/// Compute the 80th-percentile edge frequency.
///
/// Returns `1` if the graph has no edges.
fn p80_frequency(graph: &ActionGraph) -> u64 {
    let mut freqs: Vec<u64> = graph
        .graph
        .edge_references()
        .map(|e| e.weight().frequency)
        .collect();
    if freqs.is_empty() {
        return 1;
    }
    freqs.sort_unstable();
    // Safe: freqs is non-empty, so len >= 1; the multiplication result
    // fits in usize for realistic graph sizes.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let idx = ((freqs.len() as f64 * 0.8) as usize).min(freqs.len() - 1);
    freqs[idx]
}
