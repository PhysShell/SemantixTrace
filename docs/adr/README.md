# Architecture Decision Records

Nygard format, in-repo, append-only (ADR-0009). After `Accepted`, an ADR is
immutable; supersede it with a new one. New ADRs: copy
[`0000-template.md`](0000-template.md), use the next id, link it here.

| ADR | Title | Status |
|---|---|---|
| [0001](0001-use-rust-workspace.md) | Use a Rust workspace with trace-core / trace-schema / … / trace-cli crates | Accepted |
| [0002](0002-hexagonal-architecture-with-rust-traits.md) | Adopt hexagonal architecture with Rust traits as ports | Accepted |
| [0003](0003-jsonl-as-mvp-wire-format.md) | JSONL is the MVP wire format; SQLite / Parquet are additional read paths | Accepted |
| [0004](0004-forbid-unsafe-code.md) | Forbid unsafe_code at the workspace level | Accepted |
| [0005](0005-semantic-action-map-not-physical-ui-map.md) | The trace lives at the semantic action map, not the physical UI map | Accepted |
| [0006](0006-upcaster-pattern-for-schema-evolution.md) | Upcaster pattern for schema evolution (no in-place rewrites) | Accepted |
| [0007](0007-privacy-by-default-mask-and-bucket.md) | Privacy by default: mask strings, bucket numerics, raw export is opt-in | Accepted |
| [0008](0008-pin-petgraph-0-8-x.md) | Pin petgraph to 0.8.x until 0.9 stabilises | Accepted |
| [0009](0009-use-nygard-adr-format.md) | Use the Nygard ADR format, stored in-repo, append-only | Accepted |
| [0010](0010-fuzz-storage-parsers-and-upcaster-chain.md) | Fuzz-test storage parsers, the upcaster chain, and selected normalizer transforms | Accepted |
| [0011](0011-trace-as-multi-projection-source-of-truth.md) | Trace is the single source of truth; projections fan out from it | Accepted |

See also: [`../SPEC.md`](../SPEC.md), [`../glossary.md`](../glossary.md),
[`../decisions.log.md`](../decisions.log.md).
