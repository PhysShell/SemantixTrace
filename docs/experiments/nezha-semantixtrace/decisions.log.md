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

---

## 2026-08-20 — D-009: H4 narrowed to its verified subclaim; frozen H4 recorded INCONCLUSIVE

**Trigger.** Second owner re-gate: the frozen H4 wording requires
machine-reconstructable provenance "through every transform"
(source → canonical event → normalization → graph element → score →
candidate). The strengthened checker verifies a narrower property:
candidate → (session, seq) → canonical event → provenance record →
source rows / verified alarm derivation → source rows. It does NOT
reconstruct normalization → pattern/graph-transition → supports (n, a)
→ score → root-most pruning → emission (candidate records carry no
support/score derivation), and it walks S1 candidates while the frozen
wording names the graph/oracle chain. "1494/1494" therefore proves
100% *source-attribution* provenance, not 100% *end-to-end candidate
derivation* provenance.

**Decision (owner's option 2, adopted).** No further verifier is built.
The frozen preregistration stays untouched. The claim ledger is
corrected instead:

- **Frozen H4: NOT VERIFIED / INCONCLUSIVE.** Verifying it would
  require a derivation-replay checker (recompute supports, score, and
  pruning per candidate from the folded scenarios); deliberately not
  built — the experiment's own headline lesson is to stop building
  infrastructure whose main purpose is to justify infrastructure.
- **Verified post-hoc subclaim `H4-source-attribution`:** 1494/1494
  (100%) candidate chains to immutable source records, alert chains via
  materialized, verified derivations (118 derivations, 4,191 source
  refs), no special-case success (RED→GREEN in D-008).
- **Alarm-provenance DAG finding stands** (alert ← derivation ← N
  source records).
- **H1, H2, PIVOT, STOP-RCA-on-v2-linear-session: unchanged.**

Docs updated to the narrowed claim: final-report Q8/Q10/verdict,
05 §4, 04 §5.3; the final-report header's fossil "D-001…D-007" range
corrected to D-009. The subclaim is labeled *post-hoc* because this
narrowing was formulated after all outcomes were seen — consistent
with the experiment-wide exploratory labeling (D-001).

**Outcome data seen at decision time:** all E0–E3 results and both
re-gate reviews.

---

## 2026-08-21 — D-010: External review found the E3 tool source untracked; RED→GREEN

**Trigger.** Codex review P1 on PR #20: `run_e3.py` invokes an
`st-graph` binary whose source was nowhere in the repository, so the
committed S2 results were not regenerable from a fresh checkout —
a violation of the reproducible-from-clean-checkout quality gate.

**Verified root cause.** `src/bin/st-graph.rs` existed on disk and
built the committed S2 artifacts, but the root `.gitignore`'s .NET
rule `**/bin/` silently matched Rust's `src/bin/` convention
directory: `git add` skipped it and `git status` stayed clean. A
silent sink in the repository's own ignore rules — the same failure
class this experiment spent three re-gates hunting in evaluators.

**Fix (RED `80b2e20` → GREEN).** RED records the machine evidence
(`results/regate/st-graph-missing-RED.txt`: empty `git ls-files`
match, `git check-ignore -v` naming the rule). GREEN narrows the rule
to its declared .NET intent (`adapters/**/bin/`) and tracks the source
via a *plain* `git add` — proving the rule fix rather than forcing
past it. Verification (`st-graph-GREEN.txt`): fresh clone builds
st-graph; the fresh binary regenerates the cached E3 transition table
exactly (set-equality, 647/842 edges; array order differs by the known
HashMap iteration nondeterminism).

**Scope.** Tooling reproducibility only: no result, metric, document,
or verdict changes. H1/H2/H4/PIVOT untouched. The two P2 findings of
the same review (harness exit-code propagation; pinned-commit
assertion) are answered on their threads without pushes: the evidence
contract never trusted harness exit codes (acceptance gates are
downstream rank-vector comparisons, per D-004 and the never-infer-
success-from-green-exit rule), and checkout identity is recorded in
run metadata rather than asserted — post-hoc verifiable.

**Outcome data seen at decision time:** all; irrelevant to this
tooling fix.

---

## 2026-08-21 — D-011: S2 regeneration made deterministic; canonical results re-drawn (RED→GREEN)

**Trigger.** Codex incremental review P1 on PR #20 (second finding):
st-graph serialized its transition table from a randomized HashMap, and
s2_scorer's alarm dedup keeps the FIRST max-depth resource candidate —
so S2 candidate retention was order-dependent across regenerations.

**RED (`8c61fed`).** Empirical check over all 101 cases with the
unsorted binary (`results/regate/s2-order-stability-unsorted.json`):
2/101 windows produced different candidate multisets across two fresh
regenerations; 1 case changed ranks between them; **3 TT cases drifted
from the committed evaluations** — i.e. the committed S2 evaluations
for those cases were one arbitrary draw of an order-dependent process.

**GREEN (`22ee45f` + this commit).** st-graph now sorts transitions by
their (src,dst) keys before serialization. Verification
(`s2-order-stability-sorted.json`): 0 unstable windows, 0 run-to-run
rank differences — the pipeline is deterministic end-to-end. The
deterministic regeneration is adopted as the canonical S2 result of
record (`results/e3/s2-*.cases.json`, `e3-tables.md` regenerated).

**Per-case deltas, previously-committed → canonical** (4 cases):
- hipster-2022-08-22-010: inner/raw 5→7 (dedup 4 unchanged)
- hipster-2022-08-23-023: inner/raw 13→14, dedup 8→9
- ts-2023-01-30-001: inner/raw 3→14, dedup 3→7
- ts-2023-01-30-013: inner/raw 39→2, dedup 15→2

**Aggregate deltas:** every AC@1 value unchanged (OB 10.71/3.57,
TT 28.89/8.89). Tails move within tie-shuffle territory: OB inner AC@5
10.71→8.93 (−1.78 pp), TT service_dedup AC@3 46.67→48.89 (+2.22 pp),
MRR ±0.008 max; the two TT case drifts offset each other at @3/@5.
S1↔S2 comparison range stays −2.2…+6.7 pp. **H2 verdict (falsified)
and PIVOT unchanged.** Docs cite only AC@1 values, which did not move;
no document-number changes were required beyond regenerated tables.

**Note.** This is the third silent-sink class found by layered review
in this branch (evaluator drift, ignore-rule swallow, now HashMap-order
dependence) — and the same failure family as the Nezha artifact's own
run-to-run candidate instability documented in 02 §4. The S2 scorer
mirrored the artifact's order-sensitive dedup faithfully; determinism
had to come from stabilizing its input.

**Outcome data seen at decision time:** all; the fix changes tooling
determinism and re-draws 4 exploratory case evaluations, no verdicts.

---

## 2026-08-21 — D-012: Stability gate re-pointed at checked-in baseline; canonical artifacts re-verified

**Trigger.** Codex P2 on PR #20 (third-round): the S2 stability gate
loaded its "committed" baseline from the mutable `$E2_RUNROOT/results`
workspace rather than the repository's `results/e3`, so a regenerated
cache could in principle mask drift against the checked-in artifacts.

**Facts.** No recorded number is invalidated: at every recorded
execution the workspace files WERE the byte-identical source of the
repo copies (re-verified via `cmp` before the fix — both s2-*.cases.json
pairs identical). The weakness was prospective gate hygiene, the same
family as the D-009 checker-scope finding, milder.

**Fix (`de69f74` + this commit).** Baseline now defaults to the
script-relative `experiment/nezha/results/e3` (env `S2_BASELINE_DIR`
to override). Verification
(`results/regate/s2-order-stability-sorted-repobaseline.json`): the
sorted pipeline re-run over all 101 cases against the REPOSITORY
baseline — 0 unstable windows, 0 run-to-run rank differences,
**0 drift from the checked-in canonical artifacts**. The gate now
proves exactly the property it claims about exactly the artifacts
being merged.

**Outcome data seen at decision time:** all; tooling hygiene only, no
result or verdict changes.

---

## 2026-08-21 — D-013: Stability gate strengthened to full-candidate comparison; D-012 claim verified at artifact level

**Trigger.** Codex P2 on PR #20 (fourth-round, head `c18004e`): the
gate `check_s2_order_stability.py` compared regenerated runs against
the repository baseline only through each case's `evaluation` (rank
tuples), while D-012's prose claims "0 drift from the checked-in
canonical artifacts". Ranks are a projection of the artifacts: equal
ranks do not by themselves prove the candidate lists (patterns,
scores, anomaly values, depths, pods, resource tags, provenance
records) match the checked-in `s2-*.cases.json`. Claim and check were
misaligned — the same claim-vs-checker family as D-009, milder.

**Remedy chosen: strengthen the check, not narrow the claim** (the
inverse of D-009's resolution, because here the stronger check is a
~10-line addition rather than a new verification layer). The gate now
also compares, per case, the regenerated run-1 candidate list against
the committed one as a multiset of fully-serialized candidates —
every field participates, including resource attachment and
provenance — and exits non-zero on any of four drift classes
(run1↔run2 candidate sets, run1↔run2 evaluations, run1↔committed
evaluations, run1↔committed full candidates).

**Verification (`results/regate/s2-order-stability-sorted-fullcand.json`).**
Sorted st-graph binary, fresh double regeneration of all 101 windows /
101 cases, compared against the repository baseline: 0 unstable
windows, 0 run-to-run rank differences, 0 evaluation drift, **0
full-candidate drift from the checked-in canonical artifacts**.
D-012's artifact-level claim is therefore verified as stated, not
narrowed.

**Outcome data seen at decision time:** all; gate hygiene only — no
number, table, or verdict changes (H1/H2 falsified, H3 untested,
frozen H4 inconclusive, PIVOT all unchanged).

---

## 2026-08-21 — D-014: Stability gate extended to full-record + summary comparison; mutation-test RED→GREEN

**Trigger.** Codex P2 on PR #20 (fifth-round, head `cd73291`): the gate
compared regenerated runs against the committed `s2-*.cases.json` only
via evaluations and full-candidate multisets. Stable per-case fields
(case identity, `ground_truth`, `representation`, `algorithm`,
`parameters`) and the per-namespace `summary` block (aggregates,
candidate sizes) sat outside every comparison, so a corrupted or
edited committed artifact could pass the gate as long as candidates
and ranks matched — and D-013's "verified at the artifact level"
wording was, for the third time in this family (D-009, D-013), still
broader than the check.

**Remedy: strengthen the check again, with mutation-test evidence.**

**RED (`results/regate/s2-order-stability-fullrecord-mutation-RED.json`
+ `s2-stability-mutation-manifest.json`).** A copy of the committed
baseline was given five planted mutations in exactly the unchecked
fields (min_score parameter, algorithm name, ground_truth,
representation, one hipster summary aggregate), candidates and
evaluations untouched. The pre-fix gate (as of `0900a0c`) ran all
101 windows / 101 cases against it and reported 0/0/0/0, exit 0 —
machine proof of the blindness.

**GREEN (`cd73291` + this commit).** The gate now also compares, per
case, the complete record assembled exactly as `run_e3.py` writes it —
every field including the ordered candidate list, excluding only the
volatile wall-clock `runtime_ms` — and recomputes each namespace's
`summary` block with run_e3.py's exact formula. Six drift classes,
exit non-zero on any. Verification:
- vs the mutated baseline
  (`s2-order-stability-fullrecord-mutation-caught.json`): exactly the
  five planted mutations caught (4 record drifts + 1 hipster summary
  mismatch), all other classes zero, exit 1;
- vs the real repository baseline
  (`s2-order-stability-sorted-fullrecord.json`): fresh double
  regeneration, 0 across all six classes, exit 0.

The committed canonical artifacts are therefore verified at the level
D-012/D-013 claim: every stable field of every case record and both
summary blocks, with `runtime_ms` as the sole documented exclusion.

**Outcome data seen at decision time:** all; gate hygiene only — no
number, table, or verdict changes (H1/H2 falsified, H3 untested,
frozen H4 inconclusive, PIVOT all unchanged).

---

## 2026-08-21 — D-015: S1↔S2 retention confound removed; encounter-order alignment + permanent isolation gate

**Trigger.** Codex P1 on PR #20 (sixth-round, head `20d7e0a`),
independently verified before action: the checked-in S1 and S2
candidate multisets — compared over every field except S2's `anomaly`
(provenance included) — diverged in 4/101 cases. Mechanism: the
keep-first-max-depth alarm dedup retains the FIRST depth-tied
(pod, resource) candidate in list order; S1's list follows
first-encounter order over the normal sessions (`pair_support`
insertion order) while S2's followed the lexicographically sorted
st-graph transitions (D-011), so the retention draw differed. Score is
not part of the dedup key, so the draw was material: `ts-2023-01-30-001`
retained score 1.0 in S1 (rank 1/1/1) but 0.857 in S2 (14/14/7). The
05 §1/§2 claims "the only algorithmic delta is the anomaly tie-break" /
"candidate sets are identical" were therefore overclaims at the
artifact level, and the primary E3 ablation carried a retention
confound. Owner gate: **fix the ordering, not the dedup semantics**
(the mirrored Nezha dedup is the object under study; only the ordering
witness may be restored) — the inverse case of D-009's "don't build
meta-infrastructure": here the fix is small and restores the frozen
intended contrast.

**RED (`52dde2e`, `regate/s1s2-isolation-RED.json`).** New permanent
gate `scripts/check_s1s2_isolation.py` — the machine formulation of
the ablation contract: per-case S1/S2 candidate multisets over all
shared fields must be equal, counts included. Against the `20d7e0a`
canonical artifacts: 101 cases, **4 violations** (the cases above),
exit 1.

**Fix (`6b8755b`).** s2_scorer records each adjacent pair's
first-encounter position over the same normal sessions and orders the
scored patterns by it before materialization and dedup. The graph
remains the sole source of supports and anomaly scores; dedup and all
frozen scoring semantics untouched. Fail-closed counter
`encounter_unaligned_patterns` (scored pattern with no encounter
position) — 0 in all 101 windows, as the support-floor contract
requires.

**GREEN (this commit).**
- Isolation gate vs regenerated canonical: **101/101, 0 violations**
  (`regate/s1s2-isolation-GREEN.json`) — S1↔S2 identical per case in
  pattern, score (bit-exact), depth, pod, resource, provenance and
  candidate count; the only S2 ranking input beyond S1 is `anomaly`,
  so rank differences can arise only where the tie-break resolves
  ordering.
- Six-class stability gate re-run against the NEW canonical baseline:
  launched at commit time (fresh double regeneration of all 101
  windows); its artifact
  (`regate/s2-order-stability-sorted-fullrecord-d015.json`) and the
  zero-across-six-classes confirmation are adopted in the follow-up
  commit — adoption gate: any non-zero class reopens this entry per
  the no-silent-sink rule (see the D-015 addendum below).

**Old → new deltas** (all changed cases; full byte-level detail in
`regate/d015-old-new-deltas.json`; the retained candidate differs in
its non-dedup-key fields, pod/resource/depth are tied by construction):

| case | retained (old → new) | ranks inner/raw/dedup (old → new) |
|---|---|---|
| hipster-2022-08-22-010 | alert-src score 1.0 → log-src 0.857 | 7/7/4 → 7/7/4 |
| hipster-2022-08-23-010 | score 0.783 → 0.792 | 18/18/8 → 6/6/5 |
| ts-2023-01-30-001 | score 0.857 → 1.000 | 14/14/7 → 3/3/3 |
| ts-2023-01-30-002 | score 0.750 → 0.778 | 22/22/10 → 15/15/9 |

Five further cases changed list order only (stable final sort now
starts from the encounter order), with candidates and evaluations
unchanged.

**Aggregate effect.** Every AC@1 value unchanged (OB 3.57/10.71,
TT 8.89/28.89). Tails move toward S1 (TT dedup AC@5 57.78 → 60.00 =
S1's value; TT dedup AC@3 48.89 → 51.11; OB dedup AC@5 30.36 → 32.14).
Tie-shuffle counts: OB 7 → 5, TT 15 → 17 of the respective case
totals. S1↔S2 delta range −2.2…+6.7 → **−5.4…+6.7 pp** — the range
now measures the tie-break alone. Sensitivity note (post-hoc,
diagnostic only, NOT a primary result): excluding the 4 confounded
cases from the OLD artifacts eliminated the TT inner AC@1 deficit
entirely — i.e. the confound produced no hidden S2 advantage; its
removal makes the falsification cleaner, not different.

**Verdict impact.** H2 stays **FALSIFIED** — the confound could only
have flattered or penalized S2 by retention noise; with it removed,
S2 still sits 60–91 pp below N1 with small direction-inconsistent
tie-break effects. Owner formulation during the fix window: H2
falsification conclusion robust, primary S1↔S2 attribution temporarily
ungated pending this correction; final ACCEPT withdrawn and merge
HOLD until GREEN (this entry closes it). Fossil correction logged: 05
§2 said "58–91 pp below N1" where the numbers (current and prior) give
60–91, as final-report already stated; corrected to 60–91.

**Outcome data seen at decision time:** all. Post-outcome
implementation correction of the S2 condition's ordering witness; the
frozen scoring semantics, metric definitions, and all verdicts are
unchanged; results remain exploratory per D-001/D-008.

### D-015 addendum — stability re-verification adopted

The six-class stability gate launched in the GREEN (1/2) commit
completed against the NEW canonical baseline: fresh double
regeneration of all 101 windows / 101 cases with the encounter-aligned
scorer — **0 across all six drift classes** (exit 0,
`regate/s2-order-stability-sorted-fullrecord-d015.json`). The adoption
gate holds; nothing to reopen. D-015 is closed.

---

## 2026-08-29 — D-016: Isolation gate hardened against truncated artifacts

**Trigger.** Codex P2 on PR #20 (seventh-round, head `44b89a8`): the
D-015 isolation gate iterated only the S2 cases, looking each up in
S1, so a case missing from S2 was never compared — a truncated S2
artifact passed with fewer than the expected comparisons. The reviewer
demonstrated it machine-checked; independently reproduced here.

**RED (`29fd383`, `regate/s1s2-isolation-truncation-RED.json`).** With
`ts-2023-01-29-000` removed from a copy of `s2-ts.cases.json`, the
pre-fix gate passed: cases=100, isolation_violations=0, exit 0.

**Fix + GREEN (this commit).** The gate now iterates the union of the
two case-id sets; a case present on only one side is itself a
violation (`missing_from`). Verification:
- truncated copy (`regate/s1s2-isolation-truncation-caught.json`):
  cases=101, 1 violation naming the missing case, exit 1;
- real artifacts (`regate/s1s2-isolation-GREEN-d016.json`): 101/101,
  0 violations, exit 0.

Same silent-sink family as D-013/D-014 (gate weaker than its stated
contract), caught before any truncation ever occurred — no artifact
was ever actually truncated, no number, table, or verdict changes.

**Outcome data seen at decision time:** all; gate hygiene only.

---

## 2026-08-29 — D-017: H4 checker strengthened to joint-identity row matching and independent value re-parse

**Trigger.** Codex P1 on PR #20 (eighth-round, head `340e811`),
independently verified: `check_h4_provenance.py` verified a span/log
source row by pod SUBSTRING only, consulting the span id merely as a
fallback after a pod mismatch — so a provenance pointer re-aimed at a
different row of the same pod passed; and it verified a metric
derivation input by pod substring plus the derivation's own stored
`verified` flag, never re-parsing the row against the recorded value.
The checker was therefore weaker than the "exact candidate-to-source
attribution" the H4-source-attribution claim states.

**RED (`10a1425`, `regate/h4-pointer-mutation-RED.json` +
`h4-mutation-manifest.json`).** A copy of the walked runroot got two
plants: (A) one span-v1 pointer re-aimed at a different trace-CSV row
mentioning the same pod but not the span id; (B) the metric-sample
derivation input re-aimed at a row whose recorded column parses to a
different value (81.125 vs recorded 81.0794…), pod-containment
preserved. The pre-fix checker: **1494/1494, 0 failures, exit 0** —
blind to both.

**Fix + GREEN (this commit).**
- `check_source_row` now requires EVERY identity token jointly: for
  span/log chains the recorded row must contain the pod AND the
  span/log identity, with the old line-shift fallback removed (it
  proved unnecessary — the joint requirement passes everywhere on real
  data).
- Alert chains: metric-sample inputs are independently re-parsed — the
  recorded column's cell must equal the recorded value exactly (float
  equality, as the materialization used); the stored `verified` flag
  is no longer trusted as a substitute.
- The checker now exits non-zero when any chain fails (it previously
  reported failures but exited 0 — a latent gate weakness fixed in
  passing).

Verification: mutated runroot → 1482/1494 with all 12 failures tracing
exactly to the two plants (11 chains through the mutated derivation, 1
through the mutated pointer), exit 1
(`regate/h4-pointer-mutation-caught.json`); real runroot →
**1494/1494, 0 failures, exit 0** — the H4-source-attribution claim
holds at the strengthened checker's full strength. The trace-derived
pair path is untouched beyond token-list plumbing; no such derivation
occurs in the walked normal-window population (the 118/4,191 totals
are materialization-wide statistics, D-008).

**Verdict impact.** None: H4-source-attribution stays VERIFIED
1494/1494 — now under a strictly stronger checker; frozen H4 stays
INCONCLUSIVE per D-009; docs re-derived (05 §4, final-report Q8).

**Outcome data seen at decision time:** all; checker hardening with an
unchanged pass population.

---

## 2026-08-29 — D-018: Scorer fails closed on unaligned patterns; stability gate compares complete run outputs

**Trigger.** CodeRabbit's exact-head review of `340e811` (final round,
two actionable findings, both independently reproduced with machine
evidence before acting):
1. (Minor) `s2_scorer` only REPORTED `encounter_unaligned_patterns`:
   on a smoke input whose scored pattern never occurs in the normal
   sessions it exited 0 and wrote an artifact with the counter at 1 —
   although D-015 describes the counter as fail-closed.
2. (Major) the stability gate's run1↔run2 verdict compared candidate
   MULTISETS, so two regenerations identical except candidate order
   counted as stable; since D-015 the list order is a semantic input
   (encounter order feeds the order-sensitive dedup), a pure reorder
   must count as instability.

**RED (`3563218`).** `regate/s2-scorer-unaligned-RED.json` (pre-fix
scorer: exit 0, artifact written, counter=1 on the smoke input) and
`regate/s2-stability-comparator-RED.json` (two scored outputs
differing only in candidate order: multiset comparison says stable,
full-record comparison says not).

**Fix (`c385f27`).** The scorer raises before sorting or
materializing when any scored pattern lacks an encounter position
(smoke re-run: exit 1, no artifact — genuinely fail-closed; the
ordering lookup no longer needs a fallback key). The gate's stable
verdict now compares the COMPLETE run-1 and run-2 scorer outputs.

**GREEN (this commit,
`regate/s2-order-stability-sorted-fullrecord-d018.json`).** Full
101-window double regeneration with the fixed scorer under the
strengthened comparator: 0 across all six drift classes, exit 0 —
202 scorer invocations, none tripped the fail-closed guard.

**Verdict impact.** None — no number, table, or scientific verdict
changes; the D-015 "fail-closed" description is now enforced rather
than aspirational. Same silent-sink family as D-013/D-014/D-016/D-017.

**Outcome data seen at decision time:** all; gate/scorer hardening
with byte-identical canonical artifacts.

---

## 2026-08-30 — D-019: Ingestion conservation repair — owner-gated scientific re-gate, full regeneration, verdict not reopened

**Trigger.** Codex P1×2 on PR #20 (ninth round, head `c75fc79`), both
independently verified before action: (A) the importer read the
trace-ID selector CSV as a raw list and iterated it as-is, so a
repeated ID re-emitted the same deterministic session with seq
restarting at 0 — 448 duplicate (session_id, seq) pairs in
`ts/rca/2023-01-29_1544`, violating frozen session-v1 ("ONE session
per (dataset, window, TraceID)") and v2 seq identity; (B) `if not
spans: continue` silently dropped accepted log rows of spanless trace
IDs — 2,870 events across six hipster rca windows — although frozen
session-v1 has no span requirement: a silent sink. Owner gate: GO
under an explicit contract (RED before new rankings; frozen impact
radius; historical counter semantics preserved; source dirt observable
but not laundered; conditional alert scope; full 105-window importer
differential; a conservation gate attacking the defect CLASS;
preregistered verdict decision rule — no verdict declared immune).

**RED (`bda294a`, before any regeneration).** Machine records of both
defects; structural impact scan of all 105 windows freezing the
affected set at 7 abnormal windows / 7 cases (no normal window
affected); the preregistered decision rule; and the pre-result alert
scope scan — 105 windows, 29,283 span+log sessions, 0 sessions where
log pods leave span pods, so the UNION rule (session_pods = span_pods
| log_pods) was adopted, provably byte-identical for every previously
emitted session.

**Fix (`ef17c1b`).** Order-preserving selector dedup
(dict.fromkeys); `traceids_listed` keeps its historical raw meaning,
new counters `traceids_unique` / `traceids_duplicate_entries` keep the
source file's duplication observable; log-only sessions are emitted
(`log_only_sessions_emitted`, `traceids_without_events`;
`traceids_without_spans` becomes diagnostic-only); union alert scope.
Smoke on both defect windows: conservation exact (ts 9,504==9,504 with
dup_entries=1 still visible; hipster 49,321==49,321 with 9 log-only
sessions and 7 restored alert injections).

**Conservation gate (`0bd8642`,
`scripts/check_ingestion_conservation.py`).** Attacks the class:
(sid,seq) uniqueness, per-kind accepted==emitted, provenance<->event
bijection, one-session-per-unique-traceid, source dups observed but
not multiplying output. RED against the pre-fix canonical: exactly the
frozen 7 windows flagged — a third independent confirmation of the
impact set.

**GREEN (`74c0f31` + this commit).** Full 105-window regeneration with
the fixed importer:
- importer differential: **98/98 unaffected windows byte-identical**
  (events + decompressed provenance; the 13 earliest ts reports differ
  only by two counters the ORIGINAL run's importer gained after they
  were imported — schema evolution of the original run, root-caused);
  changed windows == the frozen 7, zero impact-radius violations;
- conservation on the new runroot: 105 windows, **0 violations**, the
  ts source duplicate still observed;
- S1/S2 old->new: result fields changed in exactly the frozen 7 cases;
  **no evaluation rank of any case changed in either condition**; the
  single evaluation delta is n_candidates 9->10 for ts-2023-01-29-027;
  e2-tables and e3-tables **byte-identical**; ablations differ only in
  that same case's n_candidates (`d019-case-deltas.json`);
- isolation gate 101/101; six-class stability gate **0 across all six
  classes** (`s2-order-stability-sorted-fullrecord-d019.json`); H4
  walk **1495/1495** (one candidate added by the repaired ts window),
  zero failures under the strengthened D-017 checker;
- per the preregistered decision rule (`d019-threshold-recheck.json`):
  no frozen useful-effect threshold changes its pass/fail state —
  **the scientific verdict is NOT reopened**: H1 falsified, H2
  falsified, H3 untested, frozen H4 inconclusive, H4-source-attribution
  verified (now 1495/1495), PIVOT stand.

**Fact for the record.** This was the first external finding against
the canonical DATA rather than a checker or claim: 3,318 events were
mis-multiplied or silently destroyed at ingestion, and after repair
not a single ranking moved. The evidence pipeline's verdicts were
robust to its own ingestion defects — established by regeneration
under a preregistered radius and rule, not assumed.

**Outcome data seen at decision time:** the RED, impact scan, decision
rule, and alert-scope rule were committed before any regenerated
ranking existed; everything downstream is mechanical consequence.
Results remain exploratory per D-001/D-008.

---

## 2026-08-30 — D-020: Conservation gate bound to the expected-window manifest

**Trigger.** Codex P2 on PR #20 (tenth round, head `f056a32`): the
D-019 conservation gate globbed whatever import reports existed, so an
empty runroot exited 0 with windows=0 violations=0, and a regeneration
missing entire windows could be certified green — silently accepted
missing evidence, the same truncation-blindness family as D-016/D-018.

**RED (`706a976`, `regate/d020-conservation-empty-RED.json`).** The
pre-fix gate against an empty directory: windows=0, violations=0,
exit 0.

**Fix + GREEN (this commit).** The gate now derives the expected
window manifest from the experiment's own fault lists and normal
windows (the same source run_e2 drives imports from) and counts every
missing or unexpected window as a violation. Verification: empty
runroot → 105 violations, every expected window named MISSING, exit 1
(`regate/d020-conservation-empty-caught.json`); the regenerated D-019
runroot → 105 expected / 105 found, 0 violations, source duplicate
still observed, exit 0
(`regate/d019-conservation-GREEN-d020.json`).

**Outcome data seen at decision time:** all; gate hygiene only — no
number, table, or verdict changes.

---

## 2026-08-30 — D-020 (completion): committed manifest, truncation and swap mutations, owner acceptance framing

**Owner gate on D-019/D-020.** D-019 is ACCEPTED as a scientific
re-gate; the scientific verdict is unchanged. D-020 is prospective
gate-hardening only — the D-019 canonical data and metrics are not
touched. **D-019's measurement remains valid regardless of the D-020
defect**: its preserved conservation GREEN already shows all 105
windows checked, and `d019-importer-differential.json` independently
covers 98 unchanged + 7 changed = 105; what D-020 fixes is the
permanent checker's prospective ability to detect an incomplete FUTURE
run.

**Completion beyond the first D-020 commit, per the owner contract:**
- The expected-window set is now a COMMITTED manifest
  (`manifests/expected-windows.json`, 105 entries derived once from
  the pinned fault lists and normal windows) — never derived from the
  runroot being checked; the gate additionally cross-checks the
  committed file against a fresh fault-list derivation and hard-errors
  on any disagreement, so neither can drift silently.
- Two further mutation REDs against the f056a32 gate
  (`regate/d020-conservation-mutations-RED.json`): a 104-window
  runroot (one expected window removed) — old gate exit 0; and a SWAP
  runroot (one expected window removed, a copy of another planted
  under the foreign tag `2099-01-01_0000`, count still 105) — old gate
  exit 0. The swap case defeats any lazy `count != 105` fix; the
  manifest gate catches it as one missing PLUS one unexpected window.
- GREEN (`regate/d020-conservation-mutations-caught.json`): the
  manifest gate exits 1 on both mutations with every missing and
  unexpected window named; on the real D-019 runroot
  (`regate/d019-conservation-GREEN-final.json`):
  expected_windows=105, discovered_windows=105, missing=[],
  unexpected=[], windows_with_violations=0, the ts source duplicate
  still observed, exit 0. Coverage and content conservation are now
  separate, explicit facts in the gate result — not one green light.

**Outcome data seen at decision time:** all; prospective gate
hardening only.

---

## 2026-09-01 — D-021: Preregistered §9 paired statistical analyses executed

**Trigger.** Codex P1 on PR #20 (eleventh round, head `baca6df`),
verified against the frozen text: 00-preregistration.md §9 froze
"McNemar exact test on @1 hits; bootstrap (10,000 resamples, seed
20260820) 95% CI for MRR differences" for paired per-case comparison
between conditions on the same dataset — and a repo-wide search
confirms these analyses existed nowhere outside the preregistration.
The generated reports carried point estimates alone: a genuine
preregistration-compliance gap in how the H1/H2 falsifications were
reported.

**Remedy (this commit).** `evaluators/paired_stats.py` implements §9
verbatim (exact binomial McNemar on discordant @1 pairs; Mersenne
Twister seeded 20260820, index resampling, percentile 95% CI; per
dataset, no pooling; alpha 0.05; effect sizes regardless). Cases are
paired by the fault itself (dataset, inject_time, inject_pod,
inject_type) — verified unique and identical across all condition
files (the N1 evaluator and S drivers number case_ids differently).
Machine artifact: `results/e3/paired-stats.json`.

**Results.**
- H1 (N1 vs S1, both datasets, all three rank levels): McNemar exact
  p ≤ 1.9×10⁻⁹ (OB service_dedup 1.4×10⁻¹⁴); MRR differences
  +0.54…+0.89 with 95% CIs bounded away from zero (worst lower bound
  +0.43). The representation collapse is statistically decisive.
- H2 (S1 vs S2, both datasets, all levels): McNemar exact
  p = 0.45…1.0; every MRR-difference 95% CI straddles zero
  (|point estimate| ≤ 0.042). No S2 advantage — or even direction —
  is detectable.

**Verdict impact.** None: the analyses complete the frozen reporting
and support the existing verdicts. H2's falsification rests, as
frozen, on the §7 useful-effect thresholds (which S2 fails), not on
an equivalence claim; the non-significant tests add that no direction
is detectable. Docs re-derived (04, 05 §2, final-report Q5).

**Outcome data seen at decision time:** all — the analyses were
specified in full (tests, resamples, seed, alpha) before any outcome
existed; only their execution was late. Results remain exploratory
per D-001/D-008.

---

## 2026-09-01 — D-022: Frozen §8 case-failure policy enforced in the E2/E3 drivers

**Trigger.** Codex P2 on PR #20 (eleventh round, head `baca6df`),
verified against the frozen text: 00-preregistration.md §8 says "a
fault whose pipeline run crashes is counted as unlocalized and the
cause reported separately" — but neither `scripts/run_e2.py` nor
`scripts/run_e3.py` implemented it. Any single-case crash (importer,
fold, scorer, graph build) propagated out of `main()` and aborted the
whole namespace: instead of one unlocalized case, ALL cases of the
namespace were lost. The policy existed only on paper; nothing in the
committed results is wrong (no crash ever occurred on the real data),
but the enforcement gap was real.

**RED (`regate/d022-driver-abort-RED.json`).** Sandbox = symlink copy
of the real ts runroot with one abnormal window's `scenarios.jsonl`
replaced by a corrupt JSON line (its cached `graph.json` removed).
Pre-fix `run_e2.py`: exit 1, **no `s1-ts.cases.json` written** — the
45-case namespace lost to one bad window.

**Remedy (this commit).** Both drivers wrap the per-case body in
try/except. A crashed case is appended as a failure record — case
identity, `"failure": "<ExceptionType>: <cause>"`, evaluation ranks
all `None` with `n_candidates` 0 — so it counts as unlocalized in
every aggregate (§8), and the summary gains `failed_cases` listing
each failure with its cause (§8 "separately reported"). The key is
emitted ONLY when failures exist, keeping clean runs byte-identical.

**GREEN (`regate/d022-driver-failure-GREEN.json`).** Same sandbox:
`run_e2.py` exit 0, ts-2023-01-29-001 recorded as
FAILED-counted-unlocalized (service_dedup unlocalized 11 = 10 real
+ 1 crashed), remaining 44 cases evaluated normally; `run_e3.py`
likewise (st-graph exit 65 captured as the failure cause).
Success-path identity: the fixed `run_e2.py` over the real untouched
ts runroot reproduces the committed `results/e2/s1-ts.cases.json`
with **0 differing cases** (modulo `runtime_ms`) and an equal
summary — the fix cannot change any committed number.

**Verdict impact.** None — enforcement of a frozen policy on a path
never taken by the real data. All committed artifacts untouched.

**Outcome data seen at decision time:** all; prospective enforcement
only.

---

## 2026-09-01 — D-023: Conservation gate fails closed on absent counters

**Trigger.** CodeRabbit final-round Minor on
`scripts/check_ingestion_conservation.py` (review of
`340e811..baca6df`), verified: every counter comparison used
`c.get(name, 0)`, so a MISSING counter silently read as zero. An
expected window whose report was gutted to `{"counters": {}}` with
empty `events.jsonl`/`provenance.jsonl.gz` satisfied every
conservation equation as 0 == 0 and passed the D-020 coverage check
(the directory exists) — the gate failed open on exactly the kind of
absence it was built to catch.

**RED (`regate/d023-conservation-missingcounter-RED.json`).** Sandbox
= symlink copy of the D-019 runroot with one expected window gutted as
above. Pre-fix gate: exit 0, `violations=0` — blind.

**Remedy (this commit).** Two-tier fail-closed rule matching how the
importer actually writes counters: (a) ANCHOR counters that
`import_nezha.py` assigns unconditionally (`traceids_listed`,
`traceids_unique`, `traceids_duplicate_entries`, `trace_rows_read`,
`log_rows_read`, `log_rows_repaired_tokenization`,
`log_rows_artifact_style_read`, `events_emitted`) must be PRESENT in
every window — absence is a named violation proving a gutted or
foreign report; (b) occurrence counters (`spans_accepted`,
`logs_accepted`, `alert_events`, `sessions_emitted`) are
increment-only, so absence is legitimate IFF the corresponding
emitted count is zero — nonzero emissions without the counter are a
violation. A naive "missing = violation" rule was tried first and
falsely flagged 25 real ts windows that carry no `alert_events`
counter because the window has zero alerts; the two-tier rule keeps
those true zeros. Added equation: `events_emitted` must equal the
actual `events.jsonl` line count.

**GREEN (`regate/d023-conservation-missingcounter-caught.json`,
`d023-conservation-GREEN.json`).** Fixed gate on the sandbox: exit 1,
the gutted window flagged with 8 named missing-anchor violations; a
second mutation (real window, `alert_events`=565 deleted from its
report) flagged as "565 alert events emitted but alert_events counter
missing" — both fail-closed branches proven. On the real D-019
runroot: 105/105 windows, 0 violations, ts source duplicate still
observed, exit 0.

**Verdict impact.** None; prospective gate hardening only.

**Outcome data seen at decision time:** all.

---

## 2026-09-01 — D-024: §8 failure boundary extended to the shared construct window

**Trigger.** Codex P2 on PR #20 (twelfth round, exact head `0ea15e4`),
verified: the D-022 failure boundary wrapped only the per-case body,
while the per-date construct-window `import_and_fold` in
`scripts/run_e2.py` sat before it. A construct failure crashes every
fault of its date, and the exception propagated out of `main()` — the
namespace aborted with zero failure records, so the frozen §8
full-denominator policy was still unenforced for exactly the shared
failure mode the per-case boundary cannot see.

**RED (`regate/d024-construct-abort-RED.json`).** Sandbox ts runroot:
all rca windows and the 2023-01-30 construct window cached (file
symlinks to the real runroot); the 2023-01-29 construct window's
`events.jsonl` replaced by a directory so the importer's write fails
deterministically. Pre-fix driver: exit 1, **no `s1-ts.cases.json`** —
45 cases lost. (A chmod-based mutation was tried first and did not
fail: the session runs as root, which bypasses permission bits.)

**Remedy (this commit).** `run_e2.py` reads the date's fault list
before the construct import and wraps the import/fold in the failure
boundary: on failure, every fault of the date is appended as a §8
failure record (identity, abnormal window, `construct window <win>:
<cause>`, ranks None) and the driver continues with the next date.
`run_e3.py` needs no change — it performs no construct import (E2
cache) and consumes the normal-window files inside the per-case try,
so the same failure mode already yields per-case records there.
Enforcement boundary, stated explicitly: everything downstream of
case enumeration (the fault-list read) is §8-accounted; failures of
enumeration itself or of namespace-level tooling (template dump)
still abort loudly with no results file — a visibly absent run, not a
silently distorted denominator.

**GREEN (`regate/d024-construct-failure-GREEN.json`).** Same sandbox:
exit 0, n_cases 45 — all 28 faults of 2023-01-29 recorded as
construct-window failures with the shared cause, all 17 faults of
2023-01-30 evaluated normally (dedup unlocalized 36 = 28 + 8 genuine).
Success-path identity: the fixed driver over the real untouched ts
runroot reproduces the committed `s1-ts.cases.json` with 0 differing
cases (modulo `runtime_ms`), summary equal.

**Verdict impact.** None — prospective enforcement on a path never
taken by the real data; committed artifacts untouched.

**Outcome data seen at decision time:** all.

---

## 2026-09-01 — D-025: Isolation-gate case universe certified against a committed manifest

**Trigger.** Codex P2 on PR #20 (twelfth round, exact head `0ea15e4`),
verified by direct mutation: `scripts/check_s1s2_isolation.py` built
its case universe as the union of the two result files' own id sets.
The D-016 union rule catches one-sided truncation, but a case deleted
from BOTH S1 and S2 vanished from the union entirely — the gate
compared 100 cases and exited 0, certifying a bilaterally truncated
experiment against the documented 101/101 claim. The id universe was
self-certified by the very files under check — the same defect class
D-020 eliminated from the conservation gate.

**RED (`regate/d025-isolation-bilateral-RED.json`).** Copies of the
committed e2/e3 results with `ts-2023-01-29-000` deleted from both
sides (45→44 each): pre-fix gate prints
`cases=100 isolation_violations=0`, exit 0 — blind.

**Remedy (this commit).** New committed manifest
`manifests/expected-cases.json`: the frozen 101-case universe (56
hipster + 45 ts), derived from the pinned fault lists exactly as the
drivers number cases. The gate loads it, cross-checks it against a
fresh fault-list derivation on every run (hard error on
disagreement — mirroring D-020, so neither the file nor the
derivation can drift), and iterates over manifest ∪ s1-ids ∪ s2-ids;
absence from s1, s2, or the manifest (a foreign id) is a named
violation, `missing_from` now listing every absent side.

**GREEN (`regate/d025-isolation-manifest-caught-GREEN.json`).** On the
mutated results: exit 1, `cases=101`, the bilaterally deleted case
flagged with `missing_from: [s1, s2]`. On the real committed results:
`cases=101 isolation_violations=0`, exit 0. 05 §1 updated to state
the manifest-certified universe.

**Verdict impact.** None; prospective gate hardening only.

**Outcome data seen at decision time:** all.

---

## 2026-09-01 — D-026: Trace-pair provenance verified by full recomputation; all-derivation audit gate

**Trigger.** Codex P1 on PR #20 (thirteenth round, exact head
`5d411c7`), verified against the code: in
`scripts/check_h4_provenance.py` the trace-latency-pair branch checked
only a pod substring on the child row and NOTHING on the parent row —
no `child.ParentID == parent.SpanID` linkage, no latency, no
percentile; the derivation's own stored `verified` flag was the only
value check (exactly the trust D-017 removed for metric cells but left
for trace pairs).

**Population fact the finding missed (recorded for honesty).** In the
live 1495-chain population the weak branch is dead code: walked
candidates point at NORMAL sessions, whose four windows carry only two
alert chains, both metric-sample; all 28 trace-derived-p90 derivations
live in rca windows the walk never loads. So corrupt trace pairs could
not have produced the committed 1495/1495 — but the real gap is WIDER
than claimed: outside the two walked chains, the entire
118-derivation population had NO independent verifier at all (only the
importer's own at-materialization shadow check).

**RED (`regate/d026-h4-tracepair-RED.json`).** Sandbox construct
runroot with the walked hipster derivation REPLACED by a fabricated
trace-derived-p90 (4 arbitrary pairs: real child rows of the pod,
arbitrary foreign parents, `verified: true`). Pre-fix checker:
**1495/1495, exit 0** — precisely the claimed blindness.

**Remedy (this commit).**
- `check_h4_provenance.py`: trace-derived-p90 derivations are verified
  by full independent recomputation from the referenced trace CSV
  (quoting-aware parser; child rows of the pod; parent = LAST row per
  SpanID; cross-pod links; `(parent_end − child_end)/1e6`; p90 if > 2
  samples else 10.0) with exact equality on the recorded pair list,
  value and n_samples — the ParentID↔SpanID linkage holds by
  construction. Fallback-threshold rows now re-parse the
  `NetworkP90(ms)` cell. Validated before the change: the
  recomputation reproduces all 28 real derivations exactly (28/28
  pair lists, values, n_samples).
- NEW standing gate `scripts/check_alarm_derivations.py`: audits EVERY
  materialized derivation of EVERY window (coverage certified against
  the D-020 expected-window manifest), never trusting the stored
  `verified` flag — closing the scope gap above.
- Parser honesty note: the audit surfaced 3 rca metric cells where
  CPython `float()` and pandas' C tokenizer disagree by 1 ULP; the
  recorded values equal the pandas (the artifact's own) parse. The
  metric-cell comparison is now two-tier — raw strtod fast path, the
  artifact's parser as authority on disagreement; a corrupted recorded
  value fails both.

**GREEN (`regate/d026-h4-tracepair-caught.json`,
`d026-derivation-audit-GREEN.json`).** New checker on the mutation:
exit 1, 1484/1495 with all 11 chains through the fabricated derivation
named. Audit-gate mutations (re-aimed `parent_row`; recorded p90 value
+1e-6 with pairs intact; recorded metric value +0.5): all three caught
with exact reasons, exit 1. Real runroot: walked H4 **1495/1495 exit
0** unchanged; audit gate **118 derivations across 105/105 windows, 0
failures, exit 0** (90 metric-sample + 28 trace-derived-p90; the
fallback-threshold path remains unused). 05 §4 updated.

**Verdict impact.** None — H4-source-attribution keeps exactly its
stated strength, now under a checker with no weak branch and an
audit of the full derivation population; frozen H4 remains
INCONCLUSIVE per D-009.

**Outcome data seen at decision time:** all.

---

## 2026-09-01 — D-027: E2 report generator handles §8 failure records

**Trigger.** Codex P2 on PR #20 (thirteenth round, head `5d411c7`),
verified: `scripts/make_e2_report.py` read `c["ingestion"]`
unconditionally, but §8 failure records (D-022/D-024) intentionally
carry no ingestion block — so generating the E2 report for exactly the
full-denominator runs the failure handling protects raised `KeyError`
and produced no report.

**RED (`regate/d027-report-keyerror-RED.json`).** Pre-fix copy against
a results dir with one §8 failure record (the D-022 sandbox): exit 1,
`KeyError: 'ingestion'` at the ingestion-totals line, report aborted.

**Remedy (this commit).** Ingestion totals sum over cases that carry
an ingestion block; when failure records exist, one extra line reports
them (`Failed cases (counted unlocalized, §8): N — ids`), emitted ONLY
then, keeping clean-run output byte-identical. Every other per-case
field access was audited against the failure-record shape — the
existing None-as-unlocalized aggregation already covers evaluation
ranks; nothing else needed guarding.

**GREEN (`regate/d027-report-failurecase-GREEN.json`).** Fixed script
on the failure-record dir: exit 0, complete report, totals over the 44
evaluated cases, failure line naming ts-2023-01-29-001. Clean-run
identity: stdout byte-identical to the committed
`results/e2/e2-tables.md` (sha256 match).

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-01 — D-028: §8 boundaries in the modality-ablation driver

**Trigger.** Codex P2 on PR #20 (thirteenth round, head `5d411c7`),
verified: `scripts/run_e3_ablations.py` implemented neither the
per-case §8 boundary (D-022) nor the shared-window boundary (D-024) —
one filter/fold or scorer crash escaped `main()` and the driver wrote
NO ablation artifact at all.

**RED (`regate/d028-ablation-abort-RED.json`).** Sandbox ts runroot
(file symlinks) with one abnormal window's `events.jsonl` corrupted
(window `2023-01-29_1544`, serving exactly case ts-2023-01-29-027).
Pre-fix driver: exit 1 on `JSONDecodeError`, no
`s1-ablations-ts.json`, all 3 variants × 45 cases lost, zero failure
records.

**Remedy (this commit).** Mirrors `run_e2.py`: per variant and date
the fault list is read before the shared normal-window fold; that fold
failing appends a §8 failure record for every fault of the date and
continues (D-024 boundary); the per-case body is wrapped likewise with
the case's own cause (D-022 boundary); each variant summary gains
`failed_cases` ONLY when failures exist. The aggregation already
counts None ranks as unlocalized over the full n=45 denominator.

**GREEN (`regate/d028-ablation-failure-GREEN.json`).** Same sandbox:
exit 0; in each of the three variants exactly ts-2023-01-29-027
recorded as failed (JSONDecodeError cause), 44 cases evaluated,
denominators intact (no-alerts/no-logs aggregates match the committed
artifact exactly — case 027 was already genuinely unlocalized there;
no-spans unlocalized is committed+1 since 027 scored rank 1 there
pre-corruption). Success-path identity: fixed driver over the real
runroot regenerates `s1-ablations-ts.json` byte-identical to the
committed copy (sha256 unchanged).

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-01 — D-029: Paired-stats pairing keys fail closed on duplicates

**Trigger.** CodeRabbit Major (incremental review of
`baca6df..5d411c7`) on `evaluators/paired_stats.py`, verified by
direct mutation: the pairing dicts are comprehensions, so a duplicate
fault key silently overwrites; with the same duplication planted in
ALL THREE inputs (a duplicated fault masking a missing one), both
existing assertions pass and every ts comparison silently ran on
n = 44 instead of 45 (RED: pre-fix exit 0, no complaint;
`regate/d029-pairing-duplicate-RED-caught.json`). CodeRabbit's own
scan confirmed the real inputs carry no duplicates — the defect was
latent, same fail-open class as D-020/D-023/D-025.

**Remedy (this commit).** Indexing goes through a helper that raises
`SystemExit` naming the first duplicate pairing key and its input
(`duplicate pairing key in n1/ts: (...)` on the mutation, exit 1).

**GREEN.** The fixed script over the real committed inputs
regenerates `paired-stats.json` byte-identical to the committed
artifact. Verdict impact: none.

**Outcome data seen at decision time:** all.

---

## 2026-09-01 — D-030: §8 failure records schema-compatible with success records

**Trigger.** CodeRabbit Minor (same review) on `run_e2.py`, verified:
the D-022/D-024 failure records omitted success-record keys, so
`check_s1s2_isolation.py` — which indexes `["candidates"]` on both
sides — raises `KeyError` on any failed case (RED: unhandled
traceback, a crash rather than a named violation;
`regate/d030-failure-record-schema-RED-GREEN.json`).

**Remedy (this commit).** Failure records now carry the schema:
`run_e2.py` (both failure paths) adds `candidates: []` and
`ground_truth: null`; `run_e3.py` adds those plus `abnormal_window`
and `representation`. The changes live strictly inside
except-branches — clean runs are statically unchanged (success-path
identity proven at D-022/D-024; success paths untouched). Ablation
records (D-028) keep their slim shape: no consumer indexes
candidates there.

**GREEN.** The D-022 sandbox rerun with the fixed driver produces the
new-shape record, and the isolation gate now completes and NAMES the
crashed case as an isolation violation (ts-2023-01-29-001,
n_candidates 0 vs 16, exit 1) — the semantically right outcome: a
case crashed in one condition breaks tie-break-only attribution for
that case and is flagged as data, not as a crash. Verdict impact:
none.

**Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-031: Fold caches publish atomically; all 105 cached folds re-verified

**Trigger.** Codex P1 on PR #20 (fourteenth round, exact head
`7155f76`), verified by direct mutation: `import_and_fold` treated the
mere existence of `scenarios.jsonl` as a completed fold. An
interrupted `st-fold` leaves a syntactically valid prefix under the
cache key, and every later E2/E3/ablation run silently scores the
incomplete window.

**RED (`regate/d031-cache-poisoning-RED-GREEN.json`).** Sandbox ts
runroot with one window's `scenarios.jsonl` truncated to a valid
30-line prefix of the real 61: the pre-fix driver ran to **exit 0
with no failure record** and produced a materially wrong case —
ts-2023-01-29-001 scored over 30 sessions instead of 61, 42
candidates instead of 16, service_dedup rank 5 instead of 3. Exactly
the claimed silent scoring of an incomplete window.

**Remedy (this commit).** All three cache producers fold/build to
`<name>.tmp` and publish with `os.replace` only after success:
`run_e2.py` (`scenarios.jsonl`), `run_e3.py` (`graph.json`),
`run_e3_ablations.py` (variant `scenarios.jsonl`). The cache
invariant becomes: the key exists ONLY if the producing step
completed; an interruption leaves at most a `.tmp` the existence
check never consults.

**GREEN.** With the partial parked at `scenarios.jsonl.tmp` (the only
state an interruption can now leave), the fixed driver cache-misses,
refolds byte-identically to the canonical file, and reproduces the
committed `s1-ts.cases.json` with 0 differing cases (modulo
`runtime_ms`), summary equal. The prospective invariant says nothing
about already-cached files, so every cached fold of the canonical
runroot was additionally refolded from its `events.jsonl` and
byte-compared: **105 windows, 0 mismatches** — the committed results
were built from sound caches.

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-032: Metric provenance rows validated against the report's window

**Trigger.** Codex P2 on PR #20 (fourteenth round), verified with a
LIVE mutation pair: the importer selects metric samples by window AND
value jointly, but the checkers verified only a pod substring and the
cell value — and the walked hipster 0351 adservice derivation's file
really does contain a second row with the identical value
81.07942708333333 one minute later (03:52, outside the window).
Re-aiming the input row 0 → 1 passed the pre-fix walked H4 check
(1495/1495, exit 0) and the pre-fix audit gate.

**Remedy (this commit).** `check_metric_row_identity` (shared by
`check_h4_provenance.py` and `check_alarm_derivations.py`): the
referenced row's parsed `PodName` must equal the alarmed pod exactly
and its `Time` cell must match the report's window via the importer's
own `re.search(window, Time)`; the window is derived from the runroot
directory tag (`tag_to_window`).

**GREEN (`regate/d032-metric-window-RED-caught.json`).** On the
mutation: walked H4 exit 1 with all 11 chains through the derivation
named "metric row outside window …"; audit gate names the same. Real
runroot: walked 1495/1495 exit 0; audit 118 derivations, 0 failures —
every real metric input's Time matches its window.

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-033: Alarm ↔ derivation completeness enforced in the audit gate

**Trigger.** Codex P2 on PR #20 (fourteenth round), verified: the
audit gate iterated only MATERIALIZED derivations, and its own
docstring deferred completeness to the importer — self-certification.
A report copy with one derivation deleted (alarm_list intact) was
checked as 117 instead of 118 and exited 0.

**Remedy (this commit).** Per window the expected key set
{pod|metric_type} is derived from `alarm_list` exactly as
`build_derivations` constructs keys; `expected − materialized` and
`materialized − expected` are both named violations, raised before
the content audit.

**GREEN (`regate/d033-derivation-completeness-RED-caught.json`).** The
mutation is caught as "alarm has no materialized derivation"; real
runroot: expected == materialized in every window, 118 checked, 0
failures, exit 0.

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-034: Malformed derivation inputs fail closed instead of crashing

**Trigger.** CodeRabbit Minor (final round, range
`5d411c7..7155f76` — its Merge Risk simultaneously dropped to Low),
verified at unit level against the pre-fix function: a trace
derivation with `inputs: [{}]` raised `KeyError: 'file'` at
`inputs[0]["file"]`, and a mix of present and missing file fields
raised `TypeError` in `sorted({str, None})` — a crash past the
audit's summary rather than a named failure.

**Remedy (this commit).** Every trace input must be a dict carrying a
non-empty string `file` before any dereference; otherwise the named
failure "trace derivation has an input without a source file" flows
through the normal audit path.

**GREEN (`regate/d034-malformed-input-RED-caught.json`).** A planted
`inputs: [{}]` mutation yields exit 1 with the violation named and
the summary written; real runroot unchanged (118/0 failures, walked
H4 1495/1495).

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-035: The H4 walk verifies the candidate ↔ event link itself

**Trigger.** Codex P1 on PR #20 (fifteenth round, exact head
`122c929`), verified by direct mutation: the walk checked
event↔provenance↔source but never CANDIDATE↔event — a candidate's
`(normal_session, first_seq)` pointer re-aimed at an unrelated
same-pod event passed (pre-fix checker: 1495/1495, exit 0), so the
whole chain validated the wrong event.

**Semantics note (recorded because the obvious fix is wrong).** The
naive check "scenario `actions[seq]` == `pattern[0]`" does NOT hold on
this data: folding drops events, so action index ≠ event seq
(1322/1495 match, 117 out-of-bounds, 56 shifted). The correct
independent link is at the event level — the fold preserves
`command_id` verbatim, so `pattern[0]`'s command must equal the
pointed event's `command_id`, and the candidate's `pod` must equal
the event's pod. Verified to hold **1495/1495** on the real data
BEFORE the checker change.

**Remedy (this commit).** `check_h4_provenance.py` adds both link
checks before the provenance step; either mismatch is a named
failure.

**GREEN (`regate/d035-candidate-link-RED-caught.json`).** The
mutation (seq 45 → 37, same pod, different command) is caught with
both commands named; real runroot: 1495/1495 exit 0. Granularity
caveat recorded: a re-aim to a different OCCURRENCE with the same pod
and same command stays within the claim's unit — the chain still ends
at a genuine occurrence of the same canonical action by the same pod.

**Verdict impact.** None — H4-source-attribution retains its stated
strength under a checker that now pins the candidate end of the
chain. **Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-036: Misaligned E0/E1 log cases fail closed as §8 failures

**Trigger.** Codex P2 on PR #20 (fifteenth round), verified by direct
mutation: `evaluators/independent_eval.py` recorded an alignment
mismatch but still scored the positional candidates against the
current fault. With ONE case header removed from a copy of the
hipster artifact log, the pre-fix evaluator reported 47 alignment
errors yet still scored 32 shifted positions — service_dedup
silently became top1 36 / AC@1 64.29 (committed: 52 / 92.86):
fabricated ranks, not a failure.

**Remedy (this commit).** A misaligned position is a §8 failure:
`alignment_failure` records the log case's identity, all four ranks
are None, `candidates` empty — the fault stays in the denominator as
unlocalized; aligned positions evaluate exactly as before.

**GREEN (`regate/d036-alignment-failclosed-RED-GREEN.json`).** On the
doctored log: 47 alignment_failure records, zero shifted positions
scored, aggregates degrade honestly. Identity: the fixed evaluator
over the REAL artifact logs regenerates BOTH committed eval artifacts
byte-identically (`hipster-service.eval.json`,
`ts-service.eval.json`; `alignment_errors` is empty on real data, so
the committed E0/E1 numbers carry no fabricated rank).

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-037: The derivation-level value is tied to its validated inputs

**Trigger.** Codex P1 on PR #20 (sixteenth round, exact head
`463df6a`), verified by direct mutation on BOTH gates: for
metric-sample derivations nothing related `der["value"]` back to the
validated inputs — `check_metric_cell` compares the CSV cell to the
INPUT's value and the identity checks cover pod/window, so a
corrupted derivation value with intact inputs passed the audit gate
(118 checked, 0 failures, exit 0) and the walked H4 check (1495/1495,
exit 0) while the reported alarm value was no longer derived from the
referenced telemetry.

**Remedy (this commit).** In both checkers every validated input's
`value` must equal `der["value"]` — the importer selects metric rows
BY that value, so equality holds by construction on honest data.
Trace-derived derivations already recompute `der["value"]` in full
(D-026); fallback-threshold rows are covered by the same comparison.

**GREEN (`regate/d037-derivation-value-RED-caught.json`).** Both
mutations (+0.5 on a rca audit derivation; +0.5 on the walked
construct derivation) are caught with the exact values named; real
runroot: audit 118 across 105/105 windows with 0 failures, walked H4
1495/1495 — every input value equals its derivation value.

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-02 — D-038: Named-failure guards on the remaining record dereferences

**Trigger.** CodeRabbit Minor (final round, range `7155f76..463df6a`),
verified by direct mutation: two sites added in D-033/D-035 indexed
persisted-record fields without the D-034 named-failure rule. A
malformed candidate `pattern` crashed the H4 walk with `IndexError`
past its summary; in the audit gate a pod-less `alarm_list` entry
carrying alarms crashed with `KeyError`, while a bare `{}` entry was
SILENTLY SKIPPED — a fail-open the finding did not even claim.

**Remedy (this commit).** The walk validates the pattern's shape
before dereference ("malformed candidate pattern" named failure); the
audit validates every `alarm_list` entry and every alarm entry,
naming malformed ones as violations and continuing through its normal
summary path.

**GREEN (`regate/d038-malformed-record-guards-RED-caught.json`).**
All three mutations (pattern `[[]]`; bare `{}`; pod-less entry with
alarms) yield exit 1 with the malformed record named and the summary
written; real runroot unchanged — walked H4 1495/1495, audit 118
across 105/105 windows with 0 failures.

**Verdict impact.** None. **Outcome data seen at decision time:** all.

---

## 2026-09-03 — D-039: Container-type guards in the derivation audit

**Trigger.** CodeRabbit actionable (exact-head round on `94cd0b6`,
range `463df6a..94cd0b6`), verified by direct mutation: D-038 guarded
the SHAPE of `alarm_list` entries and alarm entries but not the TYPE
of the containers themselves. A null `alarm_list` crashed the audit
gate with `TypeError` at the exact line the finding names — exit 1
via traceback, no summary, no out.json. Four same-family mutations
(null nested `alarm`; list-typed `alarm_provenance`; a derivation
record replaced by a string; string-typed derivation `inputs`)
likewise crashed with `TypeError`/`AttributeError` before the
summary — the last three beyond what the finding claimed.

**Remedy (this commit).** Every container and record is type-checked
before iteration or dereference: non-list `alarm_list` and nested
`alarm`, non-dict `alarm_provenance`, non-dict derivation records,
non-list `inputs`, and non-dict input records are named violations
through the normal summary/exit path, with coverage accounting
intact.

**GREEN (`regate/d039-malformed-collection-RED-caught.json`).** All
five mutations yield exit 1 with the malformed container/record named
first and summary + out.json written; real runroot: the old and the
fixed gate are BYTE-IDENTICAL on stdout and out.json — audit 118
across 105/105 windows, 0 failures.

**Verdict impact.** None. **Outcome data seen at decision time:** all.
