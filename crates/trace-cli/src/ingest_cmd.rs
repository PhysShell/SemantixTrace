//! `trace ingest --from jsonl --to sqlite <db> [<file>]` (S8).
//!
//! Reads JSONL events from `<file>` (or stdin when `<file>` is omitted or
//! `-`) and appends every event to the `SQLite` backend at `<db>`.  Creates
//! the database if it does not already exist.

use std::io::Write as _;
use std::path::Path;

use sysexits::ExitCode as SysExit;
use trace_core::ports::StorageBackend;
use trace_storage::sqlite::SqliteBackend;

/// Source format selector (currently only JSONL is supported).
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub(crate) enum IngestFrom {
    /// JSON Lines format (one event per line).
    Jsonl,
}

/// Target storage selector (currently only `SQLite` is supported).
#[derive(Clone, Copy, Debug, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub(crate) enum IngestTo {
    /// `SQLite` database (WAL mode, `rusqlite` bundled).
    Sqlite,
}

/// Run `trace ingest`.
pub(crate) fn run(
    db: &Path,
    file: Option<&Path>,
    _from: IngestFrom,
    _to: IngestTo,
    quiet: bool,
) -> Result<(), SysExit> {
    // Delegate I/O to the shared event reader (supports stdin + .zst).
    let input = file.unwrap_or_else(|| Path::new("-"));
    let events = crate::read_all_events(input, quiet)?;

    let mut backend = SqliteBackend::open(db).map_err(|e| {
        if !quiet {
            eprintln!("error: cannot open {}: {e}", db.display());
        }
        SysExit::CantCreat
    })?;

    let total = events.len();
    for event in &events {
        backend.append(event).map_err(|e| {
            if !quiet {
                eprintln!("error: append failed: {e}");
            }
            SysExit::IoErr
        })?;
    }

    if !quiet {
        let mut stderr = std::io::stderr().lock();
        writeln!(stderr, "ingested {total} events into {}", db.display())
            .map_err(|_| SysExit::IoErr)?;
    }
    Ok(())
}
