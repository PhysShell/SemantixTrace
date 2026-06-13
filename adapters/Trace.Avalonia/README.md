# Trace.Avalonia

Avalonia 11.x adapter for [SemantxTrace](https://github.com/PhysShell/SemantixTrace).
Implements the same `[TraceCommand]` / `[ScreenId]` / `AutoAutomationId` pattern as
`Trace.Wpf`, cross-platform (Linux, macOS, Windows).

## Quick start

```csharp
// App startup — one context per session:
var traceCtx = new FileJsonlTraceContext("trace.jsonl");

// ViewModel constructor:
SubmitCommand = new TracedRelayCommand(
    new RelayCommand(ExecuteSubmit),
    commandId: "Invoice.Submit",
    context: traceCtx);

// View class:
[ScreenId("InvoiceEditor")]
public partial class InvoiceEditorPage : UserControl { ... }
```

```xml
<!-- AXAML — enable AutomationId assignment: -->
<UserControl xmlns:trace="clr-namespace:SemantxTrace.Avalonia;assembly=Trace.Avalonia"
             trace:AutoAutomationId.IsEnabled="True">
```

## Capability matrix

| Feature | Trace.Wpf | Trace.Avalonia | Notes |
|---------|-----------|----------------|-------|
| `FileJsonlTraceContext` | ✅ | ✅ | Identical implementation; no framework dependency |
| `NoOpTraceContext` | ✅ | ✅ | Identical |
| `TracedRelayCommand` | ✅ | ✅ | Identical; `ICommand` is BCL-portable |
| `AutoAutomationId` | ✅ | ✅ | Port using Avalonia `AttachedProperty<bool>` |
| Targets | `net472;net8.0-windows` | `net8.0` | Cross-platform single target |
| Navigation | `Frame.Navigate` | `ContentControl` swap | View-first; no MVVM framework required |
| Weak-event subscription | `WeakEventManager<T>` | Static handler (no cycle) | Safe: static method holds no element reference |
| `CommandManager.RequerySuggested` | Automatic | Manual `RaiseCanExecuteChanged()` | Avalonia has no global command manager |
| `AutomationProperties.SetAutomationId` | WPF API | Avalonia API (same contract) | Same semantic behavior |
| Headless tests | WPF UI Test (requires Windows) | `Avalonia.Headless.XUnit` (any OS) | Full CI on Linux |

## Divergences from Trace.Wpf

- **`CanExecuteChanged` invalidation**: Avalonia has no equivalent of
  `CommandManager.InvalidateRequerySuggested()`. Use `RaiseCanExecuteChanged()` on your
  `RelayCommand` when command availability changes.
- **`AutoAutomationId` subscription**: WPF uses `WeakEventManager<FrameworkElement, RoutedEventArgs>`
  to avoid memory leaks. Avalonia uses a static event handler, which is equally safe because a static
  method holds no reference back to the element.
- **`Frame` navigation**: The Avalonia demo replaces `System.Windows.Controls.Frame` with an
  `Action<Control>` navigation callback wired to a `ContentControl` in `MainWindow`. The trace
  events (`NavigationOccurred`, `ScreenOpened`) are identical.
