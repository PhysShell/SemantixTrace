# 01 — Nezha artifact audit (claim ledger)

Phase E0 evidence document. Everything here is classified as one of:
**artifact claim** (what README/paper assert), **source-code fact** (verified
by reading the pinned code, with file:line references), **dataset fact**
(verified by inspecting shipped data), **third-party claim** (upstream issue
tracker, not verified here unless stated), **reproduced fact** (backed by a
machine artifact in `experiment/nezha/results/`), or **inference** (marked).

- Pinned commit: `d8140101fdb4e7dfb60d3ef9f64706f382b68470`
- Dataset manifest: `experiment/nezha/manifests/dataset-manifest.sha256` (1238 files)
- Environment: `experiment/nezha/manifests/environment.md`
- Appendices: `appendix/paper-claims.md` (verbatim paper extraction),
  `appendix/upstream-issues-audit.md`, `appendix/dataset-inventory.md`
- Reproduction status: **complete** — all four headline configurations
  reproduce the committed author logs exactly (`02-e0-reproduction.md`,
  machine artifacts in `experiment/nezha/results/e0/`); measurement audit
  in `03-e1-measurement-audit.md`.

## 0. Provenance of the artifact's own numbers

Source-code facts and dataset facts:

1. README headline blocks (OB 92.857143 / 96.428571 / 96.428571 over 56
   faults; TT 86.666667 / 97.777778 / 97.777778 over 45 faults; **identical
   for `--level service` and `--level inner`**) match the tails of the four
   committed author logs in `log/` byte-for-value (appendix/dataset-inventory.md §8).
2. The committed logs were produced 2023-08-17/20 by a `pattern_ranker.py`
   that differs from the pinned one **only** by six later-removed unused
   imports (verified: `git diff 32bd335 461b5b2 -- pattern_ranker.py`); the
   6-line shift exactly explains the log line-number offsets (343→337,
   622→616). The pinned algorithm is the one that produced the committed logs.
3. The committed rank vectors have length 54 (hipster) and 44 (ts) against
   56/45 evaluated faults: 2 + 1 faults produced no ranked hit; the percent
   values use full denominators (52/56, 39/45). Self-consistent.
4. The paper's Tables 3/4 print the same numbers for Nezha as the artifact
   logs, and the paper's AIS rows for Nezha are numerically identical to its
   AS rows (appendix/paper-claims.md §1) — the paper does not explain this.

## 1. Contract claims A–M, adjudicated against source

| # | Claim (from research contract) | Code agrees? | Where | Notes |
|---|---|---|---|---|
| A | Spans become start/end events | **YES, with extras** | data_integrate.py:254-263, 299-300 | Synthetic message is `"<service> <OperationName> start/end"`, then routed **through drain3** like a log line, so span events are drain3 cluster IDs too. Paper's async `_asyn` events (paper p.6) **do not exist in code** (grep: no hits). |
| B | Logs mapped through Drain3 templates/cluster IDs | **YES** | log_parsing.py:26-57 | Per-service hardcoded JSON unwrapping (log_parsing.py:80-153); ts logs additionally reduced to a `Class#line`-token via regex (137-145); regex failure logs an error and falls through; `json.loads` failure silently returns the raw line (151-153). |
| C | Metric anomalies become synthetic events (CpuUsageRate(%), MemoryUsageRate(%), NetworkP90(ms)) | **YES** | alarm.py:262-279; data_integrate.py:303-329 | The literal metric-type string is drain3-parsed as the event. |
| D | Intra-span edges connect consecutive events | **YES** | data_integrate.py:437-441 | After per-span timestamp sort (332). |
| E | Parent-child span relations create cross-span edges | **YES** | data_integrate.py:443-462 | Same-pod: child start attached inside parent's timeline by timestamp (450-456); cross-pod: parent's *first* event → child's first event (458-461), matching the paper's clock-drift rule. |
| F | Active "pattern miner" = adjacent event pair + occurrence count; CM-SPAM/TKG discarded | **YES** | pattern_miner.py:9-116 (all commented out), 119-134 (active); data_integrate.py:209-219 | Support = occurrence count summed across graphs (matches paper Def. 4). The paper itself never names a mining algorithm — "traversing all event graphs" (appendix/paper-claims.md §3c) — so code and paper agree more than the phrase "graph pattern mining" suggests. The paper's *Pattern Aggregator* (joining pairs into longer chains, p.7) has **no counterpart in code**; only root-most pruning exists. |
| G | Score ≈ normal/(normal+faulty), threshold ≈0.67, support floor ≈5 | **YES** | pattern_ranker.py:98-118 (expected), 64-89 (actual), floor `>5` at :67,:99, min_score=0.67 default | Floor is strictly `>5` (i.e. ≥6), slightly stronger than paper's "support less than s_min(=5)" wording. Patterns absent from the other phase get score 1.0. |
| H | Root-most/downstream pruning | **YES** | pattern_ranker.py:122-134 | Removes pattern whose source is another retained pattern's target with ≥ score, unless the target event is a metric event. |
| I-#12 | top-k undercount for resource candidates | **CONFIRMED in source** | pattern_ranker.py:266-280 (evaluation), 555-569 (evaluation_pod) | `topk` advances only on non-resource candidates; a non-matching *resource* candidate never advances the rank. Quantification in E1. |
| I-#11 | "actual pattern" sorted by event-ID keys, not scores | **CONFIRMED in source** | pattern_ranker.py:87 | `sorted(score_dict, reverse=True)` sorts dict *keys* (strings "id_id") lexicographically. Affects only the displayed "actual pattern" explanation (used at :314-318), **not** AS/AIS rank computation. |
| I-#10 | get_deepth_pod may cycle | **PLAUSIBLE in source; needs runtime check** | data_integrate.py:188-207 | The walk operates on drain3 *template IDs*, not graph nodes; distinct nodes sharing an ID are collapsed, so an ID-level cycle loops forever (`while True`). Whether the shipped data triggers it: E0 runs will show (they either hang or don't). |
| I-#14 | construct_data logs/traces incomplete | **DATASET FACT: single minute per date** | appendix/dataset-inventory.md §1.1 | construct_data has exactly one 1-minute slice per date (e.g. `03_51`). Whether that is "incomplete" or by design (paper: 1-minute construction window) is interpretation; the paper's threat-to-validity section itself flags fault-free data coverage. |
| I-metric-ts | historical metric timestamp defects | **CONFIRMED, but inert in active path** | dataset fact (appendix §9.4); alarm.py:268-270 | Exactly one file (adservice 2022-08-22, both copies) has `TimeStamp` ≈ +2×10⁸ s (issue #8). The active code matches on the `Time` *string* column via regex, never the `TimeStamp` epoch, so the corruption cannot affect the shipped pipeline. Any *external* reimplementation keying on epochs will be bitten. |
| I-baselines | paper baselines not in repo | **CONFIRMED** | repo tree; appendix/upstream-issues-audit.md (#5, #6, #9, #13) | No baseline code anywhere in the artifact; four unanswered upstream requests. Paper's data-availability statement covers only Nezha itself. |
| J | Metric alarms use fixed thresholds, not the paper's statistical description | **CONFIRMED** | alarm.py:151-189 | Active: CPU/Mem `> 80`, NetworkP90 `> 200` (hipster) / `> 300` (ts) — hardcoded (166-179). The mean/std (3-sigma) logic exists only as comments (180-189) and in the unused `generate_threshold`/`metric_threshold/` CSVs. Paper says alerts come from "the k-sigma rule or static thresholds" without values (appendix/paper-claims.md §4) — so the paper permits static thresholds but never discloses these values or that k-sigma is inactive. |
| K | generate_alarm keeps only the first alarming metric per pod | **CONFIRMED** | alarm.py:216-224 | The `alarm['alarm'].append(...)` sits *inside* `if "pod" not in alarm:`; a pod with CPU **and** Memory anomalies reports only whichever metric came first in the fixed metric order (CPU, Memory, NetworkP90). |
| L | Metric alarm events synthetically placed into request spans | **CONFIRMED** | data_integrate.py:303-329 | Alarm events are injected into **every span** of the alarmed pod (hipster: only spans with >2 events, "do not add alarm for client span"; ts: all spans) with fabricated timestamps `span_start + index + 1` ns. They are not naturally correlated telemetry. |
| M | AS (service) and AIS (inner-service) may not be distinct | **CONFIRMED — they are the same computation** | pattern_ranker.py:200-345 vs 488-624 | `evaluation` (labels AIS@k) and `evaluation_pod` (labels AS@k) contain *identical* candidate-matching logic: both match against the **inner-service** ground truth (`root_cause_<ns>.json`: resource type or log-template pair) **plus** the injected pod name. They differ only in log strings and one extra debug block. The artifact never computes a true service-level metric; the README's identical service/inner numbers and the paper's identical AS/AIS rows for Nezha follow mechanically. |

## 2. Additional source-code facts (not in the contract's list)

| # | Fact | Where | Why it matters |
|---|---|---|---|
| N1 | **No Anomaly Detector exists in the artifact.** The paper's RCA trigger (k-sigma on front-end success ratio and P90 latency, p.5) has no implementation; nothing reads `front_service.csv`. The "abnormal window" evaluated is `inject_time + 2 minutes`, computed directly from the ground-truth fault list. | pattern_ranker.py:234-250; grep `SuccessRate|front_service` = 0 hits in *.py | The evaluated pipeline is *given* the fault time. Detection difficulty is excluded from the headline numbers by construction. |
| N2 | **Hardcoded fallback pod equals a ground-truth pod.** Any candidate whose source event cannot be located in the normal graphs gets `pod = "frontend-579b9bff58-t2dbm"`, depth 1 — the literal injected frontend pod name of the OB dataset (present in 8/56 OB faults), applied in **both** namespaces including ts runs. | pattern_ranker.py:145-147 | A default guess that coincides with ground truth for frontend faults; leakage-flavored. Quantify in E1. |
| N3 | Silent data loss in ingestion: a whole trace is dropped on any exception (`except Exception: pass`), per-span log lookups swallow exceptions, log JSON-unwrap failures silently degrade to raw lines. | data_integrate.py:88-89, 95-97, 295-297, 338-340; log_parsing.py:151-153 | Violates our no-silent-sinks rule; E2's importer must count what the artifact drops invisibly. |
| N4 | Only traces listed in `traceid/*.csv` are processed; the trace CSV rows outside that list are ignored (upstream issue #2 asked exactly this; unanswered). | data_integrate.py:504-506 | The traceid file is a *selection* whose provenance is undocumented. |
| N5 | Ranking sort is `(score, deepth)` descending; rank ties are compressed during evaluation only when adjacent candidates tie on *both* score and depth. | pattern_ranker.py:189-190, 274-280 | Tie semantics live in the evaluator, not the ranker; matters for E1 corrected metrics. |
| N6 | Resource-alarm attachment: a candidate whose pod has any alarm becomes a "resource" candidate labeled with that pod's **first** alarm only; extra alarms for the same pod+resource at shallower depth are deleted (convoluted `mv_flag` block), guarded by a `try/except: pass` around `result_list.pop`. | pattern_ranker.py:148-186 | Combines with K to bound resource-candidate diversity. |
| N7 | The drain3 template miner is **stateful and learning during evaluation**: every log line (and every synthetic span/alarm message) goes through `add_log_message`, which can create clusters and persists to `log_template/<ns>.bin`. Worker processes each mutate their own copy concurrently. | log_parsing.py:50; main.py:15-19; data_integrate.py:504-516 | Reproduction hygiene (harness restores `.bin` between runs) and a nondeterminism risk if the shipped state were incomplete; run-to-run comparison in E0 checks this. |
| N8 | `evaluation_min_score` contains `json.load()` with no argument — it would crash if called; dead code in the main flow. | pattern_ranker.py:377 | Code-hygiene signal only. |
| N9 | requirements.txt omits `tqdm` (imported by pattern_ranker.py:6); an earlier revision pinned an unsatisfiable dependency set (upstream issue #2). | requirements.txt; git history 461b5b2 | Install-time reproducibility defect. |
| N10 | NetworkP90 "metric" is not a metric: it is computed at query time from the trace file as `(parent span end − child span end)` p90 per pod, with hardcoded `(10, 10)` for frontend and a CSV-default fallback for pods absent from the minute's traces. | alarm.py:84-148, 276-279 | The paper presents NetworkP90(ms) alarms as metric alerts; in code they are a trace-derived statistic with fixed thresholds (200/300). |
| N11 | The two "namespaces" get different alarm-insertion rules (hipster: only spans with >2 events; ts: every span of the pod). | data_integrate.py:303-329 | Undocumented per-dataset behavior difference. |

## 3. Dataset integrity summary (details: appendix/dataset-inventory.md)

- Fault counts match claims: 56 (OB) and 45 (TT). Types: OB 42 resource + 14
  code-defect; TT 21 resource + 24 code-defect *by our count of the shipped
  lists* (paper says 20+25; appendix §4.5 lists ts = network_delay 14 +
  cpu_contention 7 = 21 resource vs return 11 + exception 13 = 24 code).
  **Discrepancy with paper's 20/25 split — carried as unresolved.**
- `construct_data` metric directories are byte-identical to `rca_data` metric
  directories (full-day files); only log/trace/traceid distinguish the
  fault-free minute from fault windows.
- `inject_timestamp` epochs are unreliable on 2023-01-29 (UTC+8
  re-interpretation, ±60 s drift, one duplicated epoch); `inject_time`
  strings align with minute-file names and are what the code consumes.
- One metric file has a corrupted epoch column (issue #8); inert for the
  shipped pipeline (see I-metric-ts above).
- README links to four non-existent fault-list paths (wrong year/path).

## 4. Upstream tracker summary (details: appendix/upstream-issues-audit.md)

- Three independent reproduction-failure reports (#1, #2, #3; #2 reports
  AS@1 = 5.36% on Python 3.6 with the then-pinned dependencies), all closed
  without comment; zero maintainer comments on any issue ever.
- Defect reports #10/#11/#12 (all confirmed in source above) and dataset
  issues #8/#14 are open and unfixed at the pinned commit.
- No merged fix exists after the pinned commit.

## 5. E0/E1 outcomes (filled after the runs)

1. All four headline runs reproduce the committed logs exactly (rank
   vectors byte-identical; the one per-case textual diff is the PR #7
   ground-truth timestamp fix) → `02-e0-reproduction.md`.
2. Per-case ranks and candidate lists are archived per config
   (`experiment/nezha/results/e0/eval/*.eval.json`); 2 OB + 1 TT faults
   unlocalized; metrics deterministic across repeated runs, TT candidate
   *composition* below the match is not (02 §4).
3. The independent evaluator confirms the artifact's arithmetic on all 101
   cases (0 mismatches); corrected semantics shift results by at most
   +2.22 pp (in Nezha's favor, TT) → `03-e1-measurement-audit.md`.
4. Claim I-#10 (get_deepth_pod cycling): **no hang was observed in any
   run** (6 complete + 2 OOM-partial); the cycle risk remains a code-level
   hazard, not a shipped-data behavior.
