//! `trace` — `SemantxTrace` command-line interface.
//!
//! S0 placeholder: implements only the `version` and `completions <shell>`
//! subcommands. The full subcommand inventory specified in ADR-0014 §3
//! lands stage by stage (S2 ships `analyze`, S4 ships `graph` and
//! `report workflows`, etc.).

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::process::ExitCode;

use clap::error::ErrorKind;
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

    into_exit_code(dispatch(command, cli.global.output))
}

fn into_exit_code(result: Result<(), SysExit>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(u8::from(code)),
    }
}

fn dispatch(command: &Command, output: OutputFormat) -> Result<(), SysExit> {
    match *command {
        Command::Version => print_version(output),
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
