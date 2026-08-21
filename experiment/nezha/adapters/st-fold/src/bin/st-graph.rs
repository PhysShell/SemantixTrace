//! E3 experiment tool: build ActionGraphs (trace-graph crate) from
//! st-fold scenario files for a normal and an abnormal window, and emit
//! the merged transition table as JSON:
//!
//! ```json
//! {"transitions":[{"src":…,"dst":…,"n":…,"a":…,"anomaly_n":…}]}
//! ```
//!
//! `n`/`a` are the ActionGraph edge frequencies in the normal/abnormal
//! graph (crate semantics: every consecutive occurrence counts);
//! `anomaly_n` is the normal graph's Heuristics-derived anomaly score
//! for the edge (frozen S2 tie-break input).

use std::collections::HashMap;
use std::io::BufRead;
use std::process::ExitCode;

use petgraph::visit::EdgeRef;
use serde::Deserialize;
use trace_core::{CanonicalAction, Scenario};
use trace_graph::from_scenarios;

#[derive(Deserialize)]
struct SessionLine {
    actions: Vec<CanonicalAction>,
}

fn load(path: &str) -> Result<Vec<Scenario>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut out = Vec::new();
    for (i, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| format!("{path}:{}: {e}", i + 1))?;
        let parsed: SessionLine =
            serde_json::from_str(&line).map_err(|e| format!("{path}:{}: {e}", i + 1))?;
        out.push(Scenario::new(parsed.actions));
    }
    Ok(out)
}

fn edge_table(scenarios: &[Scenario]) -> HashMap<(String, String), (u64, f64)> {
    let graph = from_scenarios(scenarios);
    let mut table = HashMap::new();
    for edge in graph.graph().edge_references() {
        let src = &graph.graph()[edge.source()].action;
        let dst = &graph.graph()[edge.target()].action;
        let key = (
            serde_json::to_string(src).unwrap_or_default(),
            serde_json::to_string(dst).unwrap_or_default(),
        );
        table.insert(key, (edge.weight().frequency, edge.weight().anomaly_score));
    }
    table
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: st-graph <normal-scenarios.jsonl> <abnormal-scenarios.jsonl> [out.json]");
        return ExitCode::from(64);
    }
    let normal = match load(&args[0]) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("st-graph: {e}");
            return ExitCode::from(65);
        }
    };
    let abnormal = match load(&args[1]) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("st-graph: {e}");
            return ExitCode::from(65);
        }
    };

    let ntab = edge_table(&normal);
    let atab = edge_table(&abnormal);

    // Deterministic serialization: HashMap iteration order is randomized
    // per process, and the downstream S2 scorer's alarm dedup is
    // order-sensitive (keeps the first max-depth resource candidate), so
    // an arbitrary transition order makes regeneration non-reproducible
    // (PR #20 Codex incremental P1, D-011). Sort by the (src, dst) keys.
    let mut entries: Vec<_> = ntab.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut transitions = Vec::new();
    for ((src, dst), (freq, anomaly)) in entries {
        let a = atab.get(&(src.clone(), dst.clone())).map_or(0, |t| t.0);
        transitions.push(serde_json::json!({
            "src": serde_json::from_str::<serde_json::Value>(src).unwrap_or_default(),
            "dst": serde_json::from_str::<serde_json::Value>(dst).unwrap_or_default(),
            "n": freq,
            "a": a,
            "anomaly_n": anomaly,
        }));
    }
    let out = serde_json::json!({
        "normal_scenarios": normal.len(),
        "abnormal_scenarios": abnormal.len(),
        "normal_edges": ntab.len(),
        "abnormal_edges": atab.len(),
        "transitions": transitions,
    });
    let payload = serde_json::to_string(&out).unwrap_or_default();
    if let Some(path) = args.get(2) {
        if std::fs::write(path, payload).is_err() {
            eprintln!("st-graph: cannot write {path}");
            return ExitCode::from(73);
        }
    } else {
        println!("{payload}");
    }
    eprintln!(
        "st-graph: normal_edges={} abnormal_edges={}",
        ntab.len(),
        atab.len()
    );
    ExitCode::SUCCESS
}
