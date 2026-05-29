using System;
using System.IO;
using System.Text.Json;
using System.Windows.Input;
using SemantxTrace.Abstractions;
using SemantxTrace.Wpf;
using Xunit;

namespace SemantxTrace.Wpf.Tests;

public sealed class TracedRelayCommandTests
{
    private sealed class SimpleCommand : ICommand
    {
        private readonly Action<object?> _execute;
        private readonly Func<object?, bool> _canExecute;

        public SimpleCommand(Action<object?> execute, Func<object?, bool>? canExecute = null)
        {
            _execute = execute;
            _canExecute = canExecute ?? (_ => true);
        }

        public bool CanExecute(object? parameter) => _canExecute(parameter);
        public void Execute(object? parameter) => _execute(parameter);
        public event EventHandler? CanExecuteChanged;

        public void RaiseCanExecuteChanged() =>
            CanExecuteChanged?.Invoke(this, EventArgs.Empty);
    }

    [Fact]
    public void Execute_emits_command_executed_with_success()
    {
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        var inner = new SimpleCommand(_ => { });
        var cmd = new TracedRelayCommand(inner, "Test.Command", ctx);

        cmd.Execute(null);
        ctx.Flush();

        string[] lines = sw.ToString().Split(
            new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        Assert.Single(lines);

        var doc = JsonDocument.Parse(lines[0]);
        Assert.Equal("CommandExecuted", doc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("Test.Command", doc.RootElement.GetProperty("command_id").GetString());
        Assert.Equal("success", doc.RootElement.GetProperty("outcome").GetString());
    }

    [Fact]
    public void Execute_emits_exception_thrown_then_failure_and_rethrows()
    {
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        var inner = new SimpleCommand(_ => throw new InvalidOperationException("secret"));
        var cmd = new TracedRelayCommand(inner, "Test.Boom", ctx);

        Assert.Throws<InvalidOperationException>(() => cmd.Execute(null));
        ctx.Flush();

        string[] lines = sw.ToString().Split(
            new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        Assert.Equal(2, lines.Length);

        var exDoc = JsonDocument.Parse(lines[0]);
        Assert.Equal("ExceptionThrown", exDoc.RootElement.GetProperty("kind").GetString());
        // Message must be masked — the original "secret" must not appear.
        Assert.Equal("***", exDoc.RootElement.GetProperty("message").GetString());

        var cmdDoc = JsonDocument.Parse(lines[1]);
        Assert.Equal("CommandExecuted", cmdDoc.RootElement.GetProperty("kind").GetString());
        Assert.Equal("failure", cmdDoc.RootElement.GetProperty("outcome").GetString());
    }

    [Fact]
    public void ExceptionThrown_and_CommandExecuted_share_correlation_id()
    {
        using var sw = new StringWriter();
        using var ctx = new FileJsonlTraceContext(sw);
        var inner = new SimpleCommand(_ => throw new InvalidOperationException());
        var cmd = new TracedRelayCommand(inner, "Test.Boom", ctx);

        Assert.Throws<InvalidOperationException>(() => cmd.Execute(null));
        ctx.Flush();

        string[] lines = sw.ToString().Split(
            new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries);
        var exDoc = JsonDocument.Parse(lines[0]);
        var cmdDoc = JsonDocument.Parse(lines[1]);

        string exCorr = exDoc.RootElement.GetProperty("correlation_id").GetString()!;
        string cmdCorr = cmdDoc.RootElement.GetProperty("correlation_id").GetString()!;
        Assert.Equal(exCorr, cmdCorr);
        Assert.NotEmpty(exCorr);
    }

    [Fact]
    public void CanExecute_delegates_to_inner()
    {
        using var ctx = NoOpTraceContext.Instance;
        var inner = new SimpleCommand(_ => { }, _ => false);
        var cmd = new TracedRelayCommand(inner, "X", ctx);
        Assert.False(cmd.CanExecute(null));
    }

    [Fact]
    public void Constructor_throws_for_null_inner()
    {
        Assert.Throws<ArgumentNullException>(() =>
            new TracedRelayCommand(null!, "X", NoOpTraceContext.Instance));
    }

    [Fact]
    public void Constructor_throws_for_null_command_id()
    {
        using var ctx = NoOpTraceContext.Instance;
        var inner = new SimpleCommand(_ => { });
        Assert.Throws<ArgumentNullException>(() =>
            new TracedRelayCommand(inner, null!, ctx));
    }

    [Fact]
    public void Constructor_throws_for_null_context()
    {
        var inner = new SimpleCommand(_ => { });
        Assert.Throws<ArgumentNullException>(() =>
            new TracedRelayCommand(inner, "X", null!));
    }
}
