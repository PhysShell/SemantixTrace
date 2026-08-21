# Final report — Adversarial validation of SemantixTrace against Nezha

Experiment window: 2026-08-20. Branches:
`claude/semantixtrace-nezha-validation-nzf927` in `PhysShell/SemantixTrace`
(all deliverables) and `PhysShell/Nezha` (pinned artifact
`d8140101` + one performance-only reproduction patch, `ae34750`).
Every number below is backed by a machine artifact under
`experiment/nezha/results/`; generated tables carry their generator's
name. **Epistemic status of everything here: development/exploratory**
(D-001) — the locked confirmatory data was deliberately never consumed
(07). Phase docs: 00–07 in this directory; append-only decision log in
`decisions.log.md` (D-001…D-009).

## Answers to the contract's ten questions

**1. Do the public Nezha numbers reproduce?**
Yes — exactly. All four headline configurations of the pinned artifact
reproduce the committed author logs to the digit, with byte-identical
per-case rank vectors, on Python 3.11 + modern numpy/pandas (deviation
documented; drain3 pinned exactly). OB 92.857143/96.428571/96.428571
over 56 faults; TT 86.666667/97.777778/97.777778 over 45. The three
upstream reproduction-failure reports (#1–#3) did not reproduce here and
are most plausibly environment casualties. (`02-e0-reproduction.md`)

**2. Do they survive an independent evaluator?**
Arithmetically, yes: a first-principles evaluator consuming only ranked
candidates + frozen ground truth confirms every one of the 101 case
ranks (0 mismatches). Semantically, partially: the artifact's "service
level" (AS) and "inner-service level" (AIS) are one identical
computation printed under two labels — the artifact never computes a
distinct service-level metric, and the paper's identical AS/AIS rows for
Nezha inherit that. Under corrected semantics (dense ranks, true
service-level matching) the values move by at most **+2.22 pp — in
Nezha's favor**. The evaluation defects reported upstream (#11, #12) are
real in code and nearly inert on the shipped data. Two structural
caveats survive all corrections: the RCA window is derived from the
ground-truth injection time (the paper's Anomaly Detector does not exist
in the artifact), and the explanation surface (actual-pattern selection,
tail of the candidate list) is buggy (#11) and run-to-run unstable on
TT. (`01-…`, `03-e1-measurement-audit.md`)

**3. What is the simplest algorithm achieving competitive performance?**
The one already inside Nezha: adjacent event-pair differential frequency
(support >5, Score_min 0.67) + root-most pruning + alarm decoration,
*on Nezha's grouped representation*. Everything fancier the paper
gestures at (graph pattern mining, CM-SPAM/TKG, pattern aggregation,
k-sigma detection) is absent from the artifact that produces the
headline numbers. Nothing we built beat it.

**4. Does replacing Nezha's representation with SemantixTrace improve anything?**
No — it is catastrophic for this scorer. With the algorithm held fixed,
on the corrected primary metric: OB service AC@1 92.86% → 8.93%, inner
92.86% → 1.79%; TT 88.89% → 22.22% and 88.89% → 11.11%. The mechanism is identified and interventionally
confirmed: the v2 canonical session is one timestamp-ordered linear
stream; folding all modalities into it destroys intra-span adjacency
(code-region template pairs never form) and shrinks the stable pattern
vocabulary (12 OB cases end with zero candidates, so alarms have nothing
to decorate). Removing span events from the stream *quadruples* TT inner
accuracy (11.11 → 40.00) — the interleaving itself is the damage.
(`04-…`, `05-…` §3)

**5. Does the graph/oracle machinery add value beyond the adjacent-edge differential?**
No. ActionGraph transition frequencies are definitionally equal to
adjacent-pair supports; the only frozen S2 delta (Heuristics anomaly
tie-break) reorders ties for −2.2…+6.7 pp depending on dataset/level,
both directions, on a base 60–91 pp below the N1 baseline. The oracle
layer, frozen as annotation-only, contributes nothing measurable to
ranking. (`05-e3-algorithm.md`)

**6. Do native semantic events add value beyond logs/metrics/traces?**
Untested — E4's gate (E2/E3 justifying continuation) was not passed, and
running it on a falsified pipeline would measure noise. The question
remains open. (`06-…`)

**7. Which fault classes benefit, if any?**
Under the S-conditions, none benefit; the classes *degrade* differently.
TT code-defect faults retain the most signal, and it lives almost
entirely in the log modality: the span-free (log-dominant) ablation
reaches AC@1 40% on both the inner and service levels — beating the
full multimodal mix (22.22% service). Resource faults lose nearly
everything because alarm decoration requires surviving pattern
candidates. Alert events as vocabulary carry no signal at all
(no-alerts ≡ full S1).

**8. Which SemantixTrace components are justified by evidence?**
The source-attribution provenance machinery — with the claim stated at
exactly its verified strength (D-009). What is verified, with no
special-case success: **H4-source-attribution** — 1494/1494 (100%)
candidate chains walk mechanically from RCA candidate through canonical
event and provenance record to the exact source dataset rows, alert
chains included via materialized, verified derivations (118 across all
windows, 4,191 source refs; earlier weaker form caught by re-gate,
RED→GREEN in `results/regate/`, D-008). What is **not** verified —
frozen H4 in full: the checker does not reconstruct the
normalization → pattern/graph-transition → supports → score →
root-most-pruning → emission segment (candidate records carry no
support/score derivation), and it walks S1 candidates while frozen H4
names the graph/oracle chain. **Frozen H4: INCONCLUSIVE — not
verified**; verifying it would require a derivation-replay checker that
was deliberately not built (D-009: the experiment's own lesson argues
against building infrastructure to prove infrastructure). A product
finding stands regardless: alarm provenance is a DAG (one alert ← one
derivation ← N source records), which the evidence model must
represent. Contrast: the baseline's attribution uses a template-ID walk
with a hardcoded fallback pod equal to a ground-truth pod, and its
explanation surface is bugged and nondeterministic. Also validated in
passing: the fail-closed schema readers (every imported line passed
`read_event` round-trip), the normalizer's determinism, and the
FoldReport loss accounting (cardinality laws held on every window).

**9. Which components should be deleted, simplified, or postponed?**
Evidence-based recommendations, not decrees:
- **Postpone/abandon "SemantixTrace as an RCA engine" on the current
  representation.** The linear-session view is structurally incapable of
  feeding adjacency-differential RCA, and no amount of graph machinery
  downstream compensates (S2 ≈ S1).
- **Do not grow trace-graph toward RCA.** Its Heuristics anomaly score
  produced direction-inconsistent reorderings; nothing here justifies
  investment.
- **If RCA is ever revisited**, the prerequisite is causal grouping as a
  first-class representation feature (span/parent-child identity in a v3
  schema + a grouping-aware normalizer view) — new product scope that
  must itself pass an E2-style representation check before any RCA
  claim.
- **Keep and lean on** the provenance chain, fail-closed readers, and
  loss-accounting reports — they are the components this experiment
  repeatedly relied on and the only ones that outperformed the baseline
  at anything.

**10. What is confirmatory versus exploratory?**
Nothing here is confirmatory. All quantitative claims are development/
exploratory by construction (D-001: E0/E1 exposed per-case outcomes on
the only datasets used). The locked RCAEval confirmatory resource was
never consumed and remains intact (07). The strongest-evidence items are
the reproduced facts (E0: exact reproduction; H4-source-attribution:
100% provenance walk), which are mechanical rather than statistical
claims. Frozen H4 itself is INCONCLUSIVE (D-009).

## Verdict

**PIVOT.** SemantixTrace's demonstrated value in this experiment is as
recorder/replay/evidence infrastructure — deterministic canonical
capture with verified source-attribution provenance
(H4-source-attribution, 1494/1494; frozen end-to-end H4 itself
INCONCLUSIVE, D-009) — not as an RCA engine. On RCA specifically the evidence supports STOP for the
current architecture: H1 and H2 are falsified on development data with
effect sizes (−66.7…−83.9 pp AC@1) that no measurement correction
approaches, and the frozen kill criteria are met. A single precisely
scoped open question survives for any future RCA revisit: whether a
causally-grouped canonical representation (v3-scope) can preserve the
signal the linear session destroys — that hypothesis is untested, and
the locked external validation set is still available for it.

The cheapest and most useful result of the project, as anticipated: we
now know *before* building anything clever that the deliberately stupid
baseline — adjacent canonical event-pair differential + root-most
pruning — is not beatable *from this representation*, and we know the
exact structural reason why.
