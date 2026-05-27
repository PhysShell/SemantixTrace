# ADR 0011: Trace is the single source of truth; projections fan out from it

Date: 2026-05-27
Status: Accepted

## Context

Behavioral data about a desktop app sits at the intersection of seven
real consumer needs:

- **Analytics** — which scenarios run in production, how often?
- **Diagnostic** — what did the user do before this crash?
- **Regression testing** — can we replay this scenario and assert?
- **UX research** — where do users back out, retry, get stuck?
- **Product** — which features are dead? Which got worse after the
  redesign?
- **Support replay** — reproduce a customer's reported issue without
  a phone call.
- **Exploration** — what neighbouring scenarios might also be broken?

A common industry path treats each of these as a separate product with
its own SDK, its own wire format, and its own backend: Mixpanel for
product analytics, Datadog Session Replay for support, Selenium /
Playwright fixtures for tests, custom OTLP spans for diagnostics, and a
"workflow miner" notebook for one-off questions. Five SDKs, five
schemas, five sources of truth — none of which agree.

SemantxTrace records semantic actions (ADR-0005). The same event that
makes a regression test work (`Graph47.Recalculate` with outcome and
context) is the event that makes the analytics projection useful (the
same `Graph47.Recalculate`, counted per scenario shape) and is the
event support needs (a strict-replayable trace of the user's last 30
actions). Splitting them into different formats would (a) duplicate
the upcaster chain (ADR-0006), (b) duplicate the privacy policy
(ADR-0007), (c) duplicate fuzzing (ADR-0010), (d) guarantee schema
drift across the artefacts, and (e) drop the project's central
positioning of "semantic metrics, not contextless counters".

## Decision

**One trace. Seven projections. No second wire format.**

1. The canonical artefact is the semantic trace defined by
   `trace-schema::Current` (ADR-0006). Every consumer projection reads
   the same on-disk JSONL / SQLite / Parquet (ADR-0003) through the
   same upcaster chain.
2. The seven projections fixed at v1.0 are: `analytics`, `diagnostic`,
   `regression-test`, `ux`, `product`, `support-replay`, `exploration`.
   Each is a *read-side* of the same data; none has its own wire
   format, its own schema, or its own sink.
3. New projections are added by introducing a new reader / reporter
   in a downstream crate, not by changing the schema. If a projection
   would require new event shapes, those shapes go through the
   regular upcaster-bump procedure ([`../upcasters.md`](../upcasters.md))
   and become available to *every* projection, not just the requesting
   one.
4. The replay path explicitly exposes two modes (`strict`, `relaxed`;
   glossary §6, S11) precisely because regression-test and
   support-replay want byte-step fidelity while analytics and
   exploration want equivalence-class fidelity. Both modes operate on
   the same plan schema; the difference is in the *reduction*, not in
   the data.
5. "Semantic metrics, not contextless counters" is normative: a
   recording shape that erases scenario context (`btnExport.clicked:
   1200` with no surrounding events) is **not** an admissible event.
   Aggregate counts are computed by the analytics projection at read
   time.
6. The naming follows the convention `trace-<projection>` for the
   crates / reporters that materialise a projection's outputs (e.g.
   `trace-graph` is the analytics-projection backbone; `trace-cli`
   `report workflows` is its surface).

## Consequences

- **Upside.** One schema, one upcaster chain, one privacy policy, one
  fuzz target catalogue cover every consumer. Adding a new consumer
  (say, a Grafana dashboard or a desktop notification "this scenario
  spiked overnight") is a new reader, not a new ingestion pipeline.
  The "semantic metrics, not contextless counters" positioning becomes
  a property of the data model, not a slogan.
- **Upside.** Cross-projection invariants become enforceable. For
  example: an oracle failure surfaced in the regression-test
  projection automatically appears in the diagnostic projection's
  evidence view, because both read from the same canonical events.
- **Trade-off.** The schema must carry enough context to make each
  projection useful. This is intentional and reflects the
  "scenario-aware event" design (ADR-0005, ADR-0007); it does cost
  bytes per event compared to a counter-only model.
- **Trade-off.** Performance for analytics-heavy use cases requires
  the optional SQLite (v0.2, S8) and Parquet (v0.3, S9) read paths.
  These are derived from the JSONL canonical, not replacements
  (ADR-0003).
- **Risk.** Pressure to ship "a lighter analytics-only format" will
  arrive. It must be refused; the architecture explicitly forbids a
  parallel wire format. Any optimisation lives inside the read path
  for the existing format, never as a new format.
- **Risk.** Confusion about who SemantxTrace is *for*. Mitigation: the
  projection list in glossary §0 and the SPEC's "What SemantxTrace is"
  table call out the consumer per projection. Tech leads, UX
  researchers, product managers, and support engineers are all in
  scope — but they read different projections of the same data, and
  the developer-facing surface (CLI + Rust crates + .NET adapters)
  is the only delivery vehicle at v1.0.

## See also

- ADR-0005 (semantic action map ≠ physical UI map).
- ADR-0006 (upcaster pattern); ADR-0003 (JSONL canonicality);
  ADR-0007 (privacy by default); ADR-0010 (fuzz coverage).
- [`../glossary.md`](../glossary.md) §0 (projections), §3 (sessions /
  scenarios), §6 (replay modes), §19 (terms to avoid).
- [`../stages/S4-action-graph-and-heuristics-miner.md`](../stages/S4-action-graph-and-heuristics-miner.md)
  (first analytics-projection outputs).
- [`../stages/S7-demo-app-and-mvp-pipeline.md`](../stages/S7-demo-app-and-mvp-pipeline.md)
  (four projections materialised end-to-end at v1.0-MVP).
- [`../stages/S11-replay-planner-semantic-monkey-and-trace-mutation.md`](../stages/S11-replay-planner-semantic-monkey-and-trace-mutation.md)
  (regression-test / support-replay / exploration projections).
