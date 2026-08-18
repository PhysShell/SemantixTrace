# ADR 0015: Make wire-affecting leaf enums version-local before v1.0

Date: 2026-08-18
Status: Accepted
Tracking: PhysShell/SemantixTrace#15

## Context

`v1::TraceEventKind` is declared frozen forever (ADR-0006), yet two of
its leaf types — `trace_core::Outcome` and `trace_core::ValuePolicy` —
are shared, `#[non_exhaustive]` enums whose own documentation invites
new variants "via the upcaster chain". That invitation is a category
error: these enums are `#[serde(flatten)]`-embedded / directly
serialized inside the frozen v1 wire shape, so adding
`Outcome::PartialSuccess` to `trace-core` silently changes the set of
wire values the *v1* module can read and write. No upcaster
participates; the module is formally frozen while its wire language
depends on a live external enum. The S1–S3 hardening pass surfaced
this as more than stylistic tension — it is a hole in the versioning
model itself: the freeze guarantee is only as strong as the weakest
shared leaf type.

## Decision

Wire-affecting leaf enums become **version-local** before v1.0: each
schema version module owns its `Outcome` and `ValuePolicy` shapes
(`v1::Outcome`, `v1::ValuePolicy`, then v2 analogues or an explicit
mapping from frozen primitives), and the shared `trace-core` enums
either become the *domain-side* types that per-version wire enums map
into, or are retired from the wire entirely. We explicitly reject the
cheap fix of removing `#[non_exhaustive]` from the shared enums: these
concepts are genuinely expected to evolve, and pinning them shared
would only convert silent wire drift into a workspace-wide breaking
change at the first evolution.

This is deliberate pre-v1.0 surgery with its own slice — it is **not**
bundled into the S1–S3 hardening branch that identified it. Until the
migration lands, no new variant may be added to `trace_core::Outcome`
or `trace_core::ValuePolicy` — any such change is a review-blocker
citing this ADR.

## Consequences

- The v1 freeze becomes real: a frozen module's wire language depends
  on nothing that can move.
- S12 (v1.0 stable) gains a hard acceptance gate: this ADR must be
  implemented before release; shipping v1.0 with shared mutable wire
  enums would freeze the hole permanently.
- Cost: per-version enum duplication and mapping boilerplate at each
  bump — the same cost every other frozen wire shape already pays, now
  paid honestly instead of hidden.
- Interim risk: until the migration, the review-blocker rule above is
  the only guard; it relies on review discipline, which is exactly why
  the migration must precede v1.0 rather than trail it.
