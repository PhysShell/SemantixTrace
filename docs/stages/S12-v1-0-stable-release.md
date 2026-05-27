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
      WPF demo.
- [ ] mdBook docs site builds without dead links; getting-started
      chapter copies and runs end-to-end on a clean machine.
- [ ] crates.io and NuGet metadata complete (descriptions, repository
      links, keywords, license, MSRV).
- [ ] Blog post drafted and reviewed.

## Open questions

- Whether to bundle a `trace-cli` Docker image for downstream CI.
  Working answer: yes, post-v1.0.
- Whether to open a `discussions` forum or rely on GitHub issues only.
  Working answer: issues only at v1.0; discussions if traffic warrants.

## See also

- [`../adr/README.md`](../adr/README.md) — all decisions.
- [`../upcasters.md`](../upcasters.md) — proven across two bumps by
  S12.
- [`../glossary.md`](../glossary.md) §0 (project terms), §15 (quality).
