using System;

using SemantxTrace.Abstractions;

namespace SemantxTrace.Avalonia;

/// <summary>
/// No-op <see cref="ITraceContext"/> suitable for release builds and
/// scenarios where tracing should be silently disabled.
/// </summary>
/// <remarks>
/// All <c>Emit*</c> methods return immediately without allocating.
/// Use the <see cref="Instance"/> singleton to avoid heap allocation.
/// </remarks>
public sealed class NoOpTraceContext : ITraceContext
{
    /// <summary>Singleton no-op instance.</summary>
    public static NoOpTraceContext Instance { get; } = new NoOpTraceContext();

    private NoOpTraceContext() { }

    /// <inheritdoc />
    public Guid SessionId => Guid.Empty;

    /// <inheritdoc />
    public void EmitScreenOpened(string screenId, Guid? correlationId = null) { }

    /// <inheritdoc />
    public void EmitCommandExecuted(string commandId, long durationMs, CommandOutcome outcome, Guid? correlationId = null) { }

    /// <inheritdoc />
    public void EmitFieldChanged(string fieldId, ValuePolicy oldValue, ValuePolicy newValue, Guid? correlationId = null) { }

    /// <inheritdoc />
    public void EmitExceptionThrown(string exceptionType, string message, string? stack = null, Guid? correlationId = null) { }

    /// <inheritdoc />
    public void EmitNavigationOccurred(string fromScreenId, string toScreenId, Guid? correlationId = null) { }

    /// <inheritdoc />
    public void EmitValidationFailed(string validator, string fieldId, string reason, Guid? correlationId = null) { }

    /// <inheritdoc />
    public void EmitAsyncOperationCompleted(string operationId, long durationMs, CommandOutcome outcome, Guid? correlationId = null) { }

    /// <inheritdoc />
    public void Flush() { }

    /// <inheritdoc />
    public void Dispose() { }
}
