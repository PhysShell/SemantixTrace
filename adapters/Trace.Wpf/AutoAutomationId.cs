// Attached behavior: generates {ScreenId}.{x:Name} AutomationIds at Loaded time.
// Uses WeakEventManager to avoid rooting the visual tree (ADR-0013 §WeakEventManager).

using System;
using System.Linq;
using System.Windows;
using System.Windows.Automation;
using System.Windows.Media;

using SemantxTrace.Abstractions;

namespace SemantxTrace.Wpf;

/// <summary>
/// Attached behavior that generates <c>{ScreenId}.{x:Name}</c>
/// <c>AutomationId</c> values for all named controls in a visual subtree
/// at <c>Loaded</c> time.
/// </summary>
/// <remarks>
/// <para>
/// Set <c>local:AutoAutomationId.IsEnabled="True"</c> on the root
/// <c>UserControl</c> (or any <c>FrameworkElement</c>) to activate the
/// behavior.  The <c>ScreenId</c> is read from
/// <see cref="ScreenIdAttribute"/> on the root's type; if the attribute is
/// absent, the unqualified type name is used as a fallback.
/// </para>
/// <para>
/// Unnamed controls (those with an empty or <see langword="null"/>
/// <c>FrameworkElement.Name</c>) are skipped.
/// </para>
/// <para>
/// Subscriptions use <see cref="WeakEventManager{TEventSource,TEventArgs}"/>
/// to prevent the behavior from rooting the visual tree.
/// </para>
/// </remarks>
public static class AutoAutomationId
{
    /// <summary>
    /// Identifies the <c>AutoAutomationId.IsEnabled</c> attached property.
    /// </summary>
    public static readonly DependencyProperty IsEnabledProperty =
        DependencyProperty.RegisterAttached(
            "IsEnabled",
            typeof(bool),
            typeof(AutoAutomationId),
            new PropertyMetadata(defaultValue: false, propertyChangedCallback: OnIsEnabledChanged));

    /// <summary>Gets the value of <see cref="IsEnabledProperty"/>.</summary>
    /// <param name="element">The target element.</param>
    /// <returns><see langword="true"/> if the behavior is enabled.</returns>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="element"/> is <see langword="null"/>.
    /// </exception>
    [AttachedPropertyBrowsableForType(typeof(FrameworkElement))]
    public static bool GetIsEnabled(DependencyObject element)
    {
        if (element is null) throw new ArgumentNullException(nameof(element));
        return (bool)element.GetValue(IsEnabledProperty);
    }

    /// <summary>Sets the value of <see cref="IsEnabledProperty"/>.</summary>
    /// <param name="element">The target element.</param>
    /// <param name="value"><see langword="true"/> to enable the behavior.</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="element"/> is <see langword="null"/>.
    /// </exception>
    public static void SetIsEnabled(DependencyObject element, bool value)
    {
        if (element is null) throw new ArgumentNullException(nameof(element));
        element.SetValue(IsEnabledProperty, value);
    }

    private static void OnIsEnabledChanged(DependencyObject d, DependencyPropertyChangedEventArgs e)
    {
        if (d is not FrameworkElement element) return;

        bool enable = (bool)e.NewValue;
        if (enable)
        {
            WeakEventManager<FrameworkElement, RoutedEventArgs>.AddHandler(
                element, nameof(FrameworkElement.Loaded), OnLoaded);
        }
        else
        {
            WeakEventManager<FrameworkElement, RoutedEventArgs>.RemoveHandler(
                element, nameof(FrameworkElement.Loaded), OnLoaded);
        }
    }

    private static void OnLoaded(object? sender, RoutedEventArgs e)
    {
        if (sender is not FrameworkElement root) return;

        string screenId = GetScreenId(root);
        AssignIds(root, screenId);
    }

    private static string GetScreenId(FrameworkElement root)
    {
        var attr = root.GetType()
            .GetCustomAttributes(typeof(ScreenIdAttribute), inherit: true)
            .OfType<ScreenIdAttribute>()
            .FirstOrDefault();

        return attr?.ScreenId ?? root.GetType().Name;
    }

    private static void AssignIds(DependencyObject node, string screenId)
    {
        int count = VisualTreeHelper.GetChildrenCount(node);
        for (int i = 0; i < count; i++)
        {
            DependencyObject child = VisualTreeHelper.GetChild(node, i);
            if (child is FrameworkElement fe && !string.IsNullOrEmpty(fe.Name))
            {
                AutomationProperties.SetAutomationId(fe, $"{screenId}.{fe.Name}");
            }
            AssignIds(child, screenId);
        }
    }
}
