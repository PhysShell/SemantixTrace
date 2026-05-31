using System;
using SemantxTrace.Abstractions;
using Xunit;

namespace SemantxTrace.Abstractions.Tests;

public sealed class AttributeTests
{
    [Fact]
    public void TraceCommandAttribute_stores_command_id()
    {
        var attr = new TraceCommandAttribute("Invoice.Submit");
        Assert.Equal("Invoice.Submit", attr.CommandId);
    }

    [Fact]
    public void TraceCommandAttribute_throws_for_null()
    {
        Assert.Throws<ArgumentNullException>(() => new TraceCommandAttribute(null!));
    }

    [Fact]
    public void ScreenIdAttribute_stores_screen_id()
    {
        var attr = new ScreenIdAttribute("InvoiceEditor");
        Assert.Equal("InvoiceEditor", attr.ScreenId);
    }

    [Fact]
    public void ScreenIdAttribute_throws_for_null()
    {
        Assert.Throws<ArgumentNullException>(() => new ScreenIdAttribute(null!));
    }

    [Fact]
    public void TraceFieldAttribute_default_policy_is_masked()
    {
        var attr = new TraceFieldAttribute("InvoiceEditor.Amount");
        Assert.Equal(ValuePolicyKind.Masked, attr.Policy);
    }

    [Fact]
    public void TraceFieldAttribute_policy_settable()
    {
        var attr = new TraceFieldAttribute("InvoiceEditor.Amount")
        {
            Policy = ValuePolicyKind.Raw,
        };
        Assert.Equal(ValuePolicyKind.Raw, attr.Policy);
    }

    [Fact]
    public void TraceFieldAttribute_throws_for_null_field_id()
    {
        Assert.Throws<ArgumentNullException>(() => new TraceFieldAttribute(null!));
    }
}
