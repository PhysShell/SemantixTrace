# 00 — Preregistration: SemantixTrace vs Nezha adversarial validation

> **STATUS: DRAFT — NOT FROZEN.**
> This document freezes the comparative-experiment design (E2 and later).
> It becomes binding at the freeze commit, which will be recorded in
> `decisions.log.md` as D-FREEZE with the commit hash. Per D-001, phases E0
> (historical reproduction) and E1 (measurement audit) execute before this
> freeze and their outputs inform the corrected-metric definitions below.
> **No E2/E3 comparative run may be executed while the DRAFT banner is
> present.** Items marked `TO-FREEZE` must be resolved before the freeze
> commit; everything else is already stated in its final intended form.

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

**Held-out confirmatory data (LOCKED):** RCAEval datasets containing
metrics + logs + traces with code-level faults (target: the RCAEval RE
collection). `TO-FREEZE`: exact RCAEval subset names, download manifests
and SHA256 hashes, recorded at freeze time **without inspecting any labels
or telemetry beyond the public schema documentation**. Until the freeze
commit, no RCAEval data may be downloaded into this workspace. If any
tuning occurs after first contact with this data, it is demoted to
exploratory and this document must be amended with a D-entry saying so.

## 4. Partitions

- Calibration/development: both Nezha datasets, all faults.
- Confirmatory: the locked RCAEval subset, evaluated exactly once with the
  frozen method; no per-case inspection before the single evaluation run.
- No train/test split *within* the Nezha datasets is claimed to provide
  confirmatory power (contamination per D-001).

## 5. Metrics

Primary (per dataset, full-denominator over all faults):
- **AC@1 service-level** under the corrected evaluator (dense competition
  ranking over candidates deduplicated to first occurrence per service;
  semantics fixed by E1, `TO-FREEZE`: exact tie and dedup rule text after
  E1 review).
- **MRR** (unlocalized case contributes 0).

Secondary: AC@3, AC@5 (same semantics); inner-service AC@k under the
artifact's template/resource matching rule with a preregistered equivalence
relation (`TO-FREEZE` after E1: verbatim equivalence rule); median candidate
set size; unlocalized-case count; ingestion loss counters (read / accepted /
rejected+reasons); wall-clock per case.

## 6. Conditions and algorithms

- **N1**: Nezha representation + corrected evaluator + historical scorer
  (adjacent-pair differential, Score_min=0.67, support floor >5, root-most
  pruning) — the E1-corrected baseline.
- **S1**: SemantixTrace canonical representation of the *same* telemetry +
  the *same* scorer/evaluator (algorithm frozen, representation varies).
- **S2**: same canonical representation + SemantixTrace graph/oracle
  machinery (representation frozen, algorithm varies). `TO-FREEZE`: the
  concrete S2 method after reading the current ActionGraph/oracle layer;
  the contract's minimal-extension rule applies — no new framework.
- Ablations for any S2 gain: no normalization; adjacent pairs only; no
  root-most pruning; no metric events; no logs; no traces; graph without
  oracle; oracle without richer-than-edge structure.

Importer requirements (E2): every generated event carries provenance
(dataset, file, row/span/log/metric key, conversion rule id); no semantic
enrichment absent from the source (an OperationName maps to an
OperationName-level action, never an invented domain action); all rejected
records counted by reason. `TO-FREEZE`: field-by-field mapping table after
the SemantixTrace model summary is integrated.

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
