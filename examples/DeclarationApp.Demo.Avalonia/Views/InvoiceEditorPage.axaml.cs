using System;
using Avalonia.Controls;
using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.ViewModels;
using SemantxTrace.Abstractions;

namespace DeclarationApp.Demo.Avalonia.Views;

[ScreenId("InvoiceEditor")]
public partial class InvoiceEditorPage : UserControl
{
    public InvoiceEditorPage(Declaration declaration, ITraceContext trace, Action<Control> navigate)
    {
        InitializeComponent();
        DataContext = new InvoiceEditorViewModel(trace, navigate, declaration);
        trace.EmitScreenOpened("InvoiceEditor");
    }
}
