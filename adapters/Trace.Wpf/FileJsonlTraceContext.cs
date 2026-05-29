// Background-thread JSONL writer for SemantxTrace WPF adapter.
// Flush policy: items are queued without blocking the caller; Flush() drains
// the queue and flushes the underlying StreamWriter; Dispose() is a final flush.

using System;
using System.Collections.Concurrent;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Threading;

using SemantxTrace.Abstractions;

namespace SemantxTrace.Wpf;

/// <summary>
/// Production <see cref="ITraceContext"/> implementation: serialises trace
/// events as JSONL (one JSON object per line) to a file using a background
/// writer thread.
/// </summary>
/// <remarks>
/// The background thread is a <c>IsBackground = true</c> daemon so it never
/// prevents process shutdown.  Call <see cref="Flush"/> before the application
/// exits to guarantee all queued events are persisted.
/// </remarks>
public sealed class FileJsonlTraceContext : ITraceContext
{
    private readonly Guid _sessionId;
    private long _seq;
    private readonly TextWriter _output;
    private readonly BlockingCollection<string> _queue;
    private readonly ConcurrentQueue<ManualResetEventSlim> _pendingFlushes;
    private readonly Thread _worker;
    private int _disposed;

    // Sentinel that triggers an explicit Flush of the underlying writer.
    // ReferenceEquals identity check distinguishes it from real payloads.
    private static readonly string FlushSentinel = string.Empty;

    /// <summary>
    /// Initialises a new <see cref="FileJsonlTraceContext"/> that writes to
    /// <paramref name="filePath"/>, overwriting any existing file.
    /// </summary>
    /// <param name="filePath">Destination <c>.jsonl</c> file path.</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="filePath"/> is <see langword="null"/>.
    /// </exception>
    public FileJsonlTraceContext(string filePath)
        : this(new StreamWriter(
            filePath ?? throw new ArgumentNullException(nameof(filePath)),
            append: false,
            encoding: new UTF8Encoding(encoderShouldEmitUTF8Identifier: false)))
    { }

    /// <summary>
    /// Initialises a new <see cref="FileJsonlTraceContext"/> that writes to
    /// <paramref name="output"/> (intended for testing).
    /// </summary>
    /// <param name="output">The sink to write JSONL lines to.</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="output"/> is <see langword="null"/>.
    /// </exception>
    public FileJsonlTraceContext(TextWriter output)
    {
        _output = output ?? throw new ArgumentNullException(nameof(output));
        _sessionId = Guid.NewGuid();
        _queue = new BlockingCollection<string>(boundedCapacity: 8192);
        _pendingFlushes = new ConcurrentQueue<ManualResetEventSlim>();
        _worker = new Thread(WriteLoop)
        {
            IsBackground = true,
            Name = "SemantxTrace.Writer",
        };
        _worker.Start();
    }

    /// <inheritdoc />
    public Guid SessionId => _sessionId;

    /// <inheritdoc />
    public void EmitScreenOpened(string screenId, Guid? correlationId = null)
    {
        if (screenId is null) throw new ArgumentNullException(nameof(screenId));
        Enqueue(BuildLine(correlationId, writer =>
        {
            writer.WriteString("kind", "ScreenOpened");
            writer.WriteString("screen_id", screenId);
        }));
    }

    /// <inheritdoc />
    public void EmitCommandExecuted(string commandId, long durationMs, CommandOutcome outcome, Guid? correlationId = null)
    {
        if (commandId is null) throw new ArgumentNullException(nameof(commandId));
        Enqueue(BuildLine(correlationId, writer =>
        {
            writer.WriteString("kind", "CommandExecuted");
            writer.WriteString("command_id", commandId);
            writer.WriteNumber("duration_ms", durationMs);
            writer.WriteString("outcome", OutcomeToString(outcome));
        }));
    }

    /// <inheritdoc />
    public void EmitFieldChanged(string fieldId, ValuePolicy oldValue, ValuePolicy newValue, Guid? correlationId = null)
    {
        if (fieldId is null) throw new ArgumentNullException(nameof(fieldId));
        if (oldValue is null) throw new ArgumentNullException(nameof(oldValue));
        if (newValue is null) throw new ArgumentNullException(nameof(newValue));
        Enqueue(BuildLine(correlationId, writer =>
        {
            writer.WriteString("kind", "FieldChanged");
            writer.WriteString("field_id", fieldId);
            writer.WritePropertyName("old");
            WriteValuePolicy(writer, oldValue);
            writer.WritePropertyName("new");
            WriteValuePolicy(writer, newValue);
        }));
    }

    /// <inheritdoc />
    public void EmitExceptionThrown(string exceptionType, string message, string? stack = null, Guid? correlationId = null)
    {
        if (exceptionType is null) throw new ArgumentNullException(nameof(exceptionType));
        if (message is null) throw new ArgumentNullException(nameof(message));
        Enqueue(BuildLine(correlationId, writer =>
        {
            writer.WriteString("kind", "ExceptionThrown");
            writer.WriteString("exception_type", exceptionType);
            writer.WriteString("message", message);
            if (stack is not null)
                writer.WriteString("stack", stack);
        }));
    }

    /// <inheritdoc />
    public void EmitNavigationOccurred(string fromScreenId, string toScreenId, Guid? correlationId = null)
    {
        if (fromScreenId is null) throw new ArgumentNullException(nameof(fromScreenId));
        if (toScreenId is null) throw new ArgumentNullException(nameof(toScreenId));
        Enqueue(BuildLine(correlationId, writer =>
        {
            writer.WriteString("kind", "NavigationOccurred");
            writer.WriteString("from", fromScreenId);
            writer.WriteString("to", toScreenId);
        }));
    }

    /// <inheritdoc />
    public void EmitValidationFailed(string validator, string fieldId, string reason, Guid? correlationId = null)
    {
        if (validator is null) throw new ArgumentNullException(nameof(validator));
        if (fieldId is null) throw new ArgumentNullException(nameof(fieldId));
        if (reason is null) throw new ArgumentNullException(nameof(reason));
        Enqueue(BuildLine(correlationId, writer =>
        {
            writer.WriteString("kind", "ValidationFailed");
            writer.WriteString("validator", validator);
            writer.WriteString("field_id", fieldId);
            writer.WriteString("reason", reason);
        }));
    }

    /// <inheritdoc />
    public void EmitAsyncOperationCompleted(string operationId, long durationMs, CommandOutcome outcome, Guid? correlationId = null)
    {
        if (operationId is null) throw new ArgumentNullException(nameof(operationId));
        Enqueue(BuildLine(correlationId, writer =>
        {
            writer.WriteString("kind", "AsyncOperationCompleted");
            writer.WriteString("operation_id", operationId);
            writer.WriteNumber("duration_ms", durationMs);
            writer.WriteString("outcome", OutcomeToString(outcome));
        }));
    }

    /// <inheritdoc />
    public void Flush()
    {
        if (Volatile.Read(ref _disposed) != 0) return;
        using var signal = new ManualResetEventSlim(initialState: false);
        _pendingFlushes.Enqueue(signal);
        _queue.Add(FlushSentinel);
        signal.Wait(TimeSpan.FromSeconds(10));
    }

    /// <inheritdoc />
    public void Dispose()
    {
        if (Interlocked.Exchange(ref _disposed, 1) != 0) return;
        _queue.CompleteAdding();
        _worker.Join(TimeSpan.FromSeconds(30));
        _queue.Dispose();
        _output.Dispose();
        GC.SuppressFinalize(this);
    }

    // ----------------------------------------------------------------
    // Private helpers
    // ----------------------------------------------------------------

    private void Enqueue(string line)
    {
        if (Volatile.Read(ref _disposed) != 0) return;
        _queue.TryAdd(line, millisecondsTimeout: 100);
    }

    private string BuildLine(Guid? correlationId, Action<Utf8JsonWriter> writeFields)
    {
        long seq = Interlocked.Increment(ref _seq) - 1;
        using var ms = new MemoryStream();
        using (var writer = new Utf8JsonWriter(ms))
        {
            writer.WriteStartObject();
            writer.WriteNumber("schema_version", 1);
            writer.WriteNumber("seq", seq);
            writer.WriteString("session_id", _sessionId.ToString("D"));
            writer.WriteString("ts", DateTimeOffset.UtcNow.ToString("O"));
            if (correlationId.HasValue)
                writer.WriteString("correlation_id", correlationId.Value.ToString("D"));
            writeFields(writer);
            writer.WriteEndObject();
        }
        return Encoding.UTF8.GetString(ms.ToArray());
    }

    private static void WriteValuePolicy(Utf8JsonWriter writer, ValuePolicy policy)
    {
        writer.WriteStartObject();
        switch (policy)
        {
            case MaskedValuePolicy m:
                writer.WriteString("policy", "masked");
                writer.WriteString("display", m.Display);
                break;
            case RawValuePolicy r:
                writer.WriteString("policy", "raw");
                writer.WritePropertyName("value");
                JsonSerializer.Serialize(writer, r.Value);
                break;
            case BucketedValuePolicy b:
                writer.WriteString("policy", "bucketed");
                writer.WriteString("bucket", b.Bucket);
                break;
            case HashedValuePolicy h:
                writer.WriteString("policy", "hashed");
                writer.WriteString("hash", h.Hash);
                writer.WriteString("algo", h.Algo);
                break;
            case RemovedValuePolicy:
                writer.WriteString("policy", "removed");
                break;
            default:
                // Unknown subclass: fall back to masked (privacy-by-default, ADR-0007).
                writer.WriteString("policy", "masked");
                writer.WriteString("display", "***");
                break;
        }
        writer.WriteEndObject();
    }

    private static string OutcomeToString(CommandOutcome outcome) =>
        outcome switch
        {
            CommandOutcome.Success => "success",
            CommandOutcome.Failure => "failure",
            CommandOutcome.Cancelled => "cancelled",
            CommandOutcome.TimedOut => "timed_out",
            _ => "failure",
        };

    private void WriteLoop()
    {
        try
        {
            foreach (string item in _queue.GetConsumingEnumerable())
            {
                if (ReferenceEquals(item, FlushSentinel))
                {
                    _output.Flush();
                    while (_pendingFlushes.TryDequeue(out var signal))
                        signal.Set();
                    continue;
                }
                _output.WriteLine(item);
            }
        }
        finally
        {
            _output.Flush();
        }
    }
}
