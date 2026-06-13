using System;
using Avalonia.Controls;
using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.ViewModels;
using SemantxTrace.Abstractions;

namespace DeclarationApp.Demo.Avalonia.Views;

[SemantxTrace.Abstractions.ScreenId("InvoiceList")]
public partial class InvoiceListPage : UserControl
{
    public InvoiceListPage(ITraceContext trace, Action<Control> navigate)
    {
        InitializeComponent();
        DataContext = new InvoiceListViewModel(trace, navigate);
        trace.EmitScreenOpened("InvoiceList");
    }
}
