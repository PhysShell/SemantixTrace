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

---

## 2026-08-20 — D-007: E4 not executed; external validation not unlocked; verdict PIVOT

**Decision.** E4 (native semantic instrumentation) is not executed: its
contractual gate — E2/E3 justifying continuation — was not passed (H1
and H2 falsified on development data, kill criteria met). The locked
RCAEval confirmatory subset is deliberately left unconsumed: it exists
to confirm a promising frozen method, and none exists; burning the lock
on a falsified method would waste the experiment's only confirmatory
resource. Final verdict recorded in final-report.md: **PIVOT**
(recorder/replay/evidence infrastructure — H4 at 100% — not an RCA
engine; RCA revisit requires a causally-grouped representation and gets
a fresh E2-style check plus the still-intact external lock).

**Outcome data seen at decision time:** all E0–E3 development results
(per D-001, exploratory by construction). No RCAEval data was ever
downloaded or inspected.

---

## 2026-08-20 — D-008: Re-gate wave — evaluator drift and H4 shortcut, both RED→GREEN

**Trigger.** Owner re-gate review (merge HOLD) found two defects in the
evidence contract, both discovered AFTER all E0–E3 outcomes were seen:

1. **service_dedup evaluator drift.** Both our evaluators
   (`independent_eval.rank_service`, `s1_eval.evaluate_case`)
   implemented the preregistered primary metric as a positional rank in
   the dedup list (`len(seen)`), not as dense competition ranks over
   the dedup list — tied dedup representatives did not share a rank,
   violating the frozen §5 definition. This corrects the *measurement
   implementation*, not the frozen definition; the definition is
   unchanged. RED: `01ad936` (+ `results/regate/eval-semantics-RED.txt`);
   GREEN: `02a3729` (test 5/5).

2. **H4 alert-provenance shortcut.** The provenance checker granted
   special-case success to alert events whose provenance ended at the
   string `generate_alarm()` — a live population of 22 candidate
   chains, so the "1494/1494 to exact source rows" claim was stronger
   than what was verified. The checker is now strengthened TO the
   originally declared contract (no special-case success; alert chains
   must walk into materialized derivations whose inputs are verifiable
   source records). RED: `fbca830` (1472/1494, 22 failures,
   `results/regate/h4-*.RED.*`); GREEN: this commit — after
   `repair_alert_provenance.py` materialized 118 verified derivations
   (90 metric-sample, 28 trace-derived-p90, 4,191 source refs; 22,421
   alert provenance records rewired across 105 windows), the
   strengthened walk passes 1494/1494 with zero failures
   (`results/regate/h4-GREEN.txt`, `results/e3/h4-provenance-check.json`).
   Product finding: alarm provenance is a DAG (alert ← derivation ← N
   source records), not a single pointer.

**Per-case / aggregate deltas from correction 1** (machine records:
`results/regate/regate-dedup-deltas.json`,
`results/e0/eval/regate-dedup-deltas-e1.json`; all other rank modes
asserted unchanged case-by-case):

- E1 / N1: hipster both configs — 0 cases changed. TT both configs —
  1 case (`ts-2023-01-29-011`: 2→1); service_dedup AC@1
  86.67 → **88.89**, MRR 0.915 → 0.926.
- E2 / S1: hipster — 0 cases. TT — 3 cases; AC@1 20.00 → **22.22**,
  MRR 0.373 → 0.386.
- E3 / S2: hipster — 0 cases. TT — 2 cases; AC@1 28.89 (unchanged),
  MRR 0.411 → 0.413.
- E3 ablations (re-run): no-alerts 22.22; no-logs 11.11 (unchanged);
  no-spans service AC@1 17.78 → **40.00** — dense tie-sharing collapses
  the positional-rank inflation of the high-tie log-only condition.
  Methodological note recorded in 05 §3: tie-sharing systematically
  benefits high-tie (S-side) conditions; the frozen semantics apply
  identically to every condition, so comparisons remain internally
  consistent.

**Effect on conclusions.** None on H1/H2/PIVOT: every delta is ≤2.3 pp
on N1/S1/S2 aggregates, uniformly favorable-or-neutral, and S1/S2
remain 60–84 pp below N1 (H1 falsification also stands on service_raw,
which never had the drift). H4 is now supported at its declared
strength. **All quantitative results remain development/exploratory**
(D-001); the RCAEval lock is untouched; E4 remains unexecuted.
