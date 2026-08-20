# 00 — Preregistration: SemantixTrace vs Nezha adversarial validation

> **STATUS: FROZEN** at the commit introducing this banner (recorded as
> D-005 in `decisions.log.md`). E0/E1 completed before this freeze
> (D-001); their outputs fixed the corrected-metric definitions below.
> No E2/E3 comparative result existed at freeze time. Any
> post-freeze change requires a decisions-log entry stating whether
> outcome data had been seen and demotes affected results to exploratory.

## 1. Research questions

- RQ1 (representation): does SemantixTrace's canonical event representation
  change RCA quality when the ranking algorithm is held fixed?
- RQ2 (algorithm): given identical canonical input, does SemantixTrace
  graph/oracle machinery outperform the adjacent-pair differential?
- RQ3 (native semantics): do explicitly instrumented domain events add
  diagnostic signal beyond ordinary logs/traces/metrics? (E4; gated on
  E2/E3 outcomes.)
- RQ4 (provenance): can every RCA candidate be machine-traced back to source
  telemetry through every transform?

## 2. Hypotheses

- **H0 (default): no practically meaningful advantage** from SemantixTrace's
  representation, graph/oracle layer, or native events, versus the
  Nezha-style adjacent-pair differential over its own representation.
- H1: canonical SemantixTrace representation materially improves RCA with
  identical scoring (threshold: §7).
- H2: SemantixTrace graph/oracle materially improves RCA over adjacent-pair
  differential with identical representation.
- H3: native semantic events materially improve RCA over ordinary
  multimodal observability.
- H4: SemantixTrace produces root-cause output with machine-reconstructable
  provenance (source → canonical event → normalization → graph element →
  score → candidate) for ≥99% of emitted candidates, verified by an
  automated provenance-walk check.

Falsifiers: H1-H3 are each falsified by failing their §7 thresholds on the
designated data; H4 is falsified by any candidate whose provenance chain
cannot be mechanically reconstructed.

## 3. Datasets, inclusion/exclusion

**Development data (already exposed, never confirmatory):** the Nezha
artifact datasets — OnlineBoutique (56 faults, 2022-08-22/23) and
TrainTicket (45 faults, 2023-01-29/30), pinned by
`experiment/nezha/manifests/dataset-manifest.sha256`. All 101 faults are
included; no exclusions. Per D-001, E0/E1 exposed per-case outcomes on all
of them, so every E2/E3 result on these datasets is exploratory by
construction.

**Held-out confirmatory data (LOCKED):** the RCAEval benchmark's
multimodal dataset group that contains metrics + logs + traces and
code-level faults (per its publication this is the RE3 group across its
three systems; RE2 is the fallback if RE3 proves not to ship logs).
Locking protocol: at unlock time (after the method freeze that follows
E2/E3 development), the exact subset names, download manifests, and
SHA256 hashes are recorded in a dedicated commit **before** any telemetry
or label is inspected; only the benchmark's public README/schema docs may
be consulted to confirm naming, and that consultation must itself be
logged. No RCAEval data may be downloaded into this workspace before that
commit. If any tuning occurs after first contact with this data, it is
demoted to exploratory and this document must be amended with a D-entry
saying so. Note: RCAEval includes systems related to OnlineBoutique and
TrainTicket; its fault *instances* and telemetry are disjoint from the
Nezha artifact data, and any system-level overlap is reported alongside
results rather than silently pooled.

## 4. Partitions

- Calibration/development: both Nezha datasets, all faults.
- Confirmatory: the locked RCAEval subset, evaluated exactly once with the
  frozen method; no per-case inspection before the single evaluation run.
- No train/test split *within* the Nezha datasets is claimed to provide
  confirmatory power (contamination per D-001).

## 5. Metrics

Primary (per dataset, full-denominator over all faults):
- **AC@1 service-level (corrected)**: candidates are ranked by the
  condition's scorer output order; ranks are dense competition ranks
  (candidates with equal ranking keys — for the Nezha scorer,
  (score, depth) — share one rank; the next distinct key takes rank+1);
  the candidate list is deduplicated to the first occurrence per service
  before ranking; a candidate is correct iff
  `service(candidate pod) == service(injected pod)` where
  `service(p) = p.rsplit('-',1)[0].rsplit('-',1)[0]`. This is E1's
  `service_dedup` semantics.
- **MRR** over the same ranks (unlocalized case contributes 0).

Secondary: AC@3, AC@5 (same semantics); AC@k on the non-deduplicated list
(E1 `service_raw`); **inner-service AC@k** under this frozen equivalence
rule — for a resource-type ground truth `R`, a candidate is correct iff
it carries a resource annotation containing `R` as a substring AND its pod
equals the injected pod; for a code-region ground truth `A_B`, a candidate
pattern (src,dst) is correct iff `A` is a substring of the source event's
template/action name and `B` a substring of the destination's AND its pod
equals the injected pod (the artifact's rule from
`pattern_ranker.py:262-284`, with dense ranks instead of its counter);
median/min/max candidate-set size; unlocalized-case count; ingestion loss
counters (read / accepted / rejected+reasons per source file); wall-clock
per case.

## 6. Conditions and algorithms

- **N1**: Nezha representation + corrected evaluator + historical scorer
  (adjacent-pair differential, Score_min=0.67, support floor >5, root-most
  pruning) — the E1-corrected baseline.
- **S1**: SemantixTrace canonical representation of the *same* telemetry +
  the *same* scorer/evaluator (algorithm frozen, representation varies).
- **S2**: same canonical representation + SemantixTrace graph machinery
  (representation frozen, algorithm varies). Frozen method: build
  `ActionGraph`s (trace-graph crate) from the fault-free sessions and from
  the fault-window sessions; score each transition with the *same*
  differential formula (`freq_normal/(freq_normal+freq_fault)`, threshold
  0.67, support floor >5) applied to ActionGraph transition frequencies;
  root-most pruning along graph topology; ties broken by the graph's
  Heuristics `anomaly_score`, then depth. The oracle layer participates
  only as candidate *annotation* (violation evidence chains, H4) in the
  primary S2; an oracle-informed scoring variant is one of the ablations,
  not the primary. No new framework beyond this composition.
- Ablations for any S2 gain: no normalization; adjacent pairs only; no
  root-most pruning; no metric events; no logs; no traces; graph without
  oracle; oracle without richer-than-edge structure.

Importer requirements (E2): every generated event carries provenance
(dataset, file, row/span/log/metric key, conversion rule id); no semantic
enrichment absent from the source (an OperationName maps to an
OperationName-level action, never an invented domain action); all rejected
records counted by reason.

Frozen field-by-field mapping (source schemas per
`appendix/dataset-inventory.md` §2; target = `trace_schema` v2 JSONL —
v2 has no log/metric kinds and no span-id fields, so those identities are
encoded in reserved namespaces below; no schema bump for this experiment):

| Source | Rule id | Target v2 event |
|---|---|---|
| trace CSV row (span) | `span-v1` | `CommandExecuted{command_id="span:{service} {OperationName}", args={pod, span_id, parent_id}, duration_ms=Duration/1000, outcome=Success}`; `ts`=StartTimeUnixNano; `domain_entity_id="span:{SpanID}"` |
| log CSV row | `log-v1` | `CommandExecuted{command_id="log:{drain3 cluster_id under the artifact's shipped template state}", args={pod}}`; `ts`=TimeUnixNano; `domain_entity_id="span:{SpanID}"` |
| metric alarm (artifact's own `generate_alarm` output, unmodified) | `alert-v1` | `CommandExecuted{command_id="alert:{metric_type}", args={pod}}`; `ts`=window start; `domain_entity_id="pod:{pod}"` |
| grouping | `session-v1` | one session per (dataset, minute window, TraceID); `session_id`=UUIDv5 of that triple; `seq` by (ts, source row order); `correlation_id`=UUIDv5 of TraceID |

The alarm *detection* stage is deliberately identical to N1 (same
fixed-threshold `generate_alarm`), so E2 isolates representation, not
detection. S1's event vocabulary is produced by running the standard
trace-normalizer fold over these sessions (CanonicalAction triples);
Nezha's drain3 cluster IDs enter as `command_id` content, so the
representational delta under test is exactly: canonicalization +
abstraction + burst folding + explicit parent/child structure versus
drain3-ID event chains with timestamp-insertion heuristics.

## 7. Useful-effect thresholds and kill criteria (adopted from the contract)

Strong positive evidence requires, on held-out data, at least one of:
≥ +10 percentage points AC@1; or MRR gain ≥ +0.10; or equal localization
accuracy with ≥50% reduction in median candidate-set size; or materially
finer localization granularity at equal service-level accuracy; or a large
class-specific gain on code-level faults where ordinary telemetry is weak —
each without material regression elsewhere (material regression = >2pp drop
in any primary metric or >20% candidate-set growth).

Kill/reduce-complexity criteria: consistently tiny gains (<3pp AC@1 and
<0.03 MRR on development data across both datasets); gains that vanish on
held-out data; adjacent-pair differential performing equivalently to S2;
improvements attributable to metric alarms/ground-truth leakage (checked by
the leakage ablations); no incremental signal from native instrumentation;
complexity disproportionate to measured benefit.

These thresholds may not change after any E2+ result is seen.

## 8. Missing data, ties, failures

- A fault with no matching candidate = unlocalized: rank ∞, counts in the
  denominator, contributes 0 to MRR. Never dropped.
- A fault whose pipeline run crashes or whose output is unparseable is
  counted as unlocalized AND separately reported as a failure with cause.
- Ties: candidates with equal ranking keys share a dense competition rank;
  the tie group's rank is what enters AC@k (this is deliberately more
  conservative than "best position in group").
- Every ingestion drop is counted, categorized, and attributed to a source
  file; a run whose loss counters cannot be produced is invalid.

## 9. Statistical aggregation

Per-dataset AC@k as exact fractions (hits/n). Paired per-case comparison
between conditions on the same dataset: McNemar exact test on @1 hits;
bootstrap (10,000 resamples, seed 20260820) 95% CI for MRR differences.
No cross-dataset pooling into a single headline number; per-dataset results
are reported separately (the paper's 89.77% average style is explicitly
avoided). Significance threshold α=0.05; effect sizes reported regardless.

## 10. What may change after calibration

Only: S2 hyperparameters, chosen on development data before the freeze of
the external run; bugfixes to our own code with RED→GREEN evidence and a
decisions-log entry. Not: metric definitions, thresholds (§7), matching
rules, dataset composition, tie/missing rules.

## 11. Locked-data rules

The RCAEval confirmatory subset is locked from the moment this document
freezes: no download before freeze, single evaluation run after freeze,
any violation demotes the data to exploratory with a mandatory D-entry.
