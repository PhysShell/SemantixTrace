//! `trace-graph` — action-graph mining for `SemantxTrace` (S4).
//!
//! - [`ActionGraph`]: directed weighted graph of canonical action transitions.
//! - [`HeuristicsMiner`]: builds a dependency graph from frequency data.
//! - [`prefixspan()`]: frequent subsequence mining.
//! - Mermaid and DOT exporters: [`to_mermaid`], [`to_dot`].
//! - [`WorkflowReport`]: analytics projections (top-N, rare-failing, dead
//!   features).

#![forbid(unsafe_code)]

pub mod builder;
pub mod export;
pub mod graph;
pub mod miner;
pub mod prefixspan;
pub mod report;

#[cfg(feature = "inductive-miner")]
pub mod inductive_miner;

pub use builder::from_scenarios;
pub use export::{to_dot, to_mermaid};
pub use graph::{ActionGraph, ActionNode, Transition};
pub use miner::{HeuristicsMiner, MinerConfig};
pub use prefixspan::{prefixspan, FrequentPattern};
pub use report::{
    build_report, DeadFeature, RareFailingWorkflow, TopWorkflow, WorkflowReport,
    WorkflowReportConfig,
};

#[cfg(feature = "inductive-miner")]
pub use inductive_miner::{tree_to_graph, ImdfConfig, InductiveMiner, ProcessTree};
