// Privacy-aware wrapper for recorded field values (ADR-0007).
// Mask-by-default: strings → Masked("***"), numerics → Bucketed.
// Raw requires an explicit [TraceField(Policy = ValuePolicyKind.Raw)] opt-in.

using System;

namespace SemantxTrace.Abstractions;

/// <summary>
/// Discriminated union representing the privacy treatment applied to a
/// recorded field value (ADR-0007).
/// </summary>
/// <remarks>
/// The default for free-text strings is <see cref="MaskedValuePolicy"/> with
/// <c>display = "***"</c>; the default for numerics is
/// <see cref="BucketedValuePolicy"/>. <see cref="RawValuePolicy"/> requires
/// an explicit <see cref="TraceFieldAttribute"/> opt-in.
/// </remarks>
public abstract class ValuePolicy
{
    // Prevents external subclassing while allowing the sealed subclasses
    // within this assembly to derive from it.
    private protected ValuePolicy() { }

    /// <summary>
    /// Returns a <see cref="MaskedValuePolicy"/> with <paramref name="display"/>
    /// as the display mask.
    /// </summary>
    /// <param name="display">The display mask string (e.g. <c>"***"</c>).</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="display"/> is <see langword="null"/>.
    /// </exception>
    public static ValuePolicy Masked(string display = "***")
        => new MaskedValuePolicy(display ?? throw new ArgumentNullException(nameof(display)));

    /// <summary>
    /// Returns a <see cref="RawValuePolicy"/> wrapping <paramref name="value"/>.
    /// </summary>
    /// <param name="value">The raw value to record.</param>
    public static ValuePolicy Raw(object? value) => new RawValuePolicy(value);

    /// <summary>
    /// Returns a <see cref="BucketedValuePolicy"/> with <paramref name="bucket"/>
    /// as the bucket label.
    /// </summary>
    /// <param name="bucket">The bucket label (e.g. <c>"11-100"</c>).</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="bucket"/> is <see langword="null"/>.
    /// </exception>
    public static ValuePolicy Bucketed(string bucket)
        => new BucketedValuePolicy(bucket ?? throw new ArgumentNullException(nameof(bucket)));

    /// <summary>Returns a <see cref="HashedValuePolicy"/>.</summary>
    /// <param name="hash">The hex-encoded hash digest.</param>
    /// <param name="algo">The hash algorithm name (e.g. <c>"blake3"</c>).</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="hash"/> or <paramref name="algo"/> is <see langword="null"/>.
    /// </exception>
    public static ValuePolicy Hashed(string hash, string algo)
        => new HashedValuePolicy(
            hash ?? throw new ArgumentNullException(nameof(hash)),
            algo ?? throw new ArgumentNullException(nameof(algo)));

    /// <summary>Returns the <see cref="RemovedValuePolicy"/> singleton.</summary>
    public static ValuePolicy Removed() => RemovedValuePolicy.Instance;

    /// <summary>Returns the canonical default masked string (<c>"***"</c>).</summary>
    public static ValuePolicy DefaultMasked() => new MaskedValuePolicy("***");
}

/// <summary>
/// The masked variant of <see cref="ValuePolicy"/>: free-text strings are
/// replaced with a display mask (default <c>"***"</c>).
/// </summary>
public sealed class MaskedValuePolicy : ValuePolicy
{
    /// <summary>The display mask string.</summary>
    public string Display { get; }

    /// <summary>Initialises a new <see cref="MaskedValuePolicy"/>.</summary>
    /// <param name="display">The display mask.</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="display"/> is <see langword="null"/>.
    /// </exception>
    public MaskedValuePolicy(string display)
    {
        Display = display ?? throw new ArgumentNullException(nameof(display));
    }
}

/// <summary>
/// The raw (unmasked) variant of <see cref="ValuePolicy"/>.
/// Use only when <c>[TraceField(Policy = ValuePolicyKind.Raw)]</c> opts in.
/// </summary>
public sealed class RawValuePolicy : ValuePolicy
{
    /// <summary>The raw value.</summary>
    public object? Value { get; }

    /// <summary>Initialises a new <see cref="RawValuePolicy"/>.</summary>
    /// <param name="value">The raw value to record.</param>
    public RawValuePolicy(object? value) => Value = value;
}

/// <summary>
/// The bucketed variant of <see cref="ValuePolicy"/>: numeric values are
/// recorded as a categorical bucket label (e.g. <c>"11-100"</c>).
/// </summary>
public sealed class BucketedValuePolicy : ValuePolicy
{
    /// <summary>The bucket label.</summary>
    public string Bucket { get; }

    /// <summary>Initialises a new <see cref="BucketedValuePolicy"/>.</summary>
    /// <param name="bucket">The bucket label.</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="bucket"/> is <see langword="null"/>.
    /// </exception>
    public BucketedValuePolicy(string bucket)
    {
        Bucket = bucket ?? throw new ArgumentNullException(nameof(bucket));
    }
}

/// <summary>
/// The hashed variant of <see cref="ValuePolicy"/>: an identity-preserving
/// keyed hash for PII identifiers (e-mail, phone, IBAN, IIN/BIN).
/// </summary>
public sealed class HashedValuePolicy : ValuePolicy
{
    /// <summary>Hex-encoded hash digest.</summary>
    public string Hash { get; }

    /// <summary>Hash algorithm name (e.g. <c>"blake3"</c>).</summary>
    public string Algo { get; }

    /// <summary>Initialises a new <see cref="HashedValuePolicy"/>.</summary>
    /// <param name="hash">The hex-encoded hash digest.</param>
    /// <param name="algo">The hash algorithm name.</param>
    /// <exception cref="ArgumentNullException">
    ///   <paramref name="hash"/> or <paramref name="algo"/> is <see langword="null"/>.
    /// </exception>
    public HashedValuePolicy(string hash, string algo)
    {
        Hash = hash ?? throw new ArgumentNullException(nameof(hash));
        Algo = algo ?? throw new ArgumentNullException(nameof(algo));
    }
}

/// <summary>
/// The removed variant of <see cref="ValuePolicy"/>: the field is omitted
/// from the emitted event entirely.
/// </summary>
public sealed class RemovedValuePolicy : ValuePolicy
{
    /// <summary>The singleton instance.</summary>
    public static RemovedValuePolicy Instance { get; } = new RemovedValuePolicy();

    private RemovedValuePolicy() { }
}
