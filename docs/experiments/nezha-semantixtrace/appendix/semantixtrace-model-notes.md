# SemantixTrace model notes for E2 importer design

Exploration snapshot (2026-08-20, working tree at merge of PR #18). Purpose:
inputs for the preregistration's `TO-FREEZE` telemetry-mapping section.
File references are to this repository.

## Stage reality check

S0–S8 are landed (AGENTS.md:48; docs/stages/ status lines): schema v2 +
upcasters, normalizer, graph (S4), oracle (S5), CLI, storage JSONL+SQLite,
align (S8). Only S9–S12 are planned. The research contract's assumption
that only S1–S3 exist is out of date; the "graph/oracle machinery" of E3 is
real code, not vaporware.

## Canonical event model (crates/trace-schema)

- `Current = v2::TraceEvent` (lib.rs:43-48, CURRENT_SCHEMA_VERSION=2):
  `seq: EventSeq` (per-session monotonic u64), `session_id: SessionId`
  (UUID), `ts: DateTime<Utc>`, `correlation_id: Option<CorrelationId>`
  (UUID), `domain_entity_id: Option<DomainEntityId>` (free string),
  flattened `kind: TraceEventKind`.
- `TraceEventKind` (v1.rs:79-148, frozen by ADR-0006, not non_exhaustive):
  ScreenOpened, CommandExecuted{command_id, args, duration_ms, outcome},
  FieldChanged, ExceptionThrown{exception_type, message, stack},
  NavigationOccurred{from, to}, ValidationFailed,
  AsyncOperationCompleted{operation_id, duration_ms, outcome}.
- Wire format: JSONL of `TraceEnvelope{schema_version, ...}` via
  `write_event`/`read_event` only; `read_event` fails closed on
  version-confused lines (lib.rs:128-157).

**Gap relevant to E2:** no span-id/parent-span-id/trace-id/service fields;
no log or metric event kinds. Options: (a) v3 schema + `V2ToV3` upcaster
per docs/upcasters.md; (b) encode into `domain_entity_id`/`args` +
`correlation_id` (UUID-shaped, cannot carry 64-bit hex span ids natively).
SPEC hard rule 15 / AGENTS anti-pattern 7 forbid a parallel wire format:
imported telemetry must land as `Current` JSONL.

## Normalization (crates/trace-normalizer)

`normalize(&Session<Current>, &NormCfg) -> (Scenario, FoldReport)`:
value abstraction (numeric bucketing 0/1/2-10/11-100/101-1000/1001+,
per-field overrides) then folding of CommandExecuted into
`CanonicalAction{screen_id, command_id, abstract_args}`; burst collapse
50 ms; idle gap 5000 ms. `FoldReport` counts input/output/collapsed/
dropped/pauses — natural ingestion-loss counters. Idempotent refold.

## Graph (crates/trace-graph)

`ActionGraph` = petgraph DiGraph of `ActionNode{CanonicalAction,
visit_count}` with `Transition{frequency, failure_count, anomaly_score}`;
anomaly from Heuristics dependency `dep(a,b)=(f(ab)-f(ba))/(f(ab)+f(ba)+1)`,
`anomaly=(1-dep)/2`. Plus HeuristicsMiner, prefixspan, InductiveMiner
(feature), DOT/Mermaid export, WorkflowReport. This is a workflow graph
over canonical actions, not a span-causality graph.

## Oracle (crates/trace-oracle)

Object-safe `Rule` (`evaluate(&Session<Current>) -> OracleResult`),
schedules PerEvent/PerScenario/EndOfSession/WindowBased, `OracleViolation`
carries `evidence: Vec<EventSeq>` — the provenance hook for H4. Engine +
And/Or/WithinWindow composition; five built-in rules.

## Ingestion paths today

JSONL only (ADR-0003); CLI `trace ingest --from jsonl --to sqlite` is the
sole ingest verb; no CSV/OTLP/JSON-array reader exists. `trace-align` (S8)
gives Needleman-Wunsch session alignment with pluggable costs.

## Constraints on an E2 importer

- Home: `experiment/nezha/adapters/` (not a workspace member; Python or a
  standalone Rust binary both acceptable; if it becomes a workspace crate,
  full lint/docs gates apply: missing_docs, pedantic+nursery, -D warnings).
- unsafe forbidden; toolchain 1.85, edition 2021; cargo-deny allowlist,
  wildcard deps denied; trace-core deps frozen to serde/serde_json/chrono/
  uuid (SPEC hard rule 9); petgraph 0.8.x pinned.
- Hexagonal boundary (SPEC hard rule 3): implement `EventSource`/
  `StorageBackend` ports from crates/trace-core/src/ports.rs:29-68 rather
  than reaching through.
- Determinism (hard rule 8) and privacy defaults Masked/Bucketed (hard
  rule 5) apply to anything written as trace data.
- No version matching outside trace-schema upcasters (anti-pattern 2).
