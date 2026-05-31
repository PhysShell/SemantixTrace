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
    `schema_version` indexed. Four additional extracted columns —
    `command_id TEXT`, `screen_id TEXT`, `outcome TEXT`,
    `domain_entity_id TEXT` — are populated at ingest time from
    well-known **top-level** payload fields and indexed separately.
    `command_id` is present in `CommandExecuted`; `screen_id` in
    `ScreenOpened`; `outcome` in `CommandExecuted` and
    `AsyncOperationCompleted` (flattened, top-level in JSON);
    `domain_entity_id` is a new top-level field added in v2 (see
    `trace-schema/src/v2.rs` and `trace-event-v2.schema.json`).
    Unknown event kinds and v1 events (upcasted with `domain_entity_id =
    NULL`) leave the column `NULL`. The columns are query accelerators;
    `payload_json` remains the authoritative record.
  - `trace ingest --from jsonl --to sqlite` CLI subcommand.
  - `trace slice --by {session-id,command-id,screen-id,outcome,domain-entity-id}
    <value> <db>` CLI subcommand: reads the SQLite corpus through the
    standard upcaster chain and writes a JSONL slice to stdout.
  - `trace report similar --scenario <session-id>/<scenario-index>
    [--top N] <db>` CLI subcommand: finds the N scenarios in the corpus
    most similar to the given one by counting shared semantic dimensions
    (`command_id`, `screen_id`, `outcome`); emits a
    ranked JSONL list with a `similarity_score` field (count of matching
    dimensions, normalised 0–1). Uses the extracted index columns; no
    full-scan over `payload_json`. Default N=10.
  - `trace-graph` gains the Inductive miner (IMDF variant); CLI flag
    `--miner {heuristics,inductive}` defaults to inductive when the
    feature `inductive-miner` is enabled.
  - `trace-normalizer` improvements (config presets per app domain,
    informed by S7 feedback).

## Approach

- `SqliteBackend` reads pass through `read_event` so the upcaster chain
  still applies. Filtering by `schema_version` is a cheap WHERE clause
  on the indexed column.
- The four extracted index columns (`command_id`, `screen_id`,
  `outcome`, `domain_entity_id`) are written by a thin extractor in the
  `trace ingest` pipeline. The extractor reads well-known top-level
  payload fields by name; it does not parse `args`/`params`. Columns are
  `NULL` for event kinds that carry no such field; v1 events upcasted to
  v2 have `domain_entity_id = NULL`. They are never read back by the
  upcaster chain — their only role is SQL filtering.
- `trace report similar` computes similarity as a Jaccard-like score
  over the set of (command_id, screen_id, outcome, domain_entity_id)
  tuples present in each scenario. The query is a two-step SQL: (1) a
  candidate fetch using the indexed columns to pre-filter sessions that
  share at least one dimension with the probe scenario; (2) an
  in-process scoring pass over the candidates. This avoids a full table
  scan while keeping the scorer outside SQL for correctness and
  testability. The scoring function is pure and property-tested.
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
- `trace slice --by command-id RecalculateGraph47Command ./corpus.sqlite`
  produces a non-empty JSONL stream; every event is upcasted to
  `Current`; the output is byte-identical to filtering the same corpus
  via `iter()` + a manual command-id predicate (round-trip safety
  extends to the slice path).
- The Inductive miner reproduces a published reference output on a
  fixture corpus.
- Bench: ingesting 1M events from JSONL into SQLite takes < 60 s on
  the reference machine; querying the most-frequent path runs in < 2 s
  on the resulting database.
- `trace report similar --scenario <id> --top 5 ./corpus.sqlite` returns
  exactly 5 results (or fewer if the corpus is small), each with a
  `similarity_score` in [0, 1]; the probe scenario itself is excluded
  from results; ranking is deterministic for a fixed corpus.
- Property: `similar(a, b) == similar(b, a)` (symmetry) across 1 000
  generated corpus pairs.
- `cargo test --workspace --features sqlite,inductive-miner` is green;
  CI runs both with and without the features.

## Open questions

- Whether to use a single `events` table or per-session shards. Working
  answer: single table with `(session_id, seq)` primary key; revisit if
  multi-tenant deployments arrive.
- Whether SQLite WAL mode is the default. Working answer: yes for
  `trace ingest`; readers do not enable WAL.
- Whether oracle candidate mining (`trace oracle mine --min-support 0.95`,
  deriving frequency-based oracle rule candidates from the corpus)
  belongs in S8 or a later stage. Working answer: defer past S8; the
  SQLite corpus is a necessary prerequisite, but S8 is already scoped
  for ingest + Inductive miner. Add `trace oracle mine` in the stage
  after S8 feedback from real corpora is available.

## See also

- [`../adr/0003-jsonl-as-mvp-wire-format.md`](../adr/0003-jsonl-as-mvp-wire-format.md)
- [`../glossary.md`](../glossary.md) §4, §7
