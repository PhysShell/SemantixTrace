// Background-thread JSONL writer for SemantxTrace Avalonia adapter.
// Flush policy: items are queued without blocking the caller; Flush() drains
// the queue and flushes the underlying StreamWriter; Dispose() is a final flush.

using System;
using System.Collections.Concurrent;
using System.IO;
using System.Text;
using System.Text.Json;
using System.Threading;

using SemantxTrace.Abstractions;

namespace SemantxTrace.Avalonia;

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
    private readonly TextWriter _output;
    private readonly BlockingCollection<WorkItem> _queue;
    private readonly Thread _worker;
    private int _disposed;

    private readonly struct WorkItem
    {
        public readonly Func<long, string>? Build;
        public readonly ManualResetEventSlim? FlushSignal;
        public WorkItem(Func<long, string> build) { Build = build; FlushSignal = null; }
        public WorkItem(ManualResetEventSlim flushSignal) { Build = null; FlushSignal = flushSignal; }
    }

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
        ArgumentNullException.ThrowIfNull(output);
        _output = output;
        _sessionId = Guid.NewGuid();
        _queue = new BlockingCollection<WorkItem>(boundedCapacity: 8192);
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
        ArgumentNullException.ThrowIfNull(screenId);
        Enqueue(BuildLine(_sessionId, correlationId, writer =>
        {
            writer.WriteString("kind", "ScreenOpened");
            writer.WriteString("screen_id", screenId);
        }));
    }

    /// <inheritdoc />
    public void EmitCommandExecuted(string commandId, long durationMs, CommandOutcome outcome, Guid? correlationId = null)
    {
        ArgumentNullException.ThrowIfNull(commandId);
        if (durationMs < 0) throw new ArgumentOutOfRangeException(nameof(durationMs), durationMs, "Duration must be non-negative.");
        Enqueue(BuildLine(_sessionId, correlationId, writer =>
        {
            writer.WriteString("kind", "CommandExecuted");
            writer.WriteString("command_id", commandId);
            writer.WriteNumber("duration_ms", durationMs);
            WriteOutcome(writer, outcome);
        }));
    }

    /// <inheritdoc />
    public void EmitFieldChanged(string fieldId, ValuePolicy oldValue, ValuePolicy newValue, Guid? correlationId = null)
    {
        ArgumentNullException.ThrowIfNull(fieldId);
        ArgumentNullException.ThrowIfNull(oldValue);
        ArgumentNullException.ThrowIfNull(newValue);
        Enqueue(BuildLine(_sessionId, correlationId, writer =>
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
        ArgumentNullException.ThrowIfNull(exceptionType);
        ArgumentNullException.ThrowIfNull(message);
        Enqueue(BuildLine(_sessionId, correlationId, writer =>
        {
            writer.WriteString("kind", "ExceptionThrown");
            writer.WriteString("exception_type", exceptionType);
            // Always mask message and stack at the sink level (ADR-0007).
            writer.WriteString("message", "***");
            if (stack is not null)
                writer.WriteString("stack", "***");
        }));
    }

    /// <inheritdoc />
    public void EmitNavigationOccurred(string fromScreenId, string toScreenId, Guid? correlationId = null)
    {
        ArgumentNullException.ThrowIfNull(fromScreenId);
        ArgumentNullException.ThrowIfNull(toScreenId);
        Enqueue(BuildLine(_sessionId, correlationId, writer =>
        {
            writer.WriteString("kind", "NavigationOccurred");
            writer.WriteString("from", fromScreenId);
            writer.WriteString("to", toScreenId);
        }));
    }

    /// <inheritdoc />
    public void EmitValidationFailed(string validator, string fieldId, string reason, Guid? correlationId = null)
    {
        ArgumentNullException.ThrowIfNull(validator);
        ArgumentNullException.ThrowIfNull(fieldId);
        ArgumentNullException.ThrowIfNull(reason);
        Enqueue(BuildLine(_sessionId, correlationId, writer =>
        {
            writer.WriteString("kind", "ValidationFailed");
            writer.WriteString("validator", validator);
            writer.WriteString("field_id", fieldId);
            // Mask reason: may contain user-supplied text (ADR-0007).
            writer.WriteString("reason", "***");
        }));
    }

    /// <inheritdoc />
    public void EmitAsyncOperationCompleted(string operationId, long durationMs, CommandOutcome outcome, Guid? correlationId = null)
    {
        ArgumentNullException.ThrowIfNull(operationId);
        if (durationMs < 0) throw new ArgumentOutOfRangeException(nameof(durationMs), durationMs, "Duration must be non-negative.");
        Enqueue(BuildLine(_sessionId, correlationId, writer =>
        {
            writer.WriteString("kind", "AsyncOperationCompleted");
            writer.WriteString("operation_id", operationId);
            writer.WriteNumber("duration_ms", durationMs);
            WriteOutcome(writer, outcome);
        }));
    }

    /// <inheritdoc />
    public void Flush()
    {
        if (Volatile.Read(ref _disposed) != 0) return;
        var signal = new ManualResetEventSlim(initialState: false);
        try
        {
            _queue.Add(new WorkItem(signal));
        }
        catch (InvalidOperationException)
        {
            return;
        }
        if (!signal.Wait(TimeSpan.FromSeconds(10)))
            throw new TimeoutException("SemantxTrace.FileJsonlTraceContext: Flush timed out after 10 s. The writer thread may be stalled.");
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

    private void Enqueue(Func<long, string> build)
    {
        if (Volatile.Read(ref _disposed) != 0) return;
        _queue.TryAdd(new WorkItem(build), millisecondsTimeout: 100);
    }

    private static Func<long, string> BuildLine(Guid sessionId, Guid? correlationId, Action<Utf8JsonWriter> writeFields)
    {
        return seq =>
        {
            using var ms = new MemoryStream();
            using (var writer = new Utf8JsonWriter(ms))
            {
                writer.WriteStartObject();
                writer.WriteNumber("schema_version", 1);
                writer.WriteNumber("seq", seq);
                writer.WriteString("session_id", sessionId.ToString("D"));
                writer.WriteString("ts", DateTimeOffset.UtcNow.ToString("O"));
                if (correlationId.HasValue)
                    writer.WriteString("correlation_id", correlationId.Value.ToString("D"));
                writeFields(writer);
                writer.WriteEndObject();
            }
            return Encoding.UTF8.GetString(ms.ToArray());
        };
    }

    private static void WriteOutcome(Utf8JsonWriter writer, CommandOutcome outcome)
    {
        writer.WriteString("outcome", outcome switch
        {
            CommandOutcome.Success   => "success",
            CommandOutcome.Failure   => "failure",
            CommandOutcome.Cancelled => "cancelled",
            CommandOutcome.TimedOut  => "timed_out",
            _                        => "failure",
        });
        if (outcome == CommandOutcome.Failure)
        {
            writer.WriteStartObject("detail");
            writer.WriteString("message", "***");
            writer.WriteEndObject();
        }
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
                writer.WriteString("policy", "masked");
                writer.WriteString("display", "***");
                break;
        }
        writer.WriteEndObject();
    }

    private void WriteLoop()
    {
        long seq = 0;
        try
        {
            foreach (WorkItem item in _queue.GetConsumingEnumerable())
            {
                if (item.FlushSignal is { } signal)
                {
                    _output.Flush();
                    signal.Set();
                    continue;
                }
                _output.WriteLine(item.Build!(seq++));
            }
        }
        finally
        {
            _output.Flush();
        }
    }
}
