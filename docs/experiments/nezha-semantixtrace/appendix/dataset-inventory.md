# Nezha RCA Artifact — Forensic Dataset Inventory

- Repository: `/home/user/Nezha`
- Git commit verified: `d8140101fdb4e7dfb60d3ef9f64706f382b68470` (`Merge pull request #15 from zhjiang22/main`, 2025-05-20 16:03:05 +0800). Working tree clean.
- Inventory date: 2026-08-20
- SHA256 manifest: `dataset-manifest.sha256` (same directory as this file), **1238 entries**, generated with
  `find rca_data construct_data log_template metric_threshold log -type f | sort | xargs sha256sum`
  run from `/home/user/Nezha` (paths relative to repo root, globally sorted, deterministic).
  `find ... -type f | wc -l` independently returns 1238 — every file is in the manifest.

Total sizes (`du -sb` / `du -sh`):

| dir | bytes | human |
|---|---|---|
| rca_data | 2,965,396,038 | 2.8G |
| construct_data | 72,765,520 | 70M |
| log_template | 44,510 | 56K (on-disk) |
| metric_threshold | 11,546 | 240K (on-disk) |
| log | 690,469 | 688K (on-disk) |

---

## 1. Directory layout

Both `rca_data/` (fault-suffering) and `construct_data/` (fault-free) contain the same four date
directories, each with subdirs `log/`, `metric/`, `trace/`, `traceid/`:

- `2022-08-22`, `2022-08-23` — OnlineBoutique ("hipster")
- `2023-01-29`, `2023-01-30` — TrainTicket ("ts")

Extras:
- `rca_data/<date>/<date>-fault_list.json` — service-level ground truth (one per date; none in construct_data).
- `construct_data/root_cause_hipster.json`, `construct_data/root_cause_ts.json` — inner-service ground truth.

### 1.1 Per-subdir file counts and sizes

| path | files | bytes | naming pattern |
|---|---|---|---|
| rca_data/2022-08-22/log | 72 | 848,386,871 | `HH_MM_log.csv` |
| rca_data/2022-08-22/metric | 14 | 5,229,959 | `<pod>_metric.csv` + `front_service.csv`, `dependency.csv`, `source_50.csv`, `destination_50.csv` |
| rca_data/2022-08-22/trace | 72 | 277,247,453 | `HH_MM_trace.csv` |
| rca_data/2022-08-22/traceid | 72 | 1,137,015 | `HH_MM_traceid.csv` |
| rca_data/2022-08-23/log | 96 | 1,129,449,117 | `HH_MM_log.csv` |
| rca_data/2022-08-23/metric | 14 | 3,041,114 | same as above |
| rca_data/2022-08-23/trace | 96 | 369,249,639 | `HH_MM_trace.csv` |
| rca_data/2022-08-23/traceid | 96 | 1,550,406 | `HH_MM_traceid.csv` |
| rca_data/2023-01-29/log | 84 | 139,096,771 | `HH_MM_log.csv` |
| rca_data/2023-01-29/metric | 47 | 13,918,503 | 46 × `ts-*_metric.csv` + `front_service.csv` (NO dependency/source_50/destination_50) |
| rca_data/2023-01-29/trace | 84 | 56,647,259 | `HH_MM_trace.csv` |
| rca_data/2023-01-29/traceid | 84 | 151,470 | `HH_MM_traceid.csv` |
| rca_data/2023-01-30/log | 51 | 78,027,945 | `HH_MM_log.csv` |
| rca_data/2023-01-30/metric | 47 | 11,273,160 | same as 01-29 (different pod hashes) |
| rca_data/2023-01-30/trace | 51 | 30,884,647 | `HH_MM_trace.csv` |
| rca_data/2023-01-30/traceid | 51 | 82,038 | `HH_MM_traceid.csv` |
| construct_data/2022-08-22/log | 1 | 12,965,305 | `03_51_log.csv` |
| construct_data/2022-08-22/metric | 14 | 5,229,959 | byte-identical copy of rca_data metric dir (see anomalies) |
| construct_data/2022-08-22/trace | 1 | 4,261,034 | `03_51_trace.csv` |
| construct_data/2022-08-22/traceid | 1 | 16,499 | `03_51_traceid.csv` |
| construct_data/2022-08-23/log | 1 | 12,442,369 | `17_00_log.csv` |
| construct_data/2022-08-23/metric | 14 | 3,041,114 | identical to rca_data/2022-08-23/metric |
| construct_data/2022-08-23/trace | 1 | 4,105,111 | `17_00_trace.csv` |
| construct_data/2022-08-23/traceid | 1 | 16,499 | `17_00_traceid.csv` |
| construct_data/2023-01-29/log | 1 | 1,695,007 | `08_50_log.csv` |
| construct_data/2023-01-29/metric | 47 | 13,918,503 | identical to rca_data/2023-01-29/metric |
| construct_data/2023-01-29/trace | 1 | 840,259 | `08_50_trace.csv` |
| construct_data/2023-01-29/traceid | 1 | 2,177 | `08_50_traceid.csv` |
| construct_data/2023-01-30/log | 1 | 2,159,606 | `11_39_log.csv` |
| construct_data/2023-01-30/metric | 47 | 11,273,160 | identical to rca_data/2023-01-30/metric |
| construct_data/2023-01-30/trace | 1 | 792,660 | `11_39_trace.csv` |
| construct_data/2023-01-30/traceid | 1 | 2,013 | `11_39_traceid.csv` |

### 1.2 Minute-file structure

log/trace/traceid always come in matched triples per minute `HH_MM`. Each fault gets **3 one-minute
files** starting at the injection minute (3 files x number of faults, with hour-boundary carryover):

- 2022-08-22: 72 files = 24 faults x 3. Hour histogram (log/): 03->3, 04->21, 05->21, 06->9, 07->18. Range `03_53`..`07_55`.
- 2022-08-23: 96 = 32 x 3. Hours: 12->21, 13->21, 14->18, 15->21, 16->15. Range `12_01`..`16_42`.
- 2023-01-29: 84 = 28 x 3. Hours: 08->8, 09->27, 10->22, 11->15, 13->3, 14->6, 15->3. Range `08_43`..`15_44` (no hour 12 — no faults injected then).
- 2023-01-30: 51 = 17 x 3. Hours: 11->4, 12->21, 13->20, 14->6. Range `11_51`..`14_10`.

Note: a minute file's first rows can precede the named minute by ~30–70 s (e.g. `03_53_log.csv`
first row `2022-08-22T03:52:52Z`; `08_43_log.csv` first row `2023-01-29T08:42:22Z`).

construct_data snapshots are single minutes: hipster `03_51` (2 min before first fault on 08-22)
and `17_00` (after last fault on 08-23); ts `08_50` (between faults on 01-29) and `11_39` (12 min
before first fault on 01-30).

---

## 2. Schemas (from `head` of representative files)

### 2.1 `*_log.csv` (log/, both systems, rca and construct identical schema)

Header: `Timestamp,TimeUnixNano,Node,PodName,Container,TraceID,SpanID,Log`

Example rows:

- hipster (`rca_data/2022-08-22/log/03_53_log.csv`):
  `2022-08-22T03:52:52.373824039Z,1661140372373824039,33.33.33.115,productcatalogservice-668d5f85fb-wckp8,server,01f61383378244059db6af8716b51086,ea199712b1510bf1,"{""log"":""{\""message\"":\""TraceID: ... Query product with name and description successfully\"",\""severity\"":\""info\"",\""timestamp\"":\""2022-08-22T03:52:52.373824039Z\""}\n"",""stream"":""stdout"",""time"":""2022-08-22T03:52:52.373906819Z""}"`
- ts (`rca_data/2023-01-29/log/08_43_log.csv`):
  `2023-01-29T08:42:22.383150913Z,1674981742383150913,33.33.33.116,ts-travel-service-64469b5b48-25zj6,server,ffc8ca7e4c8f06c75142c880d5595d96,439bc949a9eeb0cc,"{""log"":""16:42:22.382 INFO t.s.TravelServiceImpl#532 TraceID: ... \n"",""stream"":""stdout"",""time"":""2023-01-29T08:42:22.383150913Z""}"`

The `Log` column embeds the raw Kubernetes JSON log line. Timezone hint: the ts application log
body carries local wall-clock `16:42:22` while the CSV `Timestamp` is `08:42:22Z` — cluster local
time is **UTC+8 (CST)**; CSV timestamps are genuine UTC.

### 2.2 `*_trace.csv` (trace/)

Header: `TraceID,SpanID,ParentID,PodName,OperationName,StartTimeUnixNano,EndTimeUnixNano,Duration`

Examples:
- hipster: `0087f677c0a2cf3f80a551987167911b,ee744a66e6efcb2e,root,frontend-579b9bff58-t2dbm,hipstershop.Frontend/Recv.,1661140374351773094,1661140374362982609,11209`
- ts: `b0103bb0161ad7fa352fe1828eeb643a,520b067330e3ede5,root,ts-gateway-service-6f6cfc45b-h5m2n,/*,1674981768697000000,1674981769015316530,318316`

`ParentID` is `root` for root spans; `Duration` is in microseconds (End-Start in ns / 1000).

### 2.3 `*_traceid.csv` (traceid/)

No header; one 32-hex trace ID per line (e.g. `0087f677c0a2cf3f80a551987167911b`).

### 2.4 `<pod>_metric.csv` (metric/, per-pod, both systems)

Header (22 cols):
`Time,TimeStamp,PodName,CpuUsage(m),CpuUsageRate(%),MemoryUsage(Mi),MemoryUsageRate(%),SyscallRead,SyscallWrite,NetworkReceiveBytes,NetworkTransmitBytes,PodClientLatencyP90(s),PodServerLatencyP90(s),PodClientLatencyP95(s),PodServerLatencyP95(s),PodClientLatencyP99(s),PodServerLatencyP99(s),PodWorkload(Ops),PodSuccessRate(%),NodeCpuUsageRate(%),NodeMemoryUsageRate(%),NodeNetworkReceiveBytes`

Example row: `2022-08-22 03:51:19.607158889 +0000 UTC m=+0.113112869,1861140279,adservice-5f6585d649-fnmft,1.7624831358803779,5.874943786281926,201.23828125,...`
(`Time` is a Go time.String() with monotonic clock suffix `m=+...`, explicitly `+0000 UTC`;
`TimeStamp` is epoch seconds; 1-minute sampling. NOTE the `1861...` value above is the corrupted
adservice file — see anomalies; all other files carry `1661...`/`1674...`/`1675...`.)

### 2.5 `front_service.csv` (metric/, one per date)

Header: `Time,TimeStamp,ServiceName,SuccessRate(%),LatencyP50(s),LatencyP90(s),LatencyP95(s),LatencyP99(s)`
Example: `2022-08-22 03:51:19.573905004 +0000 UTC m=+0.079858984,1661140279,Frontend,100,0.2406...,1.1499...,1.8249...,2.365`
ts example first row has `SuccessRate(%) = 0`: `2023-01-29 08:41:09.043... +0000 UTC ...,1674981669,Frontend,0,0.0928...,0.4874...,0.7249...,0.9450...`

### 2.6 `dependency.csv`, `source_50.csv`, `destination_50.csv` (hipster metric/ only)

- `dependency.csv` — header `Source,Target,Weight`; service-level call edges, e.g. `checkoutservice,emailservice,1` (27 lines incl. header).
- `source_50.csv` / `destination_50.csv` — header `time,timestamp,<caller>_<callee>,...` (14 pair columns); per-minute P50 latencies per dependency edge, grouped by source/destination side. Example row: `2022-08-22 03:51:20.021338403 +0000 UTC m=+0.527292383,1661140280,0.004666667,0.003,...`
- These three files DO NOT exist for TrainTicket dates (47 metric files there = 46 pods + front_service).

### 2.7 Pod name sets per date (metric filenames == pods in fault lists)

- 2022-08-22 hipster pods: adservice-5f6585d649-fnmft, cartservice-579f59597d-wc2lz, checkoutservice-578fcf4766-9csqn, currencyservice-cf787dd48-vpjrd, emailservice-55fdc5b988-f6xth, frontend-579b9bff58-t2dbm, paymentservice-9cdb6588f-554sm, productcatalogservice-668d5f85fb-**wckp8**, recommendationservice-6cfdd55578-gfj6q, shippingservice-7b598fc7d-lmggd (10 pods).
- 2022-08-23: identical except productcatalogservice-668d5f85fb-**jhwx9** (pod restarted between days).
- 2023-01-29 vs 2023-01-30: all 46 ts pod replica-hashes differ between the two days (full redeploy); deployment hashes are the same, only the random pod suffix changes (e.g. ts-basic-service-5dc8d4f9fd-46997 -> -llznp). The fault lists use the matching day's suffixes.

---

## 3. Row counts (raw `wc -l` line totals, aggregated per date per type)

log/trace/metric CSVs each have 1 header line per file; traceid has no header.
Data rows = lines − (#files) for log/trace/metric; = lines for traceid.

| path | log | trace | traceid | metric |
|---|---|---|---|---|
| rca_data/2022-08-22 | 1,672,707 | 1,550,145 | 34,455 | 15,755 |
| rca_data/2022-08-23 | 2,235,442 | 2,065,953 | 46,982 | 9,410 |
| rca_data/2023-01-29 | 172,805 | 328,931 | 4,590 | 43,240 |
| rca_data/2023-01-30 | 94,754 | 179,221 | 2,486 | 34,874 |
| construct_data/2022-08-22 | 25,575 | 23,843 | 499 | 15,755 |
| construct_data/2022-08-23 | 24,649 | 22,970 | 499 | 9,410 |
| construct_data/2023-01-29 | 2,489 | 4,890 | 65 | 43,240 |
| construct_data/2023-01-30 | 2,486 | 4,596 | 61 | 34,874 |

Per-file metric data rows (uniform within a date): 2022-08-22 -> 1,209 rows/file (dependency.csv 26,
source_50/destination_50 1,208); 2022-08-23 -> 721 (front_service & pods 721, source/destination 720,
dependency 26); 2023-01-29 -> 919; 2023-01-30 -> 741. Metric files cover the whole capture day at
1-minute resolution, not just fault windows.

---

## 4. Ground truth — fault lists (service level), full content

Format: JSON object keyed by hour string ("03".."16"); each value is a list of entries with fields
`inject_time` (string), `inject_timestamp` (epoch-seconds string), `inject_pod`, `inject_type`.
All 4 files parse cleanly with `json.load`.

### 4.1 rca_data/2022-08-22/2022-08-22-fault_list.json — 24 entries (1+7+7+3+6 across hours 03/04/05/06/07)

```
03 {"inject_time": "2022-08-22 03:53:54", "inject_timestamp": "1661140434", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "cpu_contention"}
04 {"inject_time": "2022-08-22 04:02:07", "inject_timestamp": "1661140927", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "return"}
04 {"inject_time": "2022-08-22 04:10:20", "inject_timestamp": "1661141420", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "cpu_consumed"}
04 {"inject_time": "2022-08-22 04:18:35", "inject_timestamp": "1661141915", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "exception"}
04 {"inject_time": "2022-08-22 04:27:03", "inject_timestamp": "1661142423", "inject_pod": "cartservice-579f59597d-wc2lz", "inject_type": "network_delay"}
04 {"inject_time": "2022-08-22 04:36:03", "inject_timestamp": "1661142963", "inject_pod": "cartservice-579f59597d-wc2lz", "inject_type": "cpu_contention"}
04 {"inject_time": "2022-08-22 04:44:16", "inject_timestamp": "1661143456", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "cpu_contention"}
04 {"inject_time": "2022-08-22 04:52:55", "inject_timestamp": "1661143975", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "network_delay"}
05 {"inject_time": "2022-08-22 05:01:13", "inject_timestamp": "1661144473", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "exception"}
05 {"inject_time": "2022-08-22 05:09:50", "inject_timestamp": "1661144990", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "return"}
05 {"inject_time": "2022-08-22 05:18:09", "inject_timestamp": "1661145489", "inject_pod": "currencyservice-cf787dd48-vpjrd", "inject_type": "cpu_contention"}
05 {"inject_time": "2022-08-22 05:27:06", "inject_timestamp": "1661146026", "inject_pod": "currencyservice-cf787dd48-vpjrd", "inject_type": "network_delay"}
05 {"inject_time": "2022-08-22 05:35:34", "inject_timestamp": "1661146534", "inject_pod": "emailservice-55fdc5b988-f6xth", "inject_type": "cpu_contention"}
05 {"inject_time": "2022-08-22 05:43:48", "inject_timestamp": "1661147028", "inject_pod": "emailservice-55fdc5b988-f6xth", "inject_type": "network_delay"}
05 {"inject_time": "2022-08-22 05:52:54", "inject_timestamp": "1661147574", "inject_pod": "paymentservice-9cdb6588f-554sm", "inject_type": "network_delay"}
06 {"inject_time": "2022-08-22 06:01:18", "inject_timestamp": "1661148078", "inject_pod": "paymentservice-9cdb6588f-554sm", "inject_type": "cpu_contention"}
06 {"inject_time": "2022-08-22 06:35:39", "inject_timestamp": "1661150139", "inject_pod": "productcatalogservice-668d5f85fb-wckp8", "inject_type": "cpu_consumed"}
06 {"inject_time": "2022-08-22 06:52:56", "inject_timestamp": "1661151176", "inject_pod": "recommendationservice-6cfdd55578-gfj6q", "inject_type": "network_delay"}
07 {"inject_time": "2022-08-22 07:01:25", "inject_timestamp": "1661151685", "inject_pod": "recommendationservice-6cfdd55578-gfj6q", "inject_type": "cpu_contention"}
07 {"inject_time": "2022-08-22 07:10:43", "inject_timestamp": "1661152183", "inject_pod": "recommendationservice-6cfdd55578-gfj6q", "inject_type": "cpu_consumed"}
07 {"inject_time": "2022-08-22 07:17:55", "inject_timestamp": "1661152675", "inject_pod": "shippingservice-7b598fc7d-lmggd", "inject_type": "network_delay"}
07 {"inject_time": "2022-08-22 07:26:48", "inject_timestamp": "1661153208", "inject_pod": "shippingservice-7b598fc7d-lmggd", "inject_type": "cpu_contention"}
07 {"inject_time": "2022-08-22 07:44:20", "inject_timestamp": "1661154260", "inject_pod": "adservice-5f6585d649-fnmft", "inject_type": "return"}
07 {"inject_time": "2022-08-22 07:53:33", "inject_timestamp": "1661154813", "inject_pod": "adservice-5f6585d649-fnmft", "inject_type": "exception"}
```

### 4.2 rca_data/2022-08-23/2022-08-23-fault_list.json — 32 entries (7+7+6+7+5 across hours 12/13/14/15/16)

```
12 {"inject_time": "2022-08-23 12:01:25", "inject_timestamp": "1661256085", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "cpu_contention"}
12 {"inject_time": "2022-08-23 12:10:21", "inject_timestamp": "1661256621", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "return"}
12 {"inject_time": "2022-08-23 12:18:23", "inject_timestamp": "1661257103", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "cpu_consumed"}
12 {"inject_time": "2022-08-23 12:27:23", "inject_timestamp": "1661257643", "inject_pod": "frontend-579b9bff58-t2dbm", "inject_type": "exception"}
12 {"inject_time": "2022-08-23 12:35:57", "inject_timestamp": "1661258157", "inject_pod": "cartservice-579f59597d-wc2lz", "inject_type": "network_delay"}
12 {"inject_time": "2022-08-23 12:45:08", "inject_timestamp": "1661258708", "inject_pod": "cartservice-579f59597d-wc2lz", "inject_type": "cpu_contention"}
12 {"inject_time": "2022-08-23 12:53:52", "inject_timestamp": "1661259232", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "network_delay"}
13 {"inject_time": "2022-08-23 13:02:09", "inject_timestamp": "1661259729", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "exception"}
13 {"inject_time": "2022-08-23 13:10:36", "inject_timestamp": "1661260236", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "return"}
13 {"inject_time": "2022-08-23 13:19:06", "inject_timestamp": "1661260746", "inject_pod": "checkoutservice-578fcf4766-9csqn", "inject_type": "cpu_consumed"}
13 {"inject_time": "2022-08-23 13:27:51", "inject_timestamp": "1661261271", "inject_pod": "currencyservice-cf787dd48-vpjrd", "inject_type": "cpu_contention"}
13 {"inject_time": "2022-08-23 13:37:08", "inject_timestamp": "1661261828", "inject_pod": "currencyservice-cf787dd48-vpjrd", "inject_type": "network_delay"}
13 {"inject_time": "2022-08-23 13:45:53", "inject_timestamp": "1661262353", "inject_pod": "emailservice-55fdc5b988-f6xth", "inject_type": "cpu_contention"}
13 {"inject_time": "2022-08-23 13:54:37", "inject_timestamp": "1661262877", "inject_pod": "emailservice-55fdc5b988-f6xth", "inject_type": "network_delay"}
14 {"inject_time": "2022-08-23 14:03:48", "inject_timestamp": "1661263428", "inject_pod": "emailservice-55fdc5b988-f6xth", "inject_type": "cpu_consumed"}
14 {"inject_time": "2022-08-23 14:12:52", "inject_timestamp": "1661263972", "inject_pod": "paymentservice-9cdb6588f-554sm", "inject_type": "network_delay"}
14 {"inject_time": "2022-08-23 14:21:54", "inject_timestamp": "1661264514", "inject_pod": "paymentservice-9cdb6588f-554sm", "inject_type": "cpu_contention"}
14 {"inject_time": "2022-08-23 14:30:10", "inject_timestamp": "1661265010", "inject_pod": "productcatalogservice-668d5f85fb-jhwx9", "inject_type": "network_delay"}
14 {"inject_time": "2022-08-23 14:47:40", "inject_timestamp": "1661266060", "inject_pod": "productcatalogservice-668d5f85fb-jhwx9", "inject_type": "return"}
14 {"inject_time": "2022-08-23 14:55:55", "inject_timestamp": "1661266555", "inject_pod": "productcatalogservice-668d5f85fb-jhwx9", "inject_type": "cpu_consumed"}
15 {"inject_time": "2022-08-23 15:04:28", "inject_timestamp": "1661267068", "inject_pod": "productcatalogservice-668d5f85fb-jhwx9", "inject_type": "exception"}
15 {"inject_time": "2022-08-23 15:12:49", "inject_timestamp": "1661267569", "inject_pod": "recommendationservice-6cfdd55578-gfj6q", "inject_type": "network_delay"}
15 {"inject_time": "2022-08-23 15:21:41", "inject_timestamp": "1661268101", "inject_pod": "recommendationservice-6cfdd55578-gfj6q", "inject_type": "cpu_contention"}
15 {"inject_time": "2022-08-23 15:29:49", "inject_timestamp": "1661268589", "inject_pod": "recommendationservice-6cfdd55578-gfj6q", "inject_type": "cpu_consumed"}
15 {"inject_time": "2022-08-23 15:38:52", "inject_timestamp": "1661269132", "inject_pod": "shippingservice-7b598fc7d-lmggd", "inject_type": "network_delay"}
15 {"inject_time": "2022-08-23 15:47:43", "inject_timestamp": "1661269663", "inject_pod": "shippingservice-7b598fc7d-lmggd", "inject_type": "cpu_contention"}
15 {"inject_time": "2022-08-23 15:56:24", "inject_timestamp": "1661270184", "inject_pod": "shippingservice-7b598fc7d-lmggd", "inject_type": "cpu_consumed"}
16 {"inject_time": "2022-08-23 16:05:02", "inject_timestamp": "1661270702", "inject_pod": "adservice-5f6585d649-fnmft", "inject_type": "network_delay"}
16 {"inject_time": "2022-08-23 16:13:38", "inject_timestamp": "1661271218", "inject_pod": "adservice-5f6585d649-fnmft", "inject_type": "return"}
16 {"inject_time": "2022-08-23 16:22:53", "inject_timestamp": "1661271773", "inject_pod": "adservice-5f6585d649-fnmft", "inject_type": "exception"}
16 {"inject_time": "2022-08-23 16:31:35", "inject_timestamp": "1661272295", "inject_pod": "adservice-5f6585d649-fnmft", "inject_type": "cpu_contention"}
16 {"inject_time": "2022-08-23 16:40:57", "inject_timestamp": "1661272857", "inject_pod": "adservice-5f6585d649-fnmft", "inject_type": "cpu_consumed"}
```

### 4.3 rca_data/2023-01-29/2023-01-29-fault_list.json — 28 entries (3+9+7+5+1+2+1 across hours 08/09/10/11/13/14/15)

```
08 {"inject_time": "2023-01-29 08:43:04", "inject_timestamp": "1674953044", "inject_pod": "ts-contacts-service-866bd68c97-xcqfx", "inject_type": "return"}
08 {"inject_time": "2023-01-29 08:52:06", "inject_timestamp": "1674953526", "inject_pod": "ts-contacts-service-866bd68c97-xcqfx", "inject_type": "return"}
08 {"inject_time": "2023-01-29 08:58:16", "inject_timestamp": "1674953896", "inject_pod": "ts-contacts-service-866bd68c97-xcqfx", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:03:53", "inject_timestamp": "1674954273", "inject_pod": "ts-basic-service-5dc8d4f9fd-46997", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:12:56", "inject_timestamp": "1674954273", "inject_pod": "ts-basic-service-5dc8d4f9fd-46997", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:21:39", "inject_timestamp": "1674955299", "inject_pod": "ts-basic-service-5dc8d4f9fd-46997", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:25:39", "inject_timestamp": "1674955539", "inject_pod": "ts-basic-service-5dc8d4f9fd-46997", "inject_type": "exception"}
09 {"inject_time": "2023-01-29 09:30:09", "inject_timestamp": "1674955899", "inject_pod": "ts-food-service-f5756978c-k8vqf", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:34:19", "inject_timestamp": "1674956119", "inject_pod": "ts-food-service-f5756978c-k8vqf", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:42:26", "inject_timestamp": "1674956546", "inject_pod": "ts-verification-code-service-7b6dc75c45-2z9p2", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:49:34", "inject_timestamp": "1674956974", "inject_pod": "ts-verification-code-service-7b6dc75c45-2z9p2", "inject_type": "return"}
09 {"inject_time": "2023-01-29 09:58:04", "inject_timestamp": "1674957484", "inject_pod": "ts-verification-code-service-7b6dc75c45-2z9p2", "inject_type": "return"}
10 {"inject_time": "2023-01-29 10:04:34", "inject_timestamp": "1674957874", "inject_pod": "ts-travel-service-64469b5b48-25zj6", "inject_type": "exception"}
10 {"inject_time": "2023-01-29 10:12:48", "inject_timestamp": "1674958388", "inject_pod": "ts-travel-service-64469b5b48-25zj6", "inject_type": "exception"}
10 {"inject_time": "2023-01-29 10:20:10", "inject_timestamp": "1674958810", "inject_pod": "ts-travel-service-64469b5b48-25zj6", "inject_type": "exception"}
10 {"inject_time": "2023-01-29 10:26:20", "inject_timestamp": "1674959180", "inject_pod": "ts-travel2-service-5c66d57d58-6mp2b", "inject_type": "exception"}
10 {"inject_time": "2023-01-29 10:45:10", "inject_timestamp": "1674960250", "inject_pod": "ts-travel2-service-5c66d57d58-6mp2b", "inject_type": "exception"}
10 {"inject_time": "2023-01-29 10:50:16", "inject_timestamp": "1674960616", "inject_pod": "ts-travel2-service-5c66d57d58-6mp2b", "inject_type": "exception"}
10 {"inject_time": "2023-01-29 10:56:45", "inject_timestamp": "1674961005", "inject_pod": "ts-route-service-b6c59fb57-zv8dt", "inject_type": "exception"}
11 {"inject_time": "2023-01-29 11:04:05", "inject_timestamp": "1674961505", "inject_pod": "ts-route-service-b6c59fb57-zv8dt", "inject_type": "exception"}
11 {"inject_time": "2023-01-29 11:13:56", "inject_timestamp": "1674962036", "inject_pod": "ts-route-service-b6c59fb57-zv8dt", "inject_type": "exception"}
11 {"inject_time": "2023-01-29 11:19:59", "inject_timestamp": "1674962399", "inject_pod": "ts-price-service-76846864d9-g465z", "inject_type": "exception"}
11 {"inject_time": "2023-01-29 11:29:03", "inject_timestamp": "1674962943", "inject_pod": "ts-price-service-76846864d9-g465z", "inject_type": "exception"}
11 {"inject_time": "2023-01-29 11:36:06", "inject_timestamp": "1674963426", "inject_pod": "ts-price-service-76846864d9-g465z", "inject_type": "exception"}
13 {"inject_time": "2023-01-29 13:31:54", "inject_timestamp": "1674999114", "inject_pod": "ts-contacts-service-866bd68c97-xcqfx", "inject_type": "cpu_contention"}
14 {"inject_time": "2023-01-29 14:23:31", "inject_timestamp": "1675002211", "inject_pod": "ts-verification-code-service-7b6dc75c45-2z9p2", "inject_type": "cpu_contention"}
14 {"inject_time": "2023-01-29 14:49:57", "inject_timestamp": "1675003797", "inject_pod": "ts-food-service-f5756978c-k8vqf", "inject_type": "cpu_contention"}
15 {"inject_time": "2023-01-29 15:42:00", "inject_timestamp": "1675006920", "inject_pod": "ts-preserve-service-b5ccf8557-j4txs", "inject_type": "cpu_contention"}
```

### 4.4 rca_data/2023-01-30/2023-01-30-fault_list.json — 17 entries (2+7+6+2 across hours 11/12/13/14)

```
11 {"inject_time": "2023-01-30 11:51:46", "inject_timestamp": "1675079506", "inject_pod": "ts-contacts-service-866bd68c97-dzqgd", "inject_type": "network_delay"}
11 {"inject_time": "2023-01-30 11:59:21", "inject_timestamp": "1675079961", "inject_pod": "ts-basic-service-5dc8d4f9fd-llznp", "inject_type": "cpu_contention"}
12 {"inject_time": "2023-01-30 12:06:12", "inject_timestamp": "1675080432", "inject_pod": "ts-basic-service-5dc8d4f9fd-llznp", "inject_type": "network_delay"}
12 {"inject_time": "2023-01-30 12:15:56", "inject_timestamp": "1675080896", "inject_pod": "ts-basic-service-5dc8d4f9fd-llznp", "inject_type": "network_delay"}
12 {"inject_time": "2023-01-30 12:29:29", "inject_timestamp": "1675081769", "inject_pod": "ts-verification-code-service-7b6dc75c45-r66rb", "inject_type": "network_delay"}
12 {"inject_time": "2023-01-30 12:36:54", "inject_timestamp": "1675082214", "inject_pod": "ts-verification-code-service-7b6dc75c45-r66rb", "inject_type": "network_delay"}
12 {"inject_time": "2023-01-30 12:44:36", "inject_timestamp": "1675082676", "inject_pod": "ts-food-service-f5756978c-6sb8t", "inject_type": "cpu_contention"}
12 {"inject_time": "2023-01-30 12:52:04", "inject_timestamp": "1675083124", "inject_pod": "ts-food-service-f5756978c-6sb8t", "inject_type": "network_delay"}
12 {"inject_time": "2023-01-30 12:59:36", "inject_timestamp": "1675083576", "inject_pod": "ts-food-service-f5756978c-6sb8t", "inject_type": "network_delay"}
13 {"inject_time": "2023-01-30 13:06:49", "inject_timestamp": "1675084009", "inject_pod": "ts-travel-service-64469b5b48-5rjvb", "inject_type": "cpu_contention"}
13 {"inject_time": "2023-01-30 13:15:43", "inject_timestamp": "1675084483", "inject_pod": "ts-travel-service-64469b5b48-5rjvb", "inject_type": "network_delay"}
13 {"inject_time": "2023-01-30 13:22:22", "inject_timestamp": "1675084942", "inject_pod": "ts-travel-service-64469b5b48-5rjvb", "inject_type": "network_delay"}
13 {"inject_time": "2023-01-30 13:30:52", "inject_timestamp": "1675085392", "inject_pod": "ts-preserve-service-b5ccf8557-l5l4p", "inject_type": "network_delay"}
13 {"inject_time": "2023-01-30 13:44:44", "inject_timestamp": "1675086284", "inject_pod": "ts-route-service-b6c59fb57-dz9bw", "inject_type": "network_delay"}
13 {"inject_time": "2023-01-30 13:52:20", "inject_timestamp": "1675086740", "inject_pod": "ts-route-service-b6c59fb57-dz9bw", "inject_type": "network_delay"}
14 {"inject_time": "2023-01-30 14:00:16", "inject_timestamp": "1675087216", "inject_pod": "ts-security-service-cb9788d56-bdkcx", "inject_type": "network_delay"}
14 {"inject_time": "2023-01-30 14:08:19", "inject_timestamp": "1675087699", "inject_pod": "ts-security-service-cb9788d56-bdkcx", "inject_type": "network_delay"}
```

### 4.5 Fault count reconciliation

| system | dates | counts | sum | paper/README claim | match |
|---|---|---|---|---|---|
| OnlineBoutique (hipster) | 2022-08-22 + 2022-08-23 | 24 + 32 | **56** | 56 | YES |
| TrainTicket (ts) | 2023-01-29 + 2023-01-30 | 28 + 17 | **45** | 45 | YES |

Fault type distribution (counted): hipster 56 = cpu_contention 16, network_delay 16, cpu_consumed 10,
return 7, exception 7. ts 45 = network_delay 14, exception 13, return 11, cpu_contention 7 (0 cpu_consumed).

---

## 5. Inner-service ground truth (verbatim)

### 5.1 construct_data/root_cause_hipster.json (1,969 bytes)

```json
{
    "frontend": {
        "return": "Serving product page started_GetProduct start",
        "exception": "Placing order started_Order placed complete",
        "cpu_consumed": "CpuUsageRate(%)",
        "cpu_contention": "CpuUsageRate(%)"
    },
    "checkoutservice": {
        "return": "Start charge card_Charge successfully",
        "exception": "Start charge card_Charge successfully",
        "network_delay": "NetworkP90(ms)",
        "cpu_contention": "CpuUsageRate(%)",
        "cpu_consumed": "CpuUsageRate(%)"
    },
    "cartservice": {
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "emailservice": {
        "cpu_consumed": "CpuUsageRate(%)",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "currencyservice": {
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "recommendationservice": {
        "cpu_consumed": "CpuUsageRate(%)",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "paymentservice": {
        "network_delay": "NetworkP90(ms)",
        "cpu_contention": "CpuUsageRate(%)"
    },
    "productcatalogservice": {
        "network_delay": "NetworkP90(ms)",
        "cpu_consumed": "CpuUsageRate(%)",
        "cpu_contention": "CpuUsageRate(%)",
        "return": "Query product with name_Query product successfully",
        "exception": "Query product with name_Query product successfully"
    },
    "shippingservice": {
        "cpu_consumed": "CpuUsageRate(%)",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "adservice": {
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)",
        "return": "Received ad request_No context provided",
        "exception": "Received ad request_No context provided",
        "cpu_consumed": "CpuUsageRate(%)"
    }
}
```

### 5.2 construct_data/root_cause_ts.json (2,276 bytes)

```json
{
    "ts-contacts-service": {
        "return": "c.s.ContactsServiceImpl#31_c.s.ContactsServiceImpl#34",
        "cpu_consumed": "CpuUsageRate(%)",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-basic-service": {
        "return": "f.m.s.BasicServiceImpl#405_f.m.s.BasicServiceImpl#418",
        "exception": "f.m.s.BasicServiceImpl#100_f.m.s.BasicServiceImpl#119",
        "cpu_consumed": "CpuUsageRate(%)",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-food-service": {
        "return": "f.s.FoodServiceImpl#240_f.s.FoodServiceImpl#243",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-verification-code-service": {
        "return": "v.s.i.VerifyCodeServiceImpl#114_v.s.i.VerifyCodeServiceImpl#132",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-delivery-service": {
        "return": "Delivery service_Receive delivery object",
        "exception": "Receive delivery object_Save delivery object into database success",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-travel-service": {
        "exception": "t.s.TravelServiceImpl#451_t.s.TravelServiceImpl#457",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-travel2-service": {
        "exception": "t.s.TravelServiceImpl#366_t.s.TravelServiceImpl#372",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-preserve-service": {
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-route-service": {
        "exception": "RouteController.queryByIds start_r.c.RouteController#53",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-security-service": {
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    },
    "ts-price-service": {
        "exception": "PriceController.query start_p.c.PriceController#46",
        "cpu_contention": "CpuUsageRate(%)",
        "network_delay": "NetworkP90(ms)"
    }
}
```

Note: `ts-delivery-service` has labels in root_cause_ts.json but no fault injected on it in either
fault list; conversely every injected (service, type) pair for return/exception faults appears here.

---

## 6. metric_threshold/ — 59 files

Files: 10 hipster service names (adservice, cartservice, checkoutservice, currencyservice,
emailservice, frontend, paymentservice, productcatalogservice, recommendationservice,
shippingservice) + `front_service_total.csv` + `mysql.csv` + `redis.csv` + 46 `ts-*` services.
These are per-SERVICE (pod suffix stripped), produced by `alarm.py:generate_threshold()` which,
per its docstring, "calculte mean and std for each metric of each servie" from construction-phase
metrics. Row 1 = mean, row 2 = std, used as fixed alarm thresholds (mean +/- k*std) by `alarm.py`.

Schema (service files): `CpuUsageRate(%),MemoryUsageRate(%),SyscallRead,SyscallWrite,NetworkP90(ms)` — 2 data rows.
Examples (full content):

- `adservice.csv`:
  ```
  CpuUsageRate(%),MemoryUsageRate(%),SyscallRead,SyscallWrite,NetworkP90(ms)
  8.148346470503457,81.12354461477993,0.0,0.0,3.4771357999999988
  8.77308803013819,6.50018559104104,0.0,0.0,0
  ```
- `ts-basic-service.csv`:
  ```
  CpuUsageRate(%),MemoryUsageRate(%),SyscallRead,SyscallWrite,NetworkP90(ms)
  1.2292118634893148,23.259260583524068,0.0,0.0,200.8420170000000002
  1.1606730484095311,1.7436570900563553,0.0,0.0,2.3269441484440163
  ```
- `front_service_total.csv` (different schema):
  ```
  SuccessRate(%),LatencyP99(s),LatencyP95(s)
  100.0,0.36603187805171283,0.2374710095966961
  0.0,0.13529049538410443,0.03716813377618471
  ```
- `mysql.csv` and `redis.csv` (different schema: `CpuUsageRate(%),MemoryUsageRate(%),SyscallRead,SyscallWrite,PodServerLatencyP90(s),PodClientLatencyP90(s)`) are **all zeros** in both rows; no mysql/redis pod metric files exist anywhere in rca_data/construct_data.

---

## 7. log_template/ — Drain3 configs and state

Files: `drain3_hipster.ini` (2,414 B), `drain3_ts.ini` (272 B), `hipster.bin` (21,920 B), `ts.bin` (19,904 B).

`.bin` files are ASCII: base64(zlib(jsonpickle)) serialized Drain3 `TemplateMiner` state
(top-level keys `py/object, log_cluster_depth, max_node_depth, sim_th, max_children, root_node,
profiler, extra_delimiters, max_clusters, param_str`); `clusters_counter` = **674** (hipster),
**694** (ts). Both decode and json-parse cleanly.

`drain3_hipster.ini` (key content): `[SNAPSHOT] snapshot_interval_minutes=10, compress_state=True`;
`[MASKING]` with 22 regex rules masking ITEM (11 lookbehind rules for e.g. `get results:`,
`cart items:`, `context_words=`, `visa ending`, `invoked with request`, `Query cost of products`,
`Quote shipping with items`, `user_id`, `query cost of products`, `currency_code:`, `product_ids=`),
plus TRACEID (32-hex), SPANID (16 alnum), PRODUCTID (10 upper-alnum), USERID (uuid), AMOUNT, ID
(colon-separated hex), IP (dotted quad), SEQ (7-hex), HEX (0x...), NUM (integers), CMD
(`executed cmd "..."`); `mask_prefix=<:`, `mask_suffix=:>`; `[DRAIN] sim_th=0.9, depth=4,
max_children=100, max_clusters=1024, extra_delimiters=["_"]`; `[PROFILING] enabled=True, report_sec=30`.

`drain3_ts.ini`: identical except `[MASKING] masking = []` (empty) and `[DRAIN] sim_th=0.8`.

---

## 8. log/ — committed result logs (authors' outputs)

| file | bytes | lines | internal run date |
|---|---|---|---|
| OnlineBoutique_innerservice_result.log | 120,690 | 607 | 2023-08-20 (01:49–03:06 local) |
| OnlineBoutique_service_result.log | 110,779 | 562 | 2023-08-20 |
| Trainticket_innerservice_Result.log | 233,790 | 805 | 2023-08-20 |
| Trainticket_service_result.log | 225,210 | 754 | 2023-08-17 |

(Note the filename inconsistency: capital `R` in `Trainticket_innerservice_Result.log`.)

Each log contains, per fault, `Soted Result List:` (sic), `<inject_time> Inject RCA (Pod) Result:`,
`Inject Ground Truth: <pod>, <type>` and, when ranked, `... score <rank>` lines, then a final block.
Final blocks, copied EXACTLY:

### 8.1 OnlineBoutique_service_result.log (tail)

```
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:609: [1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:622: -------- hipster Fault numbuer : 56-------
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:623: --------AS@1 Result-------
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:624: 92.857143 %
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:625: --------AS@3 Result-------
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:626: 96.428571 %
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:627: --------AS@5 Result-------
[INFO]2023-08-20 02:27:09,342 pattern_ranker.py:628: 96.428571 %
```

### 8.2 OnlineBoutique_innerservice_result.log (tail)

```
[INFO]2023-08-20 03:06:26,400 pattern_ranker.py:329: [1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]
[INFO]2023-08-20 03:06:26,401 pattern_ranker.py:343: -------- hipster Fault numbuer : 56-------
[INFO]2023-08-20 03:06:26,401 pattern_ranker.py:344: --------AIS@1 Result-------
[INFO]2023-08-20 03:06:26,401 pattern_ranker.py:345: 92.857143 %
[INFO]2023-08-20 03:06:26,401 pattern_ranker.py:346: --------AIS@3 Result-------
[INFO]2023-08-20 03:06:26,401 pattern_ranker.py:347: 96.428571 %
[INFO]2023-08-20 03:06:26,401 pattern_ranker.py:348: --------AIS@5 Result-------
[INFO]2023-08-20 03:06:26,401 pattern_ranker.py:349: 96.428571 %
```

### 8.3 Trainticket_service_result.log (tail)

```
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:609: [1, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 3, 1, 1, 1, 1, 1]
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:622: -------- ts Fault numbuer : 45-------
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:623: --------AS@1 Result-------
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:624: 86.666667 %
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:625: --------AS@3 Result-------
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:626: 97.777778 %
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:627: --------AS@5 Result-------
[INFO]2023-08-17 13:33:46,025 pattern_ranker.py:628: 97.777778 %
```

### 8.4 Trainticket_innerservice_Result.log (tail)

```
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:329: [1, 1, 1, 1, 1, 1, 1, 1, 3, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 3, 1, 1, 1, 1, 1]
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:343: -------- ts Fault numbuer : 45-------
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:344: --------AIS@1 Result-------
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:345: 86.666667 %
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:346: --------AIS@3 Result-------
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:347: 97.777778 %
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:348: --------AIS@5 Result-------
[INFO]2023-08-20 07:01:54,498 pattern_ranker.py:349: 97.777778 %
```

### 8.5 Rank-vector arithmetic (computed, not authors' text)

- Hipster (both levels): rank vector length **54** (52 ones, 2 twos) vs 56 evaluated faults
  ("Inject RCA Result" appears 56x) — 2 faults produced no ranked candidate. Percentages use the
  full 56 denominator: 52/56 = 92.857143%, 54/56 = 96.428571%. Reproduction target confirmed self-consistent.
- TS (both levels): rank vector length **44** (39 ones, 3 twos, 2 threes) vs 45 faults — 1 fault
  unranked. 39/45 = 86.666667%, 44/45 = 97.777778%. Self-consistent.
- README's quick-start blocks quote exactly these numbers. (README labels the inner-service blocks
  with line numbers 622–628, but the actual inner-service logs use pattern_ranker.py:343–349 —
  cosmetic copy-paste artifact.)
- Service-level and inner-service-level committed rank vectors are IDENTICAL per system.

---

## 9. Sanity checks and anomalies

1. **Empty files: none.** `find ... -type f -size 0` over all five dirs returns 0 files.
2. **JSON validity:** all 6 JSON files (4 fault lists + 2 root_cause) load cleanly with
   `python3 json.load`. Both drain3 `.bin` states decode (base64 -> zlib -> jsonpickle JSON) cleanly.
3. **README broken paths (all MISSING, checked with `test -e`):**
   - `rca_data/2022-08-22-fault_list` and `rca_data/2022-08-23-fault_list` (real files are inside
     the date subdir with `.json` suffix);
   - `rca_data/2022-01-29-fault_list` and `rca_data/2022-01-30-fault_list` — **wrong year** (2022
     vs 2023) AND wrong path; `rca_data/2023-01-29-fault_list` / `2023-01-30-fault_list` also do
     not exist at top level. Actual: `rca_data/<date>/<date>-fault_list.json`.
4. **Corrupted TimeStamp column in exactly one file** (both copies):
   `rca_data/2022-08-22/metric/adservice-5f6585d649-fnmft_metric.csv` (= identical
   construct_data copy). All 1,209 data rows have `TimeStamp` ~2x10^8 s in the future of the `Time`
   column (epoch prefix `18` instead of `16`, e.g. `1861140279` for `2022-08-22 03:51:19 UTC` =
   1661140279). Unique offsets observed: {200000000, 200000140, 200000200, 200000210, 200001400}.
   Every other metric file in all 8 date dirs has Time == TimeStamp within 1 s. Any reproduction
   code keying on this column for adservice on 08-22 will see year-2028 timestamps.
5. **construct_data metric dirs are byte-identical to rca_data metric dirs** for all 4 dates
   (`diff -rq` clean; same SHA256s). The "fault-free" metric data is literally the same full-day
   files as the fault-suffering data; only log/trace/traceid differ (single fault-free minute vs
   fault windows).
6. **Timezone findings:**
   - All CSV `Timestamp`/`Time` columns are UTC (`Z` / `+0000 UTC`). Cluster wall clock is UTC+8:
     ts application logs embed local time `16:42:22` for CSV time `08:42:22Z`.
   - Hipster fault lists: `inject_time` matches `inject_timestamp` interpreted as UTC (1 exception:
     2022-08-22 07:10:43 vs epoch 1661152183 = 07:09:43 UTC, 60 s off).
   - **2023-01-29 fault list:** for the 24 morning entries (hours 08–11), `inject_timestamp` is
     ~8 h BEHIND the `inject_time` string (e.g. 08:43:04 vs 1674953044 = 00:44:04 UTC = 08:44:04
     UTC+8). The string matches the data-file names/UTC data times; the epoch appears computed by
     re-interpreting the UTC string as UTC+8. The 4 afternoon entries (13:31:54, 14:23:31,
     14:49:57, 15:42:00) are UTC-consistent. Within the +8 h scheme several entries are also ±60 s
     off (08:43:04, 09:03:53, 09:12:56, 09:30:09, 09:34:19, 10:12:48, 10:45:10, 11:04:05, 11:36:06).
   - 2023-01-30 fault list: UTC-consistent except 4 entries ±60 s off (12:06:12, 12:15:56,
     13:15:43, 13:30:52).
   - **Duplicate epoch:** 2023-01-29 entries 09:03:53 and 09:12:56 share `inject_timestamp`
     "1674954273" (the second is wrong by ~9 min). Conclusion: `inject_time` strings + minute file
     names are the reliable keys; `inject_timestamp` is unreliable on 2023-01-29.
7. **Fault-to-file correspondence:** every fault has exactly 3 one-minute files in log/trace/traceid
   (file totals 72/96/84/51 = 3 x 24/32/28/17), with hour-boundary carryover (e.g. 11:59 fault on
   01-30 -> files 11_59, 12_00, 12_01).
8. **Coincidences, no data problem:** construct hipster traceid files for 08-22 and 08-23 are both
   499 lines / 16,499 bytes but different content (different SHA256, 0 common IDs).
9. Pod restarts between dates: productcatalogservice suffix changes 08-22 (wckp8) -> 08-23 (jhwx9);
   all 46 ts pod suffixes change 01-29 -> 01-30. Fault lists and metric filenames stay internally
   consistent per date.
10. `metric_threshold/mysql.csv` and `redis.csv` are all zeros and correspond to no pod metric
    files in the dataset; `front_service_total.csv` std row starts with `0.0` for SuccessRate.
11. ts front_service.csv on 2023-01-29 begins with `SuccessRate(%) = 0` in its first sample
    (08:41:09), i.e. the Frontend probe reports 0% success at capture start.
12. Trailing newlines present on spot-checked CSVs (no wc -l undercount); all files end with `\n`
    in the samples checked (log, traceid, threshold).
