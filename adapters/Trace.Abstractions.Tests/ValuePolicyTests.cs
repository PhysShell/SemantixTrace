using System;
using SemantxTrace.Abstractions;
using Xunit;

namespace SemantxTrace.Abstractions.Tests;

public sealed class ValuePolicyTests
{
    [Fact]
    public void Masked_returns_MaskedValuePolicy_with_correct_display()
    {
        var p = ValuePolicy.Masked("***");
        var masked = Assert.IsType<MaskedValuePolicy>(p);
        Assert.Equal("***", masked.Display);
    }

    [Fact]
    public void Masked_default_uses_triple_star()
    {
        var p = ValuePolicy.Masked();
        var masked = Assert.IsType<MaskedValuePolicy>(p);
        Assert.Equal("***", masked.Display);
    }

    [Fact]
    public void DefaultMasked_returns_triple_star()
    {
        var p = ValuePolicy.DefaultMasked();
        var masked = Assert.IsType<MaskedValuePolicy>(p);
        Assert.Equal("***", masked.Display);
    }

    [Fact]
    public void Raw_returns_RawValuePolicy_with_value()
    {
        var p = ValuePolicy.Raw(42);
        var raw = Assert.IsType<RawValuePolicy>(p);
        Assert.Equal(42, raw.Value);
    }

    [Fact]
    public void Raw_allows_null_value()
    {
        var p = ValuePolicy.Raw(null);
        var raw = Assert.IsType<RawValuePolicy>(p);
        Assert.Null(raw.Value);
    }

    [Fact]
    public void Bucketed_returns_BucketedValuePolicy_with_bucket()
    {
        var p = ValuePolicy.Bucketed("11-100");
        var b = Assert.IsType<BucketedValuePolicy>(p);
        Assert.Equal("11-100", b.Bucket);
    }

    [Fact]
    public void Hashed_returns_HashedValuePolicy()
    {
        var p = ValuePolicy.Hashed("deadbeef", "blake3");
        var h = Assert.IsType<HashedValuePolicy>(p);
        Assert.Equal("deadbeef", h.Hash);
        Assert.Equal("blake3", h.Algo);
    }

    [Fact]
    public void Removed_returns_singleton()
    {
        var a = ValuePolicy.Removed();
        var b = ValuePolicy.Removed();
        Assert.Same(a, b);
        Assert.IsType<RemovedValuePolicy>(a);
    }

    [Fact]
    public void RemovedValuePolicy_Instance_is_singleton()
    {
        Assert.Same(RemovedValuePolicy.Instance, RemovedValuePolicy.Instance);
    }

    [Fact]
    public void Masked_throws_for_null_display()
    {
        Assert.Throws<ArgumentNullException>(() => ValuePolicy.Masked(null!));
    }

    [Fact]
    public void Bucketed_throws_for_null_bucket()
    {
        Assert.Throws<ArgumentNullException>(() => ValuePolicy.Bucketed(null!));
    }

    [Fact]
    public void Hashed_throws_for_null_hash()
    {
        Assert.Throws<ArgumentNullException>(() => ValuePolicy.Hashed(null!, "blake3"));
    }

    [Fact]
    public void Hashed_throws_for_null_algo()
    {
        Assert.Throws<ArgumentNullException>(() => ValuePolicy.Hashed("deadbeef", null!));
    }
}
