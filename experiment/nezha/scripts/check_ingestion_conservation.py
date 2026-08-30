#!/usr/bin/env python3
"""Ingestion conservation gate (PR #20 Codex round-9, D-019).

Both round-9 defects belong to one class - multiplicity/conservation
failure: a duplicate selector MULTIPLIED data (re-emitted sessions,
colliding seqs), a spanless-trace skip DESTROYED data (accepted log
rows never emitted). This gate attacks the class, not the two
symptoms. For every window of a runroot it machine-checks:

  1. (session_id, seq) unique across the window's events;
  2. per-kind conservation: spans_accepted == emitted span-v1 events,
     logs_accepted == emitted log-v1 events, alert_events counter ==
     emitted alert-v1 events - no accepted source row disappears
     without an explicit categorized rejection;
  3. provenance <-> event bijection: the (session_id, seq) key sets of
     events.jsonl and provenance.jsonl.gz are equal, with no duplicate
     key on either side, and per-rule provenance counts match per-kind
     event counts;
  4. one emitted session per unique trace id that has accepted events:
     sessions_emitted == distinct session_ids in events, and (when the
     D-019 counters are present) sessions_emitted ==
     traceids_unique - traceids_without_events;
  5. source duplicate selectors stay OBSERVABLE (the counter is
     reported as found - a dirty input file is a fact, not a fault)
     but must not multiply output.

Usage: check_ingestion_conservation.py [runroot] [out.json]
Exit 1 on any violation in any window.
"""
import glob
import gzip
import json
import os
import sys
from collections import Counter


def check_window(d):
    rep = json.load(open(os.path.join(d, "import-report.json")))
    c = rep["counters"]
    ev_keys = Counter()
    kinds = Counter()
    sids = set()
    with open(os.path.join(d, "events.jsonl")) as f:
        for line in f:
            ev = json.loads(line)
            ev_keys[(ev["session_id"], ev["seq"])] += 1
            sids.add(ev["session_id"])
            kinds[ev.get("command_id", "").split(":", 1)[0]] += 1
    pr_keys = Counter()
    rules = Counter()
    with gzip.open(os.path.join(d, "provenance.jsonl.gz"), "rt") as f:
        for line in f:
            p = json.loads(line)
            pr_keys[(p["session_id"], p["seq"])] += 1
            rules[p.get("rule")] += 1

    problems = []
    dup_ev = sum(v - 1 for v in ev_keys.values() if v > 1)
    if dup_ev:
        problems.append(f"duplicate (session_id, seq) event keys: {dup_ev}")
    dup_pr = sum(v - 1 for v in pr_keys.values() if v > 1)
    if dup_pr:
        problems.append(f"duplicate provenance keys: {dup_pr}")
    if set(ev_keys) != set(pr_keys):
        problems.append(
            f"event/provenance key sets differ: "
            f"{len(set(ev_keys) - set(pr_keys))} event-only, "
            f"{len(set(pr_keys) - set(ev_keys))} provenance-only")
    checks = [
        ("spans_accepted", kinds["span"], "span events"),
        ("logs_accepted", kinds["log"], "log events"),
        ("alert_events", kinds["alert"], "alert events"),
    ]
    for counter_name, emitted, label in checks:
        if c.get(counter_name, 0) != emitted:
            problems.append(f"{counter_name}={c.get(counter_name, 0)} but "
                            f"{emitted} {label} emitted")
    for rule, kind in (("span-v1", "span"), ("log-v1", "log"),
                       ("alert-v1", "alert")):
        if rules.get(rule, 0) != kinds[kind]:
            problems.append(f"{rules.get(rule, 0)} {rule} provenance vs "
                            f"{kinds[kind]} {kind} events")
    if c.get("sessions_emitted", 0) != len(sids):
        problems.append(f"sessions_emitted={c.get('sessions_emitted', 0)} "
                        f"but {len(sids)} distinct session_ids")
    if "traceids_unique" in c:
        expect = c["traceids_unique"] - c.get("traceids_without_events", 0)
        if len(sids) != expect:
            problems.append(f"{len(sids)} sessions vs {expect} unique "
                            f"trace ids with events")
    return {
        "duplicate_session_seq_pairs": dup_ev,
        "accepted_span_rows_unemitted": c.get("spans_accepted", 0) - kinds["span"],
        "accepted_log_rows_unemitted": c.get("logs_accepted", 0) - kinds["log"],
        "traceids_duplicate_entries": c.get("traceids_duplicate_entries"),
        "log_only_sessions_emitted": c.get("log_only_sessions_emitted", 0),
        "problems": problems,
    }


def main():
    runroot = sys.argv[1] if len(sys.argv) > 1 else "/home/user/e2-runs"
    out_path = sys.argv[2] if len(sys.argv) > 2 else None
    windows = {}
    n_bad = 0
    dup_sources = 0
    for rp in sorted(glob.glob(os.path.join(runroot, "*", "*", "*",
                                            "import-report.json"))):
        d = os.path.dirname(rp)
        tag = "/".join(d.split("/")[-3:])
        res = check_window(d)
        if res["traceids_duplicate_entries"]:
            dup_sources += 1
        if res["problems"]:
            n_bad += 1
            windows[tag] = res
    summary = {"runroot": runroot,
               "windows_checked": len(glob.glob(os.path.join(
                   runroot, "*", "*", "*", "import-report.json"))),
               "windows_with_violations": n_bad,
               "windows_with_source_duplicates_observed": dup_sources,
               "violations": windows}
    if out_path:
        with open(out_path, "w") as f:
            json.dump(summary, f, indent=1)
    print(f"conservation: windows={summary['windows_checked']} "
          f"violations={n_bad} source-dup-windows-observed={dup_sources}")
    for tag, res in list(windows.items())[:10]:
        print(" ", tag, res["problems"])
    sys.exit(0 if n_bad == 0 else 1)


if __name__ == "__main__":
    main()
