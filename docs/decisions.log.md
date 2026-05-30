# Decisions log

Append-only Y-statements for small, non-architectural decisions. Format:

> In the context of `<situation>`, facing `<concern>`, we decided for
> `<option>` and against `<alternatives>`, to achieve `<benefit>`,
> accepting `<downside>`.

Architectural decisions go to [`adr/`](adr/) instead.

---

- 2026-05-30 — In the context of the S8 extracted index columns, facing the
  temptation to index `domain_entity_id` alongside `command_id` and
  `screen_id`, we decided **not to index `domain_entity_id` in v1** and
  against adding a nested-extraction path into `args`/`params`, because
  entity ids are buried inside the free-form `args`/`params`
  `serde_json::Value` blobs with no standard field name, and the extractor
  explicitly reads only top-level payload fields; indexing it would require
  either a v2 schema bump (a top-level `domain_entity_id` field) or a
  fragile adapter-specific extraction mapping. The third indexed column is
  `outcome TEXT` instead (`CommandExecuted`/`AsyncOperationCompleted` expose
  it as a top-level field via `#[serde(flatten)]`), accepting that entity-
  level slicing/similarity is deferred until a schema bump elevates the field.

- 2026-05-30 — In the context of scenario similarity search in S8, facing
  the choice between embedding-based / ML similarity and a count-of-shared-
  semantic-dimensions approach, we decided for **Jaccard-like scoring over
  (command_id, screen_id, outcome) tuples** and against
  vector embeddings or graph-kernel methods, to keep the scorer pure,
  deterministic, and explainable (a score of 0.75 means "3 of 4 dimensions
  match"), to avoid any ML dependency through v1.0 (SPEC §"Not a neural-
  anomaly detector"), and because the four dimensions map directly to the
  indexed columns added for parametric slicing — no new storage required,
  accepting that this similarity model is coarser than embedding-based
  approaches and will not catch semantic equivalence across different command
  names for the same intent.

- 2026-05-30 — In the context of corpus analysis in S8 (SQLite backend),
  facing the need to query traces by semantic dimensions (command, screen,
  entity) without full-scanning `payload_json` on every filter, we decided
  to **add three extracted index columns** (`command_id`, `screen_id`,
  `domain_entity_id`) to the `events` table and a `trace slice --by` CLI
  subcommand, and against JSON path expressions over the raw payload column,
  to give O(log n) parametric slicing over the corpus without changing the
  primary wire format or the upcaster chain, accepting a thin extractor at
  ingest time and the invariant that the indexed columns are always
  query-only and never the authoritative record.

- 2026-05-30 — In the context of a "corpus meta-graph" pattern (linking
  traces to external artefacts via relations such as `regression_of`,
  `fixed_by`, `caused_by`, `validated_by`, as inspired by hypergraph-
  based context-graph research), facing the appeal of semantic memory
  connecting traces to commits, tickets, and test cases, we decided to
  **defer this past v1.0** and against adding a relational meta-store to
  the current roadmap, because the basic projection pipeline must be
  battle-tested before adding a second graph layer, accepting that traces
  cannot be formally linked to the artefacts around them in v1.0; a
  `labels: Map<String, String>` field on `TraceEvent` (a future minor
  schema bump) is the smallest seed that could grow toward this if the
  need proves real.

- 2026-05-28 — In the context of the S3 normalizer's idempotency
  acceptance criterion (`glossary.md` §3 lists
  `normalize(normalize(t)) == normalize(t)`), facing the fact that
  `normalize: Session -> Scenario` is **not** endomorphic (input and
  output are different types, so the literal self-composition does not
  type-check), we decided to **realize idempotency in two endomorphic
  layers** and against forcing an artificial `Session`-shaped output:
  (1) value abstraction is a fixed point —
  `abstract_value(abstract_value(v)) == abstract_value(v)` (tagged
  `_abstract` JSON objects pass through), and (2) `refold: Scenario ->
  Scenario` is idempotent — `refold(refold(s)) == refold(s)`. Both are
  property-tested. This achieves the spirit of the criterion (a stable
  normalized fixed point) while keeping the `Session -> Scenario`
  projection honest, accepting that the glossary phrasing is satisfied
  indirectly rather than as a single literal identity.

- 2026-05-28 — In the context of the `trace normalize` output-file flag
  (the S3 stage doc sketched `-o <file>`), facing the collision with
  ADR-0014 §4's global `-o` / `--output {text,json,wide}` format flag,
  we decided to use **`--out <path>`** for the normalize output file and
  against shadowing the global `-o`, to keep the CLI grammar consistent
  with the binding ADR-0014 contract, accepting a one-character
  divergence from the original stage-doc sketch.

- 2026-05-28 — In the context of explicit `BurstAction` / `SessionPause`
  marker nodes (named in `glossary.md` §4), facing the choice between
  extending the S1-frozen `Scenario` / `CanonicalAction` types now vs
  deferring, we decided to **defer marker nodes** and record bursts /
  pauses as counts in `FoldReport` for S3, and against widening the
  `Scenario` shape this early, to keep the S4 graph builder's input
  stable; explicit marker nodes can land later behind their own change
  if the miner needs them. Burst *collapsing* (deduplicating rapid
  repeats) is implemented in S3; only the explicit marker representation
  is deferred.

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

- 2026-05-27 — In the context of positioning SemantxTrace, facing the
  original framing as "UI regression testing for Tech Leads, not COOs"
  vs the broader behavioral-observability framing, we decided for
  **trace-as-multi-projection** (ADR-0011): one canonical semantic
  trace, seven projections (analytics, diagnostic, regression test,
  UX, product, support replay, exploration), no second wire format,
  and against splitting the system across separate analytics /
  testing / support backends, to anchor the project's central pitch
  ("semantic metrics, not contextless counters") in the architecture
  itself, accepting a slightly heavier per-event schema than a
  counter-only model would carry.

- 2026-05-27 — In the context of naming the v0.4 exploration feature,
  facing the existing working name `smart-monkey` vs alternatives in
  the project's own vocabulary, we decided to **rename it to
  `semantic-monkey`** and against keeping the `smart-` prefix, to
  comply with glossary §19's ban on the word "smart" in user-facing
  copy and to align with the project's emphasis on semantic actions,
  accepting one global rename across docs, code, and CLI surface (now
  in S11). The S11 file is renamed accordingly.

- 2026-05-27 — In the context of v0.4's exploration / generation
  surface, facing the choice between a single "exploration" feature
  (random / coverage-guided walks) vs splitting generation into two
  distinct paths, we decided for **`semantic-monkey` walks + first-
  class `trace-mutation`** as separate operations in S11, and against
  collapsing mutation into "another monkey strategy", to keep the
  distinction visible — the monkey *walks the graph* generating fresh
  sequences, mutation *transforms an existing scenario* preserving
  most of its structure — accepting a slightly larger S11 surface in
  exchange for honest semantics.

- 2026-05-27 — In the context of replay determinism, facing the choice
  between a single replay mode with tolerance knobs vs two explicit
  named modes, we decided for **`strict` vs `relaxed` / `normalized`
  as distinct `ReplayMode` values** (SPEC hard rule 16, glossary §6,
  S11), and against a single mode with `--allow-reorder` /
  `--ignore-timing` flags, to make the determinism contract part of
  the type, not an option bag the caller has to remember to set,
  accepting two reduction paths in the planner.

- 2026-05-27 — In the context of style and conventions on the public
  Rust API surface, facing the choice between adopting the official
  [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
  wholesale vs writing a SemantxTrace-local style guide, we decided
  for **wholesale adoption** with project-specific commitments
  enumerated in ADR-0012 (mandatory `C-SEALED` on `Upcaster`,
  selective `C-NON-EXHAUSTIVE` rules accommodating the per-version
  event freeze from ADR-0006, `missing_docs` + pedantic clippy +
  `cargo doc -D warnings` from S0, full checklist audit at S12), and
  against a local style guide, to give downstream consumers the
  conventions they already expect from idiomatic Rust crates and to
  outsource the maintenance of the rulebook to the Rust project,
  accepting upfront discipline (every `pub` item carries a doc
  comment from S0) and a pre-release audit cost (S12). The .NET wire
  boundary is explicitly out of scope and continues to be governed by
  ADR-0006 plus the published JSON Schema.

- 2026-05-27 — In the context of conventions for our published .NET
  NuGet adapters (`Trace.Abstractions`, `Trace.Wpf`,
  `Trace.Avalonia`, `Trace.Maui`), facing the choice between writing
  a project-local style guide vs adopting the Microsoft Framework
  Design Guidelines + .NET Library Guidance wholesale, we decided
  for **wholesale adoption + the Sentry / OpenTelemetry package
  split** (ADR-0013): `Trace.Abstractions` is the contract package
  (zero third-party deps, ABI-frozen after v1.0), per-framework
  adapters carry implementations, `AnalysisMode=AllEnabledByDefault`
  + `TreatWarningsAsErrors=true` + `EnablePackageValidation` +
  `Microsoft.CodeAnalysis.PublicApiAnalyzers` are the .NET analog of
  the Rust-side clippy-pedantic + cargo-public-api gates from
  ADR-0012, and against a local style guide / a monolithic single
  NuGet, to give .NET consumers an idiomatic Sentry-shaped surface
  they recognise immediately and to outsource the rulebook
  maintenance to Microsoft. Accepting an upfront cost in `.csproj`
  boilerplate (mitigated by a shared `Directory.Build.props`) and
  the need to track `PublicAPI.Shipped.txt` baselines in PRs.
  Strong-naming intentionally deferred — modern guidance has shifted
  to "only if a downstream needs it" and Sentry / OTel follow suit.

- 2026-05-27 — In the context of `trace-cli` binary ergonomics
  (subcommand grammar, exit codes, output streams, JSON mode,
  configuration), facing the choice between inventing a SemantxTrace
  CLI style vs adopting an established standard, we decided for
  **clig.dev + POSIX + GNU + sysexits.h + Rain's Rust CLI
  recommendations**, with **Vector as the architectural precedent**
  (ADR-0014). Noun-verb subcommands (kubectl / gh / docker style),
  `-o {text,json,wide}` global output mode, data→stdout /
  diag→stderr, `--no-color` + `NO_COLOR`, sysexits.h exit codes
  (`78` `EX_CONFIG` matching Vector's `vector validate` precedent),
  shell completions via `clap_complete`, snapshot-tested help via
  `trycmd`, versioned `--output json` schemas through the same
  upcaster chain as event data (ADR-0006), and against improvising
  any of these, to meet users in the CLI category they already work
  in (kubectl / gh / cargo / vector / fly), accepting a non-trivial
  `miette` dependency in the binary crate and the cost of
  maintaining versioned schemas for every JSON-emitting subcommand.
