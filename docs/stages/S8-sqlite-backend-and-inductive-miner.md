# S8: SQLite backend and Inductive miner (v0.2)

Status: planned
Depends on: S7
ADRs: ADR-0003

## Goal

Close v0.2. Add the `SqliteBackend` (feature `sqlite`) for analysis-time
queries, implement the Inductive miner (IMDF) alongside the existing
Heuristics miner, and revisit scenario folding based on what real
corpora show.

## Inputs / Outputs

- In: v0.1 corpora from real users + the demo recording.
- Out:
  - `trace-storage` gains `SqliteBackend` behind feature `sqlite`,
    backed by `rusqlite` 0.31+ (`bundled` feature).
  - Schema: a single `events` table keyed by `(session_id, seq)`, with
    `schema_version`, `ts`, `kind`, `payload_json` columns;
    `schema_version` indexed.
  - `trace ingest --from jsonl --to sqlite` CLI subcommand.
  - `trace-graph` gains the Inductive miner (IMDF variant); CLI flag
    `--miner {heuristics,inductive}` defaults to inductive when the
    feature `inductive-miner` is enabled.
  - `trace-normalizer` improvements (config presets per app domain,
    informed by S7 feedback).

## Approach

- `SqliteBackend` reads pass through `read_event` so the upcaster chain
  still applies. Filtering by `schema_version` is a cheap WHERE clause
  on the indexed column.
- The Inductive miner implementation follows the IMDF paper (Leemans et
  al.). The output is a sound process tree; the CLI renders it back to
  the same `ActionGraph` for compatibility, with an optional
  `--format process-tree` to emit the raw tree.
- Property tests: ingesting a JSONL file and reading back from SQLite
  yields the same `TraceEvent` sequence.
- Fuzz target `sqlite_query` (structure-aware, generates arbitrary
  filter combinations): never panics, never produces an unbounded
  result set.

## Acceptance criteria

- `trace ingest` is round-trip-safe: JSONL → SQLite → iter() → JSONL
  produces a byte-identical file modulo whitespace.
- The Inductive miner reproduces a published reference output on a
  fixture corpus.
- Bench: ingesting 1M events from JSONL into SQLite takes < 60 s on
  the reference machine; querying the most-frequent path runs in < 2 s
  on the resulting database.
- `cargo test --workspace --features sqlite,inductive-miner` is green;
  CI runs both with and without the features.

## Open questions

- Whether to use a single `events` table or per-session shards. Working
  answer: single table with `(session_id, seq)` primary key; revisit if
  multi-tenant deployments arrive.
- Whether SQLite WAL mode is the default. Working answer: yes for
  `trace ingest`; readers do not enable WAL.

## See also

- [`../adr/0003-jsonl-as-mvp-wire-format.md`](../adr/0003-jsonl-as-mvp-wire-format.md)
- [`../glossary.md`](../glossary.md) §4, §7
