#!/usr/bin/env python3
"""Re-evaluate stored E2/E3 case records with the corrected (GREEN)
service_dedup semantics.

Candidates are NOT re-scored — they are read from the committed per-case
evidence records; only the evaluation block and summary aggregates are
recomputed. A hard check asserts that no field other than
rank_service_dedup changes (any other movement would indicate an
unintended semantic drift and aborts the regeneration).

Emits a machine-readable per-case delta record for D-008.
"""
import json
import os
import sys

RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")
CODE_DIRS = {"hipster": "/home/user/e0-runs/checkout-hipster",
             "ts": "/home/user/e0-runs/checkout-ts"}

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "evaluators"))
from s1_eval import evaluate_case, service_of  # noqa: E402


def rebuild_summary(ns, cases):
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
    return {"ns": ns, "n_cases": n, "aggregates": agg,
            "candidate_sizes": {"min": sizes[0],
                                "median": sizes[len(sizes) // 2],
                                "max": sizes[-1]}}


def main():
    deltas = {}
    for cond in ("s1", "s2"):
        for ns in ("hipster", "ts"):
            path = os.path.join(RUNROOT, "results", f"{cond}-{ns}.cases.json")
            data = json.load(open(path))
            templates = json.load(
                open(os.path.join(RUNROOT, f"templates-{ns}.json")))["clusters"]
            with open(os.path.join(CODE_DIRS[ns], "construct_data",
                                   f"root_cause_{ns}.json")) as f:
                root_cause_map = json.load(f)
            cond_deltas = []
            for case in data["cases"]:
                svc = service_of(case["inject_pod"])
                try:
                    rc_parts = root_cause_map[svc][case["inject_type"]].split("_")
                except KeyError:
                    rc_parts = []
                old = case["evaluation"]
                new = evaluate_case(case["candidates"], rc_parts,
                                    case["inject_pod"], templates)
                for key in old:
                    if key == "rank_service_dedup":
                        continue
                    if old[key] != new[key]:
                        print(f"ABORT: {cond}-{ns} {case['case_id']}: "
                              f"unexpected change in {key}: "
                              f"{old[key]} -> {new[key]}", file=sys.stderr)
                        sys.exit(1)
                if old["rank_service_dedup"] != new["rank_service_dedup"]:
                    cond_deltas.append({
                        "case_id": case["case_id"],
                        "inject_type": case["inject_type"],
                        "old": old["rank_service_dedup"],
                        "new": new["rank_service_dedup"]})
                case["evaluation"] = new
            old_summary = data["summary"]["aggregates"]["rank_service_dedup"]
            data["summary"] = rebuild_summary(ns, data["cases"])
            new_summary = data["summary"]["aggregates"]["rank_service_dedup"]
            with open(path, "w") as f:
                json.dump(data, f, indent=1)
            deltas[f"{cond}-{ns}"] = {
                "changed_cases": cond_deltas,
                "aggregate_old": old_summary,
                "aggregate_new": new_summary,
            }
            print(f"{cond}-{ns}: {len(cond_deltas)} case(s) changed; "
                  f"svc_dedup AC@1 {old_summary['AC@1_pct']:.2f} -> "
                  f"{new_summary['AC@1_pct']:.2f}, "
                  f"MRR {old_summary['MRR']:.3f} -> {new_summary['MRR']:.3f}")
    out = os.path.join(RUNROOT, "results", "regate-dedup-deltas.json")
    with open(out, "w") as f:
        json.dump(deltas, f, indent=1)
    print(f"deltas -> {out}")


if __name__ == "__main__":
    main()
