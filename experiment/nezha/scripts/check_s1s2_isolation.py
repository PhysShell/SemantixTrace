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

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run_e2 import DATES, CODE_DIRS  # noqa: E402

MANIFEST = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                        "..", "manifests", "expected-cases.json")


def expected_case_ids():
    """The COMMITTED expected-case manifest (D-025): the case universe
    is certified against an independent, reviewable source — never
    against the result files being checked (Codex round-12 P2: a case
    deleted from BOTH files vanished from the two-file union and the
    gate exited 0 over 100 comparisons). The manifest is cross-checked
    against a fresh derivation from the pinned fault lists; a mismatch
    is a hard error, so neither can drift silently."""
    m = json.load(open(MANIFEST))
    manifest_ids = {ns: set(ids) for ns, ids in m["cases"].items()}
    derived = {}
    for ns in ("hipster", "ts"):
        ids = set()
        for date in DATES[ns]:
            fault_data = json.load(open(os.path.join(
                CODE_DIRS[ns], "rca_data", date,
                f"{date}-fault_list.json")))
            idx = 0
            for hour in fault_data:
                for _ in fault_data[hour]:
                    ids.add(f"{ns}-{date}-{idx:03d}")
                    idx += 1
        derived[ns] = ids
    if manifest_ids != derived:
        diff = {ns: sorted(manifest_ids.get(ns, set())
                           ^ derived.get(ns, set()))
                for ns in set(manifest_ids) | set(derived)}
        raise SystemExit(f"expected-case manifest disagrees with the "
                         f"fault-list derivation: {diff}")
    return manifest_ids


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

    expected = expected_case_ids()
    violations = []
    n_cases = 0
    for ns in ("hipster", "ts"):
        s1 = json.load(open(os.path.join(base, "e2",
                                         f"s1-{ns}.cases.json")))
        s2 = json.load(open(os.path.join(base, "e3",
                                         f"s2-{ns}.cases.json")))
        s1_by_id = {c["case_id"]: c for c in s1["cases"]}
        s2_by_id = {c["case_id"]: c for c in s2["cases"]}
        # Universe = committed expected manifest ∪ both id sets: a case
        # absent from either side is a violation (one-sided truncation,
        # Codex round-7 P2, D-016), and so is a case absent from BOTH —
        # the two-file union alone certified a bilaterally truncated
        # experiment (Codex round-12 P2, D-025); a foreign id not in
        # the manifest is flagged too.
        for case_id in sorted(expected[ns] | set(s1_by_id)
                              | set(s2_by_id)):
            n_cases += 1
            c1 = s1_by_id.get(case_id)
            c2 = s2_by_id.get(case_id)
            if c1 is None or c2 is None or case_id not in expected[ns]:
                violations.append({
                    "case_id": case_id,
                    "missing_from": [side for side, present in
                                     (("s1", c1 is not None),
                                      ("s2", c2 is not None),
                                      ("expected-manifest",
                                       case_id in expected[ns]))
                                     if not present],
                })
                continue
            m1 = sorted(shared_candidate_semantics(c)
                        for c in c1["candidates"])
            m2 = sorted(shared_candidate_semantics(c)
                        for c in c2["candidates"])
            if m1 != m2:
                only1 = [c for c in m1 if c not in m2]
                only2 = [c for c in m2 if c not in m1]
                violations.append({
                    "case_id": case_id,
                    "n_candidates_s1": len(c1["candidates"]),
                    "n_candidates_s2": len(c2["candidates"]),
                    "s1_only_candidates": [json.loads(c) for c in only1],
                    "s2_only_candidates": [json.loads(c) for c in only2],
                    "s1_ranks": ranks(c1["evaluation"]),
                    "s2_ranks": ranks(c2["evaluation"]),
                })

    summary = {"cases": n_cases,
               "expected_cases": sum(len(v) for v in expected.values()),
               "isolation_violations": violations}
    if out_path:
        with open(out_path, "w") as f:
            json.dump(summary, f, indent=1)
    print(f"cases={n_cases} isolation_violations={len(violations)}"
          + ("" if not violations else " -> "
             + ", ".join(v["case_id"] for v in violations)))
    sys.exit(0 if not violations else 1)


if __name__ == "__main__":
    main()
