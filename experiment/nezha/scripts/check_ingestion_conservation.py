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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run_e2 import DATES, NORMAL_WINDOWS, CODE_DIRS, abnormal_window  # noqa: E402


MANIFEST = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                        "..", "manifests", "expected-windows.json")


def expected_windows():
    """The COMMITTED expected-window manifest (D-020): the gate
    certifies coverage against an independent, reviewable source —
    never against the runroot it is checking (Codex round-10 P2: an
    empty or partial runroot previously exited 0). The committed
    manifest is additionally cross-checked against a fresh derivation
    from the pinned fault lists and normal windows; a mismatch is a
    hard error, so neither the file nor the derivation can drift
    silently."""
    m = json.load(open(MANIFEST))
    manifest_set = {f"{w['ns']}/{w['phase']}/{w['window']}"
                    for w in m["windows"]}
    derived = set()
    for ns in ("hipster", "ts"):
        for date in DATES[ns]:
            tag = f"{date} {NORMAL_WINDOWS[date]}".replace(" ", "_").replace(":", "")
            derived.add(f"{ns}/construct/{tag}")
            fault_data = json.load(open(os.path.join(
                CODE_DIRS[ns], "rca_data", date, f"{date}-fault_list.json")))
            for hour in fault_data:
                for fault in fault_data[hour]:
                    win = abnormal_window(fault["inject_time"])
                    derived.add(f"{ns}/rca/" + win.replace(" ", "_").replace(":", ""))
    if manifest_set != derived:
        raise SystemExit(
            f"expected-window manifest disagrees with the fault-list "
            f"derivation: {sorted(manifest_set ^ derived)}")
    return manifest_set


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
    # Fail closed on ABSENT counters (CodeRabbit final round, D-023):
    # `c.get(name, 0)` let an expected window with empty outputs and an
    # empty counter block satisfy every conservation equation as
    # 0 == 0. The importer writes ANCHOR counters unconditionally (by
    # assignment), so their absence proves a gutted/foreign report;
    # occurrence counters (spans_accepted, alert_events, ...) are only
    # incremented, so absence is a true zero IFF nothing of that kind
    # was emitted.
    for counter_name in ("traceids_listed", "traceids_unique",
                         "traceids_duplicate_entries", "trace_rows_read",
                         "log_rows_read", "log_rows_repaired_tokenization",
                         "log_rows_artifact_style_read", "events_emitted"):
        if counter_name not in c:
            problems.append(f"required counter missing: {counter_name}")
    total_emitted = sum(ev_keys.values())
    if "events_emitted" in c and c["events_emitted"] != total_emitted:
        problems.append(f"events_emitted={c['events_emitted']} but "
                        f"{total_emitted} events in events.jsonl")
    for counter_name, emitted, label in checks:
        if counter_name not in c:
            if emitted:
                problems.append(f"{emitted} {label} emitted but "
                                f"{counter_name} counter missing")
        elif c[counter_name] != emitted:
            problems.append(f"{counter_name}={c[counter_name]} but "
                            f"{emitted} {label} emitted")
    for rule, kind in (("span-v1", "span"), ("log-v1", "log"),
                       ("alert-v1", "alert")):
        if rules.get(rule, 0) != kinds[kind]:
            problems.append(f"{rules.get(rule, 0)} {rule} provenance vs "
                            f"{kinds[kind]} {kind} events")
    if "sessions_emitted" not in c:
        if sids:
            problems.append(f"{len(sids)} distinct session_ids but "
                            f"sessions_emitted counter missing")
    elif c["sessions_emitted"] != len(sids):
        problems.append(f"sessions_emitted={c['sessions_emitted']} "
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
    found = set()
    for rp in sorted(glob.glob(os.path.join(runroot, "*", "*", "*",
                                            "import-report.json"))):
        d = os.path.dirname(rp)
        tag = "/".join(d.split("/")[-3:])
        found.add(tag)
        res = check_window(d)
        if res["traceids_duplicate_entries"]:
            dup_sources += 1
        if res["problems"]:
            n_bad += 1
            windows[tag] = res
    expected = expected_windows()
    missing = sorted(expected - found)
    unexpected = sorted(found - expected)
    for tag in missing:
        n_bad += 1
        windows[tag] = {"problems": ["expected window MISSING from runroot"]}
    for tag in unexpected:
        n_bad += 1
        windows[tag] = {"problems": ["window not in the expected manifest"]}
    summary = {"runroot": runroot,
               "expected_windows": len(expected),
               "discovered_windows": len(found),
               "missing_windows": missing,
               "unexpected_windows": unexpected,
               "windows_with_violations": n_bad,
               "windows_with_source_duplicates_observed": dup_sources,
               "violations": windows}
    if out_path:
        with open(out_path, "w") as f:
            json.dump(summary, f, indent=1)
    print(f"conservation: windows={summary['discovered_windows']}"
          f"/{summary['expected_windows']} "
          f"violations={n_bad} source-dup-windows-observed={dup_sources}")
    for tag, res in list(windows.items())[:10]:
        print(" ", tag, res["problems"])
    sys.exit(0 if n_bad == 0 else 1)


if __name__ == "__main__":
    main()
