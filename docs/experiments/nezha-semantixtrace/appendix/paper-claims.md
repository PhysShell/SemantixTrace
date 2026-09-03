# Forensic Claim Extraction: FSE2023 Nezha paper

Source PDF: /home/user/Nezha/FSE2023_Nezha.pdf (13 pages, ESEC/FSE '23, DOI 10.1145/3611643.3616249).
Page numbers below are PDF page numbers (1-13). All quotes are verbatim from the extracted text
(math symbols transliterated; extraction artifacts like ligature loss noted where relevant).
Evidence rule: nothing inferred; anything the PDF does not state is marked NOT STATED or AMBIGUOUS.

---

## 1. HEADLINE NUMBERS

### Table 3 — "Comparison of baselines at service level" (page 9)

Values exactly as printed (percent, two decimals as in table):

| Approach | OB AS@1 | OB AS@3 | OB AS@5 | TT AS@1 | TT AS@3 | TT AS@5 |
|---|---|---|---|---|---|---|
| MicroScope | 12.5 | 41.07 | 55.35 | 17.78 | 26.67 | 35.56 |
| MicroRCA | 16.07 | 62.5 | 92.75 | 20.00 | 31.11 | 44.44 |
| SBLD | 19.64 | 23.21 | 25.00 | 15.56 | 22.22 | 24.44 |
| LogFaultFlagger | 19.64 | 21.42 | 23.21 | 17.78 | 24.44 | 24.44 |
| MicroRank | 41.07 | 48.21 | 62.5 | 15.56 | 24.44 | 35.56 |
| TraceAnomaly | 30.35 | 33.92 | 48.21 | 13.33 | 28.89 | 33.33 |
| PDiagnose | 41.07 | 73.21 | 82.14 | 8.89 | 13.33 | 22.22 |
| Nezha w/oML | 14.28 | 17.85 | 17.85 | 6.67 | 8.89 | 11.11 |
| Nezha w/oM | 26.78 | 33.92 | 35.71 | 55.56 | 62.22 | 68.89 |
| Nezha w/oL | 64.28 | 64.28 | 64.28 | 42.22 | 44.44 | 44.44 |
| **Nezha** | **92.86** | **96.43** | **96.43** | **86.67** | **97.78** | **97.78** |

Notes: MicroRCA OB AS@5 = 92.75 is as printed (odd, since 52/56 = 92.86; possibly a typo in the paper — flagged, not resolved). Ablation variants defined on page 10: "Nezha w/oML that drops metrics and logs, Nezha w/oM that drops metrics and Nezha w/oL that drops logs."

### Table 4 — "Comparison of baselines at inner-service level" (page 9)

| Approach | OB AIS@1 | OB AIS@3 | OB AIS@5 | TT AIS@1 | TT AIS@3 | TT AIS@5 |
|---|---|---|---|---|---|---|
| SBLD | 14.28 | 17.85 | 17.85 | 15.56 | 22.22 | 24.44 |
| LogFaultFlagger | 19.64 | 21.42 | 21.42 | 15.56 | 24.44 | 24.44 |
| PDiagnose | 35.71 | 53.57 | 71.42 | 8.89 | 13.33 | 15.56 |
| Nezha w/oM | 26.78 | 33.92 | 35.71 | 55.56 | 62.22 | 68.89 |
| Nezha w/oL | 64.28 | 64.28 | 64.28 | 42.22 | 44.44 | 44.44 |
| **Nezha** | **92.86** | **96.43** | **96.43** | **86.67** | **97.78** | **97.78** |

Notes:
- Nezha's AIS rows are numerically identical to its AS rows in Table 3 (same for w/oM and w/oL). The paper does not comment on this identity. Flagged as noteworthy, not explained in the PDF.
- Table 4 has no MicroScope/MicroRCA/MicroRank/TraceAnomaly rows; page 9: "We only compare Nezha with SBLD, LogFaultFlagger, and PDiagnose because only these three baselines have the ability to identify root causes at the inner-service level."
- Table 4 has no Nezha w/oML row (present only in Table 3).

### Aggregate / prose numbers

| Claim | Page | Verbatim quote (short) | Notes |
|---|---|---|---|
| Top-1 accuracy 89.77% avg at code region & resource type level | 1 (abstract) | "Nezha achieves a high top1 accuracy (89.77%) on average at the code region and resource type level" | (92.86+86.67)/2 = 89.765 |
| Same 89.77% attributed to SERVICE level in intro | 2 | "achieves a high top-1 accuracy (89.77%) and surpasses all compared approaches by a large margin (61.45%~74.63%) when identifying root causes at the service level" | AMBIGUOUS which level 89.77 belongs to — abstract says inner-service, intro says service; numerically identical because Nezha AS and AIS rows are identical |
| Service-level averages | 9 | "achieves high accuracy in AS@1 (90%), AS@3 (97%), AS@5 (97%) on average" | rounded |
| Service-level improvement | 9 | "improves AS@1 by 61.45%~74.63% and AS@5 by 28.51%~73.28% on average compared to baselines" | |
| Inner-service averages | 9 | "achieving AIS@1 of 87%, AIS@3 of 97%, and AIS@5 of 97% on average" | |
| Inner-service improvement | 10 | "improves AIS@1 by 67.47%~74.85% and AIS@5 by 53.61%~75.96% on average compared to baselines" | |
| Inner-service improvement (intro variant) | 2 | "Nezha outperforms advanced baselines by 67.47%~74.85% in a high top-1 accuracy" | |
| Efficiency | 10 | "In a time window of 50,000 events, OnlineBoutique and TrainTicket take 16 seconds and 30 seconds to determine root causes" | Fig. 11; diagnosis time "increases linearly with the number of events"; fault-free pattern computation excluded from timing |
| Sensitivity | 10 | "Score_min = 0.67 achieves the best accuracy in both two datasets" ... "less sensitivity to Score_min when it is set above 0.6" | Fig. 12 (axis labels printed "ASI@1 ASI@3" — apparent typo for AIS) |

Other metrics: NO MRR, precision, recall, F1, or MAP anywhere in the paper. Only AS@k and AIS@k (k ∈ {1,3,5}) plus RCA time (seconds).

---

## 2. METRIC DEFINITIONS

| Claim | Page | Verbatim quote | Notes |
|---|---|---|---|
| AS@k definition | 8 | "Top-k accuracy at service level (AS@k) refers to the probability that root cause services are included in the top-k results." | |
| AIS@k definition | 8 | "Top-k accuracy at inner-service level (AIS@k) refers to the probability that the inner-service root causes (resource type or code region) are included in the top-k results." | |
| Why two metrics | 8 | "because some baselines can only localize root causes at the service level" | |

- Ties between candidate scores in the OUTPUT ranking: tie-break rule stated only for Nezha's own ranking (see Section 3, Pattern Aggregator: deeper depth ranked higher). How ties are counted when computing AS@k/AIS@k: NOT STATED.
- Missing results treatment: NOT STATED (no discussion of runs with empty output).
- Matching rule for baselines at inner-service level (page 9): "Considering that the above three baselines are designed to pinpoint error logs rather than code regions, their result would be determined to be correct if their output error logs are within the code region of root causes."

---

## 3. ALGORITHM (pipeline as described)

### 3a. Event representation (Section 4.2.1, pages 5-6)

| Claim | Page | Verbatim quote (short) | Notes |
|---|---|---|---|
| Event definition | 5 | "An event e records the execution status of a system at a point in time." (Definition 1) | |
| Metrics → alert events | 5 | "Data Integrator replaces the set of metrics with suspicious metric alerts... Metrics alerts are generated when metric values violate the k-sigma rule or static thresholds." | Also "compatible with alarm systems such as Prometheus Alertmanager" |
| Alerts repeat over alert time | 5 | "The time between the start and end of an alert is called alert time. We treat alerts as events that repeatedly occur within the alert time." | |
| Only system-level metrics in RCA | 5 | "we only consider the system-level metrics in RCA" | Application-level metrics used only for anomaly detection (p.2, p.4) |
| Regular-alert filtering | 5 | "Nezha excludes the alerts in the production phase that also occur in the construction phase." | |
| Logs → templates via Drain | 5 | "Data Integrator adopts a state-of-the-art log parsing approach Drain [19] to extract the static log templates and dynamic log parameters from the raw log messages in a streaming manner. After log parsing, we treat the static log templates as log events." | Parser: Drain, streaming; each log event carries trace ID, span ID, timestamp |
| Trace ID in logs prerequisite | 3, 8 | "we accomplish this integration by inserting trace IDs into log messages" (p.3); 1 line of logging config for Java, "2 lines of code" for other languages (p.8) | |
| Spans → trace events (sync) | 5-6 | "we consider the start and end of a span as two trace events. These two event messages can be represented as a concatenation of the span name with 'start' or 'end' string." | Fig. 5 |
| Spans → trace events (async) | 6 | "Data Integrator represents them as events consisting of span name and the 'asyn' string, e.g., e = 'Cart/GetCart_asyn'." | |

### 3b. Event graph construction (Section 4.2.2, page 6)

Definition 2 (page 6): "An event graph g_i = (E_i, Link) is a directed graph of events in the event set E_i. A directed link between e_j and e_j+1 (i.e., e_j -> e_j+1) denotes that e_j is followed by e_j+1 during the execution." One event graph per request in the time window (page 5).

Three construction steps (page 6, verbatim step headers):
1. "Order events in the same span." — "Data Integrator then chronologically orders the log and trace events into an event group based on their timestamps and adds a sequence relationship from an event to its next event in the group."
2. "Insert metric events to event groups." — "inserts the alert events after the first event of the event group if the corresponding service has alert events... If multiple alert events of the same service are detected, all alarm events will be sequentially inserted after the first event." ("It can also be inserted in other fixed locations as agreed.")
3. "Insert child groups to parent groups." — Same service instance (internal calls): "inserts child groups after the last event in parent groups whose timestamp is less than the first event in the child group"; cross-service RPC: "Data Integrator inserts the child group after the first event of its parent group based on the parent span ID to overcome the clock drift."

Rationale for graphs over sequences (page 6): "microservice applications may contain asynchronous calls. The event location of asynchronous calls may change uncertainly in the sequential sequences."

### 3c. Pattern mining (Section 4.3, page 6)

| Claim | Page | Verbatim quote | Notes |
|---|---|---|---|
| Pattern definition | 6 | "A pattern p is a subgraph of contiguous events in the set of event graphs G." (Definition 3) | |
| Mining method | 6 | "Nezha extracts the fault-free and fault-suffering patterns from G_C and G_P by traversing all event graphs in parallel." | Described only as graph traversal. NO named algorithm: no mention anywhere of CM-SPAM, TKG, sequential pattern mining, or frequent-subgraph mining libraries. |
| Finiteness argument | 6 | "The patterns in the event graphs are finite because logs have been parsed into templates and only the directly connected events are considered." | "only the directly connected events" — adjacency-based |
| Pattern length | 6, 8 | Example "a pattern e1->e2->e3 matches the event graph of the first request because the graph has e1 followed by e2 and e2 followed by e3 without other events involved" (p.6); Table 2 (p.6) lists only 2-event patterns (e1->e2 etc.); Fig. 9 demo shows 3-event pattern; Pattern Aggregator joins pairs into chains (p.7) | AMBIGUOUS: whether the miner enumerates only adjacent PAIRS (with longer chains formed later by the Aggregator) or subgraphs of arbitrary length. Definition 3 allows arbitrary contiguous subgraphs; Table 2 and the Aggregator's join rule suggest pairs. The paper never states a maximum pattern length. |
| Support definition | 6 | "Given a pattern p's count set C_p = {c1,...,ck}, where c_i denotes p occurs c_i times in the event graph g_i, the support s(p) of pattern p is the sum of the counts in all graphs, i.e., s(p) = sum_i c_i." (Definition 4) | Support = total occurrence count across graphs, NOT number of graphs containing p |
| Support floor | 6 | "we discard those pattern that rarely occurs by filtering patterns whose support less than s_min (s_min = 5 by default)." | s_min = 5 |
| Caching | 6 | "we persist the results of S_C into Pattern Storer" | fault-free supports computed once |

### 3d. Scoring formulas (Section 4.4, page 7)

Expected Pattern Ranker (Eq. 1, page 7), verbatim:
"Score_E(p) = Pr(g in G_C | p ⊑ g) = s_C(p) / (s_P(p) + s_C(p)). (1)"
"If pattern p occurs multiple times in G_C while rarely in G_P, p will be assigned a higher score." Worked example: "Score_E(e2->e3) = 3/(3+1) = 0.75".

Actual Pattern Ranker (Eq. 2, page 7), verbatim:
"Score_A(p) = Pr(g in G_P | p ⊑ g) = s_P(p) / (s_C(p) + s_P(p)). (2)"

Score threshold (page 7): "we specify a minimum score threshold Score_min to exclude such useless patterns. In this way, the pattern p is placed in the ranked score list only when Score_E(p) >= Score_min." (Stated for Score_E; whether the same threshold applies to Score_A is NOT explicitly stated — AMBIGUOUS, though "not all patterns in Expected Pattern Ranker and Actual Pattern Ranker provide useful information" precedes it.)

Score_min value (page 8): "The minimum score threshold Score_min ... is set to 0.67 (i.e., 2/3) by default. In this case, a pattern p in Expected Pattern Ranker is suspicious if the support of p in fault-suffering phase is less than half of the support of p in fault-free phase."

### 3e. Aggregation / root-most pruning (Section 4.5, pages 7-8)

| Claim | Page | Verbatim quote (short) | Notes |
|---|---|---|---|
| Redundant patterns | 7 | "we refer to the downstream patterns with the same score like e3->e5 as redundant patterns" | |
| Join rule | 7 | "If both patterns e_i->e_j and e_j->e_k are in the list and Score(e_i->e_j) >= Score(e_j->e_k), Pattern Aggregator will join e_k into e_i->e_j (i.e., e_i->e_j->e_k)." | applied "after iterating all patterns in the list" |
| Root-most retention | 7 | "Pattern Aggregator then chooses the root patterns of these anomaly graphs as final expected patterns." | Fig. 8: 6 patterns reduced to 3 |
| Expected-actual pairing | 8 | "For the expected pattern, we identify its associated actual pattern with the common prefix... If there is more than one actual pattern with the common prefix, we select the actual pattern with the highest score as the actual pattern." | |
| Final ranking | 8 | "Candidates are ranked in descending order based on the score of the candidate's expected pattern." | |
| Tie-break | 8 | "For expected patterns with the same score, we place the pattern with the deeper depth in the event graph further up the list because patterns with shallower depths are more likely to be caused by anomaly propagations." | deeper-depth-first tie-break |
| Output form | 4, 8 | "takes pattern pairs as final root cause list List_R = {...,(p_i, p_j, Score_E(p_i)),...}" (p.4); candidates without metric alert events show "the service name and the code region between events", otherwise "the metric alert event and corresponding monitoring view" (p.8) | |

### 3f. All thresholds / hyperparameters with values

| Parameter | Value | Page | Quote fragment |
|---|---|---|---|
| Anomaly detection k (k-sigma) | k = 3 | 5 | "(k = 3 by default)" |
| Time window / metric collection interval | 1 minute | 4, 5 | "1 minute by default in this study" (p.4); "The time interval value is set for one minute" (p.5) |
| Support floor s_min | 5 | 6 | "s_min = 5 by default" |
| Score_min | 0.67 (= 2/3) | 8 | "is set to 0.67 (i.e., 2/3) by default" |
| Fault duration (injection) | 3 minutes | 8 | "We set each fault duration to 3 minutes" |
| Recommended Score_min range | > 0.6 | 10 | "We recommended setting Score_min above 0.6" |

---

## 4. METRIC ANOMALY DETECTION (as claimed)

| Claim | Page | Verbatim quote | Notes |
|---|---|---|---|
| Trigger-level anomaly detection method | 5 | "Anomaly Detector uses k-sigma rules [35] to determine whether the target application is in an abnormal status. We calculate the mean mu and standard deviation sigma of success ratio and P90 latency in the construction phase. In the production phase, Anomaly Detector continually monitors the success ratio and P90 latency of front-end service in a sliding time window. If the success ratio is less than mu - k*sigma or P90 latency is greater than mu + k*sigma (k = 3 by default), Anomaly Detector declares the current time window is abnormal and triggers a root cause analysis." | Statistical (mean/std from fault-free construction phase), i.e., 3-sigma by default. Monitored signals: front-end success ratio + P90 latency only. |
| Metric ALERT events (for RCA input) | 5 | "Metrics alerts are generated when metric values violate the k-sigma rule or static thresholds." | AMBIGUOUS: for system-level metric alerts the paper allows EITHER k-sigma OR static thresholds and gives no per-metric choice, no k value specific to alerts, and no static threshold values. Also "compatible with alarm systems such as Prometheus Alertmanager." |
| Motivation-section usage | 3 | "The anomalies in metrics were identified using the k-sigma rules, with further details to be presented in § 4.1." | Fig. 3 |
| Historical baseline source | 5 | "We calculate the mean mu and standard deviation sigma ... in the construction phase" | historical/fault-free statistics, not fixed numeric thresholds, for the trigger |
| Replaceability | 5 | "this module can be easily replaced with other anomaly detection approaches (e.g., USAD [2], RRCF [31] and JumpStarter [43])" | |

---

## 5. DATASETS

| Claim | Page | Verbatim quote (short) | Notes |
|---|---|---|---|
| Applications | 8 | "We deploy two open-source microservice applications: OnlineBoutique (OB) and TrainTicket (TT)" | |
| OB fault count | 8 | "In total, we inject 56 faults (42 resource issues and 14 code defects) into OnlineBoutique" | |
| TT fault count | 8 | "We inject 45 faults (20 resource issues and 25 code defects) into TrainTicket" | 56 and 45 are consistent with AS@k denominators (e.g., 92.86 = 52/56, 86.67 = 39/45) |
| Fault types | 8 | Resource: "we inject CPU contention and network jam faults in the same way as the existing work [35, 37, 67]"; code defects: "inject error return and exception code defects following previous work [42, 72, 73]" | 4 fault types total: CPU contention, network jam, error return, exception |
| Injection tooling | 8 | "we design some language-specific fault injectors for the characteristics of program language for Java, Golang, and Python services [4, 28, 52]" | Refs resolve (p.12-13) to: [4] Java Byteman, [28] Hypno, [52] Golang Failpoint. No named tool for CPU/network faults (only "same way as existing work"). |
| Injection protocol | 8 | "We randomly inject one fault into one microservice following previous work [35, 72, 73]. We set each fault duration to 3 minutes to emulate the process between fault occurrence to fix." | one fault at a time |
| Number of services — TT | 9 | "TrainTicket with 41 microservices" | stated only in passing (PDiagnose discussion) |
| Number of services — OB | — | — | NOT STATED anywhere in the PDF |
| Platform | 8 | "a Kubernetes platform with 12 virtual machines, each of which has a 8-core 2.10GHz CPU, 16GB memory, and runs with Ubuntu 18.04 OS" | |
| Telemetry stack | 8 | Traces: Opentelemetry Collector -> Grafana Tempo; logs: Grafana Promtail -> Loki; metrics: cAdvisor (system-level), Istio (application-level), Prometheus Node Exporter -> Prometheus | |
| Observability augmentation | 8 | "we first instrument the Opentelemetry SDK [48] for each service to obtain complete traces. We then modify one line of logging pattern... to insert trace and span IDs into logs" | Augmented apps open-sourced at [46], [47] (p.2, p.11) |
| Telemetry volumes | 10, 11 | "In a time window of 50,000 events" (p.10, efficiency experiment, x-axis up to ~50,000 events/minute); "constructing datasets that include more than 600 events for a single request" (p.11) | NOT STATED: total trace/log/metric counts of the evaluation dataset. Only these two indirect figures. |
| Experiment duration | — | — | NOT STATED: total data-collection duration; only per-fault duration (3 min) and window size (1 min) are given |
| Nezha runtime environment | 8 | "prototype of Nezha built on Python 3.6... Linux server with Intel Xeon Gold 5318Y 2.10GHz CPU, 256 GB RAM, 1TB SSD Disk, and running Ubuntu 18.04" | |

---

## 6. EVALUATION PROTOCOL

| Claim | Page | Verbatim quote | Notes |
|---|---|---|---|
| Service-level ground truth | 9 | "The ground truths at the service level are the known injected services." | |
| Inner-service ground truth | 9 | "The ground truths at the inner-service level are the code region or resource type extracted from the fault-injected operation." | |
| Baseline matching at inner-service level | 9 | "their result would be determined to be correct if their output error logs are within the code region of root causes" | applies to SBLD, LogFaultFlagger, PDiagnose |
| Top-k protocol | 8 | AS@k / AIS@k = "probability that [ground truth] included in the top-k results", k ∈ {1, 3, 5} | tables report k = 1, 3, 5 only |
| How Nezha candidates map to service/code-region for scoring | — | — | NOT STATED explicitly (beyond Fig. 9's display of "Root Cause Service" + pattern); no formal rule for extracting the predicted service/code region from a pattern pair is given. AMBIGUOUS. |
| Construction-phase data | 4 | "In the construction phase, Nezha takes the fault-free observability data as input, which contains all request types of the application within a time window... We recommend setting the window size to the same interval as the metric collection (1 minute by default in this study)." | |
| RCA trigger | 4-5 | RCA runs on the "abnormal time window" flagged by Anomaly Detector | |

---

## 7. BASELINES

Six baselines, all unsupervised (page 8: "We use the following six state-of-the-art unsupervised metric-based, trace-based, and log-based RCA approaches as the baselines. We do not consider the supervised approaches because they need a large training dataset with labels").

Wait — the list on pages 8-9 actually names SEVEN: MicroScope, MicroRCA, LogFaultFlagger, SBLD, MicroRank, TraceAnomaly, PDiagnose. The paper says "six" but bullets seven. AMBIGUOUS/apparent inconsistency in the paper text (7 approaches appear in Table 3).

| Baseline | Type (per paper) | Page |
|---|---|---|
| MicroScope [35] | metric-based, correlation of metrics along dependency | 8 |
| MicroRCA [64] | metric-based, PageRank on anomaly sub-graph | 9 |
| LogFaultFlagger [1] | log-based, compares passing/failing logs | 9 |
| SBLD [55] | log-based, spectrum algorithms on log-event coverage | 9 |
| MicroRank [67] | trace-based, personalized PageRank + spectrum | 9 |
| TraceAnomaly [37] | trace-based, deep learning of normal trace patterns | 9 |
| PDiagnose [21] | multi-modal (metrics+traces+logs -> time series, voting) | 9 |

Baseline implementation release: NOT STATED. The Data-Availability Statement (page 11) covers only Nezha itself: "The data and the implementation of Nezha are publicly available at Zenodo [69] and Github [github.com/IntelligentDDS/Nezha]. The augmented OnlineBoutique and TrainTicket are available at [46] and [47]." No claim that baseline implementations/re-implementations are released, and no statement of whether baselines were re-implemented or reused from original authors' code.

---

## 8. THREATS TO VALIDITY (Section 6.2, pages 10-11) — authors' own flags

Internal validity:
1. Fault-free data quality (page 10): "The accuracy of Nezha can be affected if fault-free data is noisy or lacks certain types of requests." Mitigation: "construct fault-free data that includes a wide range of request types and has a similar number of requests to the production phase"; workload capture/replay; or short-window collection since "the production environment is predominantly in a normal status with limited faults [30]".
2. Score_min choice (pages 10-11): "the minimum score threshold Score_min is used to exclude such uninformative patterns. We recommended setting Score_min above 0.6 because a pattern p is considered more suspicious if the support of p in fault-suffering phase is less than the support of p in fault-free phase." Also (page 10): "the best configuration of Score_min highly depends on the characteristics of datasets."

External validity:
1. Trace-ID-in-logs requirement (page 11): "The integration analysis of Nezha relies on the insertion of trace ID into logs, which may not be available in some practical systems." Mitigation: standard Opentelemetry tooling; Java needs one logging-config line.
2. Generalization beyond testbed (page 11): "Nezha is evaluated on two widely-used microservice systems in a Kubernetes platform. It needs further effort to validate the effectiveness of Nezha in more complex real-world systems." Mitigations claimed: datasets "include more than 600 events for a single request and involves parallel and asynchronous service calls"; injected faults derive "from real faults in industrial systems [33, 76]".

Related limitations (Section 6.1, page 10, distinct from Threats subsection):
- "Nezha relies on the anomaly detection approach triggers for RCA, thus it cannot identify root causes for faults that escape anomaly detection." (only P90 latency + success ratio monitored)
- Byzantine faults out of scope: "Some byzantine faults, such as returning an unreasonable result to the user, cannot be identified by Nezha because these faults do not manifest themselves as any abnormal patterns."
- False-alarm burden (verbatim, including apparent editing artifact "me"): "Thus if a non-change fault is not filtered by me, we also generate a suspicion list for SRE to check, which increases the checking burden on SRE. However, SRE believes that this false alarm burden is acceptable compared to a missed alarm." AMBIGUOUS: this paragraph appears without a preceding definition of "non-change fault" and contains a first-person artifact; extracted verbatim.

---

## MISC / CROSS-CHECK NOTES (extraction-level observations, all from this PDF only)

- Page 6 cites "PDiagnose [18]" once; elsewhere PDiagnose is [21] and [18] is GMTA — apparent citation typo in the paper.
- Table 2 (page 6) example columns: Pattern | S_C | S_P | Score_E | Score_A | Rank(Score_E); rows: e1->e2 (3,3,0.5,0.5, rank 1/2 — column layout garbled in extraction, AMBIGUOUS exact header-to-column mapping), e2->e3 (3,1,0.75,0.25), e2->e4 (1,1,0.5,0.5), e3->e5 (3,1,0.75,0.25), e2->e6 (0,2,0.0,1). Footnote: "'-' means that the pattern is aggregated by Nezha."
- Problem formulation (page 4) lists final list entries as (p_i, p_j, Score_E(p_i)) and — likely a typo — writes Score_E for the actual list ("List_A = {...,(p_j, Score_E(p_j)),...}") where Section 4.4.2 defines Score_A. Extracted as printed.
- Artifact links claimed in paper: Zenodo DOI 10.5281/zenodo.8276375 [69], github.com/IntelligentDDS/Nezha (footnote 1, page 11), Augmented-OnlineBoutique [46], Augmented-TrainTicket [47].
- Received 2023-03-02; accepted 2023-07-27 (page 13).
