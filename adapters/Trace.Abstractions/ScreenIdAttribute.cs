using System;

namespace SemantxTrace.Abstractions;

/// <summary>
/// Marks a <c>UserControl</c> (or any WPF view class) with its stable
/// semantic screen identifier (ADR-0005).
/// </summary>
/// <remarks>
/// <para>
/// The identifier must be stable across renames — use the domain concept
/// (e.g. <c>"DeclarationEditor"</c>), not the class name, so that refactors
/// do not silently break recorded traces.
/// </para>
/// <para>
/// <c>AutoAutomationId</c> reads this attribute when generating
/// <c>{ScreenId}.{x:Name}</c> <c>AutomationId</c> values at
/// <c>Loaded</c> time.
/// </para>
/// </remarks>
[AttributeUsage(AttributeTargets.Class, Inherited = false, AllowMultiple = false)]
public sealed class ScreenIdAttribute : Attribute
{
    /// <summary>The stable semantic screen identifier.</summary>
    public string ScreenId { get; }

    /// <summary>Initialises a new <see cref="ScreenIdAttribute"/>.</summary>
    /// <param name="screenId">
    /// The stable semantic screen identifier (e.g. <c>"DeclarationEditor"</c>).
    /// </param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="screenId"/> is <see langword="null"/>.
    /// </exception>
    public ScreenIdAttribute(string screenId)
    {
        ScreenId = screenId ?? throw new ArgumentNullException(nameof(screenId));
    }
}
