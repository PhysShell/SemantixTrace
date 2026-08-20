#!/usr/bin/env python3
"""Corrected evaluator for S1 case outputs (E2).

Applies the preregistered semantics (00-preregistration.md §5) to the
candidate lists produced by s1_scorer.py:

- dense competition ranks over the (score, deepth)-ordered list;
- service-level: candidate service == injected service, raw and
  deduplicated-per-service variants;
- inner-service: resource ground truth `R` matches a candidate carrying
  a resource annotation containing R with the injected pod; code-region
  ground truth `A_B` matches a candidate whose pattern source text
  contains A and target text contains B, with the injected pod.
  Action text: template text for log:N ids (shipped drain3 dump),
  the command string for span:*, the metric name for alert:*.

Library + CLI (imported by run_e2.py).
"""
import json


def service_of(pod):
    return pod.rsplit("-", 1)[0].rsplit("-", 1)[0]


def action_text(action, templates):
    cmd = action[1] if isinstance(action, list) else action["command_id"]
    if cmd.startswith("log:"):
        cid = cmd[len("log:"):]
        return templates.get(cid, {}).get("template", "")
    if cmd.startswith("alert:"):
        return cmd[len("alert:"):]
    return cmd


def dense_ranks(cands):
    """Dense competition ranks over the candidates' ordering keys.
    S1 candidates key on (score, deepth); S2 candidates additionally
    carry "anomaly" (frozen tie-break), absent for S1 (None for all)."""
    ranks, rank, prev = [], 0, None
    for c in cands:
        key = (c.get("score"), c.get("anomaly"), c.get("deepth"))
        if key != prev:
            rank += 1
            prev = key
        ranks.append(rank)
    return ranks


def match_inner(cand, rc_parts, inject_pod, templates):
    if len(rc_parts) == 1:
        return ("resource" in cand
                and str(rc_parts[0]) in str(cand["resource"])
                and str(inject_pod) in str(cand["pod"]))
    if len(rc_parts) == 2:
        src_t = action_text(cand["pattern"][0], templates)
        dst_t = action_text(cand["pattern"][1], templates)
        return (rc_parts[0] in src_t and rc_parts[1] in dst_t
                and str(inject_pod) in str(cand["pod"]))
    return False


def evaluate_case(cands, rc_parts, inject_pod, templates):
    ranks = dense_ranks(cands)
    out = {"n_candidates": len(cands),
           "rank_inner": None, "rank_service_raw": None,
           "rank_service_dedup": None}
    for i, c in enumerate(cands):
        if match_inner(c, rc_parts, inject_pod, templates):
            out["rank_inner"] = ranks[i]
            break
    inject_svc = service_of(inject_pod)
    for i, c in enumerate(cands):
        if service_of(str(c.get("pod", ""))) == inject_svc:
            out["rank_service_raw"] = ranks[i]
            break
    # Preregistered primary semantics (00-preregistration.md §5):
    # deduplicate to the first occurrence per service, THEN dense-rank
    # the deduplicated list — tied representatives share a rank.
    seen = set()
    dedup = []
    for c in cands:
        svc = service_of(str(c.get("pod", "")))
        if svc in seen:
            continue
        seen.add(svc)
        dedup.append((svc, c))
    dedup_ranks = dense_ranks([c for _svc, c in dedup])
    for i, (svc, _c) in enumerate(dedup):
        if svc == inject_svc:
            out["rank_service_dedup"] = dedup_ranks[i]
            break
    return out


if __name__ == "__main__":
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--case", required=True)
    ap.add_argument("--templates", required=True)
    ap.add_argument("--inject-pod", required=True)
    ap.add_argument("--root-cause", required=True)
    args = ap.parse_args()
    templates = json.load(open(args.templates))["clusters"]
    cands = json.load(open(args.case))["candidates"]
    print(json.dumps(evaluate_case(
        cands, args.root_cause.split("_"), args.inject_pod, templates)))
