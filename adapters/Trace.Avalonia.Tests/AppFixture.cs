using Avalonia;
using Avalonia.Headless;

[assembly: AvaloniaTestApplication(typeof(SemantxTrace.Avalonia.Tests.TestApp))]

namespace SemantxTrace.Avalonia.Tests;

public sealed class TestApp : global::Avalonia.Application
{
    public static AppBuilder BuildAvaloniaApp()
        => AppBuilder.Configure<TestApp>()
                     .UseHeadless(new AvaloniaHeadlessOptions { UseHeadlessDrawing = true });
}
