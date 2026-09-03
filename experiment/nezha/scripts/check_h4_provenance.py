#!/usr/bin/env python3
"""H4 provenance-walk check: for every S1 candidate (all cases, both
datasets), mechanically reconstruct the chain

  candidate -> (normal_session, first_seq) -> canonical event in
  events.jsonl -> provenance record in provenance.jsonl.gz -> source
  file + row in the Nezha dataset -> row content consistent with the
  event (pod matches; span/log id matches) -> canonical command
  RE-DERIVED from the row itself (Codex round-17 P1, D-040): span
  commands are recomputed as span:{service_of(PodName)}
  {OperationName}, and log rows must match the recorded drain3
  cluster under the artifact's own preprocessing and the checkout's
  shipped template state (match-only; sound because every
  import-report records drain3_new_clusters == 0 — the vocabulary is
  closed on this dataset).

Alert events get NO special-case success: their provenance record must
name a materialized derivation (import-report.json `alarm_provenance`)
whose `inputs` are themselves walkable source records — metric-CSV rows
or trace-CSV row pairs (or the metric_threshold fallback CSV) — each
verified to exist and to mention the derived pod. A walk that ends at
the *name* of a computation is a failure.

Reports pass/fail counts; any break in any chain is a failure with its
reason. Nothing is sampled — every candidate of every case is walked.
"""
import csv
import gzip
import json
import os
import re
import sys
import tempfile

import numpy as np
import pandas as pd

ADAPTERS = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                        "..", "adapters")
sys.path.insert(0, ADAPTERS)
from import_nezha import make_miner, service_of  # noqa: E402

RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")
CODE_DIRS = {"hipster": "/home/user/e0-runs/checkout-hipster",
             "ts": "/home/user/e0-runs/checkout-ts"}
NORMAL_DIRS = {
    "hipster": ["2022-08-22_0351", "2022-08-23_1700"],
    "ts": ["2023-01-29_0850", "2023-01-30_1139"],
}


def tag_to_window(tag):
    """'2022-08-22_0351' -> '2022-08-22 03:51' (the importer's window
    string, needed to re-validate metric-row Time cells, D-032)."""
    date, hm = tag.rsplit("_", 1)
    return f"{date} {hm[:2]}:{hm[2:]}"


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
        self.parsed = {}

    def rows(self, path):
        if path not in self.cache:
            with open(path, errors="replace") as f:
                self.cache[path] = f.read().splitlines()
        return self.cache[path]

    def csv_rows(self, path):
        """Quoting-aware parse (header, data rows) of a source CSV."""
        if path not in self.parsed:
            with open(path, errors="replace", newline="") as f:
                rows = list(csv.reader(f))
            self.parsed[path] = (rows[0], rows[1:])
        return self.parsed[path]

    def artifact_cell(self, path, row, column):
        """The cell as the ARTIFACT's own parser reads it (pandas'
        C tokenizer): on some cells its float differs from CPython's
        strtod by 1 ULP, and the materialized values replicate the
        artifact's parse — so equality is judged against this parse
        when the raw-text parse disagrees (D-026)."""
        key = ("pd", path)
        if key not in self.cache:
            self.cache[key] = pd.read_csv(path)
        df = self.cache[key]
        if column not in df.columns or row >= len(df):
            return None
        return float(df[column][row])


def check_source_row(code_dir, rel_file, row, required_tokens, rows_cache):
    """One immutable source record: file exists, row in range, content
    mentions EVERY required token (Codex round-8 P1, D-017: a single
    pod substring also matches other rows of the same pod, so identity
    tokens such as the span id must be required jointly, never as a
    fallback). Returns None on success, reason on failure."""
    path = os.path.join(code_dir, rel_file)
    if not os.path.exists(path):
        return f"source file missing: {rel_file}"
    rows = rows_cache.rows(path)
    rownum = row + 1  # +1 for the CSV header
    if rownum >= len(rows):
        return f"row out of range: {rel_file}:{rownum}"
    for tok in required_tokens:
        if tok and tok not in rows[rownum]:
            return (f"source row content mismatch: {rel_file}:{rownum} "
                    f"(missing {tok!r})")
    return None


def build_matcher(ns, code_dir):
    """Match-only drain3 miner over the checkout's SHIPPED template
    state, plus the artifact's own log preprocessing (pod_to_service),
    for re-deriving log cluster ids row-locally (Codex round-17 P1,
    D-040). Sound on this dataset because every import-report records
    drain3_new_clusters == 0: cluster assignment during import never
    left the shipped state. make_miner copies the state to a scratch
    dir, so the checkout is never mutated."""
    sys.path.insert(0, code_dir)
    import log_parsing as nezha_logparse  # noqa: E402
    miner = make_miner(code_dir, ns, tempfile.mkdtemp(prefix="d040-"))
    return miner, nezha_logparse.pod_to_service


def check_command_derivation(code_dir, prov_rec, ev, rows_cache, miner,
                             pod_to_service):
    """The event->source TRANSFORM itself (Codex round-17 P1, D-040):
    identity tokens (D-017) prove the pointed row is the recorded one,
    but nothing proved the canonical command was DERIVED from that
    row — a same-pod row sharing the span id with a different
    OperationName, or a log pointer re-aimed to a row of another
    drain3 identity, still walked clean. span-v1 commands are exactly
    recomputed from the row; log-v1 commands must name the provenance
    cluster AND the row's body must match that cluster under the
    artifact's own preprocessing and the shipped drain3 state."""
    path = os.path.join(code_dir, prov_rec["file"])
    header, data = rows_cache.csv_rows(path)
    row = prov_rec["row"]
    if not isinstance(row, int) or row < 0 or row >= len(data):
        return f"source row out of range: {prov_rec['file']}:{row}"
    cells = data[row]

    def col(name):
        try:
            i = header.index(name)
        except ValueError:
            return None
        return cells[i] if i < len(cells) else None

    if prov_rec.get("rule") == "span-v1":
        pod, op = col("PodName"), col("OperationName")
        if pod is None or op is None:
            return (f"span source row lacks PodName/OperationName: "
                    f"{prov_rec['file']}:{row}")
        expected = f"span:{service_of(pod)} {op}"
        if ev.get("command_id") != expected:
            return (f"span command not derived from source row: row "
                    f"yields {expected!r} but event carries "
                    f"{ev.get('command_id')!r}")
        return None
    if prov_rec.get("rule") == "log-v1":
        cid = prov_rec.get("cluster_id")
        if ev.get("command_id") != f"log:{cid}":
            return (f"log command {ev.get('command_id')!r} != provenance "
                    f"cluster {cid!r}")
        raw, pod = col("Log"), col("PodName")
        if raw is None or pod is None:
            return (f"log source row lacks Log/PodName: "
                    f"{prov_rec['file']}:{row}")
        try:
            log_message, _svc = pod_to_service(raw, pod)
        except Exception as exc:  # noqa: BLE001 - named, never a crash
            return (f"log body does not preprocess: "
                    f"{type(exc).__name__}: {exc}")
        m = miner.match(log_message)
        if m is None or m.cluster_id != cid:
            return (f"log row does not match recorded cluster {cid!r}: "
                    f"shipped-state match is "
                    f"{getattr(m, 'cluster_id', None)!r}")
        return None
    return f"unknown provenance rule: {prov_rec.get('rule')!r}"


def check_metric_cell(code_dir, inp, rows_cache):
    """Independently re-parse a metric-sample input row and compare the
    recorded column's cell to the recorded value (exact float equality,
    as the materialization used) — the derivation's own 'verified' flag
    is not trusted as a substitute (Codex round-8 P1, D-017)."""
    path = os.path.join(code_dir, inp["file"])
    rows = rows_cache.rows(path)
    header = rows[0].split(",")
    if inp.get("column") not in header:
        return f"metric column {inp.get('column')!r} not in {inp['file']}"
    col = header.index(inp["column"])
    cells = rows[inp["row"] + 1].split(",")
    if col >= len(cells):
        return f"metric row too short: {inp['file']}:{inp['row'] + 1}"
    try:
        cell_value = float(cells[col])
    except ValueError:
        return (f"metric cell not numeric: {inp['file']}:{inp['row'] + 1} "
                f"col {inp['column']!r}")
    if cell_value != inp.get("value"):
        # Second tier (D-026): the materialized value replicates the
        # artifact's pandas parse, which differs from strtod by 1 ULP
        # on some cells — re-judge against that parser before failing.
        artifact_value = rows_cache.artifact_cell(path, inp["row"],
                                                  inp["column"])
        if artifact_value != inp.get("value"):
            return (f"metric value mismatch: {inp['file']}:{inp['row'] + 1} "
                    f"cell {cell_value!r} (artifact parse "
                    f"{artifact_value!r}) != recorded {inp.get('value')!r}")
    return None


def check_trace_p90_derivation(code_dir, pod, der, rows_cache):
    """Independently RECOMPUTE a trace-derived NetworkP90 derivation from
    the referenced source CSV and require the recorded pair list, value
    and n_samples to match the recomputation exactly (Codex round-13 P1,
    D-026). The previous per-pair check verified only a pod substring on
    the child row and nothing on the parent row — a re-aimed pointer or
    a fabricated pair list passed. The recomputation mirrors the
    artifact's alarm.get_netwrok_metric semantics (child rows of the
    pod; parent = LAST row per SpanID; cross-pod links only;
    (parent_end - child_end)/1e6; p90 if > 2 samples else 10.0) with an
    independent parser, so child.ParentID == parent.SpanID and the
    resulting percentile are verified by construction."""
    inputs = der.get("inputs") or []
    # A malformed input must be a NAMED failure, not a KeyError/
    # TypeError crash past the audit's summary (CodeRabbit final
    # round, D-034).
    if not all(isinstance(inp, dict) and isinstance(inp.get("file"), str)
               and inp["file"] for inp in inputs):
        return "trace derivation has an input without a source file"
    rels = {inp["file"] for inp in inputs}
    if len(rels) != 1:
        return f"trace derivation references {len(rels)} files: {sorted(rels)}"
    rel = inputs[0]["file"]
    path = os.path.join(code_dir, rel)
    if not os.path.exists(path):
        return f"source file missing: {rel}"
    header, data = rows_cache.csv_rows(path)
    try:
        c_span = header.index("SpanID")
        c_par = header.index("ParentID")
        c_pod = header.index("PodName")
        c_end = header.index("EndTimeUnixNano")
    except ValueError as exc:
        return f"trace CSV {rel} missing column: {exc}"
    last_row_by_spanid = {}
    for i, r in enumerate(data):
        last_row_by_spanid[r[c_span]] = i
    pairs, latencies = [], []
    for i, r in enumerate(data):
        if r[c_pod] != pod:
            continue
        li = last_row_by_spanid.get(r[c_par])
        if li is None:
            continue  # the artifact's silent KeyError path
        if data[li][c_pod] != pod:
            latencies.append(
                (int(data[li][c_end]) - int(r[c_end])) / 1000000)
            pairs.append((i, li))
    value = (float(np.percentile(latencies, 90))
             if len(latencies) > 2 else 10.0)
    recorded = [(inp.get("child_row"), inp.get("parent_row"))
                for inp in inputs]
    if recorded != pairs:
        div = next((k for k in range(min(len(recorded), len(pairs)))
                    if recorded[k] != pairs[k]),
                   min(len(recorded), len(pairs)))
        return (f"trace pair list mismatch in {rel}: recorded "
                f"{len(recorded)} pairs != recomputed {len(pairs)}, "
                f"first divergence at pair {div}")
    if der.get("value") != value:
        return (f"trace p90 value mismatch: recorded {der.get('value')!r} "
                f"!= recomputed {value!r} from {rel}")
    if "n_samples" in der and der["n_samples"] != len(latencies):
        return (f"trace n_samples mismatch: recorded {der['n_samples']} "
                f"!= recomputed {len(latencies)}")
    return None


def check_threshold_row(code_dir, inp, rows_cache):
    """Re-parse a fallback-threshold row's NetworkP90(ms) cell and
    require exact equality with the recorded value (D-026 — same
    no-trust rule as metric cells since D-017)."""
    header, data = rows_cache.csv_rows(os.path.join(code_dir, inp["file"]))
    if "NetworkP90(ms)" not in header:
        return f"threshold column missing in {inp['file']}"
    col = header.index("NetworkP90(ms)")
    if inp["row"] >= len(data) or col >= len(data[inp["row"]]):
        return f"threshold row out of range: {inp['file']}:{inp['row']}"
    try:
        cell = float(data[inp["row"]][col])
    except ValueError:
        return f"threshold cell not numeric: {inp['file']}:{inp['row']}"
    if cell != inp.get("value"):
        return (f"threshold value mismatch: {inp['file']}:{inp['row']} "
                f"cell {cell!r} != recorded {inp.get('value')!r}")
    return None


def check_metric_row_identity(code_dir, inp, pod, window, rows_cache):
    """The referenced metric row must belong to the alarmed pod AND the
    report's window (Codex round-14 P2, D-032): the importer selects
    samples by window and value jointly, so a pointer re-aimed at
    another same-pod row that happens to carry the same value — but a
    different Time — previously passed both the pod-substring and the
    value check. Mirrors the importer's `re.search(window, Time)`."""
    header, data = rows_cache.csv_rows(os.path.join(code_dir, inp["file"]))
    for col in ("Time", "PodName"):
        if col not in header:
            return f"metric CSV {inp['file']} missing column: {col}"
    if inp["row"] >= len(data):
        return f"metric row out of range: {inp['file']}:{inp['row']}"
    row = data[inp["row"]]
    if row[header.index("PodName")] != pod:
        return (f"metric row pod mismatch: {inp['file']}:{inp['row']} "
                f"PodName {row[header.index('PodName')]!r} != {pod!r}")
    time_cell = row[header.index("Time")]
    if not re.search(window, time_cell):
        return (f"metric row outside window: {inp['file']}:{inp['row']} "
                f"Time {time_cell!r} does not match window {window!r}")
    return None


def check_alert_derivation(code_dir, prov_rec, report, rows_cache, window):
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
    if der.get("kind") == "trace-derived-p90":
        return check_trace_p90_derivation(code_dir, pod, der, rows_cache)
    for inp in inputs:
        if inp.get("kind") == "threshold-row":
            reason = (check_source_row(code_dir, inp["file"], inp["row"],
                                       [], rows_cache)
                      or check_threshold_row(code_dir, inp, rows_cache))
        elif "row" in inp:  # metric sample row
            reason = (check_source_row(code_dir, inp["file"], inp["row"],
                                       [pod], rows_cache)
                      or check_metric_cell(code_dir, inp, rows_cache)
                      or check_metric_row_identity(code_dir, inp, pod,
                                                   window, rows_cache))
        else:
            reason = f"derivation {key!r} input has no source pointer: {inp}"
        # The derivation-level value must equal every validated input
        # value (Codex round-16 P1, D-037): the importer selects rows
        # BY that value, so a corrupted der value with intact inputs
        # previously passed — nothing related the two.
        if not reason and inp.get("value") != der.get("value"):
            reason = (f"derivation value {der.get('value')!r} != validated "
                      f"input value {inp.get('value')!r}")
        if reason:
            return reason
    return None


def main():
    total = passed = 0
    failures = []
    for ns in ("hipster", "ts"):
        windows = [load_window(ns, tag) + (tag_to_window(tag),)
                   for tag in NORMAL_DIRS[ns]]
        cases = json.load(open(
            os.path.join(RUNROOT, "results", f"s1-{ns}.cases.json")))["cases"]
        rows_cache = RowCache()
        code_dir = CODE_DIRS[ns]
        miner, pod_to_service = build_matcher(ns, code_dir)
        for case in cases:
            for cand in case["candidates"]:
                total += 1
                sid = cand["provenance"]["normal_session"]
                seq = cand["provenance"]["first_seq"]
                ev = prov_rec = report = window = None
                for events, prov, rep, win in windows:
                    if (sid, seq) in events:
                        ev = events[(sid, seq)]
                        prov_rec = prov.get((sid, seq))
                        report = rep
                        window = win
                        break
                if ev is None:
                    failures.append((case["case_id"], "event not found", sid, seq))
                    continue
                if prov_rec is None:
                    failures.append((case["case_id"], "provenance not found",
                                     sid, seq))
                    continue
                # The candidate itself must match the event it points
                # at (Codex round-15 P1, D-035): the walk previously
                # verified event<->provenance<->source but never
                # candidate<->event, so a pointer re-aimed at an
                # unrelated same-pod event validated the wrong chain.
                # The fold preserves command_id verbatim, so the
                # pattern's first action must name this event's
                # command_id and the candidate's pod must be the
                # event's pod.
                pat = cand.get("pattern")
                if (not isinstance(pat, list) or not pat
                        or not isinstance(pat[0], (list, tuple))
                        or len(pat[0]) < 2):
                    # named failure, not an IndexError past the summary
                    # (CodeRabbit final round, D-038 — D-034's rule)
                    failures.append((case["case_id"],
                                     "malformed candidate pattern",
                                     sid, seq))
                    continue
                if ev.get("command_id") != pat[0][1]:
                    failures.append(
                        (case["case_id"],
                         f"candidate/event link mismatch: pattern[0] "
                         f"names {pat[0][1]!r} but event "
                         f"carries {ev.get('command_id')!r}", sid, seq))
                    continue
                if cand.get("pod") != ev.get("args", {}).get("pod"):
                    failures.append(
                        (case["case_id"],
                         f"candidate/event pod mismatch: candidate "
                         f"{cand.get('pod')!r} but event "
                         f"{ev.get('args', {}).get('pod')!r}", sid, seq))
                    continue
                if prov_rec.get("pod") != ev.get("args", {}).get("pod"):
                    failures.append((case["case_id"], "pod mismatch", sid, seq))
                    continue
                if prov_rec.get("rule") == "alert-v1":
                    reason = check_alert_derivation(code_dir, prov_rec,
                                                    report, rows_cache,
                                                    window)
                    if reason:
                        failures.append((case["case_id"], reason, sid, seq))
                    else:
                        passed += 1
                    continue
                src_file = prov_rec["file"]
                pod = prov_rec.get("pod", "")
                span = prov_rec.get("span_id", "")
                # BOTH identity tokens required at the recorded line —
                # no fallback (Codex round-8 P1, D-017): a pod-only
                # substring accepts any row of the same pod, and a
                # fallback consulted only after a pod mismatch never
                # fires on such a mis-pointed row.
                reason = (check_source_row(code_dir, src_file,
                                           prov_rec["row"], [pod, span],
                                           rows_cache)
                          or check_command_derivation(code_dir, prov_rec,
                                                      ev, rows_cache,
                                                      miner,
                                                      pod_to_service))
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
    # a checker that reports failures must also FAIL (D-017)
    raise SystemExit(0 if not failures else 1)


if __name__ == "__main__":
    main()
