using System;
using System.Windows.Input;

using Avalonia.Controls;

using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.Views;

using SemantxTrace.Abstractions;
using SemantxTrace.Avalonia;

namespace DeclarationApp.Demo.Avalonia.ViewModels;

/// <summary>Thrown when the computed duty is negative (data-entry error).</summary>
public class NegativePaymentException : Exception
{
    public NegativePaymentException(string message) : base(message) { }
    public NegativePaymentException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>
/// ViewModel for <see cref="PaymentsCalculatorPage"/>.
///
/// BUG 1: <see cref="RecalculateCommand"/> uses integer division
/// <c>(TotalQuantity / 1000) * 1000</c>, silently truncating quantities
/// in (1000, 2000) by up to 999 units.  If the result goes negative
/// (possible when <see cref="DutyRate"/> is negative), a
/// <see cref="NegativePaymentException"/> is thrown.
/// </summary>
public class PaymentsCalculatorViewModel : ViewModelBase
{
    private readonly ITraceContext _trace;
    private readonly Action<Control> _navigate;
    private readonly Declaration _declaration;

    public PaymentsCalculatorViewModel(ITraceContext trace, Action<Control> navigate, Declaration declaration)
    {
        _trace = trace ?? throw new ArgumentNullException(nameof(trace));
        _navigate = navigate ?? throw new ArgumentNullException(nameof(navigate));
        _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

        foreach (var item in declaration.Goods)
            _totalQuantity += item.Quantity;

        _dutyRate = 0.15m;

        RecalculateCommand      = new TracedRelayCommand(new RelayCommand(ExecuteRecalculate),    "Graph47.Recalculate",             _trace);
        NavigateToExportCommand = new TracedRelayCommand(new RelayCommand(ExecuteNavigateToExport), "PaymentsCalculator.GoToExport", _trace);
    }

    private int _totalQuantity;
    public int TotalQuantity
    {
        get => _totalQuantity;
        set => SetProperty(ref _totalQuantity, value);
    }

    private decimal _dutyRate;
    public decimal DutyRate
    {
        get => _dutyRate;
        set => SetProperty(ref _dutyRate, value);
    }

    private decimal _dutyAmount;
    public decimal DutyAmount
    {
        get => _dutyAmount;
        set => SetProperty(ref _dutyAmount, value);
    }

    private string? _calculationStatus;
    public string? CalculationStatus
    {
        get => _calculationStatus;
        set => SetProperty(ref _calculationStatus, value);
    }

    [TraceCommand("Graph47.Recalculate")]
    public ICommand RecalculateCommand { get; }

    [TraceCommand("PaymentsCalculator.GoToExport")]
    public ICommand NavigateToExportCommand { get; }

    private void ExecuteRecalculate(object? _)
    {
        // BUG 1: integer division truncates quantity > 1000.
        int effectiveQuantity = (TotalQuantity / 1000) * 1000;
        decimal dutyAmount = effectiveQuantity * DutyRate;

        if (dutyAmount < 0)
            throw new NegativePaymentException(
                $"Computed duty {dutyAmount:F2} is negative; check quantity and rate.");

        DutyAmount = dutyAmount;
        _declaration.TotalDuty = dutyAmount;
        CalculationStatus = $"Duty: {dutyAmount:C} (effective qty: {effectiveQuantity})";
    }

    private void ExecuteNavigateToExport(object? _)
    {
        _trace.EmitNavigationOccurred("PaymentsCalculator", "ExportDialog");
        _navigate(new ExportDialogPage(_declaration, _trace));
    }
}
