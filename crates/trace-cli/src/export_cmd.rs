//! `trace export diagnostic` — bundle a session's events + oracle evidence
//! into a portable JSON archive (S7 acceptance criterion).
//!
//! The archive format is a single JSON object:
//! ```json
//! {
//!   "manifest":  { "tool_version": "…", "schema_version": 2, "session_id": "…" },
//!   "events":    ["<jsonl line>", …],
//!   "oracle_report": { … }
//! }
//! ```
//!
//! Re-importing on a second machine:
//! ```sh
//! jq -r '.events[]' bundle.json | trace analyze -
//! jq -r '.events[]' bundle.json | trace oracle run --rules demo -
//! ```

use std::io::{self, BufWriter, Write};
use std::path::Path;

use serde::Serialize;
use sysexits::ExitCode as SysExit;
use trace_core::Session;
use trace_oracle::{Graph47ResultMustBeNonNegative, OracleEngine};
use trace_schema::write_event;

use crate::normalize::group_by_session;

#[derive(Serialize)]
struct Manifest {
    tool_version: &'static str,
    schema_version: u32,
    session_id: String,
}

#[derive(Serialize)]
struct DiagnosticBundle {
    manifest: Manifest,
    events: Vec<String>,
    oracle_report: trace_oracle::engine::SessionReport,
}

/// Produce a diagnostic archive for `session_id` drawn from `corpus`.
/// Writes JSON to `out` (or stdout when `None`).
pub(crate) fn run_diagnostic(
    session_id: &str,
    corpus: &Path,
    out: Option<&Path>,
    quiet: bool,
) -> Result<(), SysExit> {
    let all_events = crate::read_all_events(corpus, quiet)?;
    let grouped = group_by_session(all_events);

    let (sid, session_events) = grouped
        .into_iter()
        .find(|(s, _)| s.to_string() == session_id)
        .ok_or_else(|| {
            if !quiet {
                eprintln!("error: session '{session_id}' not found in corpus");
            }
            SysExit::NoInput
        })?;

    // Re-serialise events as JSONL strings for the bundle.
    let event_lines: Vec<String> = session_events
        .iter()
        .map(|e| {
            write_event(e)
                .map(|s| s.trim_end_matches('\n').to_owned())
                .map_err(|_| SysExit::Software)
        })
        .collect::<Result<_, _>>()?;

    // Build a Session and run the oracle (builtin rules + domain rule).
    let session = Session::new(sid, session_events).map_err(|_| {
        if !quiet {
            eprintln!("error: session is empty");
        }
        SysExit::DataErr
    })?;

    let engine = OracleEngine::new()
        .with_builtin_rules()
        .with_rule(Box::new(Graph47ResultMustBeNonNegative));

    let report = engine
        .run_all(&[session])
        .into_iter()
        .next()
        .ok_or(SysExit::Software)?;

    let bundle = DiagnosticBundle {
        manifest: Manifest {
            tool_version: env!("CARGO_PKG_VERSION"),
            schema_version: trace_schema::CURRENT_SCHEMA_VERSION,
            session_id: sid.to_string(),
        },
        events: event_lines,
        oracle_report: report,
    };

    let json = serde_json::to_string_pretty(&bundle).map_err(|_| SysExit::Software)?;

    if let Some(dest) = out {
        let f = std::fs::File::create(dest).map_err(|e| {
            if !quiet {
                eprintln!("error creating {}: {e}", dest.display());
            }
            SysExit::CantCreat
        })?;
        let mut w = BufWriter::new(f);
        writeln!(w, "{json}").map_err(|_| SysExit::IoErr)?;
    } else {
        let mut stdout = io::stdout().lock();
        writeln!(stdout, "{json}").map_err(|_| SysExit::IoErr)?;
    }
    Ok(())
}
