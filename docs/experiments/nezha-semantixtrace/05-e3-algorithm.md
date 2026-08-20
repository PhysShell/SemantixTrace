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

## 2. Results (generated: results/e3/e3-tables.md)

| dataset | metric | S1 | S2 | N1 (reference) |
|---|---|---|---|---|
| OB (56) | AC@1 service_dedup | 8.93% | 10.71% | 92.86% |
| OB | AC@1 inner | 1.79% | 3.57% | 92.86% |
| TT (45) | AC@1 service_dedup | 20.00% | 28.89% | 86.67% |
| TT | AC@1 inner | 11.11% | 8.89% | 86.67% |

Candidate sets and unlocalized counts are identical between S1 and S2
(same retained patterns; only order changes). The tie-break shuffles
ranks in 7/56 (OB) and 15/45 (TT) cases, moving aggregates by −2.2 to
+8.9 pp depending on dataset and level, in both directions.

**H2 verdict: falsified.** Under the frozen kill criteria ("adjacent-edge
differential performs equivalently"), the graph layer adds no material
value: both conditions sit 58–91 pp below the N1 baseline, and the only
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
| full S1 | 20.00/44.44 | 0.373 | 10 | 11.11/26.67 | 0.203 | 22 |
| no-alerts | 20.00/42.22 | 0.364 | 10 | 11.11/24.44 | 0.197 | 22 |
| no-logs | 11.11/15.56 | 0.152 | 31 | 4.44/4.44 | 0.056 | 38 |
| no-spans | 17.78/44.44 | 0.324 | 18 | **40.00**/48.89 | 0.451 | 20 |

Three sharp findings:

1. **Alert events carry no signal in the S-representation** (no-alerts ≈
   full S1 on every metric). The Nezha algorithm exploits alarms via
   candidate *decoration*, not via alarm-event patterns; in the linear
   canonical stream the synthetic alert events are pure vocabulary noise.
2. **Logs carry most of the remaining signal** (removing them collapses
   service AC@1 to 11% and triples the unlocalized count).
3. **Span events actively destroy the log signal**: removing them
   *quadruples* inner-service AC@1 (11.11 → 40.00). This is the E2
   mechanism confirmed by intervention, not just inspection: in a single
   timestamp-ordered stream, span events interleave into log-template
   chains and break the adjacent pairs the differential feeds on.
   Nezha's representation avoids this by keeping per-span event groups.

Combined E2+E3 conclusion: the damage is not "SemantixTrace lost
information" — it is **structural flattening**. The v2 linear session
holds all the events but discards causal grouping, and an
adjacency-based differential is exactly the kind of consumer that
grouping was protecting. Any future RCA claim for the canonical
representation requires causal grouping as a first-class feature (a v3
schema / normalizer concern — new product scope, out of this
experiment per its scope control).

## 4. H4: provenance (the experiment's positive result)

`scripts/check_h4_provenance.py` mechanically walked **every candidate
of every S1 case — 1494/1494 chains (100%)** — from candidate through
(session, seq) to the canonical event, its provenance record, and the
source dataset row, verifying content consistency (pod/span id) at each
step. Zero failures. Contrast: the Nezha artifact attributes pods
through a template-ID graph walk with a hardcoded fallback pod, its
displayed "actual pattern" is selected by a sorting bug (#11), and its
candidate-list composition below the match is not stable across runs
(02 §4). **H4 is supported with the strongest evidence this experiment
produced.** The candidate a SemantixTrace-side pipeline emits can always
say exactly which source records, through which transforms, produced it.
