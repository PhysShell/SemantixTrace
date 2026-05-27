# ADR 0003: JSONL is the MVP wire format; SQLite / Parquet are additional read paths

Date: 2026-05-27
Status: Accepted

## Context

Trace events need a wire format that is human-readable for debugging,
grep / jq friendly, append-only friendly (a recording app crashes mid
session), schema-portable (the `trace-wpf` adapter writes the same JSON
the Rust core reads), and cheap to evolve via upcasters (ADR-0006).
Candidates considered: JSON Lines (JSONL), Protobuf, MessagePack, Cap'n
Proto.

| Format | Pros | Cons |
|---|---|---|
| JSONL | Human-readable, grep-able, jq-able, schema portable across .NET / Rust | Large size; slower parse |
| Protobuf | Small, fast, evolution-friendly | Opaque on disk; .proto toolchain |
| MessagePack | Small, fast, JSON-shaped | Opaque on disk |
| Cap'n Proto | Zero-copy, very fast | Complex; small .NET ecosystem |

For analysis at scale, SQLite is the obvious next step (rows, indexes,
`WHERE schema_version = …`). For long-term archival and external
analytics (DuckDB, Polars), Parquet is the obvious next step
(columnar, compressible).

## Decision

The MVP wire format is **JSONL**, one envelope per line, UTF-8, LF line
endings. The JSON Schema is versioned, lives in `trace-schema/schema/`,
and is published alongside crates.io releases for external consumers.
Compression is external (`*.jsonl.zst`); the parser handles both raw and
zstd-decompressed streams.

SQLite (v0.2) and Parquet (v0.3) ship as **additional read paths**
behind feature flags (`sqlite`, `parquet`). They are not replacements
for JSONL and never the source of truth: JSONL is the canonical recorded
artefact, and the other backends are derived (via `trace ingest …`).

All backends read through the same upcaster chain (ADR-0006); domain
code only ever sees `Current`.

## Consequences

- Operators can `grep` / `jq` / `tail -f` raw recordings on any machine.
- Wire size is large; mitigated by zstd. Acceptable for the desktop-
  trace volumes the v1.0 target uses.
- Adding a backend means adding an importer that converts JSONL → that
  backend's layout, plus optional backend-specific query extensions.
- The schema is one source of truth for both .NET and Rust; the .NET
  side validates against the published JSON Schema in CI.
