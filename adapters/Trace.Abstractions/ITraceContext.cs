using System;

namespace SemantxTrace.Abstractions;

/// <summary>
/// Contract for emitting SemantxTrace events from a WPF (or other .NET UI)
/// application.  The default implementations are
/// <c>SemantxTrace.Wpf.FileJsonlTraceContext</c> (production) and
/// <c>SemantxTrace.Wpf.NoOpTraceContext</c> (release / stripped builds).
/// </summary>
/// <remarks>
/// Implementations must be thread-safe: UI code and background tasks may call
/// <c>Emit*</c> methods concurrently.  The <see cref="Flush"/> method must
/// guarantee that all previously emitted events have been persisted before it
/// returns.  <see cref="IDisposable.Dispose"/> implies a final flush.
/// </remarks>
public interface ITraceContext : IDisposable
{
    /// <summary>The session identifier shared by all events emitted through this context.</summary>
    Guid SessionId { get; }

    /// <summary>Emits a <c>ScreenOpened</c> event.</summary>
    /// <param name="screenId">The semantic screen identifier (ADR-0005).</param>
    /// <param name="correlationId">Optional correlation identifier linking related events.</param>
    void EmitScreenOpened(string screenId, Guid? correlationId = null);

    /// <summary>Emits a <c>CommandExecuted</c> event.</summary>
    /// <param name="commandId">The semantic command identifier.</param>
    /// <param name="durationMs">Wall-clock execution duration in milliseconds.</param>
    /// <param name="outcome">The domain outcome.</param>
    /// <param name="correlationId">Optional correlation identifier.</param>
    void EmitCommandExecuted(string commandId, long durationMs, CommandOutcome outcome, Guid? correlationId = null);

    /// <summary>Emits a <c>FieldChanged</c> event.</summary>
    /// <param name="fieldId">The semantic field identifier.</param>
    /// <param name="oldValue">Previous value, privacy-wrapped.</param>
    /// <param name="newValue">New value, privacy-wrapped.</param>
    /// <param name="correlationId">Optional correlation identifier.</param>
    void EmitFieldChanged(string fieldId, ValuePolicy oldValue, ValuePolicy newValue, Guid? correlationId = null);

    /// <summary>Emits an <c>ExceptionThrown</c> event with a redacted stack trace.</summary>
    /// <param name="exceptionType">The concrete exception type name.</param>
    /// <param name="message">Human-readable message (will be masked by default, ADR-0007).</param>
    /// <param name="stack">Optional redacted stack trace.</param>
    /// <param name="correlationId">Optional correlation identifier.</param>
    void EmitExceptionThrown(string exceptionType, string message, string? stack = null, Guid? correlationId = null);

    /// <summary>Emits a <c>NavigationOccurred</c> event.</summary>
    /// <param name="fromScreenId">The departing screen's semantic identifier.</param>
    /// <param name="toScreenId">The arriving screen's semantic identifier.</param>
    /// <param name="correlationId">Optional correlation identifier.</param>
    void EmitNavigationOccurred(string fromScreenId, string toScreenId, Guid? correlationId = null);

    /// <summary>Emits a <c>ValidationFailed</c> event.</summary>
    /// <param name="validator">The validator name (e.g. <c>"Required"</c>, <c>"EmailFormat"</c>).</param>
    /// <param name="fieldId">The field that failed validation.</param>
    /// <param name="reason">Human-readable reason.</param>
    /// <param name="correlationId">Optional correlation identifier.</param>
    void EmitValidationFailed(string validator, string fieldId, string reason, Guid? correlationId = null);

    /// <summary>Emits an <c>AsyncOperationCompleted</c> event.</summary>
    /// <param name="operationId">Application-defined operation identifier.</param>
    /// <param name="durationMs">Wall-clock duration in milliseconds.</param>
    /// <param name="outcome">The domain outcome.</param>
    /// <param name="correlationId">Optional correlation identifier.</param>
    void EmitAsyncOperationCompleted(string operationId, long durationMs, CommandOutcome outcome, Guid? correlationId = null);

    /// <summary>
    /// Blocks until all queued events have been written to the underlying sink
    /// and the sink has been flushed to durable storage.
    /// </summary>
    void Flush();
}
