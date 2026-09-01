# 05 — E3: Algorithm experiment (S1 vs S2) and ablations

Status: **complete**. Machine artifacts: `experiment/nezha/results/e3/`
(S2 per-case records, ablation records, generated tables in
`e3-tables.md`); H4 check: `results/e3/h4-provenance-check.json`.
All numbers development/exploratory per D-001.

Question (H2): given the same canonical input, does the SemantixTrace
graph machinery beat the adjacent-pair differential?

## 1. S2 as frozen, and its built-in ablation

The frozen S2 (00-preregistration.md §6) is the same differential formula
over `ActionGraph` transitions built by the real `trace-graph` crate,
plus the Heuristics `anomaly_score` as a tie-break. Empirically the
crate's transition frequencies are *identical* to S1's adjacent-pair
supports (the builder counts every consecutive occurrence — verified:
228 normal edges == 228 normal patterns on the smoke window), so **the
only algorithmic delta between S1 and S2 is the anomaly tie-break** —
which means the contract's required ablation for the new component is
precisely the S1↔S2 comparison itself.

This contract is machine-enforced: `scripts/check_s1s2_isolation.py`
compares the checked-in S1/S2 candidate multisets per case over every
field except `anomaly` (provenance included) and exits non-zero on any
divergence; since D-025 the case universe itself is certified against
the committed fault-case manifest (`manifests/expected-cases.json`),
so a truncated pair of artifacts — even one truncated identically on
both sides — cannot pass with fewer comparisons. Round-6 external review caught the contract broken at the
artifact level: the lexicographically sorted transition order (D-011)
fed the order-sensitive keep-first-max-depth alarm dedup a different
retention draw than S1's encounter order in 4/101 cases, so part of
the S1↔S2 delta was a retention artifact, not the tie-break. Fixed by
re-aligning S2's pre-dedup ordering to S1's first-encounter order over
the same normal sessions (dedup semantics untouched) — RED/GREEN
artifacts in `results/regate/s1s2-isolation-{RED,GREEN}.json`, D-015.

## 2. Results (generated: results/e3/e3-tables.md)

| dataset | metric | S1 | S2 | N1 (reference) |
|---|---|---|---|---|
| OB (56) | AC@1 service_dedup | 8.93% | 10.71% | 92.86% |
| OB | AC@1 inner | 1.79% | 3.57% | 92.86% |
| TT (45) | AC@1 service_dedup | 22.22% | 28.89% | 88.89% |
| TT | AC@1 inner | 11.11% | 8.89% | 88.89% |

Candidate sets and unlocalized counts are identical between S1 and S2
(same retained patterns; only order changes) — machine-verified per
case by the isolation gate (101/101,
`regate/s1s2-isolation-GREEN.json`, D-015). The tie-break shuffles
ranks in 5/56 (OB) and 17/45 (TT) cases, moving aggregates by −5.4 to
+6.7 pp depending on dataset and level, in both directions; no AC@1
value moved in the D-015 re-draw. (History: numbers regenerated after
the re-gate service_dedup fix — hipster unchanged, TT S1
20.00 → 22.22, D-008 — and again after the encounter-order alignment
restored the isolation contract — 4 cases re-drawn, D-015.)

Preregistered paired analyses (§9; McNemar exact on @1 hits, bootstrap
10,000 resamples seed 20260820 95% CI for MRR differences;
`results/e3/paired-stats.json`, D-021): no S1↔S2 difference is
detectable on any metric or dataset — McNemar exact p = 0.45…1.0, all
MRR-difference 95% CIs straddle zero with |point estimate| ≤ 0.042.
Non-significance does not by itself establish equivalence; the H2
falsification rests, as frozen, on S2 failing every §7 useful-effect
threshold — the tests add that not even a direction is detectable.

**H2 verdict: falsified.** Under the frozen kill criteria ("adjacent-edge
differential performs equivalently"), the graph layer adds no material
value: both conditions sit 60–91 pp below the N1 baseline, and the only
S2-specific component produces small, direction-inconsistent
reorderings. The oracle layer was frozen as candidate annotation only
and therefore contributes nothing measurable to ranking (by design; an
oracle-scored variant remains unexplored and nothing in these results
motivates it).

## 3. Modality ablations (TrainTicket, S1)

Preregistered list ("no metric events; no logs; no traces"), run on the
dataset where S1 retains signal:

| variant | svc AC@1/AC@3 | svc MRR | unloc | inner AC@1/AC@3 | inner MRR | unloc |
|---|---|---|---|---|---|---|
| full S1 | 22.22/46.67 | 0.386 | 10 | 11.11/26.67 | 0.203 | 22 |
| no-alerts | 22.22/44.44 | 0.377 | 10 | 11.11/24.44 | 0.197 | 22 |
| no-logs | 11.11/15.56 | 0.152 | 31 | 4.44/4.44 | 0.056 | 38 |
| no-spans | **40.00**/51.11 | 0.467 | 18 | **40.00**/48.89 | 0.451 | 20 |

Three sharp findings:

1. **Alert events carry no signal in the S-representation** (no-alerts ≈
   full S1 on every metric). The Nezha algorithm exploits alarms via
   candidate *decoration*, not via alarm-event patterns; in the linear
   canonical stream the synthetic alert events are pure vocabulary noise.
2. **Logs carry most of the remaining signal** (removing them collapses
   service AC@1 to 11% and triples the unlocalized count).
3. **Span events actively destroy the log signal**: removing them
   *quadruples* inner-service AC@1 (11.11 → 40.00) and nearly doubles
   service-level AC@1 (22.22 → 40.00) — the log-only stream beats the
   full multimodal mix on both levels. This is the E2 mechanism
   confirmed by intervention, not just inspection: in a single
   timestamp-ordered stream, span events interleave into log-template
   chains and break the adjacent pairs the differential feeds on.
   Nezha's representation avoids this by keeping per-span event groups.
   (A methodological footnote: the preregistered tie-sharing dense
   ranks systematically benefit high-tie conditions, and the S-side
   lists carry more ties than N1's; the no-spans service jump from the
   pre-fix 17.78 to 40.00 is largely tie-group collapse. The frozen
   semantics apply identically to every condition, so comparisons
   remain internally consistent.)

Combined E2+E3 conclusion: the damage is not "SemantixTrace lost
information" — it is **structural flattening**. The v2 linear session
holds all the events but discards causal grouping, and an
adjacency-based differential is exactly the kind of consumer that
grouping was protecting. Any future RCA claim for the canonical
representation requires causal grouping as a first-class feature (a v3
schema / normalizer concern — new product scope, out of this
experiment per its scope control).

## 4. H4: source-attribution provenance (verified subclaim; frozen H4 inconclusive)

Claim scope (D-009). What `scripts/check_h4_provenance.py` verifies is
the **H4-source-attribution** subclaim: it mechanically walked **every
candidate of every S1 case — 1495/1495 chains (100%)** (1494 before the
D-019 ingestion repair; the repaired ts window contributes one further
candidate) — from candidate
through (session, seq) to the canonical event, its provenance record,
and the source dataset row, verifying content consistency at each
step; since D-017 in the strong form: the recorded source row must
contain the pod AND the span/log identity **jointly** (no fallback
acceptance), and alert chains re-parse the recorded metric column's
cell requiring exact float equality with the recorded value instead of
trusting the derivation's stored `verified` flag. Zero failures —
under a checker with **no special-case success**, mutation-tested for
blindness: a provenance pointer re-aimed at a different same-pod row
and a metric input re-aimed at a different-value row are both caught
(`regate/h4-pointer-mutation-{RED,caught}.json`, D-017). A round-13
review found the trace-latency-pair branch weaker than the rest
(child checked for pod only, parent unchecked, value trusted); since
D-026 trace-derived derivations are verified by full independent
recomputation from the source CSV (pair list, p90 value and n_samples
must match exactly), and a standing gate
(`scripts/check_alarm_derivations.py`) audits **every** materialized
derivation of every window — all 118 across the 105 windows —
independently of the importer's own at-materialization check
(mutation-tested: `regate/d026-*`). What it does **not** verify is the frozen H4 chain in full:
the segment normalization → pattern/graph transition → supports (n, a)
→ score → root-most pruning → emission is not reconstructed (candidate
records carry score but no support derivation), and the walk covers S1
candidates while the frozen wording names the graph/oracle chain.
**Frozen H4 therefore remains INCONCLUSIVE — not verified**; per D-009
the derivation-replay checker that full verification would require was
deliberately not built.

The re-gate review caught that the first version of this claim was
weaker than the preregistered chain: alert-event provenance terminated
at the string `generate_alarm()` (a computation name, not telemetry),
and the checker special-cased it as a pass — a live population of 22
candidate chains (RED artifacts: `results/regate/h4-RED.txt`,
`h4-provenance-check.RED.json`). The repair materializes a verified
*derivation* per alarm (118 across all 105 windows: 90 metric-sample,
28 trace-derived-p90; 4,191 source refs in total; the metric_threshold
fallback path was never needed). CPU/Memory alarms walk to the exact
metric-CSV row; NetworkP90 alarms walk to every contributing trace-CSV
row *pair* via an exact shadow replication of the artifact's derivation
(fail-closed float-equality check against the value the artifact used).
A useful product finding fell out of the fix: **alarm provenance is a
DAG, not a single pointer** — one alert ← one derivation ← N source
records — and the evidence model has to represent that.

Contrast: the Nezha artifact attributes pods through a template-ID
graph walk with a hardcoded fallback pod, its displayed "actual
pattern" is selected by a sorting bug (#11), and its candidate-list
composition below the match is not stable across runs (02 §4).
**H4-source-attribution is the strongest positive evidence this
experiment produced**: every candidate chain ends at immutable source
records or a materialized, verified derivation over them. The full
frozen H4 stays unclaimed (INCONCLUSIVE, D-009).
