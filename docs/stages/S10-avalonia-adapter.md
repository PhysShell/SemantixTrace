# S10: Avalonia adapter (closes v0.3)

Status: planned
Depends on: S7
ADRs: ADR-0005

## Goal

Ship `Trace.Avalonia`: the same `[TraceCommand]` / `[ScreenId]` /
`AutoAutomationId` pattern translated to Avalonia's MVVM and
`AutomationProperties`. Validate the adapter against a port of
`DeclarationApp.Demo` to Avalonia.

## Inputs / Outputs

- In: S6 WPF adapter as the reference; the published JSON Schema; the
  WPF demo app for reuse of the domain layer.
- Out:
  - `adapters/trace-avalonia/` — Avalonia 11.x library.
  - `examples/DeclarationApp.Demo.Avalonia/` — a structural port of the
    WPF demo using the same ViewModels, with Avalonia views.
  - Capability matrix in `Trace.Avalonia/README.md`, documenting where
    the adapter matches WPF and where it diverges.
  - `Avalonia.Headless`-based unit tests asserting the adapter emits
    schema-conformant JSONL.

## Approach

- Avalonia's `ICommand` model is structurally identical to WPF's; the
  source generator from S6 reuses most of its code, parameterised over
  the target framework's command base type.
- `AutomationProperties` in Avalonia matches WPF's contract; the
  `AutoAutomationId` behavior is a port of the WPF version.
- The Avalonia demo shares the `Declarations.Domain` and
  `Declarations.ViewModels` projects with the WPF demo; only the View
  projects differ. This validates that the semantic-action-map approach
  (ADR-0005) genuinely decouples tests from the UI framework.
- The same fixture recording from S7 plays back through the Avalonia
  demo via the same JSONL contract; results are compared to the WPF
  baseline.

## Acceptance criteria

- `dotnet run --project examples/DeclarationApp.Demo.Avalonia` launches
  on Linux, macOS, and Windows.
- Headless unit tests assert schema conformance.
- Running the v1.0-MVP pipeline against an Avalonia-recorded session
  catches the same three intentional bugs as the WPF version.
- The capability matrix calls out any divergences explicitly.

## Open questions

- Whether to ship a single NuGet covering both WPF and Avalonia or
  separate packages. Working answer: separate (`Trace.Wpf`,
  `Trace.Avalonia`) plus a shared `Trace.Abstractions`.
- Whether to invest in a MAUI adapter alongside Avalonia. Working
  answer: defer to S11+ and ship as **experimental** in v1.0.

## See also

- [`../adr/0005-semantic-action-map-not-physical-ui-map.md`](../adr/0005-semantic-action-map-not-physical-ui-map.md)
- [`../glossary.md`](../glossary.md) §9
