//! `trace normalize` — fold a trace file into normalized canonical
//! actions (one JSON per line), grouping events by session id.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;

use sysexits::ExitCode as SysExit;
use trace_core::{Session, SessionId};
use trace_normalizer::{normalize, FoldReport, NormCfg};
use trace_schema::Current;

pub(crate) fn run(
    path: &Path,
    out: Option<&Path>,
    report: bool,
    quiet: bool,
) -> Result<(), SysExit> {
    let events = crate::read_all_events(path, quiet)?;
    let grouped = group_by_session(events);

    let mut sink: Box<dyn Write> = match out {
        Some(dest) => {
            let file = File::create(dest).map_err(|e| {
                if !quiet {
                    eprintln!("error: cannot create {}: {e}", dest.display());
                }
                SysExit::CantCreat
            })?;
            Box::new(BufWriter::new(file))
        }
        None => Box::new(io::stdout().lock()),
    };

    let cfg = NormCfg::default();
    let mut totals = FoldReport::default();
    for (sid, session_events) in grouped {
        // Groups are non-empty by construction; skip defensively.
        let Ok(session) = Session::new(sid, session_events) else {
            continue;
        };
        let (scenario, rep) = normalize(&session, &cfg);
        accumulate(&mut totals, rep);
        for action in &scenario.actions {
            let line = serde_json::to_string(action).map_err(|_| SysExit::Software)?;
            writeln!(sink, "{line}").map_err(|_| SysExit::IoErr)?;
        }
    }
    sink.flush().map_err(|_| SysExit::IoErr)?;

    if report && !quiet {
        eprintln!(
            "fold report: input_events={} output_actions={} collapsed_bursts={} \
             dropped_noise={} session_pauses={}",
            totals.input_events,
            totals.output_actions,
            totals.collapsed_bursts,
            totals.dropped_noise,
            totals.session_pauses,
        );
    }
    Ok(())
}

/// Group events by session id, preserving first-seen order.
fn group_by_session(events: Vec<Current>) -> Vec<(SessionId, Vec<Current>)> {
    let mut order: Vec<SessionId> = Vec::new();
    let mut groups: HashMap<SessionId, Vec<Current>> = HashMap::new();
    for event in events {
        let sid = event.session_id;
        groups
            .entry(sid)
            .or_insert_with(|| {
                order.push(sid);
                Vec::new()
            })
            .push(event);
    }
    order
        .into_iter()
        .filter_map(|sid| groups.remove(&sid).map(|evs| (sid, evs)))
        .collect()
}

fn accumulate(totals: &mut FoldReport, rep: FoldReport) {
    totals.input_events += rep.input_events;
    totals.output_actions += rep.output_actions;
    totals.collapsed_bursts += rep.collapsed_bursts;
    totals.dropped_noise += rep.dropped_noise;
    totals.session_pauses += rep.session_pauses;
}
