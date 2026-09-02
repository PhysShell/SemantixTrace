#!/usr/bin/env python3
"""E3 driver: run the S2 condition (frozen ActionGraph differential) over
every fault case, reusing the E2 import/fold cache.

Per case: st-graph builds normal+abnormal ActionGraphs from the cached
scenario files; s2_scorer applies the frozen differential and ordering;
evaluation uses the same preregistered corrected semantics as S1.
"""
import json
import os
import subprocess
import sys
import time

PY = "/home/user/.venv-nezha/bin/python"
ADAPTERS = os.path.join(os.path.dirname(__file__), "..", "adapters")
STGRAPH = os.path.join(ADAPTERS, "st-fold", "target", "release", "st-graph")
SCORER = os.path.join(ADAPTERS, "s2_scorer.py")
RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "evaluators"))
from s1_eval import evaluate_case, service_of  # noqa: E402

sys.path.insert(0, os.path.dirname(__file__))
from run_e2 import DATES, NORMAL_WINDOWS, CODE_DIRS, abnormal_window, run  # noqa: E402


def main():
    ns_list = sys.argv[1:] or ["ts", "hipster"]
    for ns in ns_list:
        templates = json.load(
            open(os.path.join(RUNROOT, f"templates-{ns}.json")))["clusters"]
        code_dir = CODE_DIRS[ns]
        with open(os.path.join(
                code_dir, "construct_data", f"root_cause_{ns}.json")) as f:
            root_cause_map = json.load(f)
        cases = []
        for date in DATES[ns]:
            normal_win = f"{date} {NORMAL_WINDOWS[date]}"
            normal_dir = os.path.join(
                RUNROOT, ns, "construct",
                normal_win.replace(" ", "_").replace(":", ""))
            with open(os.path.join(
                    code_dir, "rca_data", date, f"{date}-fault_list.json")) as f:
                fault_data = json.load(f)
            idx = 0
            for hour in fault_data:
                for fault in fault_data[hour]:
                    case_id = f"{ns}-{date}-{idx:03d}"
                    idx += 1
                    win = abnormal_window(fault["inject_time"])
                    abn_dir = os.path.join(
                        RUNROOT, ns, "rca",
                        win.replace(" ", "_").replace(":", ""))
                    graph_json = os.path.join(abn_dir, "graph.json")
                    t0 = time.time()
                    try:
                        if not os.path.exists(graph_json):
                            # temp + atomic publish, same cache
                            # invariant as import_and_fold (D-031)
                            run([STGRAPH,
                                 os.path.join(normal_dir, "scenarios.jsonl"),
                                 os.path.join(abn_dir, "scenarios.jsonl"),
                                 graph_json + ".tmp"])
                            os.replace(graph_json + ".tmp", graph_json)
                        case_out = os.path.join(abn_dir, f"s2-{case_id}.json")
                        run([PY, SCORER, "--graph", graph_json,
                             "--normal-scenarios",
                             os.path.join(normal_dir, "scenarios.jsonl"),
                             "--normal-events",
                             os.path.join(normal_dir, "events.jsonl"),
                             "--alarms",
                             os.path.join(abn_dir, "import-report.json"),
                             "--out", case_out])
                        wall_ms = int((time.time() - t0) * 1000)
                        scored = json.load(open(case_out))
                        svc = service_of(fault["inject_pod"])
                        try:
                            rc = root_cause_map[svc][fault["inject_type"]]
                            rc_parts = rc.split("_")
                        except KeyError:
                            rc, rc_parts = None, []
                        ev = evaluate_case(scored["candidates"], rc_parts,
                                           fault["inject_pod"], templates)
                        cases.append({
                            "case_id": case_id, "dataset": date,
                            "inject_time": fault["inject_time"],
                            "inject_pod": fault["inject_pod"],
                            "inject_type": fault["inject_type"],
                            "ground_truth": rc,
                            "representation": "semantixtrace-v2-canonical",
                            "algorithm": scored["algorithm"],
                            "parameters": scored["parameters"],
                            "candidates": scored["candidates"],
                            "evaluation": ev,
                            "runtime_ms": wall_ms,
                        })
                        print(f"{case_id} {fault['inject_type']:>14s} "
                              f"inner={ev['rank_inner']} "
                              f"svc_dedup={ev['rank_service_dedup']} "
                              f"ncand={ev['n_candidates']}", flush=True)
                    except Exception as exc:  # noqa: BLE001
                        # Frozen §8: a crashed case is counted as
                        # unlocalized AND separately reported with its
                        # cause; the driver must not abort the
                        # namespace (Codex round-11 P2, D-022).
                        cases.append({
                            "case_id": case_id, "dataset": date,
                            "inject_time": fault["inject_time"],
                            "inject_pod": fault["inject_pod"],
                            "inject_type": fault["inject_type"],
                            "ground_truth": None,
                            "abnormal_window": win,
                            "representation": "semantixtrace-v2-canonical",
                            "failure": f"{type(exc).__name__}: {exc}",
                            # schema-compatible with success records so
                            # downstream consumers (isolation gate) can
                            # index them (CodeRabbit, D-030)
                            "candidates": [],
                            "evaluation": {"rank_inner": None,
                                           "rank_service_raw": None,
                                           "rank_service_dedup": None,
                                           "n_candidates": 0},
                            "runtime_ms": int((time.time() - t0) * 1000),
                        })
                        print(f"{case_id} FAILED (counted unlocalized): "
                              f"{exc}", flush=True)

        n = len(cases)
        agg = {}
        for mode in ("rank_inner", "rank_service_raw", "rank_service_dedup"):
            ranks = [c["evaluation"][mode] for c in cases]
            hit = [r for r in ranks if r is not None]
            agg[mode] = {
                "AC@1_pct": 100.0 * sum(1 for r in hit if r <= 1) / n,
                "AC@3_pct": 100.0 * sum(1 for r in hit if r <= 3) / n,
                "AC@5_pct": 100.0 * sum(1 for r in hit if r <= 5) / n,
                "MRR": sum(1.0 / r for r in hit) / n,
                "unlocalized": n - len(hit),
            }
        sizes = sorted(c["evaluation"]["n_candidates"] for c in cases)
        summary = {"ns": ns, "n_cases": n, "aggregates": agg,
                   "candidate_sizes": {"min": sizes[0],
                                       "median": sizes[len(sizes) // 2],
                                       "max": sizes[-1]}}
        failed = [{"case_id": c["case_id"], "failure": c["failure"]}
                  for c in cases if "failure" in c]
        if failed:  # §8: failures are separately reported with cause
            summary["failed_cases"] = failed
        outdir = os.path.join(RUNROOT, "results")
        os.makedirs(outdir, exist_ok=True)
        with open(os.path.join(outdir, f"s2-{ns}.cases.json"), "w") as f:
            json.dump({"summary": summary, "cases": cases}, f, indent=1)
        print(json.dumps(summary, indent=1), flush=True)


if __name__ == "__main__":
    main()
