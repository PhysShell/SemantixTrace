#!/usr/bin/env python3
"""Materialized provenance for Nezha metric alarms (H4 evidence contract).

An alert event's provenance must terminate at immutable source records,
not at the name of a computation. This module builds, for every alarm
the artifact's `generate_alarm` emits, a *derivation* record:

  {kind, inputs: [source refs...], value, threshold_rule, computed_by,
   verified}

- CpuUsageRate/MemoryUsageRate: `inputs` are the metric-CSV rows whose
  `Time` matches the window and whose value is the alarmed value
  (kind "metric-sample").
- NetworkP90(ms): the artifact derives it from the window's trace CSV
  (p90 of parent_end - child_end over cross-pod parent links, last
  matching parent row wins), or falls back to metric_threshold/<svc>.csv
  when the pod has no spans. `inputs` are the contributing trace row
  pairs (kind "trace-latency-pair") or the fallback threshold row.
  The shadow replication is verified for exact float equality against
  the value the artifact actually used; any mismatch raises.

Provenance here is a small DAG, not a single pointer: one alert event
-> one derivation -> N source rows.
"""
import os
import re

import numpy as np
import pandas as pd

CPU_MEM = ("CpuUsageRate(%)", "MemoryUsageRate(%)")


def service_of(pod):
    return pod.rsplit("-", 1)[0].rsplit("-", 1)[0]


def threshold_rule(metric_type, ns):
    if metric_type in CPU_MEM:
        return "> 80 (alarm.determine_alarm)"
    return ("> 200 (alarm.determine_alarm, hipster)" if ns == "hipster"
            else "> 300 (alarm.determine_alarm, ts)")


def metric_sample_inputs(window, data_root_abs, code_dir, pod, metric_type,
                         expected_value):
    date = window.split(" ")[0]
    rel = os.path.join(
        os.path.relpath(data_root_abs, code_dir), date, "metric",
        f"{pod}_metric.csv")
    path = os.path.join(code_dir, rel)
    if not os.path.exists(path):
        raise RuntimeError(f"metric file missing for alarmed pod: {rel}")
    df = pd.read_csv(path)
    inputs = []
    for i in range(len(df["Time"])):
        if re.search(window, str(df["Time"][i])) \
                and float(df[metric_type][i]) == float(expected_value):
            inputs.append({"kind": "metric-sample", "file": rel,
                           "row": int(i), "column": metric_type,
                           "value": float(df[metric_type][i])})
    if not inputs:
        raise RuntimeError(
            f"no metric row matches window={window!r} value="
            f"{expected_value!r} in {rel}")
    return inputs


def network_p90_derivation(window, data_root_abs, code_dir, pod,
                           expected_value):
    """Shadow replication of alarm.get_netwrok_metric with row refs."""
    date, hour_min = window.split(" ")
    hh, mm = hour_min.split(":")
    rel = os.path.join(os.path.relpath(data_root_abs, code_dir), date,
                       "trace", f"{hh}_{mm}_trace.csv")
    if "front" in pod:
        raise RuntimeError(
            "frontend NetworkP90 is the constant 10 and can never alarm; "
            f"an alarm for {pod} indicates a replication error")
    df = pd.read_csv(os.path.join(code_dir, rel),
                     usecols=["TraceID", "SpanID", "ParentID", "PodName",
                              "EndTimeUnixNano"])
    child_rows = df.index[df["PodName"] == pod].tolist()
    if not child_rows:
        svc = service_of(pod)
        thr_rel = os.path.join("metric_threshold", f"{svc}.csv")
        thr = pd.read_csv(os.path.join(code_dir, thr_rel),
                          usecols=["NetworkP90(ms)"])
        value = float(thr.iloc[0])
        der = {"kind": "fallback-threshold",
               "inputs": [{"kind": "threshold-row", "file": thr_rel,
                           "row": 0, "value": value}],
               "value": value}
    else:
        # the artifact's parent lookup keeps the LAST row per SpanID
        last_row_by_spanid = {}
        for i, sid in zip(df.index, df["SpanID"]):
            last_row_by_spanid[sid] = i
        latencies, pairs = [], []
        for ci in child_rows:
            li = last_row_by_spanid.get(df["ParentID"][ci])
            if li is None:
                continue  # the artifact's silent KeyError path
            if str(df["PodName"][li]) != str(pod):
                latencies.append(
                    (int(df["EndTimeUnixNano"][li])
                     - int(df["EndTimeUnixNano"][ci])) / 1000000)
                pairs.append({"kind": "trace-latency-pair", "file": rel,
                              "child_row": int(ci), "parent_row": int(li)})
        value = (float(np.percentile(latencies, 90))
                 if len(latencies) > 2 else 10.0)
        der = {"kind": "trace-derived-p90", "inputs": pairs,
               "value": value, "n_samples": len(latencies)}
    if float(der["value"]) != float(expected_value):
        raise RuntimeError(
            f"NetworkP90 shadow derivation mismatch for {pod}: "
            f"replicated {der['value']!r} vs artifact {expected_value!r}")
    return der


def build_derivations(window, data_root_abs, code_dir, ns, metric_list,
                      alarm_list):
    """One verified derivation per (alarmed pod, metric_type)."""
    values = {}
    for pod_metric in metric_list:
        for m in pod_metric["metrics"]:
            key = (pod_metric["pod"], m["metric_type"])
            values.setdefault(key, m["metric_value"])

    derivations = {}
    for alarm in alarm_list:
        pod = alarm["pod"]
        for entry in alarm["alarm"]:
            mt = entry["metric_type"]
            key = f"{pod}|{mt}"
            if key in derivations:
                continue
            expected = values.get((pod, mt))
            if expected is None:
                raise RuntimeError(
                    f"alarm for ({pod}, {mt}) has no metric_list value")
            if mt in CPU_MEM:
                der = {"kind": "metric-sample",
                       "inputs": metric_sample_inputs(
                           window, data_root_abs, code_dir, pod, mt,
                           expected),
                       "value": float(expected)}
                der["computed_by"] = "alarm.get_metric_with_time"
            else:
                der = network_p90_derivation(
                    window, data_root_abs, code_dir, pod, expected)
                der["computed_by"] = "alarm.get_netwrok_metric (replicated)"
            der["threshold_rule"] = threshold_rule(mt, ns)
            der["verified"] = True
            derivations[key] = der
    return derivations
