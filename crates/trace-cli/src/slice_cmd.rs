//! `trace slice --by <dim> <value> <db>` (S8).
//!
//! Reads the `SQLite` corpus at `<db>` through the standard upcaster chain
//! and writes a JSONL slice — all events matching the given dimension
//! filter — to stdout.

use std::io::Write as _;
use std::path::Path;

use sysexits::ExitCode as SysExit;
use trace_storage::sqlite::{SliceBy, SqliteBackend};

/// Index dimension to filter by.
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub(crate) enum SliceDim {
    /// Filter by session UUID.
    SessionId,
    /// Filter by extracted `command_id`.
    CommandId,
    /// Filter by extracted `screen_id`.
    ScreenId,
    /// Filter by outcome label (`success`, `failure`, …).
    Outcome,
    /// Filter by `domain_entity_id`.
    DomainEntityId,
}

/// Run `trace slice`.
pub(crate) fn run(db: &Path, by: SliceDim, value: &str, quiet: bool) -> Result<(), SysExit> {
    let backend = SqliteBackend::open_readonly(db).map_err(|e| {
        if !quiet {
            eprintln!("error: cannot open {}: {e}", db.display());
        }
        SysExit::NoInput
    })?;

    let filter = match by {
        SliceDim::SessionId => SliceBy::SessionId(value.to_owned()),
        SliceDim::CommandId => SliceBy::CommandId(value.to_owned()),
        SliceDim::ScreenId => SliceBy::ScreenId(value.to_owned()),
        SliceDim::Outcome => SliceBy::Outcome(value.to_owned()),
        SliceDim::DomainEntityId => SliceBy::DomainEntityId(value.to_owned()),
    };

    let events = backend.slice_events(&filter).map_err(|e| {
        if !quiet {
            eprintln!("error: query failed: {e}");
        }
        SysExit::Software
    })?;

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for event in &events {
        let line = trace_schema::write_event(event).map_err(|_| SysExit::Software)?;
        let line = line.trim_end_matches('\n');
        writeln!(handle, "{line}").map_err(|_| SysExit::IoErr)?;
    }
    Ok(())
}
