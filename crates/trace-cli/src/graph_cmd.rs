//! `trace graph <file> --format {mermaid,dot}` (S4).
//!
//! Reads a JSONL trace file, normalizes all sessions into scenarios,
//! builds the action graph, and emits a Mermaid or DOT rendering to
//! stdout.

use std::io::Write as _;
use std::path::Path;

use sysexits::ExitCode as SysExit;
use trace_core::Session;
use trace_graph::{from_scenarios, to_dot, to_mermaid};
use trace_normalizer::{normalize, NormCfg};

use crate::normalize::group_by_session;

/// Output format for the graph command.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub(crate) enum GraphFormat {
    /// Mermaid flowchart (default).
    Mermaid,
    /// Graphviz DOT.
    Dot,
}

/// Run `trace graph`: build an action graph from `path` and emit it in
/// `format` to stdout.
pub(crate) fn run(path: &Path, format: GraphFormat, quiet: bool) -> Result<(), SysExit> {
    let events = crate::read_all_events(path, quiet)?;
    let grouped = group_by_session(events);

    let cfg = NormCfg::default();
    let mut scenarios = Vec::new();
    for (sid, session_events) in grouped {
        let Ok(session) = Session::new(sid, session_events) else {
            continue;
        };
        let (scenario, _) = normalize(&session, &cfg);
        scenarios.push(scenario);
    }

    let graph = from_scenarios(&scenarios);

    let output = match format {
        GraphFormat::Mermaid => to_mermaid(&graph),
        GraphFormat::Dot => to_dot(&graph),
    };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    write!(handle, "{output}").map_err(|_| SysExit::IoErr)
}
