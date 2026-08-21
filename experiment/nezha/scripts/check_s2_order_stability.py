#!/usr/bin/env python3
"""S2 regeneration-determinism check (Codex incremental P1, PR #20).

Mechanism under test: st-graph serializes its transition table from a
randomized HashMap, s2_scorer's dicts inherit that order, and the alarm
dedup keeps the FIRST max-depth resource candidate — so two fresh
regenerations of the same window can retain different candidates when an
alarmed pod has several max-depth scored patterns.

For every distinct abnormal window of both datasets this script:
  1. regenerates graph.json TWICE with the given st-graph binary
     (separate processes => independent HashMap seeds);
  2. runs the unchanged s2_scorer on each;
  3. evaluates each with the preregistered semantics;
  4. compares run1 vs run2 (candidate identity + per-case ranks) and
     run1 vs the committed s2-*.cases.json evaluations.

Output: a machine-readable summary; exit 1 if any instability or drift
from committed evaluations is observed (RED for the unsorted binary,
expected-green for the sorted one).
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "evaluators"))
from run_e2 import DATES, NORMAL_WINDOWS, CODE_DIRS, abnormal_window, run  # noqa: E402
from s1_eval import evaluate_case, service_of  # noqa: E402

PY = "/home/user/.venv-nezha/bin/python"
ADAPTERS = os.path.join(os.path.dirname(__file__), "..", "adapters")
SCORER = os.path.join(ADAPTERS, "s2_scorer.py")
RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")


def cand_multiset(cands):
    return sorted(json.dumps(
        {k: c[k] for k in ("pattern", "score", "anomaly", "deepth", "pod")},
        sort_keys=True) for c in cands)


def main():
    stgraph = sys.argv[1]
    tag = sys.argv[2]  # label for scratch dirs, e.g. 'unsorted' or 'sorted'
    out_path = sys.argv[3]

    templates = {ns: json.load(open(
        os.path.join(RUNROOT, f"templates-{ns}.json")))["clusters"]
        for ns in ("hipster", "ts")}
    committed = {}
    for ns in ("hipster", "ts"):
        d = json.load(open(os.path.join(RUNROOT, "results",
                                        f"s2-{ns}.cases.json")))
        for c in d["cases"]:
            committed[c["case_id"]] = c["evaluation"]

    unstable_windows = []
    rank_diff_cases = []
    drift_from_committed = []
    n_windows = n_cases = 0

    for ns in ("hipster", "ts"):
        code_dir = CODE_DIRS[ns]
        root_cause_map = json.load(open(os.path.join(
            code_dir, "construct_data", f"root_cause_{ns}.json")))
        for date in DATES[ns]:
            normal_win = f"{date} {NORMAL_WINDOWS[date]}"
            normal_dir = os.path.join(
                RUNROOT, ns, "construct",
                normal_win.replace(" ", "_").replace(":", ""))
            fault_data = json.load(open(os.path.join(
                code_dir, "rca_data", date, f"{date}-fault_list.json")))
            idx = 0
            done_windows = {}
            for hour in fault_data:
                for fault in fault_data[hour]:
                    case_id = f"{ns}-{date}-{idx:03d}"
                    idx += 1
                    n_cases += 1
                    win = abnormal_window(fault["inject_time"])
                    abn_dir = os.path.join(
                        RUNROOT, ns, "rca",
                        win.replace(" ", "_").replace(":", ""))
                    if win not in done_windows:
                        n_windows += 1
                        w = os.path.join(abn_dir, f"stab-{tag}")
                        os.makedirs(w, exist_ok=True)
                        pair = []
                        for rep in ("r1", "r2"):
                            gj = os.path.join(w, f"graph-{rep}.json")
                            run([stgraph,
                                 os.path.join(normal_dir, "scenarios.jsonl"),
                                 os.path.join(abn_dir, "scenarios.jsonl"),
                                 gj])
                            sc = os.path.join(w, f"scored-{rep}.json")
                            run([PY, SCORER, "--graph", gj,
                                 "--normal-scenarios",
                                 os.path.join(normal_dir, "scenarios.jsonl"),
                                 "--normal-events",
                                 os.path.join(normal_dir, "events.jsonl"),
                                 "--alarms",
                                 os.path.join(abn_dir, "import-report.json"),
                                 "--out", sc])
                            pair.append(json.load(open(sc))["candidates"])
                        stable = cand_multiset(pair[0]) == cand_multiset(pair[1])
                        if not stable:
                            unstable_windows.append(f"{ns}/{win}")
                        done_windows[win] = pair
                    pair = done_windows[win]
                    svc = service_of(fault["inject_pod"])
                    try:
                        rc_parts = root_cause_map[svc][fault["inject_type"]].split("_")
                    except KeyError:
                        rc_parts = []
                    ev1 = evaluate_case(pair[0], rc_parts,
                                        fault["inject_pod"], templates[ns])
                    ev2 = evaluate_case(pair[1], rc_parts,
                                        fault["inject_pod"], templates[ns])
                    if ev1 != ev2:
                        rank_diff_cases.append(
                            {"case": case_id, "r1": ev1, "r2": ev2})
                    if ev1 != committed.get(case_id):
                        drift_from_committed.append(
                            {"case": case_id, "regen": ev1,
                             "committed": committed.get(case_id)})

    summary = {
        "binary": stgraph, "tag": tag,
        "windows": n_windows, "cases": n_cases,
        "unstable_windows": unstable_windows,
        "rank_diff_cases_run1_vs_run2": rank_diff_cases,
        "drift_from_committed_run1": drift_from_committed,
    }
    with open(out_path, "w") as f:
        json.dump(summary, f, indent=1)
    print(f"[{tag}] windows={n_windows} cases={n_cases} "
          f"unstable_candidate_sets={len(unstable_windows)} "
          f"rank_diffs_r1_vs_r2={len(rank_diff_cases)} "
          f"drift_vs_committed={len(drift_from_committed)}")
    ok = not unstable_windows and not rank_diff_cases and not drift_from_committed
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
