#!/usr/bin/env python3
"""Independent first-principles evaluator for Nezha E0/E1.

Consumes ONLY:
  - the artifact's run log (which contains, per fault case, the full ranked
    candidate list logged as "Soted Result List: [...]"),
  - the frozen ground-truth files shipped with the dataset
    (rca_data/<date>/<date>-fault_list.json, construct_data/root_cause_<ns>.json),
  - the drain3 template dump produced after the run (templates.json).

It never imports Nezha code. Rank metrics are recomputed from scratch under
several explicitly-named semantics so the artifact's own arithmetic can be
cross-checked (E0) and measurement defects quantified (E1):

  historical : bug-for-bug replica of pattern_ranker.evaluation's topk loop
               (resource candidates that do not match never advance the rank
               counter; adjacent (score,deepth) ties do not advance it either;
               the counter also advances when *leaving* candidate 0).
  dense      : every candidate occupies a rank; candidates with equal
               (score, deepth) share a rank (standard competition ranking on
               the already-sorted list). Match rule identical to historical.
  service    : true service-level semantics — a candidate is correct iff the
               service derived from its pod equals the service derived from
               the injected pod (resource/template content ignored). Ranks are
               computed both over raw candidates (service_raw) and over the
               list deduplicated to first occurrence per service
               (service_dedup), with dense tie handling.

Every parse failure and every skipped line class is counted and reported;
nothing is silently dropped.
"""
import argparse
import ast
import json
import re
import sys
from collections import OrderedDict, Counter

SORTED_RE = re.compile(r"Soted Result List: (\[.*\])\s*$")
CASE_RE = re.compile(
    r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}(?::\d{2})?) Inject RCA (?:Pod )?Result:")
GT_RE = re.compile(
    r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}(?::\d{2})?) Inject Ground Truth: (\S+), (\S+?)(?: score (\d+))?\s*$")
FINAL_RE = re.compile(r"--------(A[IS]*S?@\d) Result-------")
PCT_RE = re.compile(r"^\[INFO\]\S+ \S+ pattern_ranker.py:\d+: ([\d.]+) %")
FAULTNUM_RE = re.compile(r"Fault numbuer : (\d+)-")


def service_of(pod):
    s = pod.rsplit("-", 1)[0]
    return s.rsplit("-", 1)[0]


def parse_artifact_log(path):
    """Split the artifact log into per-case records, counting everything."""
    counters = Counter()
    cases = []
    last_sorted = None
    pending = None  # case dict being filled
    artifact_final = {"percent_lines": [], "fault_number": None}
    with open(path, errors="replace") as f:
        for line in f:
            counters["lines_total"] += 1
            m = SORTED_RE.search(line)
            if m:
                counters["sorted_result_lines"] += 1
                try:
                    last_sorted = ast.literal_eval(m.group(1))
                except (ValueError, SyntaxError):
                    counters["sorted_result_parse_failures"] += 1
                    last_sorted = None
                continue
            m = CASE_RE.search(line)
            if m:
                counters["case_header_lines"] += 1
                if pending is not None:
                    cases.append(pending)
                pending = {
                    "abnormal_query_time": m.group(1),
                    "candidates": last_sorted,
                    "candidates_missing": last_sorted is None,
                    "artifact_claimed_rank": None,
                    "inject_time_min": None,
                    "inject_pod": None,
                    "inject_type": None,
                }
                if last_sorted is None:
                    counters["cases_without_candidate_list"] += 1
                last_sorted = None
                continue
            m = GT_RE.search(line)
            if m and pending is not None:
                if m.group(4) is not None:
                    pending["artifact_claimed_rank"] = int(m.group(4))
                    counters["artifact_matched_lines"] += 1
                else:
                    pending["inject_time_min"] = m.group(1)
                    pending["inject_pod"] = m.group(2)
                    pending["inject_type"] = m.group(3)
                    counters["ground_truth_lines"] += 1
                continue
            m = FAULTNUM_RE.search(line)
            if m:
                artifact_final["fault_number"] = int(m.group(1))
            if FINAL_RE.search(line):
                artifact_final.setdefault("metric_labels", []).append(
                    FINAL_RE.search(line).group(1))
            m = PCT_RE.match(line)
            if m:
                artifact_final["percent_lines"].append(float(m.group(1)))
    if pending is not None:
        cases.append(pending)
    return cases, counters, artifact_final


def load_ground_truth(nezha_dir, ns):
    dates = {"hipster": ["2022-08-22", "2022-08-23"],
             "ts": ["2023-01-29", "2023-01-30"]}[ns]
    faults = []
    for d in dates:
        with open(f"{nezha_dir}/rca_data/{d}/{d}-fault_list.json") as f:
            data = json.load(f, object_pairs_hook=OrderedDict)
        for hour in data:
            for fault in data[hour]:
                fault = dict(fault)
                fault["dataset_date"] = d
                faults.append(fault)
    with open(f"{nezha_dir}/construct_data/root_cause_{ns}.json") as f:
        root_cause = json.load(f)
    return faults, root_cause


def match_candidate(cand, root_cause_parts, inject_pod, templates):
    """The artifact's per-candidate match rule, reimplemented."""
    if len(root_cause_parts) == 1:
        if "resource" not in cand:
            return False
        return (str(root_cause_parts[0]) in str(cand["resource"])
                and str(inject_pod) in str(cand["pod"]))
    if len(root_cause_parts) == 2:
        try:
            src_id, dst_id = cand["events"].split("_")
            src_t = templates.get(src_id, {}).get("template", "")
            dst_t = templates.get(dst_id, {}).get("template", "")
        except (KeyError, ValueError):
            return False
        return (root_cause_parts[0] in src_t and root_cause_parts[1] in dst_t
                and str(inject_pod) in str(cand["pod"]))
    return False


def rank_historical(cands, root_cause_parts, inject_pod, templates):
    """Bug-for-bug replica of the artifact's topk loop."""
    topk = 1
    for i, cand in enumerate(cands):
        if len(root_cause_parts) == 1:
            if "resource" in cand:
                if match_candidate(cand, root_cause_parts, inject_pod, templates):
                    return topk
                # artifact quirk: non-matching resource candidate does NOT
                # advance the rank counter
            else:
                if i > 0 and cands[i - 1]["score"] == cand["score"] \
                        and cands[i - 1]["deepth"] == cand["deepth"]:
                    pass
                else:
                    topk += 1
        elif len(root_cause_parts) == 2:
            if match_candidate(cand, root_cause_parts, inject_pod, templates):
                return topk
            if i > 0 and cands[i - 1]["score"] == cand["score"] \
                    and cands[i - 1]["deepth"] == cand["deepth"]:
                pass
            else:
                topk += 1
        else:
            return None
    return None


def dense_ranks(cands):
    """rank[i] under competition ranking by (score, deepth) group."""
    ranks = []
    rank = 0
    prev = None
    for cand in cands:
        key = (cand.get("score"), cand.get("deepth"))
        if key != prev:
            rank += 1
            prev = key
        ranks.append(rank)
    return ranks


def rank_dense(cands, root_cause_parts, inject_pod, templates):
    ranks = dense_ranks(cands)
    for i, cand in enumerate(cands):
        if match_candidate(cand, root_cause_parts, inject_pod, templates):
            return ranks[i]
    return None


def rank_service(cands, inject_pod):
    """(raw_rank, dedup_rank) where correct = candidate service == injected.

    dedup_rank implements the preregistered primary semantics
    (00-preregistration.md §5): deduplicate to the first occurrence per
    service, THEN dense-rank the deduplicated list — tied
    representatives share a rank.
    """
    inject_svc = service_of(inject_pod)
    ranks = dense_ranks(cands)
    raw = None
    for i, cand in enumerate(cands):
        if service_of(str(cand.get("pod", ""))) == inject_svc:
            raw = ranks[i]
            break
    seen = set()
    dedup = []
    for cand in cands:
        svc = service_of(str(cand.get("pod", "")))
        if svc in seen:
            continue
        seen.add(svc)
        dedup.append((svc, cand))
    dedup_ranks = dense_ranks([c for _svc, c in dedup])
    dedup_rank = None
    for i, (svc, _c) in enumerate(dedup):
        if svc == inject_svc:
            dedup_rank = dedup_ranks[i]
            break
    return raw, dedup_rank


def aggregate(ranks, n_cases):
    out = {}
    hit = [r for r in ranks if r is not None]
    for k in (1, 3, 5):
        out[f"top{k}"] = sum(1 for r in hit if r <= k)
        out[f"AC@{k}_pct"] = 100.0 * out[f"top{k}"] / n_cases if n_cases else 0.0
    out["MRR"] = sum(1.0 / r for r in hit) / n_cases if n_cases else 0.0
    out["localized"] = len(hit)
    out["unlocalized"] = n_cases - len(hit)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--artifact-log", required=True)
    ap.add_argument("--templates", required=True)
    ap.add_argument("--nezha-dir", required=True)
    ap.add_argument("--ns", required=True, choices=["hipster", "ts"])
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    with open(args.templates) as f:
        templates = json.load(f)["clusters"]

    cases, counters, artifact_final = parse_artifact_log(args.artifact_log)
    faults, root_cause = load_ground_truth(args.nezha_dir, args.ns)

    records = []
    alignment_errors = []
    for idx, fault in enumerate(faults):
        rec = {
            "case_id": f"{args.ns}-{fault['dataset_date']}-{idx:03d}",
            "dataset": fault["dataset_date"],
            "inject_time": fault["inject_time"],
            "inject_pod": fault["inject_pod"],
            "inject_type": fault["inject_type"],
        }
        svc = service_of(fault["inject_pod"])
        try:
            rc_str = root_cause[svc][fault["inject_type"]]
            rc_parts = rc_str.split("_")
        except KeyError as e:
            rec["ground_truth_error"] = f"missing root_cause entry: {e}"
            rc_str, rc_parts = None, []
        rec["root_cause"] = rc_str

        if idx < len(cases):
            case = cases[idx]
            # alignment check: log order must equal fault-list order
            if (case["inject_pod"] != fault["inject_pod"]
                    or case["inject_type"] != fault["inject_type"]):
                alignment_errors.append(
                    {"idx": idx, "log": (case["inject_pod"], case["inject_type"]),
                     "fault_list": (fault["inject_pod"], fault["inject_type"])})
                # A misaligned log case must NOT be scored against this
                # fault (Codex round-15 P2, D-036): the positional
                # candidates belong to a different case, so any rank
                # they yield is fabricated. Frozen §8: the fault stays
                # in the denominator as unlocalized, cause recorded.
                rec["alignment_failure"] = {
                    "log_case": (case["inject_pod"], case["inject_type"])}
                rec["n_candidates"] = 0
                rec["rank_historical"] = None
                rec["rank_dense"] = None
                rec["rank_service_raw"] = None
                rec["rank_service_dedup"] = None
                rec["candidates"] = []
            elif case.get("candidates_missing"):
                # Frozen §8 (Codex round-17 P2, D-041): a case whose
                # candidate list is missing or unparseable is a
                # REPORTED failure with its cause on the record — not
                # a silent, indistinguishable zero-candidate result.
                # The fault stays in the denominator as unlocalized.
                rec["parse_failure"] = ("candidate list missing or "
                                       "unparseable in artifact log")
                rec["n_candidates"] = 0
                rec["artifact_claimed_rank"] = case["artifact_claimed_rank"]
                rec["rank_historical"] = None
                rec["rank_dense"] = None
                rec["rank_service_raw"] = None
                rec["rank_service_dedup"] = None
                rec["candidates"] = []
            else:
                cands = case["candidates"] or []
                rec["n_candidates"] = len(cands)
                rec["artifact_claimed_rank"] = case["artifact_claimed_rank"]
                rec["rank_historical"] = rank_historical(
                    cands, rc_parts, fault["inject_pod"], templates)
                rec["rank_dense"] = rank_dense(
                    cands, rc_parts, fault["inject_pod"], templates)
                raw, dedup = rank_service(cands, fault["inject_pod"])
                rec["rank_service_raw"] = raw
                rec["rank_service_dedup"] = dedup
                rec["candidates"] = cands
        else:
            rec["missing_in_log"] = True
            counters["cases_missing_in_log"] += 1
        records.append(rec)

    if len(cases) > len(faults):
        counters["extra_cases_in_log"] = len(cases) - len(faults)

    n = len(faults)
    evaluated = [r for r in records if "missing_in_log" not in r]
    summary = {
        "ns": args.ns,
        "n_faults_ground_truth": n,
        "n_cases_in_log": len(cases),
        "alignment_errors": alignment_errors,
        "parse_counters": dict(counters),
        "artifact_reported": artifact_final,
        "claim_check_artifact_vs_historical": None,
        "aggregates": {
            "historical": aggregate([r.get("rank_historical") for r in evaluated], n),
            "dense": aggregate([r.get("rank_dense") for r in evaluated], n),
            "service_raw": aggregate([r.get("rank_service_raw") for r in evaluated], n),
            "service_dedup": aggregate([r.get("rank_service_dedup") for r in evaluated], n),
        },
        "candidate_set_sizes": sorted(
            r.get("n_candidates", 0) for r in evaluated),
    }
    # cross-check our historical replica against the artifact's own claims
    mism = []
    for r in evaluated:
        if r.get("artifact_claimed_rank") != r.get("rank_historical"):
            mism.append({"case_id": r["case_id"],
                         "artifact": r.get("artifact_claimed_rank"),
                         "replica": r.get("rank_historical")})
    summary["claim_check_artifact_vs_historical"] = {
        "mismatch_count": len(mism), "mismatches": mism}

    with open(args.out, "w") as f:
        json.dump({"summary": summary, "cases": records}, f, indent=1)

    s = summary["aggregates"]
    print(f"ns={args.ns} n={n} cases_in_log={len(cases)} "
          f"alignment_errors={len(alignment_errors)}")
    for mode in ("historical", "dense", "service_raw", "service_dedup"):
        a = s[mode]
        print(f"  {mode:14s} AC@1={a['AC@1_pct']:6.2f}% AC@3={a['AC@3_pct']:6.2f}% "
              f"AC@5={a['AC@5_pct']:6.2f}% MRR={a['MRR']:.3f} "
              f"unlocalized={a['unlocalized']}")
    print(f"  artifact reported %: {artifact_final['percent_lines']}")
    print(f"  artifact-vs-replica mismatches: "
          f"{summary['claim_check_artifact_vs_historical']['mismatch_count']}")
    if counters.get("sorted_result_parse_failures"):
        print(f"  WARNING: {counters['sorted_result_parse_failures']} "
              f"unparseable candidate lists", file=sys.stderr)


if __name__ == "__main__":
    main()
