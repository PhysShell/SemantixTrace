# Trace.Abstractions

Contract-only NuGet for [SemantxTrace](https://github.com/PhysShell/SemantixTrace).

Holds the interfaces, attributes, and value types that framework-specific
adapters (`Trace.Wpf`, `Trace.Avalonia`, `Trace.Maui`) implement.

Targets **netstandard2.0**. Zero third-party dependencies (ABI-frozen after
v1.0 per ADR-0013).

## Quick start

```csharp
using SemantxTrace.Abstractions;

// 1. Mark your screen (on the UserControl class):
[ScreenId("InvoiceEditor")]
public partial class InvoiceEditorView : UserControl { ... }

// 2. Mark your commands (on the ViewModel):
[TraceCommand("Invoice.Submit")]
public ICommand SubmitCommand { get; }

// 3. Mark traced fields with policy overrides (optional):
[TraceField("InvoiceEditor.Amount", Policy = ValuePolicyKind.Bucketed)]
public decimal Amount { ... }
```

## API surface

| Type | Purpose |
|------|---------|
| `ITraceContext` | Sink for trace events; injected into ViewModels |
| `ValuePolicy` | Abstract base for the five privacy policies |
| `CommandOutcome` | `Success / Failure / Cancelled / TimedOut` |
| `ValuePolicyKind` | Per-field policy selector for `[TraceField]` |
| `[TraceCommand]` | Marks `ICommand` properties for tracing |
| `[ScreenId]` | Attaches a stable semantic id to a view class |
| `[TraceField]` | Marks bound properties with a privacy policy |

## Minimum traceability checklist

Every screen that participates in SemantxTrace MUST:

- [ ] Have `[ScreenId("…")]` on the `UserControl` class.
- [ ] Have `AutoAutomationId.IsEnabled="True"` in XAML (or set in code-behind).
- [ ] Wrap every user-facing `ICommand` in `TracedRelayCommand` (or decorate with `[TraceCommand]`).
- [ ] Ensure all `string` bound fields use `Masked` (default) unless explicitly opted in to `Raw` with a reviewer sign-off.

See [`docs/glossary.md`](../../docs/glossary.md) §10 for the full enforcement requirements.
