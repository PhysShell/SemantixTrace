namespace SemantxTrace.Abstractions;

/// <summary>
/// Privacy-policy selector used by <see cref="TraceFieldAttribute"/> to
/// override the default policy for a specific field (ADR-0007).
/// </summary>
public enum ValuePolicyKind
{
    /// <summary>
    /// Replace the value with a fixed display mask (default for strings).
    /// </summary>
    Masked = 0,

    /// <summary>
    /// Record the raw value. Requires an explicit opt-in audit entry.
    /// </summary>
    Raw = 1,

    /// <summary>
    /// Record a categorical bucket label (default for numerics).
    /// </summary>
    Bucketed = 2,

    /// <summary>
    /// Record an identity-preserving keyed hash (for PII identifiers).
    /// </summary>
    Hashed = 3,

    /// <summary>Omit the field from the emitted event entirely.</summary>
    Removed = 4,
}
