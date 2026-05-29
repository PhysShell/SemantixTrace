# Trace.Wpf

WPF adapter for [SemantxTrace](https://github.com/PhysShell/SemantixTrace).

Targets **net472** and **net8.0-windows**. Depends on `Trace.Abstractions`.

## Quick start

```csharp
// 1. Create a context at application startup (one per session):
var traceCtx = new FileJsonlTraceContext("trace.jsonl");

// 2. Inject it into your ViewModel and wrap commands:
public InvoiceEditorViewModel(ITraceContext ctx)
{
    _ctx = ctx;
    SubmitCommand = new TracedRelayCommand(
        new RelayCommand(ExecuteSubmit),
        commandId: "Invoice.Submit",
        context: ctx);
}

// 3. Activate AutoAutomationId on the root UserControl in XAML:
// <UserControl local:AutoAutomationId.IsEnabled="True" ...>

// 4. Flush and dispose on app exit:
traceCtx.Flush();
traceCtx.Dispose();
```

## Components

| Type | Purpose |
|------|---------|
| `FileJsonlTraceContext` | Background JSONL writer (production) |
| `NoOpTraceContext` | Silent no-op (stripped / release builds) |
| `TracedRelayCommand` | Wraps `ICommand` with timing, correlation-id, exception capture |
| `AutoAutomationId` | Generates `{ScreenId}.{x:Name}` `AutomationId`s at `Loaded` |

## Privacy defaults (ADR-0007)

- Exception **messages** are always masked to `"***"`.  Exception type names are recorded as-is (not PII).
- Exception **stack traces** are never recorded.
- String field values default to `Masked("***")` unless `[TraceField(Policy=Raw)]` opts in.
- Numeric field values default to `Bucketed`.

## Capability matrix

| Feature | net472 | net8.0-windows |
|---------|--------|---------------|
| `FileJsonlTraceContext` | ✔ | ✔ |
| `NoOpTraceContext` | ✔ | ✔ |
| `TracedRelayCommand` | ✔ | ✔ |
| `AutoAutomationId` | ✔ | ✔ |
| UI integration tests | ✔ (Windows) | ✔ (Windows) |
| Library compilation | ✔ (Windows) | ✔ (Linux + Windows) |
