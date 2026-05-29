namespace SemantxTrace.Abstractions;

/// <summary>
/// Domain outcome for a <c>CommandExecuted</c> or <c>AsyncOperationCompleted</c>
/// trace event. Maps to the JSON Schema <c>outcome</c> enum (ADR-0005).
/// </summary>
public enum CommandOutcome
{
    /// <summary>The command completed without error.</summary>
    Success = 0,

    /// <summary>The command completed with a domain or infrastructure error.</summary>
    Failure = 1,

    /// <summary>The user or system cancelled the command before completion.</summary>
    Cancelled = 2,

    /// <summary>The command exceeded its allowed execution window.</summary>
    TimedOut = 3,
}
