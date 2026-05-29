//! `trace oracle run` — run oracle rules against a JSONL trace file.
//!
//! Exit codes (per S5 spec):
//! - `0` — all rules passed.
//! - `1` — at least one Warning or Error violation found.
//! - `2` — at least one Critical violation found.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use trace_core::Session;
use trace_oracle::engine::SessionReport;
use trace_oracle::rule::Severity;
use trace_oracle::{HtmlReporter, OracleEngine};

/// Which rule set to use.
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub(crate) enum RuleSet {
    /// All five built-in rules with default configuration.
    #[default]
    Builtin,
}

/// Run oracle rules, print text summary to stdout, optionally write HTML to
/// `out`.  Returns the exit code (`0`, `1`, or `2`).
pub(crate) fn run_text(file: &Path, _rules: RuleSet, out: Option<&Path>, quiet: bool) -> u8 {
    // Read all events.
    let events = match crate::read_all_events(file, quiet) {
        Ok(ev) => ev,
        Err(sys) => return u8::from(sys),
    };

    // Group by session id.
    let grouped = crate::normalize::group_by_session(events);

    // Build sessions (skip empty groups — defensive only).
    let sessions: Vec<_> = grouped
        .into_iter()
        .filter_map(|(sid, evs)| Session::new(sid, evs).ok())
        .collect();

    // Build engine.
    let engine = OracleEngine::new().with_builtin_rules();

    // Run.
    let reports: Vec<SessionReport> = engine.run_all(&sessions);

    // Determine exit code.
    let exit = exit_code_for(&reports);

    // Print text summary to stdout.
    let mut stdout = io::stdout().lock();
    print_text_summary(&reports, &mut stdout, quiet);

    // Optionally write HTML to file.
    if let Some(dest) = out {
        match File::create(dest) {
            Ok(f) => {
                let mut w = BufWriter::new(f);
                if let Err(e) = HtmlReporter::write(&reports, &mut w) {
                    if !quiet {
                        eprintln!("error writing HTML: {e}");
                    }
                }
            }
            Err(e) => {
                if !quiet {
                    eprintln!("error creating {}: {e}", dest.display());
                }
            }
        }
    }

    exit
}

fn print_text_summary(reports: &[SessionReport], sink: &mut impl Write, quiet: bool) {
    let total = reports.len();
    let passed = reports.iter().filter(|r| r.all_passed()).count();
    let failed = total - passed;
    if let Err(e) = writeln!(
        sink,
        "sessions: {total}  passed: {passed}  failed: {failed}"
    ) {
        if !quiet {
            eprintln!("error writing output: {e}");
        }
        return;
    }
    for report in reports {
        for result in &report.results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            let _ = writeln!(
                sink,
                "  [{status}] session={} rule={}",
                report.session_id, result.rule
            );
            for v in &result.violations {
                let _ = writeln!(sink, "       {:?}: {}", v.severity, v.message);
            }
        }
    }
}

fn exit_code_for(reports: &[SessionReport]) -> u8 {
    let max = reports.iter().filter_map(SessionReport::max_severity).max();
    match max {
        Some(Severity::Critical) => 2,
        Some(Severity::Error | Severity::Warning) => 1,
        None => 0,
    }
}
