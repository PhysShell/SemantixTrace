using System;

namespace SemantxTrace.Abstractions;

/// <summary>
/// Marks a bound property on a ViewModel as a traceable field and optionally
/// overrides the default privacy policy (ADR-0007).
/// </summary>
/// <remarks>
/// <para>
/// When <see cref="Policy"/> is not set, the adapter applies the
/// privacy-by-default rules: <see cref="ValuePolicyKind.Masked"/> for
/// <c>string</c> properties and <see cref="ValuePolicyKind.Bucketed"/> for
/// numeric properties.
/// </para>
/// <para>
/// Setting <see cref="Policy"/> to <see cref="ValuePolicyKind.Raw"/> requires
/// an explicit reviewer sign-off (documented in
/// <c>docs/privacy.md</c> §"Opt-in audit").
/// </para>
/// </remarks>
[AttributeUsage(AttributeTargets.Property, Inherited = false, AllowMultiple = false)]
public sealed class TraceFieldAttribute : Attribute
{
    /// <summary>The stable semantic field identifier.</summary>
    public string FieldId { get; }

    /// <summary>
    /// Privacy policy override.  Defaults to
    /// <see cref="ValuePolicyKind.Masked"/> (the project-wide
    /// privacy-by-default posture per ADR-0007).
    /// </summary>
    public ValuePolicyKind Policy { get; set; } = ValuePolicyKind.Masked;

    /// <summary>Initialises a new <see cref="TraceFieldAttribute"/>.</summary>
    /// <param name="fieldId">
    /// The stable dot-delimited semantic field identifier
    /// (e.g. <c>"DeclarationEditor.Amount"</c>).
    /// </param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="fieldId"/> is <see langword="null"/>.
    /// </exception>
    public TraceFieldAttribute(string fieldId)
    {
        FieldId = fieldId ?? throw new ArgumentNullException(nameof(fieldId));
    }
}
