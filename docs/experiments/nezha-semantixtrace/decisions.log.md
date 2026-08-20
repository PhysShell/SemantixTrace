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

---

## 2026-08-20 — D-004: OOM on hipster runs; performance-only worker-count patch

**Event.** The first harness execution completed both TrainTicket configs
successfully (results valid, deterministic, matching the committed author
logs) but both OnlineBoutique configs died: hipster/service rc=137 after
23/56 cases (memory-cgroup OOM killer, worker RSS ~670 MB each, dmesg
evidence preserved), hipster/inner rc=1 (BrokenProcessPool after its
workers were OOM-killed). Root cause: the artifact hardcodes
`ProcessPoolExecutor(max_workers=64)`; upstream ran on a 256 GB host, this
host has 15 GB. Failed run outputs preserved as
`hipster-{service,inner}-FAILED-oom` in the run root.

**Decision.** Commit `ae34750` on the Nezha fork branch makes the worker
count env-configurable (`NEZHA_MAX_WORKERS`, default 64 = upstream
behavior). Hipster configs re-run sequentially with `NEZHA_MAX_WORKERS=8`
via `run_e0_hipster_retry.sh`.

**Why this is not an algorithm modification.** Worker count changes neither
the task set nor scoring nor aggregation values; pool completion order is
nondeterministic upstream at 64 workers already (measured: ts/service run1
vs run2 differ in candidate-list order/composition while rank vectors and
metrics are identical). Validation gate: hipster reproduction is accepted
only if its rank vector matches the committed author log, same as ts.

**Expected direction of effect:** none on metrics. **Outcome data seen at
decision time:** TT results (development data per D-001) and the partial
23-case hipster log.

---

## 2026-08-20 — D-005: Preregistration FROZEN

**Decision.** `00-preregistration.md` is frozen in this commit. All
TO-FREEZE items were resolved using E0/E1 outputs (corrected-evaluator
semantics, per D-001 already development-exposed) and the SemantixTrace
model notes; the S2 method is fixed as an ActionGraph-transition
differential with the same scoring formula (minimal-extension rule);
the telemetry mapping is fixed field-by-field with reserved
`span:`/`log:`/`alert:` command-id namespaces on schema v2 (no schema
bump for the experiment).

**Outcome data seen at freeze time:** E0/E1 results on the two Nezha
datasets only (all development data by prior declaration). No E2/E3
comparative result existed; no RCAEval data has been downloaded or
inspected. The confirmatory layer (locked RCAEval subset) remains
untouched.

**Binding consequences.** No E2/E3 run may deviate from the frozen
metrics, thresholds (§7), mapping, or tie/missing rules without a new
D-entry that demotes affected results to exploratory.

---

## 2026-08-20 — D-006: E2 design completions frozen; single-case smoke disclosure

**Decision.** The frozen mapping table left three operational points
open; they are fixed as documented in 04-e2-representation.md §2 (alert
session assignment, seq tie-break, linear depth analog, provenance-based
pod attribution, TraceID-based log join) before the full E2 run.

**Outcome data seen at decision time.** One mechanical smoke case
(hipster 2022-08-22 03:53:54 cpu_contention) was scored to validate
pipeline plumbing before this entry; its S1 output was inspected (top
candidate: the background adservice memory alarm; no frontend
candidate). No design parameter was changed in response. All E2 numbers
remain development/exploratory per D-001 regardless.
