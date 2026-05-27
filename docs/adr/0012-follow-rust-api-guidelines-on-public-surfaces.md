# ADR 0012: Follow the Rust API Guidelines on public surfaces

Date: 2026-05-27
Status: Accepted

## Context

SemantxTrace publishes a set of Rust crates that other code will compile
against: `trace-core` (port traits, value objects), `trace-schema`
(`Current`, `Upcaster`, `read_event`), `trace-storage`
(`StorageBackend`), `trace-oracle` (`OracleRule`),
`trace-replay-planner` (`ReplayPlan`, mutations). After S12 these
crates carry semver guarantees forever. Any inconsistency in naming,
trait derivations, error shape, sealed-vs-open traits, or
`#[non_exhaustive]` decisions becomes a downstream-breaking change the
moment we try to fix it.

The Rust project maintains a checklist precisely for this:
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
(`C-*` recommendations under Naming, Interoperability, Macros,
Documentation, Predictability, Flexibility, Type safety, Dependability,
Debuggability, Future-proofing, Necessities). Adopting it is cheaper
than inventing a project-local style guide and gives downstream
consumers the conventions they already expect from idiomatic Rust
crates.

Wire-format compatibility across the Rust↔.NET boundary is **not**
covered by API Guidelines — that boundary is the JSON Schema published
under `trace-schema/schema/` and the upcaster chain (ADR-0006). API
Guidelines apply only to Rust↔Rust public surfaces.

A future FFI surface (`netcorehost`, `LibraryImport`) does not exist
yet. When it lands, it gets its own ADR layering additional rules on
top of API Guidelines; this ADR does not anticipate that.

## Decision

1. **Scope.** Every `pub` item in a crate published to crates.io
   follows the Rust API Guidelines. Crates in scope at v1.0:
   `trace-core`, `trace-schema`, `trace-storage`, `trace-normalizer`,
   `trace-graph`, `trace-oracle`, `trace-replay-planner`, `trace-cli`
   (binary-only surfaces still observe naming + doc rules).
   `trace-viewer` (deferred), `examples/*`, and `fuzz/` are
   **out of scope**.
2. **`pub(crate)` and private items.** Not strictly bound by C-* rules,
   but the project prefers consistency; reviewers may request changes
   on style grounds without escalating to "API Guidelines violation".
3. **Specific C-* commitments.** The following are non-negotiable and
   verified during the pre-v1.0 audit (S12):
   - `C-CASE`, `C-CONV`, `C-GETTER`, `C-ITER`, `C-ITER-TY`,
     `C-FEATURE`, `C-WORD-ORDER` — naming.
   - `C-COMMON-TRAITS` — every public type derives `Debug`, `Clone`,
     `PartialEq`, `Eq`, `Hash` where the inner data permits;
     `Default` where a meaningful default exists; `Display` for
     newtypes that are stringly-rendered.
   - `C-SEND-SYNC` — public types are `Send + Sync` unless the docs
     say otherwise and explain why.
   - `C-GOOD-ERR` — error types implement `std::error::Error`, are
     `Send + Sync + 'static`, and carry source via `#[from]` /
     `#[source]` (via `thiserror`).
   - `C-SERDE` — serde derives live behind a `serde` feature on
     crates whose serde usage is optional; `trace-schema` is
     unconditional (serde is its reason to exist).
   - `C-DEBUG`, `C-DEBUG-NONEMPTY` — every public type implements
     `Debug` and produces non-empty output.
   - `C-NEWTYPE` — domain identifiers are newtypes
     (`SessionId(uuid::Uuid)`, `CommandId(String)`, …), already
     baseline for the project.
   - `C-VALIDATE` — wire-boundary functions (`read_event`,
     `JsonlBackend::iter`, `OracleRule::evaluate`,
     `plan_from(...)`) validate inputs and return typed errors.
     No `unwrap()` on user-controlled data anywhere outside
     `fuzz/`.
   - `C-NO-PANIC` (well, our crate-local variant): public library
     functions either do not panic or document the panic conditions
     explicitly with a `# Panics` doc section.
   - `C-SEALED` — closed-set traits are sealed so downstream cannot
     break our invariants. **Mandatory sealing** for: the
     `Upcaster` trait (downstream `impl` would break the
     version-dispatch); the per-version `EventKindRepr` markers in
     `trace-schema` if introduced; the `BackendCapability` marker
     traits if introduced for SQLite-specific or Parquet-specific
     query extensions.
   - `C-NON-EXHAUSTIVE` — applied to public enums that may grow
     post-v1.0 without a major bump: `Outcome`, `OracleSchedule`,
     `Severity`, `ReplayMode` (if a third mode lands),
     `MutationKind`, `ValuePolicy` extensions added after v1.0.
     **Not** applied to per-version event enums (`v1::TraceEventKind`,
     `v2::TraceEventKind`, …) — they are frozen forever by ADR-0006,
     and `#[non_exhaustive]` would impose downstream-match cost for
     no benefit since new variants land in a new version module.
   - `C-DOC`, `C-EXAMPLE`, `C-FAILURE`, `C-LINK` — every public item
     has a doc comment; every public function with non-trivial
     behaviour has an `# Examples` section that compiles; `# Errors`
     and `# Panics` sections where applicable; intra-doc links to
     related items.
   - `C-METADATA`, `C-RELNOTES`, `C-HTML-ROOT`, `C-CI` — Cargo
     manifest metadata complete; CHANGELOG.md present and updated;
     `#![doc(html_root_url = "https://docs.rs/<crate>/<version>")]`
     where needed; CI runs the public-API checks.
4. **Tooling.** The following gates run in CI and block merges:
   - `cargo clippy --all-targets --all-features -- -D warnings -W
     clippy::pedantic`.
   - `cargo doc --all-features --no-deps -D warnings` (catches
     broken intra-doc links and `missing_docs` violations).
   - `#![warn(missing_docs, missing_debug_implementations,
     missing_copy_implementations, rust_2018_idioms,
     unreachable_pub, single_use_lifetimes,
     unused_qualifications)]` at the crate root of every in-scope
     crate.
   - `cargo deny check` for licenses + supply-chain.
   - From S12: `cargo public-api --diff-git-checkouts <prev-tag>
     HEAD` runs nightly and on every PR; surface diffs land as
     required release-notes entries.
5. **Pre-v1.0 audit (S12).** A line-by-line walk of the API
   Guidelines checklist against every in-scope crate. Findings file
   issues; release does not happen until every finding is resolved
   or explicitly waived in `decisions.log.md` with a rationale.
6. **Cross-language boundary.** Out of scope. The JSON Schema files
   under `trace-schema/schema/` plus ADR-0006 (upcaster chain) define
   the .NET-facing contract. .NET adapters' own .NET API design
   guidelines apply on their side; they are not bound by Rust's.

## Consequences

- Public surfaces become predictable for downstream consumers and
  reviewable against a public checklist. Newcomers can run the
  guidelines themselves before opening a PR.
- The `missing_docs` lint + `cargo doc -D warnings` forces every
  public item to carry a doc comment from day one. This is non-trivial
  upfront cost but pays for itself the first time someone reads the
  crate docs cold.
- Sealing `Upcaster` closes the door on a potentially useful
  extensibility point (third-party schema upcasters). The trade-off is
  intentional: the version-dispatch invariant is more valuable than
  pluggability, and we have no real use case for third-party upcasters.
- `#[non_exhaustive]` on `Outcome`, `OracleSchedule`, `MutationKind`
  costs downstream a default arm in every match. The cost is
  unavoidable if we want to grow those enums post-v1.0; declaring
  it now is the honest move.
- `cargo public-api` baseline becomes part of release ritual.
  Accidental SemVer-breaks become a CI failure instead of a
  user-filed bug six months later.
- The audit at S12 may surface dozens of small renames / re-derivations
  / doc-additions. Scheduling that work into S12 instead of "discover
  it the day before release" is part of why the stage exists.

## See also

- External: [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
  (checklist landing at `/checklist.html`); `rust-lang/api-guidelines`
  repo on GitHub.
- External tools: `cargo-public-api`, `cargo-semver-checks`,
  `cargo-deny`.
- ADR-0001 (workspace), ADR-0002 (hexagonal — ports = the public
  trait surface this ADR most cares about), ADR-0004 (`unsafe_code =
  "forbid"`), ADR-0006 (per-version enums frozen forever; `Upcaster`
  sealed), ADR-0009 (Nygard ADR format), ADR-0011 (one trace, many
  projections — the projection traits are public surface and bound by
  this ADR).
- [`../stages/S0-baseline-workspace.md`](../stages/S0-baseline-workspace.md)
  (initial lint config) and
  [`../stages/S12-v1-0-stable-release.md`](../stages/S12-v1-0-stable-release.md)
  (audit checklist).
- [`../glossary.md`](../glossary.md) §12 (Rust workspace), new
  entries `C-* checks`, `cargo-public-api`, `cargo-semver-checks`.
