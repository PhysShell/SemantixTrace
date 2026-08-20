//! E2 experiment tool: read a v2 JSONL trace file produced by the Nezha
//! telemetry importer, group events by session (preserving first-seen
//! order, exactly like `trace normalize`), fold each session through the
//! real `trace-normalizer`, and emit one JSON object per session:
//!
//! ```json
//! {"session_id":"…","actions":[{"screen_id":…,"command_id":…,
//!   "abstract_args":…,"first_seq":n,"seqs":[…]}],"fold_report":{…}}
//! ```
//!
//! Each action additionally carries the `seq` numbers of the source
//! events that produced it (H4 provenance). The alignment is computed by
//! replaying the fold rules with the crate's own `abstract_value`, then
//! VERIFIED against the authoritative `normalize()` output — any
//! divergence aborts (fail-closed, no silent sinks).

use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::process::ExitCode;

use serde::Serialize;
use trace_core::{CanonicalAction, CommandId, ScreenId, Session, SessionId};
use trace_normalizer::abstraction::abstract_value;
use trace_normalizer::{normalize, FoldReport, NormCfg};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;

#[derive(Serialize)]
struct ActionOut<'a> {
    screen_id: &'a ScreenId,
    command_id: &'a CommandId,
    abstract_args: &'a serde_json::Value,
    first_seq: u64,
    seqs: &'a [u64],
}

#[derive(Serialize)]
struct SessionOut<'a> {
    session_id: &'a SessionId,
    actions: Vec<ActionOut<'a>>,
    fold_report: FoldReport,
}

/// Replay of `trace_normalizer::fold::normalize` that additionally
/// records, per emitted action, the seq numbers of contributing events
/// (the action's own event plus burst-collapsed repeats).
fn fold_with_alignment(
    session: &Session<Current>,
    cfg: &NormCfg,
) -> (Vec<CanonicalAction>, Vec<Vec<u64>>) {
    let mut current_screen = ScreenId::new("<unknown>");
    let mut actions: Vec<CanonicalAction> = Vec::new();
    let mut seqs: Vec<Vec<u64>> = Vec::new();
    let mut last_action_ts: Option<chrono::DateTime<chrono::Utc>> = None;

    for event in session.events() {
        match &event.kind {
            TraceEventKind::ScreenOpened { screen_id, .. } => {
                current_screen = screen_id.clone();
            }
            TraceEventKind::NavigationOccurred { to, .. } => {
                current_screen = to.clone();
            }
            TraceEventKind::CommandExecuted {
                command_id, args, ..
            } => {
                let action = CanonicalAction {
                    screen_id: current_screen.clone(),
                    command_id: command_id.clone(),
                    abstract_args: abstract_value(args, cfg),
                };
                let is_burst = actions.last() == Some(&action)
                    && last_action_ts.is_some_and(|t| {
                        let delta = (event.ts - t).num_milliseconds();
                        delta >= 0 && delta < cfg.burst_gap_ms
                    });
                if is_burst {
                    if let Some(last) = seqs.last_mut() {
                        last.push(event.seq.0);
                    }
                } else {
                    actions.push(action);
                    seqs.push(vec![event.seq.0]);
                }
                last_action_ts = Some(event.ts);
            }
            _ => {}
        }
    }
    (actions, seqs)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: st-fold <events.jsonl> [out.jsonl]");
        return ExitCode::from(64);
    };
    let out_path = args.next();

    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("st-fold: cannot open {path}: {e}");
            return ExitCode::from(66);
        }
    };
    let reader = std::io::BufReader::new(file);

    let mut order: Vec<SessionId> = Vec::new();
    let mut groups: HashMap<SessionId, Vec<Current>> = HashMap::new();
    let mut line_no: u64 = 0;
    for item in trace_storage::read_events(reader) {
        line_no += 1;
        match item {
            Ok(event) => {
                let sid = event.session_id;
                groups
                    .entry(sid)
                    .or_insert_with(|| {
                        order.push(sid);
                        Vec::new()
                    })
                    .push(event);
            }
            Err(e) => {
                eprintln!("st-fold: {path}: line {line_no}: {e}");
                return ExitCode::from(65);
            }
        }
    }

    let sink: Box<dyn Write> = match &out_path {
        Some(p) => match std::fs::File::create(p) {
            Ok(f) => Box::new(BufWriter::new(f)),
            Err(e) => {
                eprintln!("st-fold: cannot create {p}: {e}");
                return ExitCode::from(73);
            }
        },
        None => Box::new(BufWriter::new(std::io::stdout().lock())),
    };
    let mut sink = sink;

    let cfg = NormCfg::default();
    let mut sessions = 0_u64;
    let mut totals = FoldReport::default();
    for sid in &order {
        let Some(events) = groups.remove(sid) else {
            continue;
        };
        let session = match Session::new(*sid, events) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("st-fold: session {sid:?}: {e}");
                return ExitCode::from(65);
            }
        };
        let (scenario, report) = normalize(&session, &cfg);
        let (aligned_actions, aligned_seqs) = fold_with_alignment(&session, &cfg);
        if aligned_actions != scenario.actions {
            eprintln!(
                "st-fold: ALIGNMENT DIVERGENCE in session {sid:?}: replay \
                 produced {} actions, normalize() produced {} — aborting",
                aligned_actions.len(),
                scenario.actions.len()
            );
            return ExitCode::from(70);
        }
        totals.input_events += report.input_events;
        totals.output_actions += report.output_actions;
        totals.collapsed_bursts += report.collapsed_bursts;
        totals.dropped_noise += report.dropped_noise;
        totals.session_pauses += report.session_pauses;
        sessions += 1;
        let out = SessionOut {
            session_id: sid,
            actions: scenario
                .actions
                .iter()
                .zip(aligned_seqs.iter())
                .map(|(a, s)| ActionOut {
                    screen_id: &a.screen_id,
                    command_id: &a.command_id,
                    abstract_args: &a.abstract_args,
                    first_seq: s[0],
                    seqs: s,
                })
                .collect(),
            fold_report: report,
        };
        match serde_json::to_string(&out) {
            Ok(json) => {
                if writeln!(sink, "{json}").is_err() {
                    eprintln!("st-fold: write failed");
                    return ExitCode::from(74);
                }
            }
            Err(e) => {
                eprintln!("st-fold: serialize failed: {e}");
                return ExitCode::from(70);
            }
        }
    }
    if sink.flush().is_err() {
        eprintln!("st-fold: flush failed");
        return ExitCode::from(74);
    }
    eprintln!(
        "st-fold: sessions={sessions} input_events={} output_actions={} \
         collapsed_bursts={} dropped_noise={} session_pauses={}",
        totals.input_events,
        totals.output_actions,
        totals.collapsed_bursts,
        totals.dropped_noise,
        totals.session_pauses,
    );
    ExitCode::SUCCESS
}
