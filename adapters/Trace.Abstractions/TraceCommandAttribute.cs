using System;

namespace SemantxTrace.Abstractions;

/// <summary>
/// Marks an <c>ICommand</c> property on a ViewModel as a semantic command
/// that should be traced by <c>TracedRelayCommand</c> (ADR-0005).
/// </summary>
/// <remarks>
/// <para>
/// The <see cref="CommandId"/> should be a stable dot-delimited identifier
/// (e.g. <c>"DeclarationEditor.Submit"</c>).  Identifiers are frozen after
/// v1.0 per ADR-0006.
/// </para>
/// <para>
/// Code-behind event handlers that do not expose an <c>ICommand</c> are
/// out-of-scope for this attribute; use
/// <c>ITraceContext.EmitCommandExecuted</c> directly instead.
/// </para>
/// </remarks>
[AttributeUsage(AttributeTargets.Property, Inherited = false, AllowMultiple = false)]
public sealed class TraceCommandAttribute : Attribute
{
    /// <summary>The stable semantic command identifier.</summary>
    public string CommandId { get; }

    /// <summary>Initialises a new <see cref="TraceCommandAttribute"/>.</summary>
    /// <param name="commandId">
    /// The stable dot-delimited semantic command identifier
    /// (e.g. <c>"DeclarationEditor.Submit"</c>).
    /// </param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="commandId"/> is <see langword="null"/>.
    /// </exception>
    public TraceCommandAttribute(string commandId)
    {
        CommandId = commandId ?? throw new ArgumentNullException(nameof(commandId));
    }
}
