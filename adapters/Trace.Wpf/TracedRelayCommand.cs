// Wraps any ICommand to emit CommandExecuted / ExceptionThrown events.
// Exception messages are masked (ADR-0007); stack traces are omitted by default.

using System;
using System.Diagnostics;
using System.Windows.Input;

using SemantxTrace.Abstractions;

namespace SemantxTrace.Wpf;

/// <summary>
/// Wraps an <see cref="ICommand"/> to emit
/// <c>CommandExecuted</c> and <c>ExceptionThrown</c> trace events
/// via an <see cref="ITraceContext"/>.
/// </summary>
/// <remarks>
/// <para>
/// Decorate ViewModel <c>ICommand</c> properties with
/// <see cref="TraceCommandAttribute"/> and replace the backing
/// field with a <see cref="TracedRelayCommand"/> wrapping the real command.
/// </para>
/// <para>
/// Exception messages are masked to <c>"***"</c> by default (ADR-0007).
/// Stack traces are never recorded.  The original exception is always
/// re-thrown after the <c>ExceptionThrown</c> event is emitted.
/// </para>
/// <para>
/// <c>CanExecuteChanged</c> delegates to the inner command's event so
/// that WPF command-manager invalidation propagates correctly.
/// </para>
/// </remarks>
// CA1001 suppressed: TracedRelayCommand does not OWN the injected
// ITraceContext; the caller created it and is responsible for disposal.
[System.Diagnostics.CodeAnalysis.SuppressMessage(
    "Design",
    "CA1001:TypesThatOwnDisposableFieldsShouldBeDisposable",
    Justification = "The ITraceContext is injected and owned by the caller, not by this decorator.")]
public sealed class TracedRelayCommand : ICommand
{
    private readonly ICommand _inner;
    private readonly string _commandId;
    private readonly ITraceContext _context;

    /// <summary>
    /// Initialises a new <see cref="TracedRelayCommand"/>.
    /// </summary>
    /// <param name="inner">The command to wrap.</param>
    /// <param name="commandId">The stable semantic command identifier.</param>
    /// <param name="context">The trace context used for emitting events.</param>
    /// <exception cref="ArgumentNullException">
    ///   Any parameter is <see langword="null"/>.
    /// </exception>
    public TracedRelayCommand(ICommand inner, string commandId, ITraceContext context)
    {
        _inner = inner ?? throw new ArgumentNullException(nameof(inner));
        _commandId = commandId ?? throw new ArgumentNullException(nameof(commandId));
        _context = context ?? throw new ArgumentNullException(nameof(context));
    }

    /// <inheritdoc />
    public bool CanExecute(object? parameter) => _inner.CanExecute(parameter);

    /// <summary>
    /// Executes the wrapped command, recording a <c>CommandExecuted</c> event
    /// on both success and failure, and an <c>ExceptionThrown</c> event if the
    /// inner command throws.
    /// </summary>
    /// <param name="parameter">Data used by the command.</param>
    public void Execute(object? parameter)
    {
        var correlationId = Guid.NewGuid();
        var sw = Stopwatch.StartNew();
        try
        {
            _inner.Execute(parameter);
            sw.Stop();
            _context.EmitCommandExecuted(_commandId, sw.ElapsedMilliseconds, CommandOutcome.Success, correlationId);
        }
        catch (Exception ex)
        {
            sw.Stop();
            // Mask the exception message (ADR-0007); type name is not PII.
            _context.EmitExceptionThrown(
                exceptionType: ex.GetType().FullName ?? ex.GetType().Name,
                message: "***",
                stack: null,
                correlationId: correlationId);
            _context.EmitCommandExecuted(_commandId, sw.ElapsedMilliseconds, CommandOutcome.Failure, correlationId);
            throw;
        }
    }

    /// <inheritdoc />
    /// <remarks>
    /// Delegates to the inner command's <c>CanExecuteChanged</c> so that
    /// WPF's <see cref="CommandManager"/> invalidation propagates correctly.
    /// </remarks>
    public event EventHandler? CanExecuteChanged
    {
        add => _inner.CanExecuteChanged += value;
        remove => _inner.CanExecuteChanged -= value;
    }
}
