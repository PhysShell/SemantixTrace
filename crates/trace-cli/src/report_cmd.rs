//! `trace report workflows <file> [--top-n N]` (S4) and
//! `trace report similar --scenario <session-id> [--top N] <db>` (S8).

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

// ---------------------------------------------------------------------------
// `trace report similar` (S8, requires `sqlite` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
use serde::Serialize;
#[cfg(feature = "sqlite")]
use trace_storage::sqlite::{DimensionSet, SliceBy, SqliteBackend};

/// One result entry from `trace report similar`.
#[cfg(feature = "sqlite")]
#[derive(Serialize)]
struct SimilarEntry {
    session_id: String,
    similarity_score: f64,
    shared_dimensions: usize,
    total_dimensions: usize,
}

/// Run `trace report similar --scenario <session-id> [--top N] <db>`.
///
/// Finds the `top_n` sessions most similar to the probe session by
/// Jaccard similarity over their `(command_id, screen_id, outcome)`
/// dimension sets.  The probe session is excluded from results.
#[cfg(feature = "sqlite")]
pub(crate) fn run_similar(
    db: &Path,
    session_id: &str,
    top_n: usize,
    output: OutputFormat,
    quiet: bool,
) -> Result<(), SysExit> {
    let backend = SqliteBackend::open_readonly(db).map_err(|e| {
        if !quiet {
            eprintln!("error: cannot open {}: {e}", db.display());
        }
        SysExit::NoInput
    })?;

    // Load probe session events and build its dimension set.
    let probe_events = backend
        .slice_events(&SliceBy::SessionId(session_id.to_owned()))
        .map_err(|e| {
            if !quiet {
                eprintln!("error: {e}");
            }
            SysExit::Software
        })?;
    let probe_dims = DimensionSet::from_events(&probe_events);
    let probe_tuples = dims_to_set(&probe_dims);

    // Candidate pre-filter via index columns.
    let candidates = backend
        .candidate_sessions(&probe_dims, session_id)
        .map_err(|e| {
            if !quiet {
                eprintln!("error: {e}");
            }
            SysExit::Software
        })?;

    // Score each candidate in-process.
    let mut scored: Vec<SimilarEntry> = candidates
        .iter()
        .filter_map(|cand_id| {
            let events = backend
                .slice_events(&SliceBy::SessionId(cand_id.clone()))
                .ok()?;
            let cand_dims = DimensionSet::from_events(&events);
            let cand_tuples = dims_to_set(&cand_dims);
            let shared = probe_tuples.intersection(&cand_tuples).count();
            let total = probe_tuples.union(&cand_tuples).count();
            let score = if total == 0 {
                1.0_f64
            } else {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "dimension counts are small enough for f64"
                )]
                let s = shared as f64 / total as f64;
                s
            };
            Some(SimilarEntry {
                session_id: cand_id.clone(),
                similarity_score: score,
                shared_dimensions: shared,
                total_dimensions: total,
            })
        })
        .collect();

    // Sort by score descending, then truncate.
    scored.sort_by(|a, b| {
        b.similarity_score
            .partial_cmp(&a.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_n);

    let mut stdout = std::io::stdout().lock();
    match output {
        OutputFormat::Json => {
            let s = serde_json::to_string(&scored).map_err(|_| SysExit::Software)?;
            writeln!(stdout, "{s}").map_err(|_| SysExit::IoErr)?;
        }
        OutputFormat::Text | OutputFormat::Wide => {
            writeln!(
                stdout,
                "=== Similar sessions to {} (top {}) ===",
                session_id,
                scored.len()
            )
            .map_err(|_| SysExit::IoErr)?;
            if scored.is_empty() {
                writeln!(stdout, "  (none)").map_err(|_| SysExit::IoErr)?;
            } else {
                for (i, entry) in scored.iter().enumerate() {
                    writeln!(
                        stdout,
                        "  {}. {}  score={:.4}  shared={}/{}",
                        i + 1,
                        entry.session_id,
                        entry.similarity_score,
                        entry.shared_dimensions,
                        entry.total_dimensions,
                    )
                    .map_err(|_| SysExit::IoErr)?;
                }
            }
        }
    }
    Ok(())
}

/// Convert a [`DimensionSet`] into a set of prefixed dimension strings.
#[cfg(feature = "sqlite")]
fn dims_to_set(ds: &DimensionSet) -> HashSet<String> {
    let mut set = HashSet::new();
    for id in &ds.command_ids {
        set.insert(format!("cmd:{id}"));
    }
    for id in &ds.screen_ids {
        set.insert(format!("scr:{id}"));
    }
    for id in &ds.outcomes {
        set.insert(format!("out:{id}"));
    }
    set
}
