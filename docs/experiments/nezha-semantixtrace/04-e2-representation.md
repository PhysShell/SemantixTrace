# 04 — E2: Representation experiment (N1 vs S1)

Status: **design frozen, run in progress**. Results section is appended
only from machine artifacts (`experiment/nezha/results/e2/`).

Question (H1): does the SemantixTrace canonical representation change RCA
quality when the ranking algorithm is held fixed?

## 1. Conditions

- **N1** — Nezha representation + historical scorer + corrected
  evaluator. Numbers come from the E0 runs re-evaluated under the
  preregistered semantics (03-e1-measurement-audit.md §1): these are the
  dense / service_raw / service_dedup rows.
- **S1** — the same telemetry mapped through the frozen importer
  (00-preregistration.md §6) into `trace_schema` v2 sessions, folded by
  the real `trace-normalizer` (st-fold), scored by a component-by-
  component mirror of the artifact's active algorithm
  (`adapters/s1_scorer.py`, mapping documented in its docstring),
  evaluated with the same corrected semantics (`evaluators/s1_eval.py`).

Same fault windows (inject+2min, artifact's own derivation mirrored),
same per-date construct windows, same alarm detection (the artifact's
`generate_alarm`, invoked unmodified), same scoring constants
(support > 5, Score_min 0.67).

## 2. Design completions beyond the frozen mapping table (D-006)

Fixed before the full run; disclosed contamination: a single-case
mechanical smoke test (hipster case 000) was executed to validate the
pipeline plumbing before this freeze; no design parameter was changed in
response to its output.

1. **Alert session assignment**: one alert event per (session, alarmed
   pod) for sessions whose spans touch the pod; ts = window start (so
   alerts sort by timestamp among real events, which for traces starting
   before the window boundary places them mid-session).
2. **seq tie-break** at equal timestamps: (kind_rank, source_row),
   alert=0 < span=1 < log=2.
3. **Depth analog** for the linear canonical representation: depth of an
   action occurrence = 1 + number of preceding `span:` actions in its
   session; candidate depth = max over construct-window sessions
   (mirrors get_deepth_pod's "count start events on the walk to root",
   which has no non-linear analog in a folded scenario).
4. **Pod attribution via provenance**: the pod of the source event
   backing the max-depth occurrence (st-fold's verified action↔event
   alignment). The artifact's hardcoded fallback pod has no analog —
   attribution cannot fail by construction.
5. **Log→session join** via the log rows' own TraceID column (present in
   the source schema), rather than the artifact's SpanID-based join.

## 3. Ingestion parity and loss accounting

Per window, `import-report.json` counts every read/accepted/rejected/
excluded record by reason, plus a drain3 new-cluster tripwire (E0
established the shipped template vocabulary is closed on this dataset:
674/694 clusters, zero new/changed across full runs — verified again on
every imported window). The v2 JSONL is consumed through the crates'
fail-closed `read_event` path; st-fold verifies its provenance alignment
against `normalize()` on every session and aborts on divergence.

## 4. Results

(appended from machine artifacts after the run; every number generated,
none hand-typed)
