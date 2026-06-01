//! `trace graph <file> --format {mermaid,dot,process-tree} [--miner {heuristics,inductive}]`
//! (S4/S8).
//!
//! Reads a JSONL trace file, normalizes all sessions into scenarios,
//! builds the action graph with the chosen miner, and emits a rendering
//! to stdout.

use std::io::Write as _;
use std::path::Path;

use sysexits::ExitCode as SysExit;
use trace_core::Session;
use trace_graph::{from_scenarios, to_dot, to_mermaid};
use trace_normalizer::{normalize, NormCfg};

use crate::normalize::group_by_session;

/// Output format for the graph command.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub(crate) enum GraphFormat {
    /// Mermaid flowchart (default).
    Mermaid,
    /// Graphviz DOT.
    Dot,
    /// Raw process tree (requires `--miner inductive`).
    ProcessTree,
}

/// Mining algorithm for building the action graph.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub(crate) enum MinerAlgorithm {
    /// Heuristics miner (always available).
    Heuristics,
    /// Inductive miner IMDF (requires `inductive-miner` feature).
    Inductive,
}

/// Run `trace graph`: build an action graph from `path` and emit it in
/// `format` to stdout.
pub(crate) fn run(
    path: &Path,
    format: GraphFormat,
    miner: MinerAlgorithm,
    quiet: bool,
) -> Result<(), SysExit> {
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

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    match miner {
        MinerAlgorithm::Heuristics => {
            if matches!(format, GraphFormat::ProcessTree) {
                if !quiet {
                    eprintln!("error: --format process-tree requires --miner inductive");
                }
                return Err(SysExit::Usage);
            }
            let graph = from_scenarios(&scenarios);
            let output = match format {
                GraphFormat::Mermaid => to_mermaid(&graph),
                GraphFormat::Dot => to_dot(&graph),
                GraphFormat::ProcessTree => unreachable!(),
            };
            write!(handle, "{output}").map_err(|_| SysExit::IoErr)?;
        }
        MinerAlgorithm::Inductive => {
            #[cfg(feature = "inductive-miner")]
            {
                use trace_graph::{ImdfConfig, InductiveMiner};
                let miner_cfg = ImdfConfig::default();
                let im = InductiveMiner::new(miner_cfg);
                let (graph, tree) = im.mine(&scenarios);
                match format {
                    GraphFormat::ProcessTree => {
                        writeln!(handle, "{}", tree.display()).map_err(|_| SysExit::IoErr)?;
                    }
                    GraphFormat::Mermaid => {
                        write!(handle, "{}", to_mermaid(&graph)).map_err(|_| SysExit::IoErr)?;
                    }
                    GraphFormat::Dot => {
                        write!(handle, "{}", to_dot(&graph)).map_err(|_| SysExit::IoErr)?;
                    }
                }
            }
            #[cfg(not(feature = "inductive-miner"))]
            {
                if !quiet {
                    eprintln!("error: --miner inductive requires --features inductive-miner");
                }
                return Err(SysExit::Usage);
            }
        }
    }

    Ok(())
}
