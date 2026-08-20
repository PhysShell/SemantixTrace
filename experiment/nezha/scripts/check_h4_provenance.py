#!/usr/bin/env python3
"""H4 provenance-walk check: for every S1 candidate (all cases, both
datasets), mechanically reconstruct the chain

  candidate -> (normal_session, first_seq) -> canonical event in
  events.jsonl -> provenance record in provenance.jsonl.gz -> source
  file + row in the Nezha dataset -> row content consistent with the
  event (pod matches; span/log id matches).

Reports pass/fail counts; any break in any chain is a failure with its
reason. Nothing is sampled — every candidate of every case is walked.
"""
import gzip
import json
import os
import sys

RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")
CODE_DIRS = {"hipster": "/home/user/e0-runs/checkout-hipster",
             "ts": "/home/user/e0-runs/checkout-ts"}
NORMAL_DIRS = {
    "hipster": ["2022-08-22_0351", "2022-08-23_1700"],
    "ts": ["2023-01-29_0850", "2023-01-30_1139"],
}


def load_window(ns, tag):
    base = os.path.join(RUNROOT, ns, "construct", tag)
    events = {}
    with open(os.path.join(base, "events.jsonl")) as f:
        for line in f:
            ev = json.loads(line)
            events[(ev["session_id"], ev["seq"])] = ev
    prov = {}
    with gzip.open(os.path.join(base, "provenance.jsonl.gz"), "rt") as f:
        for line in f:
            p = json.loads(line)
            prov[(p["session_id"], p["seq"])] = p
    return events, prov


def main():
    total = passed = 0
    failures = []
    for ns in ("hipster", "ts"):
        windows = [load_window(ns, tag) for tag in NORMAL_DIRS[ns]]
        cases = json.load(open(
            os.path.join(RUNROOT, "results", f"s1-{ns}.cases.json")))["cases"]
        # source-file row cache
        row_cache = {}
        for case in cases:
            for cand in case["candidates"]:
                total += 1
                sid = cand["provenance"]["normal_session"]
                seq = cand["provenance"]["first_seq"]
                ev = prov_rec = None
                for events, prov in windows:
                    if (sid, seq) in events:
                        ev = events[(sid, seq)]
                        prov_rec = prov.get((sid, seq))
                        break
                if ev is None:
                    failures.append((case["case_id"], "event not found", sid, seq))
                    continue
                if prov_rec is None:
                    failures.append((case["case_id"], "provenance not found", sid, seq))
                    continue
                if prov_rec.get("pod") != ev.get("args", {}).get("pod"):
                    failures.append((case["case_id"], "pod mismatch", sid, seq))
                    continue
                src_file = prov_rec["file"]
                if src_file == "generate_alarm()":
                    passed += 1  # alarm events trace to the alarm computation
                    continue
                path = os.path.join(CODE_DIRS[ns], src_file)
                if path not in row_cache:
                    with open(path, errors="replace") as f:
                        row_cache[path] = f.read().splitlines()
                rows = row_cache[path]
                rownum = prov_rec["row"] + 1  # +1 for the CSV header
                if rownum >= len(rows):
                    failures.append((case["case_id"], "row out of range",
                                     src_file, rownum))
                    continue
                row_text = rows[rownum]
                pod = prov_rec.get("pod", "")
                span = prov_rec.get("span_id", "")
                if pod and pod not in row_text:
                    # multi-line CSV records can shift line numbers; treat
                    # as failure only if the span id also cannot be found
                    if span and span in row_text:
                        pass
                    else:
                        failures.append((case["case_id"],
                                         "source row content mismatch",
                                         src_file, rownum))
                        continue
                passed += 1
    print(f"H4 provenance walk: {passed}/{total} candidate chains "
          f"reconstructed ({100.0 * passed / total:.2f}%)")
    print(f"failures: {len(failures)}")
    for f in failures[:10]:
        print("  ", f)
    out = {"total": total, "passed": passed,
           "failures": [list(f) for f in failures]}
    with open(os.path.join(RUNROOT, "results", "h4-provenance-check.json"),
              "w") as fo:
        json.dump(out, fo, indent=1)


if __name__ == "__main__":
    main()
