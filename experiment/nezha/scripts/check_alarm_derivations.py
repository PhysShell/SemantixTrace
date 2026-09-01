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

Scope note: alarm -> derivation completeness is enforced fail-closed at
import time (a missing derivation raises there); this gate certifies
that every derivation that WAS materialized is faithful to its source
rows.

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
    check_threshold_row, check_trace_p90_derivation)


def check_derivation(code_dir, key, der, rows_cache):
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
                      or check_metric_cell(code_dir, inp, rows_cache))
        else:
            reason = f"input has no source pointer: {inp}"
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
        rep = json.load(open(rp))
        for key, der in (rep.get("alarm_provenance") or {}).items():
            n_der += 1
            kinds[der.get("kind")] += 1
            reason = check_derivation(CODE_DIRS[ns], key, der, caches[ns])
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
