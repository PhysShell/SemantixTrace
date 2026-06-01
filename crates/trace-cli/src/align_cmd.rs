//! `trace align <db> <session-a> <session-b>` (S8).
//!
//! Loads the two sessions from the `SQLite` corpus at `<db>`, computes
//! the global Needleman-Wunsch edit-distance alignment using the
//! structural cost function, and reports the total cost, similarity
//! score, and match rate.

use std::io::Write as _;
use std::path::Path;

use serde::Serialize;
use sysexits::ExitCode as SysExit;
use trace_align::{align, StructuralCost};
use trace_storage::sqlite::{SliceBy, SqliteBackend};

use crate::OutputFormat;

#[derive(Serialize)]
struct AlignOutput {
    session_a: String,
    session_b: String,
    total_cost: f64,
    similarity: f64,
    match_rate: f64,
    a_len: usize,
    b_len: usize,
}

/// Run `trace align`.
pub(crate) fn run(
    db: &Path,
    session_a: &str,
    session_b: &str,
    output: OutputFormat,
    quiet: bool,
) -> Result<(), SysExit> {
    let backend = SqliteBackend::open_readonly(db).map_err(|e| {
        if !quiet {
            eprintln!("error: cannot open {}: {e}", db.display());
        }
        SysExit::NoInput
    })?;

    let events_a = backend
        .slice_events(&SliceBy::SessionId(session_a.to_owned()))
        .map_err(|e| {
            if !quiet {
                eprintln!("error: {e}");
            }
            SysExit::Software
        })?;
    let events_b = backend
        .slice_events(&SliceBy::SessionId(session_b.to_owned()))
        .map_err(|e| {
            if !quiet {
                eprintln!("error: {e}");
            }
            SysExit::Software
        })?;

    // Reject missing sessions before computing anything — two absent sessions
    // would otherwise produce total_cost=0 / similarity=1.0 / lengths 0/0,
    // which looks like a perfect match instead of an invalid request.
    if events_a.is_empty() {
        if !quiet {
            eprintln!("error: session '{session_a}' not found in {}", db.display());
        }
        return Err(SysExit::NoInput);
    }
    if events_b.is_empty() {
        if !quiet {
            eprintln!("error: session '{session_b}' not found in {}", db.display());
        }
        return Err(SysExit::NoInput);
    }

    let cost_fn = StructuralCost::default();
    let alignment = align(&events_a, &events_b, &cost_fn);

    let out = AlignOutput {
        session_a: session_a.to_owned(),
        session_b: session_b.to_owned(),
        total_cost: alignment.total_cost,
        similarity: alignment.similarity(),
        match_rate: alignment.match_rate(),
        a_len: alignment.a_len,
        b_len: alignment.b_len,
    };

    let mut stdout = std::io::stdout().lock();
    match output {
        OutputFormat::Json => {
            let s = serde_json::to_string(&out).map_err(|_| SysExit::Software)?;
            writeln!(stdout, "{s}").map_err(|_| SysExit::IoErr)?;
        }
        OutputFormat::Text | OutputFormat::Wide => {
            writeln!(stdout, "Align {} \u{2194} {}", out.session_a, out.session_b)
                .map_err(|_| SysExit::IoErr)?;
            writeln!(stdout, "  Cost:       {:.4}", out.total_cost).map_err(|_| SysExit::IoErr)?;
            writeln!(stdout, "  Similarity: {:.4}", out.similarity).map_err(|_| SysExit::IoErr)?;
            writeln!(stdout, "  Match rate: {:.4}", out.match_rate).map_err(|_| SysExit::IoErr)?;
            writeln!(stdout, "  Length:     {} / {}", out.a_len, out.b_len)
                .map_err(|_| SysExit::IoErr)?;
        }
    }
    Ok(())
}
