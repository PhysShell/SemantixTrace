//! [`HtmlReporter`] — renders [`SessionReport`]s as an HTML page.

use std::io::{self, Write};

use crate::engine::SessionReport;
use crate::rule::Severity;

/// Renders a collection of [`SessionReport`]s as an HTML page.
///
/// The output is stable (no wall-clock timestamps) so it can be
/// snapshot-tested with `trycmd`.
#[derive(Clone, Copy, Debug, Default)]
pub struct HtmlReporter;

impl HtmlReporter {
    /// Write HTML for `reports` to the given `sink`.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if writing to `sink` fails.
    pub fn write<W: Write>(reports: &[SessionReport], sink: &mut W) -> io::Result<()> {
        write_html(reports, sink)
    }
}

/// Render `reports` as an HTML page, writing to `sink`.
///
/// # Errors
///
/// Returns `io::Error` if writing to `sink` fails.
pub fn write_html(reports: &[SessionReport], sink: &mut impl Write) -> io::Result<()> {
    let total = reports.len();
    let passed = reports.iter().filter(|r| r.all_passed()).count();
    let failed = total - passed;
    let has_critical = reports
        .iter()
        .any(|r| r.max_severity() == Some(Severity::Critical));

    writeln!(sink, "<!DOCTYPE html>")?;
    writeln!(sink, "<html lang=\"en\"><head><meta charset=\"utf-8\">")?;
    writeln!(sink, "<title>SemantxTrace Oracle Report</title>")?;
    writeln!(
        sink,
        "<style>\
         body{{font-family:sans-serif;margin:2em}} \
         table{{border-collapse:collapse;width:100%}} \
         td,th{{border:1px solid #ccc;padding:.4em .8em}} \
         .pass{{color:green}} \
         .fail{{color:red}} \
         .crit{{color:darkred;font-weight:bold}}\
         </style>"
    )?;
    writeln!(sink, "</head><body>")?;
    writeln!(sink, "<h1>Oracle Report</h1>")?;
    writeln!(
        sink,
        "<p>Sessions: {total} &nbsp;|&nbsp; Passed: {passed} &nbsp;|&nbsp; Failed: {failed}{}</p>",
        if has_critical {
            " &nbsp;| <strong class=\"crit\">CRITICAL</strong>"
        } else {
            ""
        }
    )?;

    writeln!(
        sink,
        "<table><thead><tr>\
         <th>Session</th><th>Rule</th><th>Result</th><th>Violations</th>\
         </tr></thead><tbody>"
    )?;
    for report in reports {
        let sid = escape_html(&report.session_id.to_string());
        for result in &report.results {
            let cls = if result.passed { "pass" } else { "fail" };
            let status = if result.passed { "PASS" } else { "FAIL" };
            let rule_name = escape_html(&result.rule);
            let viols = result
                .violations
                .iter()
                .map(|v| {
                    let seqs: Vec<String> = v.evidence.iter().map(ToString::to_string).collect();
                    format!(
                        "[{}] {} (seq {})",
                        format_severity(v.severity),
                        escape_html(&v.message),
                        seqs.join(", ")
                    )
                })
                .collect::<Vec<_>>()
                .join("<br>");
            writeln!(
                sink,
                "<tr><td>{sid}</td><td>{rule_name}</td>\
                 <td class=\"{cls}\">{status}</td><td>{viols}</td></tr>"
            )?;
        }
    }
    writeln!(sink, "</tbody></table></body></html>")?;
    Ok(())
}

/// Escape `s` so it is safe to embed in an HTML element or attribute.
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            c => out.push(c),
        }
    }
    out
}

const fn format_severity(s: Severity) -> &'static str {
    match s {
        Severity::Warning => "WARN",
        Severity::Error => "ERR",
        Severity::Critical => "CRIT",
    }
}
