#!/usr/bin/env python3
"""S1 scorer: the Nezha adjacent-pair differential applied to the
SemantixTrace canonical representation (E2, frozen design).

Mirrors the ACTIVE algorithm of pattern_ranker.py at commit d8140101,
component by component, over per-session canonical-action scenarios
(st-fold output) instead of drain3-ID event graphs:

  1. support: every adjacent action pair occurrence counts 1, summed
     across sessions (== EventGraph.get_support + get_pattern_support);
  2. expected patterns: normal-support > 5, score n/(n+a), pruned < 0.67
     (== pattern_ranker.py:98-118);
  3. root-most pruning: drop (x,y) when a retained (w,x) has >= score,
     unless y is an alert action (== :122-134, whose metric-target test
     maps to command_id "alert:");
  4. depth/pod: for the source action x, depth of an occurrence is
     1 + count of preceding "span:" actions in its session (the linear
     analog of get_deepth_pod counting "start" events on the walk-up);
     max across normal sessions; pod from the provenance of the
     occurrence achieving that max (first in file order). Provenance
     always resolves, so the artifact's hardcoded fallback pod
     (pattern_ranker.py:145-147) has no analog here, by design;
  5. resource attachment: candidate pod carrying any abnormal-window
     alarm becomes a resource candidate with that pod's FIRST alarm
     (== :148-159 including the first-alarm-only quirk, kept for
     algorithm parity);
  6. alarm dedup keep-deepest (== :161-186);
  7. final order: (score, depth) descending (== :189-190).

Pattern keys join two action identities with the ASCII record separator
0x1e, which cannot occur inside json.dumps output (control characters
are escaped), so the join is unambiguous.
"""
import argparse
import json

SEP = "\x1e"


def action_key(a):
    return json.dumps(
        [a["screen_id"], a["command_id"], a["abstract_args"]],
        sort_keys=True, separators=(",", ":"))


def load_scenarios(path):
    with open(path) as f:
        return [json.loads(line) for line in f]


def pair_support(sessions):
    support = {}
    for sess in sessions:
        keys = [action_key(a) for a in sess["actions"]]
        for i in range(1, len(keys)):
            k = keys[i - 1] + SEP + keys[i]
            support[k] = support.get(k, 0) + 1
    return support


def cmd_of(key_half):
    return json.loads(key_half)[1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--normal-scenarios", action="append", required=True)
    ap.add_argument("--normal-events", action="append", required=True,
                    help="events.jsonl matching each --normal-scenarios")
    ap.add_argument("--abnormal-scenarios", required=True)
    ap.add_argument("--alarms", required=True,
                    help="import-report.json of the abnormal window "
                         "(alarm_list field)")
    ap.add_argument("--min-score", type=float, default=0.67)
    ap.add_argument("--support-floor", type=int, default=5)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    normal_sessions = []
    for p in args.normal_scenarios:
        normal_sessions.extend(load_scenarios(p))
    abnormal_sessions = load_scenarios(args.abnormal_scenarios)

    # (session_id, seq) -> pod, from the normal windows' events files
    pod_of = {}
    for p in args.normal_events:
        with open(p) as f:
            for line in f:
                ev = json.loads(line)
                pod = ev.get("args", {}).get("pod")
                if pod is not None:
                    pod_of[(ev["session_id"], ev["seq"])] = pod

    alarm_list = json.load(open(args.alarms)).get("alarm_list", [])

    normal_support = pair_support(normal_sessions)
    abnormal_support = pair_support(abnormal_sessions)

    # expected patterns (pattern_ranker.py:98-118)
    score = {}
    for k, n in normal_support.items():
        if n > args.support_floor:
            a = abnormal_support.get(k)
            s = 1.0 if a is None else n / (a + n)
            if s >= args.min_score:
                score[k] = s

    # root-most pruning (pattern_ranker.py:122-134): drop k when a
    # retained k1 with target(k1) == source(k) has >= score, unless
    # k's target is an alert action
    targets = {}
    for k in score:
        src, dst = k.split(SEP)
        targets.setdefault(dst, []).append(k)
    drop = set()
    for k in score:
        src, dst = k.split(SEP)
        if cmd_of(dst).startswith("alert:"):
            continue
        for k1 in targets.get(src, []):
            if score[k] <= score[k1]:
                drop.add(k)
                break
    for k in drop:
        score.pop(k)

    # depth/pod attribution from normal sessions (provenance-based)
    occ_index = {}  # action_key -> (depth, session_id, first_seq)
    for sess in normal_sessions:
        span_count = 0
        for act in sess["actions"]:
            k = action_key(act)
            depth = 1 + span_count
            cur = occ_index.get(k)
            if cur is None or depth > cur[0]:
                occ_index[k] = (depth, sess["session_id"], act["first_seq"])
            if act["command_id"].startswith("span:"):
                span_count += 1

    pod_alarms = {a["pod"]: a["alarm"] for a in alarm_list}
    result_list = []
    deepth_dict = {}
    unattributed = 0
    for k, s in score.items():
        src, dst = k.split(SEP)
        occ = occ_index.get(src)
        if occ is None:  # cannot happen: normal-support patterns only
            unattributed += 1
            continue
        depth, sess_id, first_seq = occ
        pod = pod_of.get((sess_id, first_seq), "")
        if pod == "":
            unattributed += 1
            continue
        if pod not in deepth_dict or deepth_dict[pod] < depth:
            deepth_dict[pod] = depth
        cand = {"pattern": [json.loads(src), json.loads(dst)],
                "score": s, "deepth": depth, "pod": pod,
                "provenance": {"normal_session": sess_id,
                               "first_seq": first_seq}}
        if pod in pod_alarms:
            cand["resource"] = pod_alarms[pod][0]["metric_type"]
        result_list.append(cand)

    # alarm dedup keep-deepest (pattern_ranker.py:161-186)
    move = set()
    for item in alarm_list:
        pod = item["pod"]
        if pod not in deepth_dict:
            continue
        max_deep = deepth_dict[pod]
        kept_one = False
        for i, cand in enumerate(result_list):
            if "resource" in cand and cand["pod"] == pod \
                    and cand["resource"] == item["alarm"][0]["metric_type"]:
                if max_deep > cand["deepth"]:
                    move.add(i)
                elif max_deep == cand["deepth"] and kept_one:
                    move.add(i)
                else:
                    kept_one = True
    result_list = [c for i, c in enumerate(result_list) if i not in move]

    result_list.sort(key=lambda c: (c["score"], c["deepth"]), reverse=True)

    out = {
        "algorithm": "nezha-adjacent-differential-on-canonical",
        "parameters": {"min_score": args.min_score,
                       "support_floor": args.support_floor},
        "normal_sessions": len(normal_sessions),
        "abnormal_sessions": len(abnormal_sessions),
        "normal_patterns": len(normal_support),
        "abnormal_patterns": len(abnormal_support),
        "unattributed_patterns": unattributed,
        "candidates": result_list,
    }
    with open(args.out, "w") as f:
        json.dump(out, f, indent=1)
    print(f"candidates={len(result_list)} "
          f"normal_patterns={len(normal_support)} "
          f"abnormal_patterns={len(abnormal_support)} "
          f"unattributed={unattributed}")


if __name__ == "__main__":
    main()
