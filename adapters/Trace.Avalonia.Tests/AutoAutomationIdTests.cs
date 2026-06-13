using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Headless.XUnit;

using SemantxTrace.Abstractions;
using SemantxTrace.Avalonia;

using Xunit;

namespace SemantxTrace.Avalonia.Tests;

public sealed class AutoAutomationIdTests
{
    [AvaloniaFact]
    public void IsEnabled_assigns_screen_dot_name_automation_id()
    {
        var window = new Window();
        var page = new AnnotatedPage();
        var btn = new Button { Name = "SaveButton" };
        page.Content = btn;
        AutoAutomationId.SetIsEnabled(page, true);
        window.Content = page;
        window.Show();

        Assert.Equal("AnnotatedPage.SaveButton", AutomationProperties.GetAutomationId(btn));
    }

    [AvaloniaFact]
    public void IsEnabled_falls_back_to_type_name_without_screen_id_attribute()
    {
        var window = new Window();
        var root = new ContentControl { Name = "UnattributedControl" };
        var btn = new Button { Name = "Btn" };
        root.Content = btn;
        AutoAutomationId.SetIsEnabled(root, true);
        window.Content = root;
        window.Show();

        Assert.Equal("ContentControl.Btn", AutomationProperties.GetAutomationId(btn));
    }

    [AvaloniaFact]
    public void IsEnabled_skips_unnamed_controls()
    {
        var window = new Window();
        var page = new AnnotatedPage();
        var btn = new Button(); // no Name
        page.Content = btn;
        AutoAutomationId.SetIsEnabled(page, true);
        window.Content = page;
        window.Show();

        // Unnamed controls must not receive an AutomationId.
        Assert.True(string.IsNullOrEmpty(AutomationProperties.GetAutomationId(btn)));
    }

    [AvaloniaFact]
    public void GetIsEnabled_returns_set_value()
    {
        var ctrl = new Button();
        AutoAutomationId.SetIsEnabled(ctrl, true);
        Assert.True(AutoAutomationId.GetIsEnabled(ctrl));
    }
}

/// <summary>Helper control with a <see cref="ScreenIdAttribute"/> applied.</summary>
[ScreenId("AnnotatedPage")]
internal sealed class AnnotatedPage : ContentControl { }
