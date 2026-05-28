//! `trace report workflows <file> [--top-n N] [--format {text,json}]`
//! (S4).
//!
//! Reads a JSONL trace file, normalizes all sessions, builds a
//! [`WorkflowReport`], and emits it as text or JSON to stdout.

use std::collections::HashSet;
use std::io::Write as _;
use std::path::Path;

use sysexits::ExitCode as SysExit;
use trace_core::Session;
use trace_graph::{build_report, WorkflowReport, WorkflowReportConfig};
use trace_normalizer::{normalize, NormCfg};
use trace_schema::v1::TraceEventKind;

use crate::normalize::group_by_session;
use crate::OutputFormat;

/// Run `trace report workflows`.
pub(crate) fn run_workflows(
    path: &Path,
    top_n: usize,
    output: OutputFormat,
    quiet: bool,
) -> Result<(), SysExit> {
    let events = crate::read_all_events(path, quiet)?;

    // Identify sessions with ExceptionThrown events.
    let mut error_sessions: HashSet<uuid::Uuid> = HashSet::new();
    for event in &events {
        if matches!(event.kind, TraceEventKind::ExceptionThrown { .. }) {
            error_sessions.insert(*event.session_id.as_uuid());
        }
    }

    let grouped = group_by_session(events);
    let cfg = NormCfg::default();
    let mut scenario_pairs: Vec<(uuid::Uuid, trace_core::Scenario)> = Vec::new();

    for (sid, session_events) in grouped {
        let Ok(session) = Session::new(sid, session_events) else {
            continue;
        };
        let uuid = *sid.as_uuid();
        let (scenario, _) = normalize(&session, &cfg);
        scenario_pairs.push((uuid, scenario));
    }

    let report_cfg = WorkflowReportConfig {
        top_n,
        ..Default::default()
    };
    let report = build_report(&scenario_pairs, &error_sessions, &report_cfg);

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();

    match output {
        OutputFormat::Json => {
            let s = serde_json::to_string(&report).map_err(|_| SysExit::Software)?;
            writeln!(handle, "{s}").map_err(|_| SysExit::IoErr)?;
        }
        OutputFormat::Text | OutputFormat::Wide => {
            let text = format_report_text(&report);
            write!(handle, "{text}").map_err(|_| SysExit::IoErr)?;
        }
    }
    Ok(())
}

/// Render a [`WorkflowReport`] as human-readable text.
fn format_report_text(report: &WorkflowReport) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push("=== Top workflows ===".to_owned());
    if report.top_workflows.is_empty() {
        lines.push("  (none)".to_owned());
    } else {
        for (i, wf) in report.top_workflows.iter().enumerate() {
            let actions: Vec<String> = wf
                .actions
                .iter()
                .map(|a| format!("{}:{}", a.screen_id, a.command_id))
                .collect();
            lines.push(format!(
                "  {}. [{}] (freq={})",
                i + 1,
                actions.join(" \u{2192} "),
                wf.frequency
            ));
        }
    }

    lines.push(String::new());
    lines.push("=== Rare failing workflows ===".to_owned());
    if report.rare_failing.is_empty() {
        lines.push("  (none)".to_owned());
    } else {
        for wf in &report.rare_failing {
            let actions: Vec<String> = wf
                .actions
                .iter()
                .map(|a| format!("{}:{}", a.screen_id, a.command_id))
                .collect();
            lines.push(format!(
                "  [{}] freq={} ({:.1}%) error_rate={:.1}%",
                actions.join(" \u{2192} "),
                wf.frequency,
                wf.frequency_fraction * 100.0,
                wf.error_rate * 100.0,
            ));
        }
    }

    lines.push(String::new());
    lines.push("=== Dead features ===".to_owned());
    if report.dead_features.is_empty() {
        lines.push("  (none)".to_owned());
    } else {
        for df in &report.dead_features {
            lines.push(format!(
                "  {} freq={} ({:.2}%)",
                df.command_id,
                df.frequency,
                df.frequency_fraction * 100.0,
            ));
        }
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}
