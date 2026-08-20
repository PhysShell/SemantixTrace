#!/usr/bin/env python3
"""H4 GREEN repair: materialize verified alarm derivations for every
imported window and rewire alert provenance records to reference them.

For each window directory under E2_RUNROOT/{ns}/{construct,rca}/:
  1. recompute the artifact's metric_list/alarm_list (unmodified code)
     and FAIL CLOSED if the recomputed alarm_list differs from the one
     stored at import time;
  2. build verified derivations (alarm_provenance module) and store them
     under `alarm_provenance` in import-report.json;
  3. rewrite provenance.jsonl.gz: every rule=alert-v1 record loses the
     'generate_alarm()' endpoint and gains a `derivation` key.

Must run under the Nezha venv (pandas + artifact imports).
"""
import glob
import gzip
import json
import os
import sys

RUNROOT = os.environ.get("E2_RUNROOT", "/home/user/e2-runs")
CODE_DIRS = {"hipster": "/home/user/e0-runs/checkout-hipster",
             "ts": "/home/user/e0-runs/checkout-ts"}

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "adapters"))
from alarm_provenance import build_derivations  # noqa: E402


def repair_window(window_dir, nezha_alarm, ns, code_dir):
    report_path = os.path.join(window_dir, "import-report.json")
    report = json.load(open(report_path))
    window = report["window"]
    phase = report["phase"]
    alarm_list = report.get("alarm_list", [])
    data_root = os.path.join(
        code_dir, "construct_data" if phase == "construct" else "rca_data")

    if not alarm_list:
        report["alarm_provenance"] = {}
    else:
        metric_list = nezha_alarm.get_metric_with_time(window, data_root)
        recomputed = nezha_alarm.generate_alarm(metric_list, ns)
        if recomputed != alarm_list:
            raise RuntimeError(
                f"{window_dir}: recomputed alarm_list differs from the "
                f"one stored at import time:\n stored: {alarm_list}\n "
                f"recomputed: {recomputed}")
        report["alarm_provenance"] = build_derivations(
            window, data_root, code_dir, ns, metric_list, alarm_list)

    prov_path = os.path.join(window_dir, "provenance.jsonl.gz")
    rewritten = 0
    if os.path.exists(prov_path):
        records = []
        with gzip.open(prov_path, "rt") as f:
            for line in f:
                rec = json.loads(line)
                if rec.get("rule") == "alert-v1":
                    key = f"{rec['pod']}|{rec['metric_type']}"
                    if key not in report["alarm_provenance"]:
                        raise RuntimeError(
                            f"{window_dir}: alert record references "
                            f"underived alarm {key}")
                    rec.pop("file", None)
                    rec["derivation"] = key
                    rewritten += 1
                records.append(rec)
        with gzip.open(prov_path, "wt") as f:
            for rec in records:
                f.write(json.dumps(rec, separators=(",", ":")) + "\n")

    with open(report_path, "w") as f:
        json.dump(report, f, indent=1, default=str)
    return len(report["alarm_provenance"]), rewritten


def main():
    totals = {"windows": 0, "derivations": 0, "alert_records": 0}
    for ns, code_dir in CODE_DIRS.items():
        # artifact import needs cwd = checkout (relative metric_threshold)
        os.chdir(code_dir)
        sys.path.insert(0, code_dir)
        import importlib
        import alarm as nezha_alarm  # noqa: E402
        importlib.reload(nezha_alarm)
        for window_dir in sorted(
                glob.glob(os.path.join(RUNROOT, ns, "*", "*"))):
            if not os.path.isdir(window_dir) or \
                    not os.path.exists(os.path.join(window_dir,
                                                    "import-report.json")):
                continue
            n_der, n_rec = repair_window(window_dir, nezha_alarm, ns,
                                         code_dir)
            totals["windows"] += 1
            totals["derivations"] += n_der
            totals["alert_records"] += n_rec
            if n_der:
                print(f"{os.path.relpath(window_dir, RUNROOT)}: "
                      f"{n_der} derivation(s), {n_rec} alert record(s)",
                      flush=True)
        sys.path.remove(code_dir)
    print(json.dumps(totals))


if __name__ == "__main__":
    main()
