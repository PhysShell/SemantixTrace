# S12: v1.0 stable release

Status: planned
Depends on: S11
ADRs: all accepted ADRs

## Goal

Cut the v1.0 stable release. Freeze the public API of `trace-core`,
`trace-schema`, and the JSON schemas. Prove the upcaster chain end-to-
end through at least two real schema bumps. Publish crates and NuGets;
deploy the mdBook docs site.

## Inputs / Outputs

- In: S0..S11 in a green state.
- Out:
  - crates.io releases for all `trace-*` Rust crates with semver
    guarantees stated in their READMEs.
  - NuGet releases for `Trace.Abstractions`, `Trace.Wpf`,
    `Trace.Avalonia`; `Trace.Maui` shipped marked **experimental**.
  - Tagged release `v1.0.0` on the GitHub repository.
  - mdBook docs site deployed to GitHub Pages (or equivalent).
  - Blog post: "SemantxTrace v1.0: production-informed UI regression
    testing for desktop, in Rust", anchored on the semantic-action-map
    angle (ADR-0005) and the upcaster pattern (ADR-0006).
  - Comparison table in the README, updated with current competitor
    pricing snapshot.
  - Final `decisions.log.md` entry recording the release.

## Approach

- Run the full property suite, the full fuzz regression corpus, and the
  end-to-end demo on a clean machine; record artefacts.
- API freeze: any new `pub` items added past this point require a minor
  bump; any signature change requires a major.
- Schema freeze: `trace_schema::v1` and `v_current` (whatever number it
  is) are frozen forever. The upcaster chain may grow but never shrink.
- Run a bounded fuzz sweep (4h per target on `jsonl_parse`,
  `upcaster_*`, `normalize_fold`, `oracle_replay`, `replay_plan_parse`);
  any finding blocks the release.
- A v1.0-readiness checklist lives in this file (see below) and must be
  fully green.

## Acceptance criteria (v1.0-readiness checklist)

- [ ] All ADRs `Accepted` and consistent with the codebase.
- [ ] Glossary §0 entries for every stage have `Status: done`.
- [ ] At least two schema bumps have shipped (`v1 → v2 → v_current`),
      proven by real recordings, with property tests for every step.
- [ ] Fuzz corpora green for 30+ consecutive nightlies.
- [ ] `DeclarationApp.Demo` (WPF) and `DeclarationApp.Demo.Avalonia`
      both run the v1.0-MVP pipeline end-to-end with identical oracle
      verdicts.
- [ ] Replay-planner reproduces all three intentional bugs against the
      WPF demo, in both `strict` and `relaxed` modes.
- [ ] mdBook docs site builds without dead links; getting-started
      chapter copies and runs end-to-end on a clean machine.
- [ ] crates.io and NuGet metadata complete (descriptions, repository
      links, keywords, license, MSRV).
- [ ] Blog post drafted and reviewed.
- [ ] **Rust API Guidelines audit (ADR-0012) complete**, signed off
      crate-by-crate against the full
      [checklist](https://rust-lang.github.io/api-guidelines/checklist.html):
  - [ ] Naming: `C-CASE`, `C-CONV`, `C-GETTER`, `C-ITER`, `C-ITER-TY`,
        `C-FEATURE`, `C-WORD-ORDER`.
  - [ ] Interoperability: `C-COMMON-TRAITS`, `C-CONV-TRAITS`,
        `C-COLLECT`, `C-SERDE`, `C-SEND-SYNC`, `C-GOOD-ERR`,
        `C-NUM-FMT`, `C-RW-VALUE`.
  - [ ] Macros: `C-EVOCATIVE`, `C-MACRO-ATTR`, `C-ANYWHERE`,
        `C-MACRO-VIS`, `C-MACRO-TY`.
  - [ ] Documentation: `C-CRATE-DOC`, `C-EXAMPLE`, `C-QUESTION-MARK`,
        `C-ERROR-DOC`, `C-PANIC-DOC`, `C-LINK`, `C-METADATA`,
        `C-HTML-ROOT`, `C-RELNOTES`, `C-HIDDEN`.
  - [ ] Predictability: `C-SMART-PTR`, `C-CONV-SPECIFIC`,
        `C-METHOD`, `C-NO-OUT`, `C-OVERLOAD`, `C-DEREF`,
        `C-CTOR`.
  - [ ] Flexibility: `C-INTERMEDIATE`, `C-CALLER-CONTROL`,
        `C-GENERIC`, `C-OBJECT`.
  - [ ] Type safety: `C-NEWTYPE`, `C-CUSTOM-TYPE`, `C-BOOL`,
        `C-CTYPE`.
  - [ ] Dependability: `C-VALIDATE`, `C-DTOR-FAIL`,
        `C-DTOR-BLOCK`.
  - [ ] Debuggability: `C-DEBUG`, `C-DEBUG-NONEMPTY`.
  - [ ] Future-proofing: `C-SEALED` (mandatory for `Upcaster`),
        `C-STRUCT-PRIVATE`, `C-NEWTYPE-HIDE`, `C-STRUCT-BOUNDS`,
        `C-NON-EXHAUSTIVE` (yes on `Outcome`, `OracleSchedule`,
        `Severity`, `MutationKind`; **no** on per-version event
        enums frozen by ADR-0006).
  - [ ] Necessities: `C-STABLE`, `C-PERMISSIVE`.
- [ ] `cargo public-api --diff-git-checkouts v0.4.0 HEAD` produces
      only documented intentional additions; no accidental removals or
      signature changes. The diff is committed as the v1.0 release
      note appendix.
- [ ] `cargo semver-checks check-release` passes against the previous
      released version for every crate (where the previous version
      exists).
- [ ] Every crate in scope has a CHANGELOG.md updated for v1.0 and a
      Quick-start example in its README that compiles
      (`cargo test --doc`).
- [ ] CI gates from S0 (clippy pedantic, `cargo doc -D warnings`,
      `missing_docs`, `cargo deny`) remain green on the release
      commit.
- [ ] **.NET Framework Design Guidelines audit (ADR-0013)
      complete**, signed off per-package
      (`Trace.Abstractions`, `Trace.Wpf`, `Trace.Avalonia`):
  - [ ] `dotnet build -c Release -warnaserror` green on Linux,
        macOS, Windows; `AnalysisMode=AllEnabledByDefault` produces
        no warnings (or each suppression is justified in
        `.editorconfig` with a comment).
  - [ ] `PackageValidation` green for each package against the
        previous published version; baseline files in
        `CompatibilitySuppressions.xml` reviewed.
  - [ ] `PublicAPI.Shipped.txt` files committed and consistent
        with the released surface;
        `PublicAPI.Unshipped.txt` empty at release tag.
  - [ ] `dotnet pack` produces `.nupkg` + `.snupkg` with full
        metadata (description, license expression, repository URL,
        readme, tags, icon, source-link).
  - [ ] Per-package `README.md` packed into the `.nupkg`; the
        Quick start sample under `examples/` builds in CI.
  - [ ] `Trace.Abstractions` has zero third-party dependencies
        (verified via `dotnet list package` + manifest review).
  - [ ] Multi-targeting matrix passes:
        `netstandard2.0` for `Trace.Abstractions`,
        `net472;net8.0-windows` for `Trace.Wpf`,
        `net8.0` for `Trace.Avalonia`.
- [ ] **`trace-cli` ergonomics audit (ADR-0014) complete**:
  - [ ] Noun-verb grammar walk-test passes: every subcommand
        matches the inventory in ADR-0014 §3; no three-level
        depth without an ADR.
  - [ ] Exit-code smoke test passes: each error class produces
        the documented `sysexits.h` code (`0`/`1`/`2`/`64`/`65`/
        `66`/`70`/`73`/`74`/`78`).
  - [ ] `trycmd` snapshots green for every subcommand's
        success-shape, every named error-shape, and every
        `--help` page.
  - [ ] `--output json` payloads for every emitting subcommand
        validate against their published versioned schemas
        (`crates/trace-cli/schema/`); the upcaster chains for
        those schemas are exercised by their own property tests.
  - [ ] Stdout/stderr discipline test passes: piping every
        `--output json` invocation through `jq` yields zero
        diagnostic contamination on stdout.
  - [ ] `--no-color` + `NO_COLOR` env var both disable ANSI
        codes; `isatty(stdout) == false` autodetect works.
  - [ ] `trace completions {bash,zsh,fish,powershell}` emit
        scripts that source cleanly in each shell.
  - [ ] `EXIT CODES:` section present in every subcommand's
        `--help`.
  - [ ] mdBook installation chapter documents the shell-completion
        install paths.

## Open questions

- Whether to bundle a `trace-cli` Docker image for downstream CI.
  Working answer: yes, post-v1.0.
- Whether to open a `discussions` forum or rely on GitHub issues only.
  Working answer: issues only at v1.0; discussions if traffic warrants.

## See also

- [`../adr/README.md`](../adr/README.md) — all decisions.
- [`../adr/0012-follow-rust-api-guidelines-on-public-surfaces.md`](../adr/0012-follow-rust-api-guidelines-on-public-surfaces.md)
  — the Rust API Guidelines audit acceptance items above come from
  here.
- [`../adr/0013-follow-dotnet-framework-design-guidelines.md`](../adr/0013-follow-dotnet-framework-design-guidelines.md)
  — the .NET adapter audit acceptance items come from here.
- [`../adr/0014-trace-cli-ergonomics-clig-posix-sysexits-vector.md`](../adr/0014-trace-cli-ergonomics-clig-posix-sysexits-vector.md)
  — the CLI ergonomics audit acceptance items come from here.
- [`../upcasters.md`](../upcasters.md) — proven across two bumps by
  S12.
- [`../glossary.md`](../glossary.md) §0 (project terms), §15 (quality).
- External: <https://rust-lang.github.io/api-guidelines/checklist.html>,
  <https://learn.microsoft.com/en-us/dotnet/standard/design-guidelines/>,
  <https://learn.microsoft.com/en-us/dotnet/standard/library-guidance/>,
  <https://clig.dev/>,
  <https://rust-cli-recommendations.sunshowers.io/>.
