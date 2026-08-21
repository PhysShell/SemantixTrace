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
     run1 vs the committed s2-*.cases.json artifacts: evaluations,
     full-candidate multisets, the complete per-case record (every
     field except the volatile wall-clock `runtime_ms`), and the
     per-namespace `summary` block recomputed with run_e3.py's exact
     formula (Codex round-5 P2 on PR #20: stable fields such as
     algorithm/parameters/ground_truth and the summary aggregates must
     participate, or the "artifact level" claim is broader than the
     check).

Output: a machine-readable summary; exit 1 if any instability or drift
from the committed artifacts is observed (RED for the unsorted binary,
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
    """Full-candidate multiset: every field, including resource tags and
    provenance records, participates in the equality (Codex P2 #2 on
    PR #20: comparing only ranks understates the claim)."""
    return sorted(json.dumps(c, sort_keys=True) for c in cands)


def summarize(records, ns):
    """Recompute the per-namespace summary block with run_e3.py's exact
    formula (same expressions, same iteration order) so the committed
    `summary` can be compared bit-for-bit against the regeneration."""
    n = len(records)
    agg = {}
    for mode in ("rank_inner", "rank_service_raw", "rank_service_dedup"):
        ranks = [r["evaluation"][mode] for r in records]
        hit = [x for x in ranks if x is not None]
        agg[mode] = {
            "AC@1_pct": 100.0 * sum(1 for x in hit if x <= 1) / n,
            "AC@3_pct": 100.0 * sum(1 for x in hit if x <= 3) / n,
            "AC@5_pct": 100.0 * sum(1 for x in hit if x <= 5) / n,
            "MRR": sum(1.0 / x for x in hit) / n,
            "unlocalized": n - len(hit),
        }
    sizes = sorted(r["evaluation"]["n_candidates"] for r in records)
    return {"ns": ns, "n_cases": n, "aggregates": agg,
            "candidate_sizes": {"min": sizes[0],
                                "median": sizes[len(sizes) // 2],
                                "max": sizes[-1]}}


def main():
    stgraph = sys.argv[1]
    tag = sys.argv[2]  # label for scratch dirs, e.g. 'unsorted' or 'sorted'
    out_path = sys.argv[3]

    templates = {ns: json.load(open(
        os.path.join(RUNROOT, f"templates-{ns}.json")))["clusters"]
        for ns in ("hipster", "ts")}
    committed = {}
    # Baseline = the CHECKED-IN canonical results, not the mutable
    # workspace (Codex P2 on PR #20): the gate must validate the
    # artifacts being adopted, independently of $E2_RUNROOT state.
    baseline_dir = os.environ.get(
        "S2_BASELINE_DIR",
        os.path.join(os.path.dirname(__file__), "..", "results", "e3"))
    committed_cands = {}
    committed_records = {}
    committed_summary = {}
    for ns in ("hipster", "ts"):
        d = json.load(open(os.path.join(baseline_dir,
                                        f"s2-{ns}.cases.json")))
        committed_summary[ns] = d["summary"]
        for c in d["cases"]:
            committed[c["case_id"]] = c["evaluation"]
            committed_cands[c["case_id"]] = cand_multiset(c["candidates"])
            # full record minus the volatile wall-clock field
            committed_records[c["case_id"]] = {
                k: v for k, v in c.items() if k != "runtime_ms"}

    unstable_windows = []
    rank_diff_cases = []
    drift_from_committed = []
    drift_candidates_from_committed = []
    drift_records_from_committed = []
    run1_records = {"hipster": [], "ts": []}
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
                            pair.append(json.load(open(sc)))
                        stable = (cand_multiset(pair[0]["candidates"])
                                  == cand_multiset(pair[1]["candidates"]))
                        if not stable:
                            unstable_windows.append(f"{ns}/{win}")
                        done_windows[win] = pair
                    pair = done_windows[win]
                    svc = service_of(fault["inject_pod"])
                    try:
                        rc = root_cause_map[svc][fault["inject_type"]]
                        rc_parts = rc.split("_")
                    except KeyError:
                        rc, rc_parts = None, []
                    ev1 = evaluate_case(pair[0]["candidates"], rc_parts,
                                        fault["inject_pod"], templates[ns])
                    ev2 = evaluate_case(pair[1]["candidates"], rc_parts,
                                        fault["inject_pod"], templates[ns])
                    if ev1 != ev2:
                        rank_diff_cases.append(
                            {"case": case_id, "r1": ev1, "r2": ev2})
                    if ev1 != committed.get(case_id):
                        drift_from_committed.append(
                            {"case": case_id, "regen": ev1,
                             "committed": committed.get(case_id)})
                    # full-artifact comparison: candidate lists (all
                    # fields incl. provenance) as multisets
                    if cand_multiset(pair[0]["candidates"]) \
                            != committed_cands.get(case_id):
                        drift_candidates_from_committed.append(case_id)
                    # complete case record, assembled exactly as
                    # run_e3.py writes it (minus volatile runtime_ms):
                    # covers case identity, ground truth,
                    # representation, algorithm, parameters, and the
                    # ordered candidate list (Codex round-5 P2)
                    record1 = {
                        "case_id": case_id, "dataset": date,
                        "inject_time": fault["inject_time"],
                        "inject_pod": fault["inject_pod"],
                        "inject_type": fault["inject_type"],
                        "ground_truth": rc,
                        "representation": "semantixtrace-v2-canonical",
                        "algorithm": pair[0]["algorithm"],
                        "parameters": pair[0]["parameters"],
                        "candidates": pair[0]["candidates"],
                        "evaluation": ev1,
                    }
                    run1_records[ns].append(record1)
                    if record1 != committed_records.get(case_id):
                        drift_records_from_committed.append(case_id)

    # committed summary block vs run_e3's formula applied to the run-1
    # regeneration: catches drifted aggregates / candidate-size stats
    summary_mismatches = []
    for ns in ("hipster", "ts"):
        if summarize(run1_records[ns], ns) != committed_summary.get(ns):
            summary_mismatches.append(ns)

    summary = {
        "binary": stgraph, "tag": tag,
        "windows": n_windows, "cases": n_cases,
        "unstable_windows": unstable_windows,
        "rank_diff_cases_run1_vs_run2": rank_diff_cases,
        "drift_from_committed_run1": drift_from_committed,
        "drift_candidates_from_committed_run1": drift_candidates_from_committed,
        "drift_records_from_committed_run1": drift_records_from_committed,
        "summary_mismatches": summary_mismatches,
    }
    with open(out_path, "w") as f:
        json.dump(summary, f, indent=1)
    print(f"[{tag}] windows={n_windows} cases={n_cases} "
          f"unstable_candidate_sets={len(unstable_windows)} "
          f"rank_diffs_r1_vs_r2={len(rank_diff_cases)} "
          f"drift_vs_committed={len(drift_from_committed)} "
          f"candidate_drift_vs_committed={len(drift_candidates_from_committed)} "
          f"record_drift_vs_committed={len(drift_records_from_committed)} "
          f"summary_mismatches={len(summary_mismatches)}")
    ok = (not unstable_windows and not rank_diff_cases
          and not drift_from_committed
          and not drift_candidates_from_committed
          and not drift_records_from_committed
          and not summary_mismatches)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
