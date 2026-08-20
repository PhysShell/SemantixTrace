# 03 — E1: Measurement audit

Status: **complete** for the evaluator-side items; two items are
structural (not correctable in an evaluator-only variant) and are
explicitly carried into E2/E3 as ablations. The RCA algorithm was not
modified anywhere in this phase.

Variants (per the research contract):
- **NEZHA-HISTORICAL** — the artifact unmodified (E0).
- **NEZHA-EVAL-FIXED** — same runs, same ranked candidate lists, corrected
  *evaluation* semantics recomputed by
  `experiment/nezha/evaluators/independent_eval.py` from the E0 run
  artifacts. No re-execution of the pipeline was needed: the defects under
  audit live in the rank-counting/matching loop, which the independent
  evaluator replaces.

Generated per-case tables: `experiment/nezha/results/e0/eval/e1-tables.md`
(source of every number below).

## 1. HISTORICAL vs CORRECTED headline table

Semantics: *historical* = bug-for-bug replica of the artifact's counting;
*dense* = every candidate occupies a rank, equal (score,depth) share one
(fixes issue #12's resource-skip and the leave-candidate-0 tie increment);
*service_raw* / *service_dedup* = true service-level correctness
(candidate's service == injected service), on the raw list and on the list
deduplicated to first candidate per service.

| dataset | metric | HISTORICAL @1/@3/@5 | DENSE @1/@3/@5 | SERVICE_raw @1 | SERVICE_dedup @1 |
|---|---|---|---|---|---|
| OnlineBoutique (n=56) | labeled AS@k and AIS@k | 92.86 / 96.43 / 96.43 | 92.86 / 96.43 / 96.43 | 92.86 | 92.86 |
| TrainTicket (n=45) | labeled AS@k and AIS@k | 86.67 / 97.78 / 97.78 | 88.89 / 97.78 / 97.78 | 88.89 | 86.67 |

**Every changed case** (contract requirement): exactly one —
`ts-2023-01-29-011` (ts-verification-code-service, return, 09:58:04):
artifact rank 2 → dense rank 1. It is tied on (score, depth) with the
top candidate; the artifact's counter advances when leaving candidate 0
even inside a tie, so the corrected tie semantics *raise* Nezha's TT AC@1
by 2.22 pp. No OB case changes under any semantics.

## 2. The eight audit items

1. **Issue #12 (resource candidates never advance the rank counter)** —
   confirmed in source (`pattern_ranker.py:266-280`); measured effect on
   the shipped data: **zero cases** on either dataset. The reason is
   structural: resource faults are almost always matched at the first
   resource candidate (OB median candidate list = 2; alarm dedup keeps one
   resource candidate per pod), so no non-matching resource candidate
   precedes a match. The bug is real; its observed inflation is 0 pp here,
   but it is data-dependent and would bite any dataset with multiple
   simultaneous alarms.
2. **True service-level vs fine-grained ground truth** — the artifact's
   "service-level" run matches inner-service ground truth + pod name
   (claim M). Recomputed under genuine service-level correctness the
   numbers are 92.86 (OB) and 88.89/86.67 (TT raw/dedup): the *values*
   are robust within ±2.3 pp on these datasets; the *labels* are not —
   the artifact's AS and AIS are one number printed twice, and paper
   Tables 3/4 inherit that identity for all Nezha rows.
3. **Tie semantics** — artifact: adjacent (score,depth) ties do not
   advance the counter, but the counter always advances when leaving
   position 0, so a candidate tied with the top gets rank 2; corrected
   dense ranking gives rank 1. Net measured effect: the single TT case
   above, *in Nezha's favor* when fixed.
4. **Missing-result semantics** — unlocalized faults (no matching
   candidate anywhere in the list): 2 on OB, 1 on TT. The artifact leaves
   them out of the rank vector but keeps them in the denominator —
   arithmetically sound; our evaluator makes them explicit
   (rank ∞ / MRR 0).
5. **Denominator handling** — verified exact: 52/56, 54/56; 39/45, 44/45.
   No case is dropped from denominators anywhere.
6. **Issue #11 (actual-pattern ordering)** — confirmed: `sorted(dict)`
   sorts key strings, so the "actual pattern" shown next to an expected
   pattern is selected by lexicographic event-ID order, not score
   (`pattern_ranker.py:87`, used at `:314-318`). **No effect on AS/AIS**
   (display only), but it degrades the artifact's interpretability story:
   the "actual behavior" explanation is essentially arbitrary among
   candidates sharing a source event. Combined with the run-to-run
   candidate-composition instability on TT (02 §4), explanation output is
   not a reliable artifact surface.
7. **Duplicate/equivalent candidates** — TT lists (median 18) contain
   multiple candidates per service; deduplication to first-per-service
   moves TT AC@1 from 88.89 (raw) back to 86.67: the tie-fixed case sits
   behind another candidate of a *different* service after dedup. OB lists
   (median 2) are unaffected. Duplicates matter at the margin only.
8. **Evaluator/pipeline use of root-cause knowledge** — three findings:
   - *Structural (not evaluator-fixable):* the RCA window is
     `inject_time + 2 min` from the ground-truth file; the paper's Anomaly
     Detector does not exist in the artifact (01 §2 N1). All headline
     numbers therefore measure localization-given-detection.
   - *Hardcoded fallback pod* equals the OB ground-truth frontend pod
     (`pattern_ranker.py:145-147`). Measured on TT (where it is provably
     foreign): present in 7/45 case lists, never top-1 — zero effect. On
     OB it is not separable from genuine frontend attribution without an
     algorithm ablation → carried into E3's ablation list.
   - *Matching leniency:* ground-truth matching is substring containment
     (`root_cause[0] in template`) — documented; no counterexample
     of a false-positive containment was observed in the 101 cases.

## 3. E1 verdict

The artifact's evaluation code contains real defects (#11, #12, tie
counting, mislabeled AS), but on the shipped datasets their net numeric
effect is **at most 2.22 pp, and the largest correction moves the score in
Nezha's favor**. The headline numbers survive an independent evaluator and
corrected semantics as *measures of inner-service localization given the
fault window*. They do not survive as (a) evidence that service-level and
inner-service accuracy were separately measured, or (b) evidence about
end-to-end RCA including detection.

Consequences for the comparative phases:
- The corrected evaluator (dense + explicit service-level and
  inner-service-level rules) becomes the single evaluator for E2/E3 (N1
  baseline = Nezha representation + historical scorer + corrected
  evaluator), to be frozen in the preregistration.
- OB is near ceiling (median 2 candidates, 92.86 AC@1): little headroom
  for any method; TT has modest headroom (86.67-88.89 AC@1, median 18
  candidates). Effect-size expectations in the preregistration must
  account for this.
- The fault-window leakage (no anomaly detector) applies equally to every
  condition in E2/E3 (all consume the same windows), so comparisons remain
  internally valid — but no absolute number from this benchmark should be
  read as end-to-end RCA accuracy.
