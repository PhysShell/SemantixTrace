#!/usr/bin/env python3
"""E3 modality ablations on the S1 condition (preregistered list:
"no metric events; no logs; no traces"), TrainTicket only — the dataset
where S1 retains measurable signal. Each variant filters the imported
events by command_id namespace, re-folds through st-fold, re-scores with
the unchanged S1 scorer, and evaluates with the preregistered semantics.
"""
import json
import os
import subprocess
import sys
import tempfile

ADAPTERS = os.path.join(os.path.dirname(__file__), "..", "adapters")
STFOLD = os.path.join(ADAPTERS, "st-fold", "target", "release", "st-fold")
SCORER = os.path.join(ADAPTERS, "s1_scorer.py")
PY = "/home/user/.venv-nezha/bin/python"
RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "evaluators"))
from s1_eval import evaluate_case, service_of  # noqa: E402
sys.path.insert(0, os.path.dirname(__file__))
from run_e2 import DATES, NORMAL_WINDOWS, CODE_DIRS, abnormal_window, run  # noqa: E402

VARIANTS = {
    "no-alerts": lambda cmd: not cmd.startswith("alert:"),
    "no-logs": lambda cmd: not cmd.startswith("log:"),
    "no-spans": lambda cmd: not cmd.startswith("span:"),
}


def filter_and_fold(window_dir, variant, keep):
    vdir = os.path.join(window_dir, f"ablation-{variant}")
    scen = os.path.join(vdir, "scenarios.jsonl")
    if os.path.exists(scen):
        return vdir
    os.makedirs(vdir, exist_ok=True)
    src = os.path.join(window_dir, "events.jsonl")
    dst = os.path.join(vdir, "events.jsonl")
    kept = dropped = 0
    with open(src) as fi, open(dst, "w") as fo:
        # seq must stay strictly monotonic per session after filtering;
        # renumber per session preserving order.
        per_session = {}
        for line in fi:
            ev = json.loads(line)
            if keep(ev["command_id"]):
                sid = ev["session_id"]
                ev["seq"] = per_session.get(sid, 0)
                per_session[sid] = ev["seq"] + 1
                fo.write(json.dumps(ev, separators=(",", ":")) + "\n")
                kept += 1
            else:
                dropped += 1
    with open(os.path.join(vdir, "filter-report.json"), "w") as f:
        json.dump({"variant": variant, "kept": kept, "dropped": dropped}, f)
    run([STFOLD, dst, scen])
    return vdir


def main():
    ns = "ts"
    templates = json.load(
        open(os.path.join(RUNROOT, f"templates-{ns}.json")))["clusters"]
    code_dir = CODE_DIRS[ns]
    with open(os.path.join(code_dir, "construct_data",
                           f"root_cause_{ns}.json")) as f:
        root_cause_map = json.load(f)

    results = {}
    for variant, keep in VARIANTS.items():
        cases = []
        for date in DATES[ns]:
            normal_win = f"{date} {NORMAL_WINDOWS[date]}"
            normal_dir = os.path.join(
                RUNROOT, ns, "construct",
                normal_win.replace(" ", "_").replace(":", ""))
            with open(os.path.join(code_dir, "rca_data", date,
                                   f"{date}-fault_list.json")) as f:
                fault_data = json.load(f)
            try:
                v_normal = filter_and_fold(normal_dir, variant, keep)
            except Exception as exc:  # noqa: BLE001
                # Frozen §8 (Codex round-13 P2, D-028): the normal fold
                # is shared by every fault of its date — count each as
                # unlocalized with the shared cause and keep going.
                faults = [f_ for hour in fault_data
                          for f_ in fault_data[hour]]
                for idx, fault in enumerate(faults):
                    case_id = f"{ns}-{date}-{idx:03d}"
                    cases.append({
                        "case_id": case_id,
                        "inject_type": fault["inject_type"],
                        "failure": f"normal ablation fold {variant} "
                                   f"{normal_win}: "
                                   f"{type(exc).__name__}: {exc}",
                        "evaluation": {"rank_inner": None,
                                       "rank_service_raw": None,
                                       "rank_service_dedup": None,
                                       "n_candidates": 0}})
                    print(f"{case_id} FAILED (counted unlocalized): "
                          f"{exc}", flush=True)
                continue
            idx = 0
            for hour in fault_data:
                for fault in fault_data[hour]:
                    case_id = f"{ns}-{date}-{idx:03d}"
                    idx += 1
                    win = abnormal_window(fault["inject_time"])
                    abn_dir = os.path.join(
                        RUNROOT, ns, "rca",
                        win.replace(" ", "_").replace(":", ""))
                    try:
                        v_abn = filter_and_fold(abn_dir, variant, keep)
                        case_out = os.path.join(v_abn, f"s1-{case_id}.json")
                        run([PY, SCORER,
                             "--normal-scenarios",
                             os.path.join(v_normal, "scenarios.jsonl"),
                             "--normal-events",
                             os.path.join(v_normal, "events.jsonl"),
                             "--abnormal-scenarios",
                             os.path.join(v_abn, "scenarios.jsonl"),
                             "--alarms",
                             os.path.join(abn_dir, "import-report.json"),
                             "--out", case_out])
                        scored = json.load(open(case_out))
                        svc = service_of(fault["inject_pod"])
                        try:
                            rc = root_cause_map[svc][fault["inject_type"]]
                            rc_parts = rc.split("_")
                        except KeyError:
                            rc, rc_parts = None, []
                        ev = evaluate_case(scored["candidates"], rc_parts,
                                           fault["inject_pod"], templates)
                        cases.append({"case_id": case_id,
                                      "inject_type": fault["inject_type"],
                                      "evaluation": ev})
                    except Exception as exc:  # noqa: BLE001
                        # Frozen §8: crashed case counted unlocalized,
                        # cause reported (Codex round-13 P2, D-028).
                        cases.append({
                            "case_id": case_id,
                            "inject_type": fault["inject_type"],
                            "failure": f"{type(exc).__name__}: {exc}",
                            "evaluation": {"rank_inner": None,
                                           "rank_service_raw": None,
                                           "rank_service_dedup": None,
                                           "n_candidates": 0}})
                        print(f"{case_id} FAILED (counted unlocalized): "
                              f"{exc}", flush=True)
        n = len(cases)
        agg = {}
        for mode in ("rank_inner", "rank_service_dedup"):
            ranks = [c["evaluation"][mode] for c in cases]
            hit = [r for r in ranks if r is not None]
            agg[mode] = {
                "AC@1_pct": 100.0 * sum(1 for r in hit if r <= 1) / n,
                "AC@3_pct": 100.0 * sum(1 for r in hit if r <= 3) / n,
                "MRR": sum(1.0 / r for r in hit) / n,
                "unlocalized": n - len(hit),
            }
        results[variant] = {"n": n, "aggregates": agg, "cases": cases}
        failed = [{"case_id": c["case_id"], "failure": c["failure"]}
                  for c in cases if "failure" in c]
        if failed:  # §8: failures are separately reported with cause
            results[variant]["failed_cases"] = failed
        print(variant, json.dumps(agg), flush=True)

    outdir = os.path.join(RUNROOT, "results")
    with open(os.path.join(outdir, f"s1-ablations-{ns}.json"), "w") as f:
        json.dump(results, f, indent=1)


if __name__ == "__main__":
    main()
