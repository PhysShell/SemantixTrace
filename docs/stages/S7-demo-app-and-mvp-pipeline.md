# S7: DeclarationApp.Demo and the v1.0-MVP / PH-launch pipeline

Status: planned
Depends on: S6
ADRs: ADR-0005, ADR-0007, ADR-0013, ADR-0014

## Goal

Close the v1.0-MVP / PH-launch milestone. Ship `DeclarationApp.Demo`
(a fake customs application), the end-to-end pipeline
`record → analyze → graph → oracle → report → diagnostic-package`,
a 3–5-minute demo video, a README GIF, and the mdBook docs site at v0.x.

The pipeline must materialise **at least four of the seven projections
named in ADR-0011** end-to-end on the recorded fixture: `analytics`
(top-N + rare-but-failing reports from S4), `regression-test` (oracle
results), `support-replay` (diagnostic package export), and
`diagnostic` (HTML report jumping from oracle failure to the offending
events). `ux` / `product` / `exploration` projections are read-throughs
on the same data and ship in S11 (`exploration`) and as user-facing
walkthroughs in the demo video.

## Inputs / Outputs

- In: S6 WPF adapter, S5 oracles, S4 graph, S3 normalizer, S2 storage,
  S1 schema.
- Out:
  - `examples/DeclarationApp.Demo/` — WPF app with 4–6 screens
    (`InvoiceEditor`, `GoodsEditor`, `PaymentsCalculator`,
    `ExportDialog`, supporting list/detail screens).
  - Three intentional bugs:
    1. `PaymentsCalculator` rounds quantities > 1000 down by one,
       producing a negative-payment edge case caught by a domain oracle
       `Graph47.ResultMustBeNonNegative`.
    2. `ExportDialog` shows an error modal on duplicate IIN but leaves
       the Export button enabled, caught by `NoErrorModalAfterCommand`.
    3. Async validation in `GoodsEditor` occasionally returns after
       submit, caught by `ValidationsPassBeforeSubmit`.
  - A scripted recording producing a `.jsonl.zst` session file.
  - End-to-end demo: `trace analyze`, `trace graph --format mermaid`,
    `trace report workflows --format json`, `trace oracle run`,
    `trace report --format html`, `trace export --diagnostic
    <session_id>` against the recording.
  - A README section titled **"Semantic metrics, not contextless
    counters"** showing the top-N / rare-but-failing JSON output
    side-by-side with a strawman "btnExport.clicked: 1200" counter to
    make the positioning concrete.
  - mdBook docs site published at v0.x (deployable to GitHub Pages or
    similar).
  - README GIF (15–30 s, looped, < 5 MB) plus a 3–5-minute walkthrough
    video.

## Approach

- The demo app exercises every conformant pattern in the WPF adapter's
  capability matrix: `[TraceCommand]`, `[ScreenId]`, `AutoAutomationId`,
  `[TraceField]` overrides, Stateless-driven workflow state machine,
  FluentValidation pipelines, `WeakEventManager`-based subscriptions.
- The scripted recording is a small .NET test (xUnit) that drives the
  demo via the WPF adapter's own command surface, producing a stable
  fixture session under `examples/DeclarationApp.Demo/fixtures/`.
- The HTML report ships with stable, snapshot-friendly output (no
  timestamps, no random ids in the body).
- mdBook layout mirrors [`../glossary.md`](../glossary.md) §16: one
  chapter per crate, a getting-started chapter, an architecture chapter
  with diagrams from `trace graph`.

## Acceptance criteria

- `dotnet run --project examples/DeclarationApp.Demo` launches the demo
  on Windows.
- The scripted recording produces a deterministic JSONL file
  byte-identical across runs on Windows / Linux (the recording itself
  is Windows-only but the file is checked in).
- Running the full pipeline against the recorded fixture catches all
  three intentional bugs as `Error`-severity oracle results.
- The top-N workflows report ranks the demo's "happy path" first and
  surfaces the bug-3 (async-validation) chain as a rare-but-failing
  workflow.
- `trace export diagnostic <session_id>` produces a single archive
  (trace + app/schema/dependency versions + oracle evidence) that a
  second machine can re-import via `trace analyze` and reproduce the
  same oracle verdicts. Subcommand follows the noun-verb grammar
  from ADR-0014 §3 (note: `trace export diagnostic`, not
  `trace export --diagnostic`).
- `trycmd` snapshots cover the demo's end-to-end pipeline (every
  subcommand invoked has its success-shape + at least one
  error-shape snapshot blessed), per ADR-0014 §12.
- `dotnet pack -c Release` for `Trace.Abstractions` and `Trace.Wpf`
  passes `PackageValidation` against the previous (S6) baseline.
- mdBook builds locally with `mdbook build` and renders without dead
  links.
- README contains the GIF and a "Quickstart" section that copies and
  runs end-to-end.
- Demo video uploaded (private link in `decisions.log.md`); the public
  link goes in the README on launch day.

## Open questions

- Whether to bundle a `trace-cli` GitHub Action for downstream users.
  Working answer: yes, post-v1.0 (S12 territory).
- License of the recorded fixture: MIT alongside the code; the demo
  app is intentionally fake so no real PII issues.

## See also

- [`../glossary.md`](../glossary.md) §10 (WPF specifics)
- [`../adr/0005-semantic-action-map-not-physical-ui-map.md`](../adr/0005-semantic-action-map-not-physical-ui-map.md)
