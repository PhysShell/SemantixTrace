# ADR 0004: Forbid unsafe_code at the workspace level

Date: 2026-05-27
Status: Accepted

## Context

SemantxTrace processes externally produced JSONL and exposes a stable
schema across language boundaries. Any memory-unsafety bug in the
parser, the upcaster chain, or a backend implementation becomes a wire-
exploitable issue. `unsafe` is also unnecessary for any planned MVP
functionality: serde-based parsing, petgraph traversal, and `rusqlite` /
`parquet` are safe APIs.

## Decision

We set `unsafe_code = "forbid"` workspace-wide via
`[workspace.lints.rust]`. No exception without a superseding ADR. The
isolated `fuzz/` crate is the only exception (libFuzzer harness emits
`extern "C"` glue); the isolation is documented in ADR-0010.

## Consequences

- Whole categories of bugs are impossible at the language level.
- Domain crates can never use `transmute`, `from_raw_parts`, or
  hand-rolled SIMD. If such functionality becomes desirable, it must
  land via a vetted dependency (e.g. `bytemuck` for safe transmutes)
  rather than a local `unsafe` block.
- FFI to .NET (future, post-v1.0) requires either a safe wrapper crate
  in a separate workspace member with its own ADR opening this rule, or
  IPC-based interop (Named Pipes / gRPC).
