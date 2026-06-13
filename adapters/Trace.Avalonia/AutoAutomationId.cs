// Attached behavior: generates {ScreenId}.{Name} AutomationIds at Loaded time.
// Subscribes via the Avalonia property-changed class-handler pattern (no WeakEventManager
// needed since OnLoaded is static — the element holds a delegate to a static method,
// creating no reference cycle).

using System;
using System.Linq;

using Avalonia;
using Avalonia.Automation;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.VisualTree;

using SemantxTrace.Abstractions;

namespace SemantxTrace.Avalonia;

/// <summary>
/// Attached behavior that generates <c>{ScreenId}.{Name}</c>
/// <c>AutomationId</c> values for all named controls in a visual subtree
/// at <c>Loaded</c> time.
/// </summary>
/// <remarks>
/// <para>
/// Set <c>trace:AutoAutomationId.IsEnabled="True"</c> on the root
/// <c>UserControl</c> (or any <c>Control</c>) to activate the behavior.
/// The <c>ScreenId</c> is read from <see cref="ScreenIdAttribute"/> on the
/// root's type; if the attribute is absent the unqualified type name is used.
/// </para>
/// <para>
/// Unnamed controls (those with an empty or <see langword="null"/>
/// <c>Control.Name</c>) are skipped.
/// </para>
/// </remarks>
public static class AutoAutomationId
{
    /// <summary>
    /// Identifies the <c>AutoAutomationId.IsEnabled</c> attached property.
    /// </summary>
    public static readonly AttachedProperty<bool> IsEnabledProperty =
        AvaloniaProperty.RegisterAttached<AutoAutomationId, Control, bool>("IsEnabled");

    static AutoAutomationId()
    {
        IsEnabledProperty.Changed.AddClassHandler<Control>(OnIsEnabledChanged);
    }

    /// <summary>Gets the value of <see cref="IsEnabledProperty"/>.</summary>
    /// <param name="element">The target control.</param>
    /// <returns><see langword="true"/> if the behavior is enabled.</returns>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="element"/> is <see langword="null"/>.
    /// </exception>
    public static bool GetIsEnabled(Control element)
    {
        ArgumentNullException.ThrowIfNull(element);
        return element.GetValue(IsEnabledProperty);
    }

    /// <summary>Sets the value of <see cref="IsEnabledProperty"/>.</summary>
    /// <param name="element">The target control.</param>
    /// <param name="value"><see langword="true"/> to enable the behavior.</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="element"/> is <see langword="null"/>.
    /// </exception>
    public static void SetIsEnabled(Control element, bool value)
    {
        ArgumentNullException.ThrowIfNull(element);
        element.SetValue(IsEnabledProperty, value);
    }

    private static void OnIsEnabledChanged(Control element, AvaloniaPropertyChangedEventArgs e)
    {
        if (e.NewValue is true)
            element.Loaded += OnLoaded;
        else
            element.Loaded -= OnLoaded;
    }

    private static void OnLoaded(object? sender, RoutedEventArgs e)
    {
        if (sender is not Control root) return;
        string screenId = GetScreenId(root);
        AssignIds(root, screenId);
    }

    private static string GetScreenId(Control root)
    {
        return root.GetType()
            .GetCustomAttributes(typeof(ScreenIdAttribute), inherit: true)
            .OfType<ScreenIdAttribute>()
            .FirstOrDefault()
            ?.ScreenId ?? root.GetType().Name;
    }

    private static void AssignIds(Visual node, string screenId)
    {
        foreach (Visual child in node.GetVisualChildren())
        {
            if (child is Control { Name: { Length: > 0 } name } ctrl)
                AutomationProperties.SetAutomationId(ctrl, $"{screenId}.{name}");
            AssignIds(child, screenId);
        }
    }
}
