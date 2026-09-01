#!/usr/bin/env python3
"""Preregistered paired statistical analyses (00-preregistration.md §9;
PR #20 Codex round-11 P1, D-021).

Frozen specification, implemented verbatim:
  - paired per-case comparison between conditions on the same dataset;
  - McNemar exact test on @1 hits;
  - bootstrap (10,000 resamples, seed 20260820) 95% CI for MRR
    differences;
  - no cross-dataset pooling; alpha = 0.05; effect sizes reported
    regardless of significance.

Comparisons: N1(corrected) vs S1 (H1) and S1 vs S2 (H2), per dataset.
Primary metric per §5 is service_dedup; service_raw and inner are
reported as supplementary. The N1 evaluator's corrected inner-list
rank field is `rank_dense`; it pairs with the S-side `rank_inner`.

The bootstrap RNG is Python's random.Random(20260820) (Mersenne
Twister), resampling case indices with replacement; the seed and
resample count are the frozen values. MRR uses the frozen §5/§8
semantics: reciprocal rank, unlocalized contributes 0, denominator n.

Usage: paired_stats.py <n1-eval-dir> <s1-dir> <s2-dir> <out.json>
  n1-eval-dir: contains {ns}-service.eval.json
  s1-dir/s2-dir: contain s1-/{s2-}{ns}.cases.json
"""
import json
import math
import os
import random
import sys

ALPHA = 0.05
SEED = 20260820
RESAMPLES = 10_000


def mcnemar_exact(b, c):
    """Two-sided exact McNemar p-value: binomial(b+c, 0.5) on the
    discordant pairs. p = min(1, 2 * P(X <= min(b, c)))."""
    n = b + c
    if n == 0:
        return 1.0
    k = min(b, c)
    tail = sum(math.comb(n, i) for i in range(k + 1)) * (0.5 ** n)
    return min(1.0, 2.0 * tail)


def bootstrap_ci(diffs):
    rng = random.Random(SEED)
    n = len(diffs)
    means = []
    for _ in range(RESAMPLES):
        s = 0.0
        for _ in range(n):
            s += diffs[rng.randrange(n)]
        means.append(s / n)
    means.sort()
    lo = means[int(0.025 * RESAMPLES)]
    hi = means[int(0.975 * RESAMPLES) - 1]
    return lo, hi


def rr(rank):
    return 0.0 if rank is None else 1.0 / rank


def compare(cases_a, cases_b, field_a, field_b, label_a, label_b):
    ids = sorted(cases_a)
    assert set(ids) == set(cases_b)
    hits_a = [cases_a[i][field_a] == 1 for i in ids]
    hits_b = [cases_b[i][field_b] == 1 for i in ids]
    b = sum(1 for x, y in zip(hits_a, hits_b) if x and not y)
    c = sum(1 for x, y in zip(hits_a, hits_b) if not x and y)
    diffs = [rr(cases_a[i][field_a]) - rr(cases_b[i][field_b]) for i in ids]
    n = len(ids)
    mrr_a = sum(rr(cases_a[i][field_a]) for i in ids) / n
    mrr_b = sum(rr(cases_b[i][field_b]) for i in ids) / n
    lo, hi = bootstrap_ci(diffs)
    p = mcnemar_exact(b, c)
    return {
        "n_cases": n,
        "ac1_" + label_a: sum(hits_a), "ac1_" + label_b: sum(hits_b),
        "ac1_diff_pp": round(100.0 * (sum(hits_a) - sum(hits_b)) / n, 4),
        "mcnemar_discordant": {label_a + "_only": b, label_b + "_only": c},
        "mcnemar_exact_p": p,
        "mcnemar_significant_at_0.05": p < ALPHA,
        "mrr_" + label_a: mrr_a, "mrr_" + label_b: mrr_b,
        "mrr_diff": mrr_a - mrr_b,
        "mrr_diff_bootstrap_95ci": [lo, hi],
        "ci_excludes_zero": (lo > 0) or (hi < 0),
    }


def main():
    n1_dir, s1_dir, s2_dir, out_path = sys.argv[1:5]
    out = {"frozen_spec": "00-preregistration.md §9: McNemar exact on @1 "
                          "hits; bootstrap 10,000 resamples seed 20260820 "
                          "95% CI for MRR differences; per-dataset, no "
                          "pooling; alpha 0.05; effect sizes regardless",
           "rng": "python random.Random(20260820), index resampling",
           "primary_metric": "service_dedup (§5)",
           "comparisons": {}}
    for ns in ("hipster", "ts"):
        # Pairing key: the fault itself (dataset, inject_time, pod,
        # type) — verified unique and identical across all condition
        # files; the N1 evaluator and the S drivers number their
        # case_ids differently (continuous vs per-date), so ids do not
        # align but faults do.
        def key(c):
            return (c["dataset"], c["inject_time"],
                    c["inject_pod"], c["inject_type"])
        n1 = {key(c): c for c in json.load(open(os.path.join(
            n1_dir, f"{ns}-service.eval.json")))["cases"]}
        s1 = {key(c): c["evaluation"] for c in json.load(open(
            os.path.join(s1_dir, f"s1-{ns}.cases.json")))["cases"]}
        s2 = {key(c): c["evaluation"] for c in json.load(open(
            os.path.join(s2_dir, f"s2-{ns}.cases.json")))["cases"]}
        assert len(n1) == len(s1) == len(s2)
        block = {}
        for metric, f_n1, f_s in (
                ("service_dedup", "rank_service_dedup", "rank_service_dedup"),
                ("service_raw", "rank_service_raw", "rank_service_raw"),
                ("inner", "rank_dense", "rank_inner")):
            block[f"N1_vs_S1.{metric}"] = compare(
                n1, s1, f_n1, f_s, "n1", "s1")
        for metric, f in (("service_dedup", "rank_service_dedup"),
                          ("service_raw", "rank_service_raw"),
                          ("inner", "rank_inner")):
            block[f"S1_vs_S2.{metric}"] = compare(
                s1, s2, f, f, "s1", "s2")
        out["comparisons"][ns] = block
    with open(out_path, "w") as f:
        json.dump(out, f, indent=1)
    for ns, block in out["comparisons"].items():
        for name, r in block.items():
            print(f"{ns:8s} {name:24s} p={r['mcnemar_exact_p']:.3g} "
                  f"sig={r['mcnemar_significant_at_0.05']} "
                  f"mrr_diff={r['mrr_diff']:+.4f} "
                  f"ci=[{r['mrr_diff_bootstrap_95ci'][0]:+.4f},"
                  f"{r['mrr_diff_bootstrap_95ci'][1]:+.4f}] "
                  f"ci_excl0={r['ci_excludes_zero']}")


if __name__ == "__main__":
    main()
