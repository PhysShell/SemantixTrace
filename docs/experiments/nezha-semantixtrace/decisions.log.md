# Decisions log — Nezha × SemantixTrace adversarial validation

Append-only. Every methodological decision, deviation, and
outcome-data-exposure event is recorded here with its date and rationale.

---

## 2026-08-20 — D-001: Phase order — E0/E1 run before preregistration is frozen

**Decision.** Execute E0 (historical reproduction of the Nezha artifact) and
E1 (measurement audit) *before* freezing `00-preregistration.md`; freeze the
preregistration before any E2/E3 comparative run.

**Reason.** The research contract requires preregistration "before
implementing or running the comparative experiment" (E2+). E0/E1 are not the
comparative experiment: they establish what the baseline artifact actually
computes and whether its published numbers are real. Writing an E2/E3
preregistration without knowing the artifact's true metric semantics would
freeze wrong definitions. Ordering directive from the experiment owner:
first milestone is "which of Nezha's 92.86% / 86.67% are real and why";
SemantixTrace work is gated on that.

**Contamination consequence (recorded now, before any E2/E3 design).** E0/E1
expose per-case outcomes on both Nezha datasets (OnlineBoutique, TrainTicket).
Therefore all E2/E3 results on these two datasets are *development/exploratory*
evidence by construction. Confirmatory evidence for any hypothesis must come
from data locked at preregistration time (planned: RCAEval-hosted datasets not
inspected until the method is frozen). The preregistration must state this
split explicitly.

**Outcome data seen at decision time:** none beyond the artifact's own
committed logs (which are inputs to E0, not our outcomes).

---

## 2026-08-20 — D-002: Environment deviation for reproduction

**Decision.** Run NEZHA-HISTORICAL on Python 3.11.15 with numpy 1.26.4 /
pandas 1.5.3 (artifact pins numpy 1.15.4 / pandas 0.23.4, which do not build
on Python 3.11); drain3 pinned exactly at 0.9.10.

**Reason & risk assessment:** `experiment/nezha/manifests/environment.md`.
**Expected direction of effect:** none (numeric operations used are
version-stable); validated case-by-case against the authors' committed logs.
**Outcome data seen at decision time:** none.

---

## 2026-08-20 — D-003: Reproduction runs the artifact unmodified; evaluation is log-parsing

**Decision.** NEZHA-HISTORICAL is executed with zero source modifications
(harness only restores the pristine drain3 `.bin` state between runs and
captures outputs). Per-case ranked candidate lists for the independent
evaluator are obtained by parsing the artifact's own DEBUG log stream
("Soted Result List: [...]"), which pattern_ranker emits for every fault case,
plus a post-run dump of the drain3 template state. No instrumentation patch
is applied to the baseline in E0.

**Reason.** Rule 4 of the research contract (reproduce warts included) and
rule 3 (no silent sinks): the log stream already contains the full ranked
list per case, so instrumenting the artifact is unnecessary and would risk
perturbing it. All parse failures are counted by the evaluator
(`experiment/nezha/evaluators/independent_eval.py`).
**Outcome data seen at decision time:** none.
