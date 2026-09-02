#!/usr/bin/env python3
"""Alarm-derivation audit gate (PR #20 Codex round-13 P1, D-026).

The H4 provenance walk covers candidate chains, and candidates point at
NORMAL sessions — so the trace-derived-p90 derivations, which in this
dataset all live in abnormal (rca) windows, were verified only by the
importer's own at-materialization check. This gate closes that scope
gap: it independently re-verifies EVERY materialized alarm derivation
of EVERY window of a runroot, using the strengthened checkers from
check_h4_provenance (D-017 metric-cell re-parse; D-026 full trace-p90
recomputation and threshold re-parse) — never trusting the stored
`verified` flag. Window coverage is certified against the committed
expected-window manifest (D-020) so a truncated runroot cannot pass.

Alarm -> derivation completeness is enforced HERE as well as at import
time (D-033): the (pod, metric_type) key set derived from alarm_list
must equal the materialized derivation key set — a report that
retained an alarm but lost its derivation previously just checked
fewer derivations and passed. Metric rows are additionally validated
against the report's window (D-032).

Usage: check_alarm_derivations.py [runroot] [out.json]
Exit 1 on any unsound derivation or coverage violation.
"""
import glob
import json
import os
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from run_e2 import CODE_DIRS  # noqa: E402
from check_ingestion_conservation import expected_windows  # noqa: E402
from check_h4_provenance import (  # noqa: E402
    RowCache, check_source_row, check_metric_cell,
    check_metric_row_identity, check_threshold_row,
    check_trace_p90_derivation, tag_to_window)


def check_derivation(code_dir, key, der, rows_cache, window):
    pod = key.split("|", 1)[0]
    inputs = der.get("inputs") or []
    if not inputs:
        return "derivation has no source inputs"
    if not der.get("verified"):
        return "derivation not marked verified at materialization"
    if der.get("kind") == "trace-derived-p90":
        return check_trace_p90_derivation(code_dir, pod, der, rows_cache)
    for inp in inputs:
        if inp.get("kind") == "threshold-row":
            reason = (check_source_row(code_dir, inp["file"], inp["row"],
                                       [], rows_cache)
                      or check_threshold_row(code_dir, inp, rows_cache))
        elif "row" in inp:
            reason = (check_source_row(code_dir, inp["file"], inp["row"],
                                       [pod], rows_cache)
                      or check_metric_cell(code_dir, inp, rows_cache)
                      or check_metric_row_identity(code_dir, inp, pod,
                                                   window, rows_cache))
        else:
            reason = f"input has no source pointer: {inp}"
        # derivation value must equal every validated input value
        # (Codex round-16 P1, D-037) — see check_h4_provenance
        if not reason and inp.get("value") != der.get("value"):
            reason = (f"derivation value {der.get('value')!r} != validated "
                      f"input value {inp.get('value')!r}")
        if reason:
            return reason
    return None


def main():
    runroot = sys.argv[1] if len(sys.argv) > 1 else "/home/user/e2-runs"
    out_path = sys.argv[2] if len(sys.argv) > 2 else None
    kinds = Counter()
    failures = []
    found = set()
    caches = {ns: RowCache() for ns in CODE_DIRS}
    n_der = 0
    for rp in sorted(glob.glob(os.path.join(runroot, "*", "*", "*",
                                            "import-report.json"))):
        tag = "/".join(os.path.dirname(rp).split("/")[-3:])
        found.add(tag)
        ns = tag.split("/")[0]
        window = tag_to_window(tag.split("/")[-1])
        rep = json.load(open(rp))
        materialized = rep.get("alarm_provenance") or {}
        # Completeness fails closed (Codex round-14 P2, D-033): every
        # alarm_list entry must have a materialized derivation and
        # vice versa — auditing only what WAS materialized let a
        # report that dropped a derivation pass with fewer checks.
        expected_keys = set()
        for a in rep.get("alarm_list", []):
            # malformed entries are named violations, never KeyError
            # crashes past the summary (CodeRabbit final round, D-038)
            if not isinstance(a, dict) or "pod" not in a:
                failures.append({"window": tag,
                                 "reason": f"malformed alarm_list "
                                           f"entry: {a!r}"})
                continue
            for e in a.get("alarm", []):
                if not isinstance(e, dict) or "metric_type" not in e:
                    failures.append({"window": tag,
                                     "reason": f"malformed alarm entry "
                                               f"for pod {a['pod']}: "
                                               f"{e!r}"})
                    continue
                expected_keys.add(f"{a['pod']}|{e['metric_type']}")
        for key in sorted(expected_keys - set(materialized)):
            failures.append({"window": tag, "derivation": key,
                             "reason": "alarm has no materialized "
                                       "derivation"})
        for key in sorted(set(materialized) - expected_keys):
            failures.append({"window": tag, "derivation": key,
                             "reason": "derivation has no alarm_list "
                                       "entry"})
        for key, der in materialized.items():
            n_der += 1
            kinds[der.get("kind")] += 1
            reason = check_derivation(CODE_DIRS[ns], key, der, caches[ns],
                                      window)
            if reason:
                failures.append({"window": tag, "derivation": key,
                                 "reason": reason})
    expected = expected_windows()
    missing = sorted(expected - found)
    for tag in missing:
        failures.append({"window": tag,
                         "reason": "expected window MISSING from runroot"})
    summary = {"runroot": runroot,
               "windows_scanned": len(found),
               "expected_windows": len(expected),
               "missing_windows": missing,
               "derivations_checked": n_der,
               "derivations_by_kind": dict(kinds),
               "failures": failures}
    if out_path:
        with open(out_path, "w") as f:
            json.dump(summary, f, indent=1)
    print(f"alarm derivations: {n_der} checked across "
          f"{len(found)}/{len(expected)} windows "
          f"{dict(kinds)} failures={len(failures)}")
    for f in failures[:10]:
        print("  ", f)
    sys.exit(0 if not failures else 1)


if __name__ == "__main__":
    main()
