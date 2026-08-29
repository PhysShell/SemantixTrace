#!/usr/bin/env python3
"""S1<->S2 isolation gate (PR #20 Codex round-6 P1, D-015).

Machine formulation of the frozen E3 ablation contract: S2's only
candidate-level delta vs S1 is the `anomaly` ranking input. For every
case, the checked-in S1 (results/e2) and S2 (results/e3) candidate
lists must be identical as multisets over ALL fields — pattern, score,
deepth, pod, resource, provenance — once S2's `anomaly` field is
removed, and candidate counts must match. Any violation means the
alarm dedup (or anything else upstream of ranking) retained different
candidates, i.e. the S1<->S2 comparison is no longer attributable to
the anomaly tie-break alone.

If a document ever again claims "the only delta is the tie-break"
while the artifacts disagree, this gate exits 1.

Usage: check_s1s2_isolation.py [results-dir] [out.json]
"""
import json
import os
import sys


def shared_candidate_semantics(cand):
    """Every field except S2's anomaly ranking input."""
    return json.dumps({k: v for k, v in cand.items() if k != "anomaly"},
                      sort_keys=True)


def ranks(ev):
    return [ev["rank_inner"], ev["rank_service_raw"],
            ev["rank_service_dedup"]]


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(__file__), "..", "results")
    out_path = sys.argv[2] if len(sys.argv) > 2 else None

    violations = []
    n_cases = 0
    for ns in ("hipster", "ts"):
        s1 = json.load(open(os.path.join(base, "e2",
                                         f"s1-{ns}.cases.json")))
        s2 = json.load(open(os.path.join(base, "e3",
                                         f"s2-{ns}.cases.json")))
        s1_by_id = {c["case_id"]: c for c in s1["cases"]}
        for c2 in s2["cases"]:
            n_cases += 1
            c1 = s1_by_id.get(c2["case_id"])
            m1 = (sorted(shared_candidate_semantics(c)
                         for c in c1["candidates"])
                  if c1 is not None else None)
            m2 = sorted(shared_candidate_semantics(c)
                        for c in c2["candidates"])
            if m1 != m2:
                only1 = [] if m1 is None else [c for c in m1 if c not in m2]
                only2 = m2 if m1 is None else [c for c in m2 if c not in m1]
                violations.append({
                    "case_id": c2["case_id"],
                    "n_candidates_s1": None if c1 is None
                    else len(c1["candidates"]),
                    "n_candidates_s2": len(c2["candidates"]),
                    "s1_only_candidates": [json.loads(c) for c in only1],
                    "s2_only_candidates": [json.loads(c) for c in only2],
                    "s1_ranks": None if c1 is None
                    else ranks(c1["evaluation"]),
                    "s2_ranks": ranks(c2["evaluation"]),
                })

    summary = {"cases": n_cases, "isolation_violations": violations}
    if out_path:
        with open(out_path, "w") as f:
            json.dump(summary, f, indent=1)
    print(f"cases={n_cases} isolation_violations={len(violations)}"
          + ("" if not violations else " -> "
             + ", ".join(v["case_id"] for v in violations)))
    sys.exit(0 if not violations else 1)


if __name__ == "__main__":
    main()
