# S9: Parquet archive tier (part of v0.3)

Status: planned
Depends on: S8
ADRs: ADR-0003

## Goal

Add the `ParquetBackend` for long-term archival and external analytics
(DuckDB, Polars, Pandas). Pin the `arrow` / `parquet` versions and ship
a partitioned-by-day default layout.

## Inputs / Outputs

- In: JSONL or SQLite corpora.
- Out:
  - `trace-storage` gains `ParquetBackend` behind feature `parquet`,
    backed by `arrow` + `parquet` (pinned in `Cargo.toml`).
  - `trace ingest --to parquet --partition-by day` CLI subcommand.
  - Read path identical in semantics to JSONL / SQLite (upcaster chain
    applies on materialisation).
  - Compression: zstd (default) or snappy (configurable).

## Approach

- The Parquet schema is generated from the v_current event schema via
  a small build-time helper; on a schema bump the build fails until the
  Parquet schema-derivation code is updated, forcing the maintainer to
  acknowledge the change.
- Partition columns: `dt=YYYY-MM-DD` (UTC date of session start).
- The reader supports both row-group projection and predicate pushdown
  on `(session_id, schema_version, kind)`.
- DuckDB compatibility: emit the files in a layout DuckDB can read with
  `read_parquet('archive/dt=*/*.parquet')` without modification.
- Fuzz target `parquet_round_trip`: arbitrary event sequences write to
  a Parquet file and read back equal.

## Acceptance criteria

- `trace ingest --to parquet` produces files readable by DuckDB without
  errors.
- Round-trip property test green: JSONL → Parquet → iter() → events
  match input.
- Bench: ingesting 1M events to a zstd-compressed Parquet archive runs
  in < 90 s on the reference machine; resulting size is < 35% of the
  source JSONL.
- `cargo test --workspace --features parquet` is green; CI runs both
  with and without the feature.

## Open questions

- Whether to expose row-group sizing in the CLI. Working answer: yes
  via `--row-group-size`, default 64k.
- Whether to support reading partitioned data via the `iter` interface
  or only through a separate query API. Working answer: `iter` works
  but emits a warning at > 10 M events; predicate pushdown is the
  blessed path past that volume.

## See also

- [`../adr/0003-jsonl-as-mvp-wire-format.md`](../adr/0003-jsonl-as-mvp-wire-format.md)
- [`../glossary.md`](../glossary.md) §7
