#!/usr/bin/env python3
"""S2 scorer (E3, frozen spec): the same differential formula applied to
ActionGraph transitions built by the trace-graph crate (st-graph output),
with the frozen tie-break (score desc, anomaly_score desc, depth desc).

Everything downstream of scoring (root-most pruning, depth/pod
attribution from normal sessions via provenance, resource attachment,
alarm dedup) mirrors s1_scorer.py; the deliberate deltas vs S1 are:
  - supports come from the crate's ActionGraph edges (empirically equal
    to adjacent-pair supports; the crate is authoritative here), and
  - the Heuristics anomaly score participates in ordering.
"""
import argparse
import json

SEP = "\x1e"


def action_key_from_value(v):
    return json.dumps(
        [v["screen_id"], v["command_id"], v["abstract_args"]],
        sort_keys=True, separators=(",", ":"))


def load_scenarios(path):
    with open(path) as f:
        return [json.loads(line) for line in f]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--graph", required=True, help="st-graph output json")
    ap.add_argument("--normal-scenarios", action="append", required=True)
    ap.add_argument("--normal-events", action="append", required=True)
    ap.add_argument("--alarms", required=True)
    ap.add_argument("--min-score", type=float, default=0.67)
    ap.add_argument("--support-floor", type=int, default=5)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    gdata = json.load(open(args.graph))
    normal_sessions = []
    for p in args.normal_scenarios:
        normal_sessions.extend(load_scenarios(p))

    pod_of = {}
    for p in args.normal_events:
        with open(p) as f:
            for line in f:
                ev = json.loads(line)
                pod = ev.get("args", {}).get("pod")
                if pod is not None:
                    pod_of[(ev["session_id"], ev["seq"])] = pod

    alarm_list = json.load(open(args.alarms)).get("alarm_list", [])

    # differential over ActionGraph transitions
    score = {}
    anomaly = {}
    for t in gdata["transitions"]:
        n, a = t["n"], t["a"]
        if n > args.support_floor:
            s = 1.0 if a == 0 else n / (a + n)
            if s >= args.min_score:
                src = action_key_from_value(t["src"])
                dst = action_key_from_value(t["dst"])
                k = src + SEP + dst
                score[k] = s
                anomaly[k] = t["anomaly_n"]

    def cmd_of(key_half):
        return json.loads(key_half)[1]

    # root-most pruning (same rule as S1)
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

    # Encounter-order alignment (PR #20 Codex round-6 P1, D-015). The
    # alarm dedup below keeps the FIRST max-depth candidate, so list
    # order is semantically significant before the anomaly tie-break
    # ever applies. S1 builds its dicts in first-encounter order over
    # the normal sessions (pair_support insertion order); the sorted
    # st-graph transition order differs, which retained different
    # depth-tied (pod, resource) candidates in 4/101 cases. Restore
    # S1's ordering witness: record each adjacent pair's
    # first-encounter position over the same normal sessions and order
    # the scored patterns by it. The graph remains the sole source of
    # supports and anomaly scores; the dedup rule itself (mirrored
    # Nezha semantics) is untouched.
    first_seen = {}
    for sess in normal_sessions:
        keys = [action_key_from_value(a) for a in sess["actions"]]
        for i in range(1, len(keys)):
            k = keys[i - 1] + SEP + keys[i]
            if k not in first_seen:
                first_seen[k] = len(first_seen)
    unaligned = sum(1 for k in score if k not in first_seen)
    score = dict(sorted(score.items(),
                        key=lambda kv: first_seen.get(kv[0],
                                                      len(first_seen))))

    # depth/pod attribution (same as S1)
    occ_index = {}
    for sess in normal_sessions:
        span_count = 0
        for act in sess["actions"]:
            k = action_key_from_value(act)
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
        if occ is None:
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
                "score": s, "anomaly": anomaly[k], "deepth": depth,
                "pod": pod,
                "provenance": {"normal_session": sess_id,
                               "first_seq": first_seq}}
        if pod in pod_alarms:
            cand["resource"] = pod_alarms[pod][0]["metric_type"]
        result_list.append(cand)

    # alarm dedup keep-deepest (same as S1)
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

    # FROZEN S2 ordering: score desc, anomaly desc, depth desc
    result_list.sort(key=lambda c: (c["score"], c["anomaly"], c["deepth"]),
                     reverse=True)

    out = {
        "algorithm": "actiongraph-differential",
        "parameters": {"min_score": args.min_score,
                       "support_floor": args.support_floor,
                       "tie_break": "score,anomaly,deepth desc"},
        "normal_sessions": len(normal_sessions),
        "normal_edges": gdata["normal_edges"],
        "abnormal_edges": gdata["abnormal_edges"],
        "unattributed_patterns": unattributed,
        "encounter_unaligned_patterns": unaligned,
        "candidates": result_list,
    }
    with open(args.out, "w") as f:
        json.dump(out, f, indent=1)
    print(f"candidates={len(result_list)} normal_edges={gdata['normal_edges']} "
          f"abnormal_edges={gdata['abnormal_edges']}")


if __name__ == "__main__":
    main()
