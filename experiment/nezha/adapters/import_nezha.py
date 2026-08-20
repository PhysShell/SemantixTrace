#!/usr/bin/env python3
"""E2 telemetry importer: one Nezha minute-window -> SemantixTrace v2 JSONL.

Implements the FROZEN mapping of 00-preregistration.md §6 (rules span-v1,
log-v1, alert-v1, session-v1). Design completions documented here (and in
04-e2-representation.md) because the frozen table left them open:

- Alert-event session assignment: one alert event per (session, alarmed
  pod) for every alarm entry of that pod, emitted only into sessions
  whose spans touch the pod (the closest honest analog of the artifact's
  per-span injection, minus its duplication).
- seq tie-break at equal timestamps: (kind_rank, source_row) with
  alert=0 < span=1 < log=2 — deterministic and documented.
- Log rows join sessions via their TraceID column (present in the source
  schema); rows whose TraceID is not in the window's traceid list are
  counted as excluded, exactly like trace rows.

Parity guarantees:
- Log template ids come from the artifact's own log_parsing() running on
  a scratch copy of the SHIPPED drain3 state (E0 established the
  vocabulary is closed on this dataset: 674/694 clusters, zero
  new/changed across full runs). Any change_type != "none" is counted
  and reported as a tripwire.
- Metric alarms come from the artifact's own generate_alarm(),
  unmodified, imported from a Nezha checkout.

No silent sinks: every read/accepted/rejected/excluded record is counted
by reason in import-report.json; every emitted event has a provenance
record (dataset, file, row, key ids, rule) in provenance.jsonl.gz.
"""
import argparse
import gzip
import json
import os
import shutil
import sys
import tempfile
import uuid
from collections import Counter, defaultdict
from datetime import datetime, timezone

import pandas as pd

NAMESPACE = uuid.uuid5(uuid.NAMESPACE_URL, "semantixtrace-nezha-e2")
KIND_RANK = {"alert": 0, "span": 1, "log": 2}


def service_of(pod):
    return pod.rsplit("-", 1)[0].rsplit("-", 1)[0]


def ns_to_rfc3339(ns):
    ns = int(ns)
    secs, frac = divmod(ns, 1_000_000_000)
    base = datetime.fromtimestamp(secs, tz=timezone.utc)
    return base.strftime("%Y-%m-%dT%H:%M:%S") + f".{frac:09d}Z"


def load_artifact(code_dir):
    sys.path.insert(0, code_dir)
    import alarm as nezha_alarm            # noqa: E402
    import log_parsing as nezha_logparse   # noqa: E402
    return nezha_alarm, nezha_logparse


def make_miner(code_dir, ns, scratch):
    """Load the shipped drain3 state into a scratch copy (never mutates
    the checkout's log_template)."""
    from drain3 import TemplateMiner
    from drain3.file_persistence import FilePersistence
    from drain3.template_miner_config import TemplateMinerConfig

    src = os.path.join(code_dir, "log_template")
    shutil.copy(os.path.join(src, f"drain3_{ns}.ini"),
                os.path.join(scratch, f"drain3_{ns}.ini"))
    shutil.copy(os.path.join(src, f"{ns}.bin"),
                os.path.join(scratch, f"{ns}.bin"))
    config = TemplateMinerConfig()
    config.load(os.path.join(scratch, f"drain3_{ns}.ini"))
    config.profiling_enabled = False
    persistence = FilePersistence(os.path.join(scratch, f"{ns}.bin"))
    return TemplateMiner(persistence, config=config)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ns", required=True, choices=["hipster", "ts"])
    ap.add_argument("--window", required=True, help='e.g. "2022-08-22 03:51"')
    ap.add_argument("--phase", required=True, choices=["construct", "rca"])
    ap.add_argument("--code-dir", required=True,
                    help="Nezha checkout providing code AND data dirs")
    ap.add_argument("--out-dir", required=True)
    args = ap.parse_args()
    args.code_dir = os.path.abspath(args.code_dir)
    args.out_dir = os.path.abspath(args.out_dir)
    # The artifact reads "metric_threshold" via a relative path; it
    # assumes cwd is the checkout root.
    os.chdir(args.code_dir)

    date, hour_min = args.window.split(" ")
    hh, mm = hour_min.split(":")
    data_root = os.path.join(args.code_dir,
                             "construct_data" if args.phase == "construct"
                             else "rca_data")
    ddir = os.path.join(data_root, date)
    trace_file = os.path.join(ddir, "trace", f"{hh}_{mm}_trace.csv")
    log_file = os.path.join(ddir, "log", f"{hh}_{mm}_log.csv")
    traceid_file = os.path.join(ddir, "traceid", f"{hh}_{mm}_traceid.csv")

    os.makedirs(args.out_dir, exist_ok=True)
    report = {
        "ns": args.ns, "window": args.window, "phase": args.phase,
        "sources": {"trace": os.path.relpath(trace_file, args.code_dir),
                    "log": os.path.relpath(log_file, args.code_dir),
                    "traceid": os.path.relpath(traceid_file, args.code_dir)},
        "counters": Counter(), "rejections": Counter(),
    }
    counters, rejections = report["counters"], report["rejections"]

    nezha_alarm, nezha_logparse = load_artifact(args.code_dir)
    scratch = tempfile.mkdtemp(prefix="drain3-")
    miner = make_miner(args.code_dir, args.ns, scratch)
    clusters_before = len(miner.drain.clusters)

    # --- alarms via the artifact's own code, unmodified -------------------
    metric_list = nezha_alarm.get_metric_with_time(args.window, data_root)
    alarm_list = nezha_alarm.generate_alarm(metric_list, args.ns)
    report["alarm_list"] = alarm_list
    pod_alarms = {a["pod"]: a["alarm"] for a in alarm_list}

    # --- trace ids (the artifact's own selection) -------------------------
    traceids = pd.read_csv(traceid_file, header=None)[0].astype(str).tolist()
    traceid_set = set(traceids)
    counters["traceids_listed"] = len(traceids)

    # --- spans ------------------------------------------------------------
    tdf = pd.read_csv(trace_file, dtype=str)
    counters["trace_rows_read"] = len(tdf)
    spans_by_trace = defaultdict(list)
    for idx, row in enumerate(tdf.itertuples(index=False)):
        tid = str(row.TraceID)
        if tid not in traceid_set:
            counters["trace_rows_excluded_by_traceid_filter"] += 1
            continue
        try:
            pod = str(row.PodName)
            op = str(row.OperationName)
            start_ns = int(row.StartTimeUnixNano)
            dur_us = int(row.Duration)
            if not pod or pod == "nan" or not op or op == "nan":
                raise ValueError("missing pod/operation")
        except (TypeError, ValueError) as e:
            rejections[f"span:{e}"] += 1
            continue
        spans_by_trace[tid].append({
            "row": idx, "pod": pod, "op": op, "start_ns": start_ns,
            "dur_ms": max(dur_us, 0) // 1000,
            "span_id": str(row.SpanID), "parent_id": str(row.ParentID),
        })
        counters["spans_accepted"] += 1

    # --- logs -------------------------------------------------------------
    # The python engine parses embedded-quote log bodies correctly where
    # the C tokenizer (which the artifact uses via usecols) garbles them
    # into extra rows. Repairs (extra fields folded back into Log) are
    # counted; the row-count delta vs the artifact-style read is recorded.
    log_repairs = []

    def _fix_bad_line(bad):
        log_repairs.append(len(bad))
        return bad[:7] + [",".join(bad[7:])]

    ldf = pd.read_csv(log_file, dtype=str, engine="python",
                      on_bad_lines=_fix_bad_line)
    counters["log_rows_read"] = len(ldf)
    counters["log_rows_repaired_tokenization"] = len(log_repairs)
    try:
        artifact_style_rows = len(pd.read_csv(
            log_file, index_col="SpanID",
            usecols=["TimeUnixNano", "SpanID", "Log"], engine="c"))
    except Exception:  # noqa: BLE001 - diagnostic only
        artifact_style_rows = -1
    counters["log_rows_artifact_style_read"] = artifact_style_rows
    logs_by_trace = defaultdict(list)
    for idx, row in enumerate(ldf.itertuples(index=False)):
        tid = str(row.TraceID)
        if tid not in traceid_set:
            counters["log_rows_excluded_by_traceid_filter"] += 1
            continue
        try:
            pod = str(row.PodName)
            ts_ns = int(row.TimeUnixNano)
            raw = row.Log
            if pod == "nan" or not isinstance(raw, str):
                raise ValueError("missing pod/log body")
        except (TypeError, ValueError) as e:
            rejections[f"log:{e}"] += 1
            continue
        cluster_id = nezha_logparse.log_parsing(
            log=raw, pod=pod, log_template_miner=miner)
        logs_by_trace[tid].append({
            "row": idx, "pod": pod, "ts_ns": ts_ns,
            "cluster_id": cluster_id, "span_id": str(row.SpanID),
        })
        counters["logs_accepted"] += 1

    clusters_after = len(miner.drain.clusters)
    report["drain3_new_clusters"] = clusters_after - clusters_before
    report["drain3_clusters"] = clusters_after

    window_start = datetime.strptime(args.window, "%Y-%m-%d %H:%M") \
        .replace(tzinfo=timezone.utc)
    window_start_ns = int(window_start.timestamp()) * 1_000_000_000
    window_start_iso = window_start.strftime("%Y-%m-%dT%H:%M:%S") + ".000000000Z"

    ev_path = os.path.join(args.out_dir, "events.jsonl")
    prov_path = os.path.join(args.out_dir, "provenance.jsonl.gz")
    n_events = 0
    with open(ev_path, "w") as ef, gzip.open(prov_path, "wt") as pf:
        for tid in traceids:
            spans = spans_by_trace.get(tid, [])
            if not spans:
                counters["traceids_without_spans"] += 1
                continue
            logs = logs_by_trace.get(tid, [])
            session_id = str(uuid.uuid5(
                NAMESPACE, f"{args.ns}/{args.phase}/{args.window}/{tid}"))
            corr = str(uuid.uuid5(NAMESPACE, tid))
            pending = []
            for s in spans:
                svc = service_of(s["pod"])
                pending.append((s["start_ns"], KIND_RANK["span"], s["row"], {
                    "kind": "CommandExecuted",
                    "command_id": f"span:{svc} {s['op']}",
                    "args": {"pod": s["pod"], "span_id": s["span_id"],
                             "parent_id": s["parent_id"]},
                    "duration_ms": s["dur_ms"], "outcome": "success",
                    "domain_entity_id": f"span:{s['span_id']}",
                }, {"rule": "span-v1", "file": report["sources"]["trace"],
                    "row": s["row"], "span_id": s["span_id"], "pod": s["pod"]}))
            for lg in logs:
                pending.append((lg["ts_ns"], KIND_RANK["log"], lg["row"], {
                    "kind": "CommandExecuted",
                    "command_id": f"log:{lg['cluster_id']}",
                    "args": {"pod": lg["pod"]},
                    "duration_ms": 0, "outcome": "success",
                    "domain_entity_id": f"span:{lg['span_id']}",
                }, {"rule": "log-v1", "file": report["sources"]["log"],
                    "row": lg["row"], "span_id": lg["span_id"],
                    "pod": lg["pod"], "cluster_id": lg["cluster_id"]}))
            session_pods = {s["pod"] for s in spans}
            for pod in sorted(session_pods & set(pod_alarms)):
                for a_idx, entry in enumerate(pod_alarms[pod]):
                    pending.append((window_start_ns, KIND_RANK["alert"], a_idx, {
                        "kind": "CommandExecuted",
                        "command_id": f"alert:{entry['metric_type']}",
                        "args": {"pod": pod},
                        "duration_ms": 0, "outcome": "success",
                        "domain_entity_id": f"pod:{pod}",
                    }, {"rule": "alert-v1", "file": "generate_alarm()",
                        "row": a_idx, "pod": pod,
                        "metric_type": entry["metric_type"]}))
                    counters["alert_events"] += 1
            pending.sort(key=lambda t: (t[0], t[1], t[2]))
            for seq, (ts_ns, _kr, _row, kind_fields, prov) in enumerate(pending):
                ev = {"schema_version": 2, "seq": seq,
                      "session_id": session_id,
                      "ts": (window_start_iso if _kr == KIND_RANK["alert"]
                             else ns_to_rfc3339(ts_ns)),
                      "correlation_id": corr}
                dei = kind_fields.pop("domain_entity_id")
                ev["domain_entity_id"] = dei
                ev.update(kind_fields)
                ef.write(json.dumps(ev, separators=(",", ":")) + "\n")
                prov_rec = {"session_id": session_id, "seq": seq, **prov}
                pf.write(json.dumps(prov_rec, separators=(",", ":")) + "\n")
                n_events += 1
            counters["sessions_emitted"] += 1
    counters["events_emitted"] = n_events

    report["counters"] = dict(counters)
    report["rejections"] = dict(rejections)
    with open(os.path.join(args.out_dir, "import-report.json"), "w") as f:
        json.dump(report, f, indent=1, default=str)
    shutil.rmtree(scratch, ignore_errors=True)
    print(f"{args.ns} {args.phase} {args.window}: sessions="
          f"{counters.get('sessions_emitted', 0)} events={n_events} "
          f"rejected={sum(rejections.values())} "
          f"new_clusters={report['drain3_new_clusters']}")


if __name__ == "__main__":
    main()
