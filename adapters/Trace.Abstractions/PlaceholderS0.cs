namespace SemantxTrace.Abstractions;

/// <summary>
/// S0 placeholder. The real contract surface (<c>ITraceContext</c>,
/// <c>ValuePolicy</c>, <c>[TraceCommand]</c>, <c>[ScreenId]</c>,
/// <c>[TraceField]</c>) lands in S6 alongside the WPF adapter.
/// </summary>
public static class PlaceholderS0
{
    /// <summary>Returns a stable marker string proving the assembly built.</summary>
    public static string Marker() => "Trace.Abstractions (S0 placeholder)";
}
