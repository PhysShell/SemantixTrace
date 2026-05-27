# S2: JSONL storage backend and CLI skeleton

Status: planned
Depends on: S1
ADRs: ADR-0002, ADR-0003, ADR-0014

## Goal

Land the first storage backend (`JsonlBackend`) and the `trace` CLI
skeleton with `analyze` and `version` subcommands, so an external
recorder can append events and an operator can inspect them.

## Inputs / Outputs

- In: S1 schema v1 and the `StorageBackend` port from `trace-core`.
- Out:
  - `trace-storage`: `JsonlBackend` implementing `StorageBackend`,
    append-only writes (buffered), iter-over-file reads going through
    `trace_schema::read_event`, optional zstd decompression on read.
  - `trace-cli`: clap 4.x derive, subcommands `version` (prints schema
    + binary version) and `analyze <file>` (summary stats: session
    count, event count per kind, error rate). Both subcommands honour
    the global flags from ADR-0014 §4 — `-o {text,json,wide}`,
    `--no-color`, `-q/-v`, `--manifest-path`.
  - `trycmd` snapshot tests for `analyze` text + json outputs and
    for the help text (ADR-0014 §12). Blessing via `TRACE_BLESS=1`.
  - Versioned JSON schemas for `trace version --output json` and
    `trace analyze --output json` published alongside the wire
    schema under `crates/trace-cli/schema/` (ADR-0014 §11). Identity
    upcaster chains for now; they bump exactly like the event
    schema (ADR-0006).

## Approach

- TDD for parser-adjacent code. Property test: an `iter` over a file
  written by `append` yields the same events in order.
- The `analyze` command writes to stdout in a stable, snapshot-friendly
  format (no random ids, no map iteration order).
- File layout: one event per line, UTF-8, LF, no BOM, no trailing
  whitespace.
- `JsonlBackend` opens files in append mode and `fsync`s on a
  configurable flush policy (default: every 64 events or 250 ms,
  whichever fires first).
- Read-path: detect `*.zst` extension; decompress via the `zstd` crate.

## Acceptance criteria

- Property test: `iter(append(events)) == events` for all event-kind
  enumerations.
- `trace version` prints `{ binary: "<semver>", schema: 1 }`,
  exit code `0`; `--output json` validates against the published
  `trace-version-v1.schema.json` (ADR-0014 §11).
- `trace analyze fixtures/multi_session.jsonl` produces the blessed
  golden output; exit codes per ADR-0014 §6 — `66 EX_NOINPUT` on
  missing file, `65 EX_DATAERR` on parser failure, `0` on success.
- `trace analyze` writes data to stdout, diagnostics to stderr
  (clig.dev / ADR-0014 §5); a `--quiet` flag suppresses
  diagnostics; piping `… | jq` works without contamination.
- CI fuzz smoke runs `jsonl_parse_v1` (60s bounded) and the regression
  corpus from S1.
- The `JsonlBackend` does not depend on anything beyond `trace-core`,
  `trace-schema`, `serde_json`, `zstd`.

## Open questions

- Default flush policy: 64 events / 250 ms is provisional. Will revisit
  during S6 once the WPF adapter feeds it.
- Whether `analyze` should learn `--json` mode now or only at S7. Will
  add now if it costs less than half a day.

## See also

- [`../adr/0003-jsonl-as-mvp-wire-format.md`](../adr/0003-jsonl-as-mvp-wire-format.md)
- [`../glossary.md`](../glossary.md) §7
