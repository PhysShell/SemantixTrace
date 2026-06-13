using System;
using System.Windows.Input;

using Avalonia.Controls;

using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.Views;

using SemantxTrace.Abstractions;
using SemantxTrace.Avalonia;

namespace DeclarationApp.Demo.Avalonia.ViewModels;

/// <summary>
/// ViewModel for <see cref="InvoiceEditorPage"/>.
/// ExporterIin and DeclarationNumber are masked in trace events (ADR-0007).
/// </summary>
public class InvoiceEditorViewModel : ViewModelBase
{
    private readonly ITraceContext _trace;
    private readonly Action<Control> _navigate;
    private readonly Declaration _declaration;

    public InvoiceEditorViewModel(ITraceContext trace, Action<Control> navigate, Declaration declaration)
    {
        _trace = trace ?? throw new ArgumentNullException(nameof(trace));
        _navigate = navigate ?? throw new ArgumentNullException(nameof(navigate));
        _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

        _declarationNumber = declaration.Number ?? string.Empty;
        _exporterIin       = declaration.ExporterIin ?? string.Empty;

        SaveCommand            = new TracedRelayCommand(new RelayCommand(ExecuteSave),          "Declaration.Save",     _trace);
        NavigateToGoodsCommand = new TracedRelayCommand(new RelayCommand(ExecuteNavigateToGoods), "Declaration.GoToGoods", _trace);
    }

    private string _declarationNumber;
    [TraceField("InvoiceEditor.DeclarationNumber", Policy = ValuePolicyKind.Masked)]
    public string DeclarationNumber
    {
        get => _declarationNumber;
        set
        {
            if (SetProperty(ref _declarationNumber, value))
                _trace.EmitFieldChanged("InvoiceEditor.DeclarationNumber", ValuePolicy.Masked("***"), ValuePolicy.Masked("***"));
        }
    }

    private string _exporterIin;
    [TraceField("InvoiceEditor.ExporterIin", Policy = ValuePolicyKind.Hashed)]
    public string ExporterIin
    {
        get => _exporterIin;
        set
        {
            if (SetProperty(ref _exporterIin, value))
                _trace.EmitFieldChanged("InvoiceEditor.ExporterIin", ValuePolicy.Masked("***"), ValuePolicy.Masked("***"));
        }
    }

    [TraceCommand("Declaration.Save")]
    public ICommand SaveCommand { get; }

    [TraceCommand("Declaration.GoToGoods")]
    public ICommand NavigateToGoodsCommand { get; }

    private void ExecuteSave(object? _)
    {
        _declaration.Number      = _declarationNumber;
        _declaration.ExporterIin = _exporterIin;
    }

    private void ExecuteNavigateToGoods(object? _)
    {
        _declaration.Number      = _declarationNumber;
        _declaration.ExporterIin = _exporterIin;
        _trace.EmitNavigationOccurred("InvoiceEditor", "GoodsEditor");
        _navigate(new GoodsEditorPage(_declaration, _trace, _navigate));
    }
}
