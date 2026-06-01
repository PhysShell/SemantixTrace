//! `trace` — `SemantxTrace` command-line interface.
//!
//! Implemented subcommands: `version`, `analyze`, `normalize`,
//! `graph`, `report workflows`, `oracle run`, `export diagnostic`,
//! `completions <shell>`. S8 adds `ingest`, `slice`, `align`, and
//! `report similar` (behind the `sqlite` feature).

#![forbid(unsafe_code)]

mod analyze;
mod export_cmd;
mod graph_cmd;
mod normalize;
mod oracle_cmd;
mod report_cmd;

#[cfg(feature = "sqlite")]
mod align_cmd;
#[cfg(feature = "sqlite")]
mod ingest_cmd;
#[cfg(feature = "sqlite")]
mod slice_cmd;

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use serde::Serialize;
use sysexits::ExitCode as SysExit;
use trace_core::ports::StorageBackend;
use trace_schema::Current;
use trace_storage::{JsonlBackend, JsonlError};

/// Schema version supported by this binary build.
///
/// Bumped in lock-step with `trace-schema` (S1 onward); see ADR-0006
/// and `docs/upcasters.md`.
const SCHEMA_VERSION: u32 = 1;

#[derive(Parser, Debug)]
#[command(
    name = "trace",
    about = "SemantxTrace — behavioral observability for desktop UI applications",
    long_about = None,
    disable_version_flag = true,
)]
struct Cli {
    #[command(flatten)]
    global: GlobalOptions,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args, Debug)]
struct GlobalOptions {
    /// Output format for structured commands.
    #[arg(
        short = 'o',
        long = "output",
        global = true,
        value_enum,
        default_value_t = OutputFormat::Text,
    )]
    output: OutputFormat,

    /// Reduce diagnostic noise on stderr.
    #[arg(short = 'q', long = "quiet", global = true)]
    quiet: bool,

    /// Increase diagnostic verbosity (repeatable: `-v`, `-vv`, `-vvv`).
    #[arg(short = 'v', long = "verbose", global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable ANSI colour output (also honoured: `NO_COLOR` env var).
    #[arg(long = "no-color", global = true)]
    no_color: bool,

    /// Print binary and schema versions and exit.
    ///
    /// Same payload as `trace version`; honours `--output {text,json,wide}`
    /// per ADR-0014 §4 / §11.
    #[arg(short = 'V', long = "version", global = true)]
    version: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print binary and schema versions.
    Version,
    /// Summarize a JSONL trace file: session / event counts, error rate.
    Analyze {
        /// Path to a `.jsonl` (or `.jsonl.zst`) trace file, or `-` to
        /// read JSONL from stdin (ADR-0014 §5).
        file: PathBuf,
    },
    /// Fold a trace into normalized canonical actions (one JSON per line).
    Normalize {
        /// Path to a `.jsonl` (or `.jsonl.zst`) trace file, or `-` for
        /// stdin (ADR-0014 §5). Events are grouped by session id.
        file: PathBuf,
        /// Write normalized output to this path instead of stdout.
        /// (`-o` / `--output` is the global format flag, so the output
        /// file uses `--out`.)
        #[arg(long = "out")]
        out: Option<PathBuf>,
        /// Print the fold report (events / actions / collapses / pauses)
        /// to stderr.
        #[arg(long = "report")]
        report: bool,
    },
    /// Build and render an action graph from a JSONL trace file.
    Graph {
        /// Path to a `.jsonl` (or `.jsonl.zst`) trace file.
        file: PathBuf,
        /// Output format for the graph.
        #[arg(long = "format", value_enum, default_value_t = graph_cmd::GraphFormat::Mermaid)]
        format: graph_cmd::GraphFormat,
        /// Mining algorithm to use.
        #[arg(long = "miner", value_enum, default_value_t = graph_cmd::MinerAlgorithm::Heuristics)]
        miner: graph_cmd::MinerAlgorithm,
    },
    /// Produce workflow analytics reports from a JSONL trace file.
    Report {
        #[command(subcommand)]
        subcommand: ReportCommand,
    },
    /// Run oracle rule checks against a JSONL trace file.
    Oracle {
        #[command(subcommand)]
        subcommand: OracleCommand,
    },
    /// Export a session as a portable diagnostic archive.
    Export {
        #[command(subcommand)]
        subcommand: ExportCommand,
    },
    /// Ingest a JSONL corpus into a `SQLite` database (S8).
    #[cfg(feature = "sqlite")]
    Ingest {
        /// Path to the `SQLite` database (created if absent).
        db: PathBuf,
        /// Input JSONL file, or `-` to read from stdin.
        file: Option<PathBuf>,
        /// Source format.
        #[arg(long = "from", value_enum, default_value_t = ingest_cmd::IngestFrom::Jsonl)]
        from: ingest_cmd::IngestFrom,
        /// Target storage.
        #[arg(long = "to", value_enum, default_value_t = ingest_cmd::IngestTo::Sqlite)]
        to: ingest_cmd::IngestTo,
    },
    /// Slice a `SQLite` corpus by a dimension filter, writing JSONL to stdout (S8).
    #[cfg(feature = "sqlite")]
    Slice {
        /// Dimension to filter by.
        #[arg(long = "by", value_enum)]
        by: slice_cmd::SliceDim,
        /// Value to match.
        value: String,
        /// Path to the `SQLite` database.
        db: PathBuf,
    },
    /// Compute edit-distance alignment between two sessions in a `SQLite` corpus (S8).
    #[cfg(feature = "sqlite")]
    Align {
        /// Path to the `SQLite` database.
        db: PathBuf,
        /// First session UUID.
        session_a: String,
        /// Second session UUID.
        session_b: String,
    },
    /// Emit a shell-completion script for the given shell.
    Completions {
        /// Target shell.
        shell: Shell,
    },
}

/// Sub-subcommands for `trace export`.
#[derive(Subcommand, Debug)]
enum ExportCommand {
    /// Bundle a session's events + oracle evidence into a JSON archive.
    ///
    /// The archive can be re-imported on another machine:
    ///   `jq -r '.events[]' bundle.json | trace analyze -`
    Diagnostic {
        /// Session id to export (UUID).
        session_id: String,
        /// Path to the JSONL corpus that contains the session.
        corpus: PathBuf,
        /// Write the archive to this path (default: stdout).
        #[arg(long = "out")]
        out: Option<PathBuf>,
    },
}

/// Sub-subcommands for `trace oracle`.
#[derive(Subcommand, Debug)]
enum OracleCommand {
    /// Run oracle rules against a JSONL trace file.
    ///
    /// Exit codes: 0 = all pass, 1 = Warning/Error found, 2 = Critical found.
    Run {
        /// Path to a `.jsonl` (or `.jsonl.zst`) trace file.
        file: PathBuf,
        /// Rule set to apply.
        #[arg(long = "rules", value_enum, default_value_t = oracle_cmd::RuleSet::Builtin)]
        rules: oracle_cmd::RuleSet,
        /// Write HTML report to this file (otherwise HTML goes to stdout).
        #[arg(long = "out")]
        out: Option<PathBuf>,
    },
}

/// Sub-subcommands for `trace report`.
#[derive(Subcommand, Debug)]
enum ReportCommand {
    /// Emit top-N workflows, rare-but-failing scenarios, and dead-feature
    /// candidates.
    Workflows {
        /// Path to a `.jsonl` (or `.jsonl.zst`) trace file.
        file: PathBuf,
        /// Maximum number of top workflows to show.
        #[arg(long = "top-n", default_value_t = 20)]
        top_n: usize,
    },
    /// Find the N most similar sessions in a `SQLite` corpus (S8).
    #[cfg(feature = "sqlite")]
    Similar {
        /// Session UUID of the probe scenario (format: `<uuid>`).
        #[arg(long = "scenario")]
        scenario: String,
        /// Maximum number of results to return.
        #[arg(long = "top", default_value_t = 10)]
        top: usize,
        /// Path to the `SQLite` database.
        db: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[clap(rename_all = "lowercase")]
enum OutputFormat {
    /// Human-readable text (default).
    Text,
    /// Stable, versioned JSON (see ADR-0014 §11).
    Json,
    /// Human-readable text with extra columns where applicable.
    Wide,
}

#[derive(Debug, Serialize)]
struct VersionOutput {
    binary: &'static str,
    schema: u32,
}

fn main() -> ExitCode {
    match Cli::try_parse() {
        Ok(cli) => run(&cli),
        Err(err) => handle_clap_error(&err),
    }
}

/// Map a `clap` parse error to the right sysexits code per ADR-0014 §6.
///
/// - `DisplayHelp` / `DisplayHelpOnMissingArgumentOrSubcommand` /
///   `DisplayVersion` print to stdout and exit `0`.
/// - All other parse errors (unknown arg, missing required, bad value, …)
///   print to stderr and exit `64` `EX_USAGE`.
fn handle_clap_error(err: &clap::Error) -> ExitCode {
    let _ = err.print();
    match err.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::DisplayVersion => ExitCode::SUCCESS,
        _ => ExitCode::from(u8::from(SysExit::Usage)),
    }
}

fn run(cli: &Cli) -> ExitCode {
    // Global `--version` flag short-circuits the subcommand path, per
    // ADR-0014 §4: `-V` / `--version` honours `--output {text,json,wide}`
    // and emits the same payload as `trace version`.
    if cli.global.version {
        return into_exit_code(print_version(cli.global.output));
    }

    let Some(command) = cli.command.as_ref() else {
        // No subcommand and no `--version` — print the long help on
        // stdout, exit `64` per ADR-0014 §6 (missing required argument).
        let mut cmd = Cli::command();
        let _ = cmd.print_help();
        return ExitCode::from(u8::from(SysExit::Usage));
    };

    into_exit_code(dispatch(command, &cli.global))
}

fn into_exit_code(result: Result<(), SysExit>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(u8::from(code)),
    }
}

fn dispatch(command: &Command, global: &GlobalOptions) -> Result<(), SysExit> {
    match command {
        Command::Version => print_version(global.output),
        Command::Analyze { file } => run_analyze(file, global.output, global.quiet),
        Command::Normalize { file, out, report } => {
            normalize::run(file, out.as_deref(), *report, global.quiet)
        }
        Command::Graph {
            file,
            format,
            miner,
        } => graph_cmd::run(file, *format, *miner, global.quiet),
        Command::Report { subcommand } => dispatch_report(subcommand, global),
        Command::Oracle { subcommand } => {
            // Oracle sub-commands return ExitCode directly (exit codes 0/1/2);
            // map to Ok(()) since the ExitCode is propagated via std::process.
            dispatch_oracle(subcommand, global)
        }
        Command::Export { subcommand } => dispatch_export(subcommand, global),
        #[cfg(feature = "sqlite")]
        Command::Ingest { db, file, from, to } => {
            ingest_cmd::run(db, file.as_deref(), *from, *to, global.quiet)
        }
        #[cfg(feature = "sqlite")]
        Command::Slice { by, value, db } => slice_cmd::run(db, *by, value, global.quiet),
        #[cfg(feature = "sqlite")]
        Command::Align {
            db,
            session_a,
            session_b,
        } => align_cmd::run(db, session_a, session_b, global.output, global.quiet),
        Command::Completions { shell } => {
            emit_completions(*shell);
            Ok(())
        }
    }
}

fn dispatch_report(cmd: &ReportCommand, global: &GlobalOptions) -> Result<(), SysExit> {
    match cmd {
        ReportCommand::Workflows { file, top_n } => {
            report_cmd::run_workflows(file, *top_n, global.output, global.quiet)
        }
        #[cfg(feature = "sqlite")]
        ReportCommand::Similar { scenario, top, db } => {
            report_cmd::run_similar(db, scenario, *top, global.output, global.quiet)
        }
    }
}

fn dispatch_export(cmd: &ExportCommand, global: &GlobalOptions) -> Result<(), SysExit> {
    match cmd {
        ExportCommand::Diagnostic {
            session_id,
            corpus,
            out,
        } => export_cmd::run_diagnostic(session_id, corpus, out.as_deref(), global.quiet),
    }
}

fn dispatch_oracle(cmd: &OracleCommand, global: &GlobalOptions) -> Result<(), SysExit> {
    match cmd {
        OracleCommand::Run { file, rules, out } => {
            // Oracle uses exit codes 0/1/2 distinct from the sysexits used by
            // other sub-commands.  Run the oracle logic, get the u8 exit code,
            // then exit the process directly so the exact code is preserved.
            let code = oracle_cmd::run(file, *rules, out.as_deref(), global.output, global.quiet);
            std::process::exit(i32::from(code));
        }
    }
}

fn run_analyze(path: &Path, output: OutputFormat, quiet: bool) -> Result<(), SysExit> {
    let events = read_all_events(path, quiet)?;
    let report = analyze::analyze_events(events);

    let mut stdout = io::stdout().lock();
    match output {
        OutputFormat::Json => {
            let s = serde_json::to_string(&report).map_err(|_| SysExit::Software)?;
            writeln!(stdout, "{s}").map_err(|_| SysExit::IoErr)?;
        }
        OutputFormat::Text | OutputFormat::Wide => {
            write!(stdout, "{report}").map_err(|_| SysExit::IoErr)?;
        }
    }
    Ok(())
}

/// Read every event from a path (or stdin when `path` is `-`), mapping
/// storage errors to sysexits codes and printing diagnostics unless
/// `quiet` (ADR-0014 §5/§6). Shared by `analyze`, `normalize`, and S8
/// commands.
fn read_all_events(path: &Path, quiet: bool) -> Result<Vec<Current>, SysExit> {
    if path == Path::new("-") {
        let reader = io::BufReader::new(io::stdin().lock());
        collect_events(trace_storage::read_events(reader), quiet)
    } else {
        let backend = JsonlBackend::new(path);
        let iter = backend
            .events()
            .map_err(|e| report_jsonl_error(&e, quiet))?;
        collect_events(iter, quiet)
    }
}

fn collect_events<I>(iter: I, quiet: bool) -> Result<Vec<Current>, SysExit>
where
    I: IntoIterator<Item = Result<Current, JsonlError>>,
{
    let mut events = Vec::new();
    for item in iter {
        events.push(item.map_err(|e| report_jsonl_error(&e, quiet))?);
    }
    Ok(events)
}

/// Map a storage error to the right sysexits code (ADR-0014 §6), printing
/// a diagnostic to stderr unless `--quiet` was set (ADR-0014 §4).
fn report_jsonl_error(err: &JsonlError, quiet: bool) -> SysExit {
    if !quiet {
        eprintln!("error: {err}");
    }
    match err {
        JsonlError::Io(source) if source.kind() == io::ErrorKind::NotFound => SysExit::NoInput,
        JsonlError::Io(_) => SysExit::IoErr,
        // Schema parse failures, ZstdUnsupported, and any future
        // non-exhaustive variant are all "bad input data".
        _ => SysExit::DataErr,
    }
}

fn print_version(output: OutputFormat) -> Result<(), SysExit> {
    let payload = VersionOutput {
        binary: env!("CARGO_PKG_VERSION"),
        schema: SCHEMA_VERSION,
    };
    let mut stdout = io::stdout().lock();
    match output {
        OutputFormat::Json => {
            let s = serde_json::to_string(&payload).map_err(|_| SysExit::Software)?;
            writeln!(stdout, "{s}").map_err(|_| SysExit::IoErr)?;
        }
        OutputFormat::Text | OutputFormat::Wide => {
            writeln!(
                stdout,
                "trace {} (schema {})",
                payload.binary, payload.schema
            )
            .map_err(|_| SysExit::IoErr)?;
        }
    }
    Ok(())
}

fn emit_completions(shell: Shell) {
    let mut cmd = Cli::command();
    let mut stdout = io::stdout().lock();
    generate(shell, &mut cmd, "trace", &mut stdout);
}
