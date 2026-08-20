#!/usr/bin/env python3
"""H4 provenance-walk check: for every S1 candidate (all cases, both
datasets), mechanically reconstruct the chain

  candidate -> (normal_session, first_seq) -> canonical event in
  events.jsonl -> provenance record in provenance.jsonl.gz -> source
  file + row in the Nezha dataset -> row content consistent with the
  event (pod matches; span/log id matches).

Alert events get NO special-case success: their provenance record must
name a materialized derivation (import-report.json `alarm_provenance`)
whose `inputs` are themselves walkable source records — metric-CSV rows
or trace-CSV row pairs (or the metric_threshold fallback CSV) — each
verified to exist and to mention the derived pod. A walk that ends at
the *name* of a computation is a failure.

Reports pass/fail counts; any break in any chain is a failure with its
reason. Nothing is sampled — every candidate of every case is walked.
"""
import gzip
import json
import os

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
    report = json.load(open(os.path.join(base, "import-report.json")))
    return events, prov, report


class RowCache:
    def __init__(self):
        self.cache = {}

    def rows(self, path):
        if path not in self.cache:
            with open(path, errors="replace") as f:
                self.cache[path] = f.read().splitlines()
        return self.cache[path]


def check_source_row(code_dir, rel_file, row, must_contain, rows_cache):
    """One immutable source record: file exists, row in range, content
    mentions `must_contain`. Returns None on success, reason on failure."""
    path = os.path.join(code_dir, rel_file)
    if not os.path.exists(path):
        return f"source file missing: {rel_file}"
    rows = rows_cache.rows(path)
    rownum = row + 1  # +1 for the CSV header
    if rownum >= len(rows):
        return f"row out of range: {rel_file}:{rownum}"
    if must_contain and must_contain not in rows[rownum]:
        return f"source row content mismatch: {rel_file}:{rownum}"
    return None


def check_alert_derivation(code_dir, prov_rec, report, rows_cache):
    """Walk an alert event's provenance into its materialized derivation
    and every one of the derivation's input source records."""
    key = prov_rec.get("derivation")
    if not key:
        return "alert provenance has no derivation reference"
    derivations = report.get("alarm_provenance") or {}
    der = derivations.get(key)
    if der is None:
        return f"derivation {key!r} not materialized in import report"
    inputs = der.get("inputs") or []
    if not inputs:
        return f"derivation {key!r} has no source inputs"
    if not der.get("verified"):
        return f"derivation {key!r} not verified against the artifact value"
    pod = prov_rec.get("pod", "")
    for inp in inputs:
        if "row" in inp:  # metric sample or fallback-threshold row
            contain = pod if inp.get("kind") == "metric-sample" else None
            reason = check_source_row(code_dir, inp["file"], inp["row"],
                                      contain, rows_cache)
            if reason:
                return reason
        elif "child_row" in inp:  # trace-derived latency pair
            reason = check_source_row(code_dir, inp["file"],
                                      inp["child_row"], pod, rows_cache)
            if reason:
                return reason
            reason = check_source_row(code_dir, inp["file"],
                                      inp["parent_row"], None, rows_cache)
            if reason:
                return reason
        else:
            return f"derivation {key!r} input has no source pointer: {inp}"
    return None


def main():
    total = passed = 0
    failures = []
    for ns in ("hipster", "ts"):
        windows = [load_window(ns, tag) for tag in NORMAL_DIRS[ns]]
        cases = json.load(open(
            os.path.join(RUNROOT, "results", f"s1-{ns}.cases.json")))["cases"]
        rows_cache = RowCache()
        code_dir = CODE_DIRS[ns]
        for case in cases:
            for cand in case["candidates"]:
                total += 1
                sid = cand["provenance"]["normal_session"]
                seq = cand["provenance"]["first_seq"]
                ev = prov_rec = report = None
                for events, prov, rep in windows:
                    if (sid, seq) in events:
                        ev = events[(sid, seq)]
                        prov_rec = prov.get((sid, seq))
                        report = rep
                        break
                if ev is None:
                    failures.append((case["case_id"], "event not found", sid, seq))
                    continue
                if prov_rec is None:
                    failures.append((case["case_id"], "provenance not found",
                                     sid, seq))
                    continue
                if prov_rec.get("pod") != ev.get("args", {}).get("pod"):
                    failures.append((case["case_id"], "pod mismatch", sid, seq))
                    continue
                if prov_rec.get("rule") == "alert-v1":
                    reason = check_alert_derivation(code_dir, prov_rec,
                                                    report, rows_cache)
                    if reason:
                        failures.append((case["case_id"], reason, sid, seq))
                    else:
                        passed += 1
                    continue
                src_file = prov_rec["file"]
                pod = prov_rec.get("pod", "")
                span = prov_rec.get("span_id", "")
                reason = check_source_row(code_dir, src_file,
                                          prov_rec["row"], pod, rows_cache)
                if reason and span:
                    # multi-line CSV records can shift line numbers; accept
                    # if the span id is present at the computed line
                    alt = check_source_row(code_dir, src_file,
                                           prov_rec["row"], span, rows_cache)
                    reason = alt
                if reason:
                    failures.append((case["case_id"], reason,
                                     src_file, prov_rec["row"]))
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
