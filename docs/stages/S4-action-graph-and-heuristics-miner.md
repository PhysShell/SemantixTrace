# S4: Action graph and Heuristics miner

Status: planned
Depends on: S3
ADRs: ADR-0008

## Goal

Build the `ActionGraph` from normalized scenarios, implement a Heuristics
miner, ship Mermaid + DOT exporters, and produce the first **analytics
projection** outputs (ADR-0011): top-N workflows by frequency, rare-but-
failing scenarios, dead-feature candidates. Add anomaly detection via
frequency thresholds.

## Inputs / Outputs

- In: a corpus of normalized `Scenario`s.
- Out:
  - `trace-graph` crate: `ActionGraph = DiGraph<ActionNode, Transition>`,
    builder, Heuristics miner, `most_frequent_path`,
    `anomaly_transitions`, Mermaid / DOT exporters.
  - PrefixSpan implementation for motif discovery.
  - `trace graph <file> --format {mermaid,dot}` CLI subcommand.
  - `trace report workflows <file>` CLI subcommand emitting:
    - **Top-N workflows** by frequency (default N=20),
    - **Rare-but-failing workflows** (frequency ≤ 1% AND error_rate ≥
      5% on any edge, configurable),
    - **Dead-feature candidates** (graph nodes referenced by the
      adapter's published `CommandId` catalogue but absent from the
      corpus, or present below a configurable floor).
  - The same reports re-emitted as JSON for downstream tooling
    (`--format json`).

## Approach

- petgraph 0.8.x pinned (ADR-0008).
- Heuristics miner port from PM4Py / ProM literature, ~600–1000 lines.
  Test the miner against a hand-built fixture corpus with known frequent
  paths and known anomalies.
- Property tests:
  - `ActionGraph::from(scenarios)` is deterministic.
  - For a single scenario, the graph contains exactly its canonical
    action transitions, each with `frequency = 1`.
  - `is_cyclic_directed` on the normalized graph equals the
    `NormCfg::allow_cycles` policy.
- PrefixSpan returns frequent subsequences with support ≥ threshold;
  property: every returned sequence is in the input as a contiguous
  subsequence (or all-contiguous variants per algorithm semantics —
  decided inline).
- Mermaid exporter wraps nodes in `[]`, edges with `-->`, includes
  `freq=N` annotation on edges above a configurable percentile.
- DOT exporter delegates rendering to system `dot`.

## Acceptance criteria

- Heuristics miner reproduces a published fixture's expected paths
  byte-identically.
- `trace report workflows fixtures/multi_session.jsonl --format json`
  produces a blessed snapshot containing top-N, rare-but-failing, and
  dead-feature sections — i.e. the first concrete analytics-projection
  output (ADR-0011).
- Mermaid output for a 5-scenario fixture matches a blessed snapshot.
- DOT output renders to SVG without warnings via `dot -Tsvg`.
- `graph_build` fuzz target (structure-aware: arbitrary scenarios) green
  for the 60s bounded run.
- PrefixSpan property tests green across 10 000 generated sequences.

## Open questions

- Whether to expose the Heuristics miner thresholds via CLI flags or
  config file. CLI flags for S4; a config file lands when v0.2 ships.
- Whether anomaly detection writes a sidecar report or annotates the
  graph. Decision: annotates the graph (`Transition.anomaly_score`)
  plus a sidecar JSON summary.

## See also

- [`../adr/0008-pin-petgraph-0-8-x.md`](../adr/0008-pin-petgraph-0-8-x.md)
- [`../glossary.md`](../glossary.md) §4
