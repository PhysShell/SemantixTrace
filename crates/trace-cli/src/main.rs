//! `trace` — `SemantxTrace` command-line interface.
//!
//! S0 placeholder: implements only the `version` and `completions <shell>`
//! subcommands. The full subcommand inventory specified in ADR-0014 §3
//! lands stage by stage (S2 ships `analyze`, S4 ships `graph` and
//! `report workflows`, etc.).

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use serde::Serialize;
use sysexits::ExitCode as SysExit;

/// Schema version supported by this binary build.
///
/// Bumped in lock-step with `trace-schema` (S1 onward); see ADR-0006
/// and `docs/upcasters.md`.
const SCHEMA_VERSION: u32 = 1;

#[derive(Parser, Debug)]
#[command(
    name = "trace",
    version,
    about = "SemantxTrace — behavioral observability for desktop UI applications",
    long_about = None,
    propagate_version = true,
)]
struct Cli {
    #[command(flatten)]
    global: GlobalOptions,

    #[command(subcommand)]
    command: Command,
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
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Print binary and schema versions.
    Version,
    /// Emit a shell-completion script for the given shell.
    Completions {
        /// Target shell.
        shell: Shell,
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
    let cli = Cli::parse();
    match dispatch(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(u8::from(code)),
    }
}

fn dispatch(cli: &Cli) -> Result<(), SysExit> {
    match cli.command {
        Command::Version => print_version(cli.global.output),
        Command::Completions { shell } => {
            emit_completions(shell);
            Ok(())
        }
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
