# Decisions log

Append-only Y-statements for small, non-architectural decisions. Format:

> In the context of `<situation>`, facing `<concern>`, we decided for
> `<option>` and against `<alternatives>`, to achieve `<benefit>`,
> accepting `<downside>`.

Architectural decisions go to [`adr/`](adr/) instead.

---

- 2026-05-27 — In the context of seeding the SemantxTrace knowledge base
  before any code is written, facing the choice between drafting docs
  alongside an initial workspace skeleton vs landing documentation first,
  we decided for **documentation-first commits** (no `Cargo.toml`, no
  CI yet) and against a mixed doc+skeleton seed PR, to make the SPEC,
  glossary, ADRs, stage plan, and upcaster pattern reviewable in
  isolation, accepting that S0 will land the workspace skeleton in a
  follow-up commit.

- 2026-05-27 — In the context of the project's canonical name, facing a
  three-way collision (repo slug `SemantixTrace`, report wording
  `SemanticTrace`, working title `SemantxTrace`), we decided for
  **`SemantxTrace`** as the in-repo canonical name and kept the GitHub
  repository at `PhysShell/SemantixTrace` for legacy reasons, against
  renaming the repo or rebranding to `SemanticTrace`, to avoid churning
  the URL while keeping the work-title consistent with prior
  discussion, accepting that the URL and the brand differ.

- 2026-05-27 — In the context of repository text language, facing a
  Russian planning document and English code/community norms, we
  decided for **English-only repository text** (SPEC, glossary, ADRs,
  stage docs, code, comments, commits, fixture filenames) and against
  bilingual docs or Russian-default, to maximise hiring signal and
  outside contributor readability per Section 6.C of issue #1,
  accepting that the planning conversation between maintainers stays
  in Russian.

- 2026-05-27 — In the context of scoping "до v1.0" for the
  documentation seed, facing the choice between v1.0 = PH-launch MVP
  (Section 5.A) vs v1.0 = stable release with v0.2/v0.3/v0.4 included,
  we decided for **v1.0 = stable release** and against ending the
  roadmap at S7, to make the upcaster commitment (ADR-0006) credible
  end-to-end (two real schema bumps before v1.0), accepting a larger
  initial doc set (13 stages) and a longer notional timeline.

- 2026-05-27 — In the context of the upcaster pattern documentation,
  facing the choice between embedding it inside ADR-0006 vs splitting
  it across ADR + a working reference, we decided for **ADR-0006 +
  `docs/upcasters.md`** (the ADR fixes the commitment, the reference
  carries the pattern, the worked examples, the property-test contract,
  and the bump-procedure) and against either a single 30 KB ADR or
  scattering the material across stage docs, to keep ADRs immutable
  and concise per ADR-0009 while allowing the pattern reference to
  evolve as the chain grows, accepting that two files must be kept in
  sync.
