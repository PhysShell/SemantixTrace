#!/usr/bin/env python3
"""Semantics tests for the preregistered primary metric implementation.

00-preregistration.md §5 (frozen): "ranks are dense competition ranks
(candidates with equal ranking keys ... share one rank; the next
distinct key takes rank+1); the candidate list is deduplicated to the
first occurrence per service BEFORE ranking".

Therefore: dedup first, then dense ranks over the deduplicated list;
tied dedup-representatives share a rank. A positional `len(seen)` rank
violates the frozen definition whenever the top of the dedup list
contains ties.

Plain-assert script: exits non-zero on the first violation.
"""
import sys

sys.path.insert(0, __file__.rsplit("/", 1)[0])
from independent_eval import rank_service  # noqa: E402
import s1_eval  # noqa: E402


def check(name, got, want):
    status = "ok" if got == want else "FAIL"
    print(f"{status:4s} {name}: got {got}, preregistered {want}")
    return got == want


ok = True

# --- independent_eval.rank_service (N1 evaluator) -----------------------
# Two services tied on (score, deepth); injected service is the SECOND
# candidate. Dedup list = [A, B]; dense ranks = [1, 1]; expected rank 1.
cands = [
    {"pod": "aaa-5f6585d649-x1", "score": 1.0, "deepth": 3},
    {"pod": "bbb-5f6585d649-x1", "score": 1.0, "deepth": 3},
]
_, dedup = rank_service(cands, "bbb-5f6585d649-x1")
ok &= check("N1 dedup tie shares rank", dedup, 1)

# Duplicate of a leading service must not consume a rank position, and
# the tie between the two remaining representatives must be shared.
cands = [
    {"pod": "aaa-5f6585d649-x1", "score": 1.0, "deepth": 3},
    {"pod": "aaa-5f6585d649-x1", "score": 1.0, "deepth": 3},
    {"pod": "bbb-5f6585d649-x1", "score": 1.0, "deepth": 3},
    {"pod": "ccc-5f6585d649-x1", "score": 0.8, "deepth": 3},
]
_, dedup = rank_service(cands, "ccc-5f6585d649-x1")
ok &= check("N1 dedup-then-rank (distinct key after tie)", dedup, 2)

# Non-tied case: unchanged by the fix (regression guard).
cands = [
    {"pod": "aaa-5f6585d649-x1", "score": 1.0, "deepth": 3},
    {"pod": "bbb-5f6585d649-x1", "score": 0.9, "deepth": 3},
]
_, dedup = rank_service(cands, "bbb-5f6585d649-x1")
ok &= check("N1 dedup non-tie", dedup, 2)

# --- s1_eval.evaluate_case (S1/S2 evaluator) ----------------------------
# Ranking key there is (score, anomaly, deepth).
cands = [
    {"pod": "aaa-5f6585d649-x1", "score": 1.0, "deepth": 3,
     "pattern": [["<u>", "span:a x", {}], ["<u>", "span:a y", {}]]},
    {"pod": "bbb-5f6585d649-x1", "score": 1.0, "deepth": 3,
     "pattern": [["<u>", "span:b x", {}], ["<u>", "span:b y", {}]]},
]
ev = s1_eval.evaluate_case(cands, [], "bbb-5f6585d649-x1", {})
ok &= check("S1 dedup tie shares rank", ev["rank_service_dedup"], 1)

cands = [
    {"pod": "aaa-5f6585d649-x1", "score": 1.0, "deepth": 3, "pattern": []},
    {"pod": "aaa-5f6585d649-x1", "score": 0.9, "deepth": 3, "pattern": []},
    {"pod": "bbb-5f6585d649-x1", "score": 0.9, "deepth": 3, "pattern": []},
]
ev = s1_eval.evaluate_case(cands, [], "bbb-5f6585d649-x1", {})
ok &= check("S1 duplicate service does not consume a rank",
            ev["rank_service_dedup"], 2)

sys.exit(0 if ok else 1)
