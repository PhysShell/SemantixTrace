# S6: WPF adapter (Trace.Wpf NuGet)

Status: landed
Depends on: S5
ADRs: ADR-0005, ADR-0007, ADR-0013

## Goal

Ship the .NET / NuGet adapter that lets WPF apps emit a SemantxTrace-
compatible JSONL stream with minimal boilerplate. Implement
`[TraceCommand]`, `[ScreenId]`, `AutoAutomationId`, `TracedRelayCommand`,
`ITraceContext`, and the JSONL sink.

## Inputs / Outputs

- In: the published `trace-schema` v1 JSON Schema; the JSONL wire
  format; the `adapters/Directory.Build.props` from S0 (ADR-0013 §3).
- Out:
  - `Trace.Abstractions` NuGet (NetStandard 2.0, ABI-frozen post-v1.0
    per ADR-0013): `ITraceContext`, `ValuePolicy`,
    `[TraceCommand]`, `[ScreenId]`, `[TraceField]`. Zero
    third-party dependencies (the .NET analog of `trace-core`
    discipline from ADR-0002 / ADR-0012).
  - `Trace.Wpf` NuGet (`net472;net8.0-windows`):
    - `TraceCommandAttribute(string commandId)` on `ICommand`
      properties;
    - `ScreenIdAttribute(string screenId)` on `UserControl`s;
    - `AutoAutomationId` attached behavior generating
      `{ScreenId}.{x:Name}` AutomationIds at `Loaded`;
    - `TracedRelayCommand` decorator wrapping `ICommand.Execute` with
      `CorrelationId` + duration + exception capture;
    - `ITraceContext` interface, default `FileJsonlTraceContext` writer,
      release-build `NoOpTraceContext`;
    - `WeakEventManager`-based subscriptions throughout;
    - `ValuePolicy`-aware `[TraceField]` attribute for per-field
      privacy overrides.
  - .NET unit tests asserting emitted JSONL parses against the published
    JSON Schema.
  - WPF capability matrix in the adapter's README.
  - `PublicAPI.Shipped.txt` baselines for both
    `Trace.Abstractions` and `Trace.Wpf` (ADR-0013 §4); CI
    blocks PRs that change the public surface without updating
    the unshipped file.
  - Per-package `README.md` packed into each `.nupkg` (ADR-0013
    §12); each Quick start sample is built in CI from
    `examples/`.

## Approach

- A CommunityToolkit.Mvvm source generator backs `[TraceCommand]` so no
  runtime reflection is needed in the hot path.
- The JSONL sink uses a background writer with the same flush policy
  defaults as `trace-storage::JsonlBackend` (S2).
- PII scanning is regex-only, applied per `[TraceField]` policy; the
  default for free-text strings is `Masked` (ADR-0007).
- CI runs `cargo test --workspace` and `dotnet test
  adapters/trace-wpf/` on Windows and Linux (the latter compiles only
  the `Trace.Wpf` library against .NET 8 framework references — UI
  integration tests are Windows-only).
- A "minimum traceability checklist" file in the adapter README mirrors
  the [`../glossary.md`](../glossary.md) §10 enforcement requirements.

## Acceptance criteria

- A toy WPF view with one `[TraceCommand]` and one `[ScreenId]` emits a
  syntactically valid JSONL stream that the Rust core's `analyze`
  command processes without errors.
- Generated AutomationIds match `{ScreenId}.{x:Name}` for all named
  controls; unnamed controls are skipped.
- `TracedRelayCommand` rethrows exceptions but emits an
  `ExceptionThrown` event with a redacted stack first.
- Round-trip property test (on .NET side): emitted events validate
  against `trace-event-v1.schema.json`.
- No memory leaks across 10 000 cycles of view create/destroy (tested
  with dotMemory or equivalent).
- `dotnet build -c Release -warnaserror` green on Linux (libraries
  + reference-assembly check for `Trace.Wpf`) and Windows (full
  build + UI tests). All CA-rules from
  `AnalysisMode=AllEnabledByDefault` clean or explicitly suppressed
  in `.editorconfig` with a justification comment.
- `dotnet pack -c Release` produces `.nupkg` + `.snupkg` for
  `Trace.Abstractions` and `Trace.Wpf`; `PackageValidation` (built
  into the .NET SDK per ADR-0013 §3) green against an empty
  baseline for v1.0.0.

## Open questions

- Whether to ship a Roslyn analyzer flagging ViewModels with
  `RelayCommand` properties lacking `[TraceCommand]`. Working answer:
  yes, opt-in via `<TraceWpfStrict>true</TraceWpfStrict>` MSBuild
  property.
- How to handle code-behind event handlers (no `ICommand`). Working
  answer: documented as out-of-scope; provide a `TraceContext.Emit(...)`
  manual API for niche cases.

## See also

- [`../adr/0005-semantic-action-map-not-physical-ui-map.md`](../adr/0005-semantic-action-map-not-physical-ui-map.md)
- [`../adr/0007-privacy-by-default-mask-and-bucket.md`](../adr/0007-privacy-by-default-mask-and-bucket.md)
- [`../adr/0013-follow-dotnet-framework-design-guidelines.md`](../adr/0013-follow-dotnet-framework-design-guidelines.md)
- [`../glossary.md`](../glossary.md) §10, §11 (.NET conventions)
