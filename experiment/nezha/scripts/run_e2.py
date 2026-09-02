#!/usr/bin/env python3
"""E2 driver: run the S1 condition over every fault case of both datasets.

For each namespace:
  1. import each date's construct window once (importer -> st-fold);
  2. per fault: compute the abnormal window exactly like the artifact
     (inject minute + 2, with its hour-rollover quirk), import + fold it
     (cached per distinct window), run s1_scorer against the date's
     construct window, evaluate with the preregistered corrected
     semantics;
  3. write per-case evidence records and a summary JSON.

All heavy artifacts stay under RUNROOT; the summary + per-case records
(candidate lists included) are small and get copied into the repo by the
caller.
"""
import json
import os
import subprocess
import sys
import time
import uuid

PY = "/home/user/.venv-nezha/bin/python"
ADAPTERS = os.path.join(os.path.dirname(__file__), "..", "adapters")
STFOLD = os.path.join(ADAPTERS, "st-fold", "target", "release", "st-fold")
IMPORTER = os.path.join(ADAPTERS, "import_nezha.py")
SCORER = os.path.join(ADAPTERS, "s1_scorer.py")
RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "evaluators"))
from s1_eval import evaluate_case, service_of  # noqa: E402

DATES = {"hipster": ["2022-08-22", "2022-08-23"],
         "ts": ["2023-01-29", "2023-01-30"]}
NORMAL_WINDOWS = {"2022-08-22": "03:51", "2022-08-23": "17:00",
                  "2023-01-29": "08:50", "2023-01-30": "11:39"}
CODE_DIRS = {"hipster": "/home/user/e0-runs/checkout-hipster",
             "ts": "/home/user/e0-runs/checkout-ts"}


def abnormal_window(inject_time):
    """Mirror of the artifact's window derivation (pattern_ranker.py
    :234-250), including the zero-padding quirks."""
    minute = int(inject_time.split(":")[1]) + 2
    if minute >= 60:
        hour_min = inject_time.split(" ")[1]
        hour = int(hour_min.split(":")[0])
        if hour < 9:
            return inject_time.split(" ")[0] + " 0" + str(hour + 1) + ":0" + str(minute - 60)
        return inject_time.split(" ")[0] + " " + str(hour + 1) + ":0" + str(minute - 60)
    if minute < 10:
        return inject_time.split(":")[0] + ":0" + str(minute)
    return inject_time.split(":")[0] + ":" + str(minute)


def run(cmd, **kw):
    r = subprocess.run(cmd, capture_output=True, text=True, **kw)
    if r.returncode != 0:
        raise RuntimeError(
            f"command failed ({r.returncode}): {' '.join(cmd)}\n"
            f"stdout tail: {r.stdout[-1500:]}\nstderr tail: {r.stderr[-1500:]}")
    return r


def import_and_fold(ns, window, phase, out_dir):
    scen = os.path.join(out_dir, "scenarios.jsonl")
    if os.path.exists(scen):
        return  # cached — the key exists ONLY after a completed fold
    os.makedirs(out_dir, exist_ok=True)
    run([PY, IMPORTER, "--ns", ns, "--window", window, "--phase", phase,
         "--code-dir", CODE_DIRS[ns], "--out-dir", out_dir])
    # Fold to a temp name and publish atomically: an interrupted
    # st-fold must never leave a syntactically valid prefix under the
    # cache key (Codex round-14 P1, D-031).
    run([STFOLD, os.path.join(out_dir, "events.jsonl"), scen + ".tmp"])
    os.replace(scen + ".tmp", scen)


def main():
    ns_list = sys.argv[1:] or ["ts", "hipster"]
    templates = {}
    for ns in ns_list:
        tpath = os.path.join(RUNROOT, f"templates-{ns}.json")
        run([PY, os.path.join(os.path.dirname(__file__), "dump_templates.py"),
             CODE_DIRS[ns], ns, tpath])
        templates[ns] = json.load(open(tpath))["clusters"]

    for ns in ns_list:
        cases = []
        code_dir = CODE_DIRS[ns]
        with open(os.path.join(
                code_dir, "construct_data", f"root_cause_{ns}.json")) as f:
            root_cause_map = json.load(f)

        for date in DATES[ns]:
            normal_win = f"{date} {NORMAL_WINDOWS[date]}"
            normal_dir = os.path.join(RUNROOT, ns, "construct",
                                      normal_win.replace(" ", "_").replace(":", ""))
            with open(os.path.join(
                    code_dir, "rca_data", date, f"{date}-fault_list.json")) as f:
                fault_data = json.load(f)
            t0 = time.time()
            try:
                import_and_fold(ns, normal_win, "construct", normal_dir)
            except Exception as exc:  # noqa: BLE001
                # Frozen §8 (Codex round-12 P2, D-024): the construct
                # window is shared by every fault of its date, so its
                # failure crashes them all — each is counted
                # unlocalized with the shared cause instead of the
                # namespace aborting before any record is written.
                wall_ms = int((time.time() - t0) * 1000)
                faults = [f_ for hour in fault_data
                          for f_ in fault_data[hour]]
                for idx, fault in enumerate(faults):
                    case_id = f"{ns}-{date}-{idx:03d}"
                    cases.append({
                        "case_id": case_id, "dataset": date,
                        "inject_time": fault["inject_time"],
                        "inject_pod": fault["inject_pod"],
                        "inject_type": fault["inject_type"],
                        "ground_truth": None,
                        "abnormal_window":
                            abnormal_window(fault["inject_time"]),
                        "failure": f"construct window {normal_win}: "
                                   f"{type(exc).__name__}: {exc}",
                        "candidates": [],
                        "evaluation": {"rank_inner": None,
                                       "rank_service_raw": None,
                                       "rank_service_dedup": None,
                                       "n_candidates": 0},
                        "runtime_ms": wall_ms,
                    })
                    print(f"{case_id} FAILED (counted unlocalized): "
                          f"construct window failed: {exc}", flush=True)
                continue
            idx = 0
            for hour in fault_data:
                for fault in fault_data[hour]:
                    case_id = f"{ns}-{date}-{idx:03d}"
                    idx += 1
                    win = abnormal_window(fault["inject_time"])
                    abn_dir = os.path.join(RUNROOT, ns, "rca",
                                           win.replace(" ", "_").replace(":", ""))
                    t0 = time.time()
                    try:
                        import_and_fold(ns, win, "rca", abn_dir)
                        case_out = os.path.join(abn_dir, f"s1-{case_id}.json")
                        run([PY, SCORER,
                             "--normal-scenarios",
                             os.path.join(normal_dir, "scenarios.jsonl"),
                             "--normal-events",
                             os.path.join(normal_dir, "events.jsonl"),
                             "--abnormal-scenarios",
                             os.path.join(abn_dir, "scenarios.jsonl"),
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
                                           fault["inject_pod"], templates[ns])
                        imp_report = json.load(open(
                            os.path.join(abn_dir, "import-report.json")))
                        rec = {
                            "case_id": case_id, "dataset": date,
                            "inject_time": fault["inject_time"],
                            "inject_pod": fault["inject_pod"],
                            "inject_type": fault["inject_type"],
                            "ground_truth": rc,
                            "abnormal_window": win,
                            "representation": "semantixtrace-v2-canonical",
                            "algorithm": scored["algorithm"],
                            "parameters": scored["parameters"],
                            "ingestion": {
                                "counters": imp_report["counters"],
                                "rejections": imp_report["rejections"],
                                "drain3_new_clusters":
                                    imp_report["drain3_new_clusters"],
                            },
                            "alarm_list": imp_report.get("alarm_list", []),
                            "scoring": {k: scored[k] for k in (
                                "normal_sessions", "abnormal_sessions",
                                "normal_patterns", "abnormal_patterns",
                                "unattributed_patterns")},
                            "candidates": scored["candidates"],
                            "evaluation": ev,
                            "runtime_ms": wall_ms,
                        }
                        cases.append(rec)
                        print(f"{case_id} {fault['inject_type']:>14s} "
                              f"inner={ev['rank_inner']} "
                              f"svc_dedup={ev['rank_service_dedup']} "
                              f"ncand={ev['n_candidates']}", flush=True)
                    except Exception as exc:  # noqa: BLE001
                        # Frozen §8: a fault whose pipeline run crashes
                        # is counted as unlocalized AND separately
                        # reported as a failure with its cause — the
                        # driver must not abort the namespace (Codex
                        # round-11 P2, D-022).
                        cases.append({
                            "case_id": case_id, "dataset": date,
                            "inject_time": fault["inject_time"],
                            "inject_pod": fault["inject_pod"],
                            "inject_type": fault["inject_type"],
                            "ground_truth": None,
                            "abnormal_window": win,
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
        with open(os.path.join(outdir, f"s1-{ns}.cases.json"), "w") as f:
            json.dump({"summary": summary, "cases": cases}, f, indent=1)
        print(json.dumps(summary, indent=1), flush=True)


if __name__ == "__main__":
    main()
