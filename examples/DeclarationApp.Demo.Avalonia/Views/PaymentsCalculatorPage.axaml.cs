using System;
using Avalonia.Controls;
using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.ViewModels;
using SemantxTrace.Abstractions;

namespace DeclarationApp.Demo.Avalonia.Views;

[ScreenId("PaymentsCalculator")]
public partial class PaymentsCalculatorPage : UserControl
{
    public PaymentsCalculatorPage(Declaration declaration, ITraceContext trace, Action<Control> navigate)
    {
        InitializeComponent();
        DataContext = new PaymentsCalculatorViewModel(trace, navigate, declaration);
        trace.EmitScreenOpened("PaymentsCalculator");
    }
}
