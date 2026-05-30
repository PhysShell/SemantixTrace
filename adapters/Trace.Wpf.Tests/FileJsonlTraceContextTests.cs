// Round-trip acceptance tests for FileJsonlTraceContext.
// Each emitted JSONL line is parsed and validated against the published
// trace-event-v1.schema.json (ADR-0006 acceptance criterion for S6).

using System;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using Json.Schema;
using SemantxTrace.Abstractions;
using SemantxTrace.Wpf;
using Xunit;

namespace SemantxTrace.Wpf.Tests;

public sealed class FileJsonlTraceContextTests
{
    private static JsonSchema LoadSchema()
    {
        string path = Path.Combine(AppContext.BaseDirectory, "trace-event-v1.schema.json");
        string json = File.ReadAllText(path);
        return JsonSchema.FromText(json);
    }

    private static void AssertValidSchema(JsonSchema schema, string line)
    {
        var node = JsonNode.Parse(line);
        var result = schema.Evaluate(node, new EvaluationOptions
        {
            OutputFormat = OutputFormat.Basic,
        });
        Assert.True(result.IsValid, $"JSONL line failed schema validation:\n{line}");
    }

    [Fact]
    public void EmitScreenOpened_produces_valid_jsonl()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitScreenOpened("InvoiceEditor");
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal(1, doc.RootElement.GetProperty("schema_version").GetInt32());
        Assert.Equal("ScreenOpened", doc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("InvoiceEditor", doc.RootElement.GetProperty("screen_id").GetString());
        Assert.Equal(0, doc.RootElement.GetProperty("seq").GetInt64());
    }

    [Fact]
    public void EmitCommandExecuted_success_produces_valid_jsonl()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitCommandExecuted("Invoice.Submit", durationMs: 42, CommandOutcome.Success);
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("CommandExecuted", doc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("Invoice.Submit", doc.RootElement.GetProperty("command_id").GetString());
        Assert.Equal(42, doc.RootElement.GetProperty("duration_ms").GetInt64());
        Assert.Equal("success", doc.RootElement.GetProperty("outcome").GetString());
    }

    [Fact]
    public void EmitFieldChanged_masked_produces_valid_jsonl()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitFieldChanged(
            "InvoiceEditor.Amount",
            ValuePolicy.Bucketed("0-10"),
            ValuePolicy.Bucketed("11-100"));
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("FieldChanged", doc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("bucketed", doc.RootElement.GetProperty("old").GetProperty("policy").GetString());
        Assert.Equal("bucketed", doc.RootElement.GetProperty("new").GetProperty("policy").GetString());
    }

    [Fact]
    public void EmitExceptionThrown_produces_valid_jsonl()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitExceptionThrown("System.InvalidOperationException", "***");
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("ExceptionThrown", doc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("System.InvalidOperationException", doc.RootElement.GetProperty("exception_type").GetString());
        Assert.Equal("***", doc.RootElement.GetProperty("message").GetString());
    }

    [Fact]
    public void EmitExceptionThrown_masks_caller_message_at_sink()
    {
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitExceptionThrown("System.Exception", "sensitive PII here");
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        var doc = JsonDocument.Parse(line);
        Assert.Equal("***", doc.RootElement.GetProperty("message").GetString());
        Assert.DoesNotContain("sensitive", line, StringComparison.Ordinal);
    }

    [Fact]
    public void EmitCommandExecuted_failure_includes_detail_object()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitCommandExecuted("Invoice.Submit", durationMs: 5, CommandOutcome.Failure);
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("failure", doc.RootElement.GetProperty("outcome").GetString());
        var detail = doc.RootElement.GetProperty("detail");
        Assert.Equal(JsonValueKind.Object, detail.ValueKind);
        Assert.Equal("***", detail.GetProperty("message").GetString());
    }

    [Fact]
    public void EmitAsyncOperationCompleted_failure_includes_detail_object()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitAsyncOperationCompleted("SaveDocument", durationMs: 5, CommandOutcome.Failure);
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("failure", doc.RootElement.GetProperty("outcome").GetString());
        var detail = doc.RootElement.GetProperty("detail");
        Assert.Equal(JsonValueKind.Object, detail.ValueKind);
        Assert.Equal("***", detail.GetProperty("message").GetString());
    }

    [Fact]
    public void EmitNavigationOccurred_produces_valid_jsonl()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitNavigationOccurred("InvoiceList", "InvoiceEditor");
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("NavigationOccurred", doc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("InvoiceList", doc.RootElement.GetProperty("from").GetString());
        Assert.Equal("InvoiceEditor", doc.RootElement.GetProperty("to").GetString());
    }

    [Fact]
    public void EmitValidationFailed_produces_valid_jsonl()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitValidationFailed("Required", "InvoiceEditor.Amount", "Field is required");
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("ValidationFailed", doc.RootElement.GetProperty("kind").GetString());
    }

    [Fact]
    public void EmitAsyncOperationCompleted_produces_valid_jsonl()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitAsyncOperationCompleted("SaveDocument", durationMs: 250, CommandOutcome.Success);
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        AssertValidSchema(schema, line);

        var doc = JsonDocument.Parse(line);
        Assert.Equal("AsyncOperationCompleted", doc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("success", doc.RootElement.GetProperty("outcome").GetString());
    }

    [Fact]
    public void Sequence_numbers_increment_across_events()
    {
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitScreenOpened("Screen1");
        ctx.EmitScreenOpened("Screen2");
        ctx.EmitScreenOpened("Screen3");
        ctx.Flush();

        string[] lines = sw.ToString().Split(
            new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        Assert.Equal(3, lines.Length);

        for (int i = 0; i < lines.Length; i++)
        {
            var doc = JsonDocument.Parse(lines[i]);
            Assert.Equal(i, doc.RootElement.GetProperty("seq").GetInt32());
        }
    }

    [Fact]
    public void All_events_share_same_session_id()
    {
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        Guid sessionId = ctx.SessionId;
        ctx.EmitScreenOpened("A");
        ctx.EmitScreenOpened("B");
        ctx.Flush();

        string[] lines = sw.ToString().Split(
            new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        foreach (string line in lines)
        {
            var doc = JsonDocument.Parse(line);
            Assert.Equal(sessionId.ToString("D"), doc.RootElement.GetProperty("session_id").GetString());
        }
    }

    [Fact]
    public void CorrelationId_appears_when_provided()
    {
        var correlationId = Guid.NewGuid();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitScreenOpened("X", correlationId);
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        var doc = JsonDocument.Parse(line);
        Assert.Equal(correlationId.ToString("D"), doc.RootElement.GetProperty("correlation_id").GetString());
    }

    [Fact]
    public void CorrelationId_absent_when_not_provided()
    {
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        ctx.EmitScreenOpened("X");
        ctx.Flush();

        string line = sw.ToString().TrimEnd('\r', '\n');
        var doc = JsonDocument.Parse(line);
        Assert.False(doc.RootElement.TryGetProperty("correlation_id", out _));
    }

    [Fact]
    public void ValuePolicy_all_variants_validate_against_schema()
    {
        var schema = LoadSchema();
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);

        ctx.EmitFieldChanged("f", ValuePolicy.Masked(), ValuePolicy.Raw(42));
        ctx.EmitFieldChanged("f", ValuePolicy.Bucketed("0-10"), ValuePolicy.Hashed("abc", "blake3"));
        ctx.EmitFieldChanged("f", ValuePolicy.Removed(), ValuePolicy.DefaultMasked());
        ctx.Flush();

        string[] lines = sw.ToString().Split(
            new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        Assert.Equal(3, lines.Length);
        foreach (string line in lines)
            AssertValidSchema(schema, line);
    }
}
