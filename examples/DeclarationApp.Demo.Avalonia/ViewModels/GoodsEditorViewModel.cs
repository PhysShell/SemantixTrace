using System;
using System.Diagnostics;
using System.Threading.Tasks;
using System.Windows.Input;

using Avalonia.Controls;
using Avalonia.Threading;

using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.Views;

using SemantxTrace.Abstractions;
using SemantxTrace.Avalonia;

namespace DeclarationApp.Demo.Avalonia.ViewModels;

/// <summary>
/// ViewModel for <see cref="GoodsEditorPage"/>.
///
/// BUG 3: <see cref="SubmitCommand"/> fires before <see cref="AsyncCustomsCodeValidator"/>
/// completes, so a <c>ValidationFailed</c> event may arrive AFTER the
/// <c>CommandExecuted</c> event — an out-of-order signal the analyser flags as a race.
/// </summary>
public class GoodsEditorViewModel : ViewModelBase
{
    private readonly ITraceContext _trace;
    private readonly Action<Control> _navigate;
    private readonly Declaration _declaration;

#pragma warning disable CS0414
    private Task _pendingValidationTask = Task.CompletedTask;
#pragma warning restore CS0414

    public GoodsEditorViewModel(ITraceContext trace, Action<Control> navigate, Declaration declaration)
    {
        _trace = trace ?? throw new ArgumentNullException(nameof(trace));
        _navigate = navigate ?? throw new ArgumentNullException(nameof(navigate));
        _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

        var first = declaration.Goods.Count > 0 ? declaration.Goods[0] : null;
        _customsCode = first?.CustomsCode ?? string.Empty;
        _quantity    = first?.Quantity ?? 1;
        _unitPrice   = first?.UnitPrice ?? 0m;

        SubmitCommand = new TracedRelayCommand(new RelayCommand(ExecuteSubmit), "GoodsDeclaration.Submit", _trace);
    }

    private string _customsCode;
    public string CustomsCode
    {
        get => _customsCode;
        set
        {
            if (SetProperty(ref _customsCode, value))
                _pendingValidationTask = Task.Run(() => AsyncCustomsCodeValidator(value));
        }
    }

    private int _quantity;
    public int Quantity
    {
        get => _quantity;
        set => SetProperty(ref _quantity, value);
    }

    private decimal _unitPrice;
    public decimal UnitPrice
    {
        get => _unitPrice;
        set => SetProperty(ref _unitPrice, value);
    }

    private string? _validationStatus;
    public string? ValidationStatus
    {
        get => _validationStatus;
        set => SetProperty(ref _validationStatus, value);
    }

    [TraceCommand("GoodsDeclaration.Submit")]
    public ICommand SubmitCommand { get; }

    private void AsyncCustomsCodeValidator(string code)
    {
        var sw = Stopwatch.StartNew();
        System.Threading.Thread.Sleep(200);
        bool isValid = code.Length == 7 && code[4] == '.';
        sw.Stop();

        if (!isValid)
        {
            // BUG 3: this ValidationFailed may arrive AFTER the submit CommandExecuted.
            _trace.EmitValidationFailed("AsyncCustomsCodeValidator", "GoodsEditor.CustomsCode", "***");
        }

        Dispatcher.UIThread.Post(() =>
        {
            ValidationStatus = isValid ? "Customs code: OK" : "Customs code: INVALID (must be HS format NNNN.NN)";
        });
    }

    private void ExecuteSubmit(object? _)
    {
        // BUG 3: we do NOT await _pendingValidationTask — TracedRelayCommand will emit
        // CommandExecuted immediately, but AsyncCustomsCodeValidator may still be running.
        var item = new GoodsItem
        {
            Id          = Guid.NewGuid(),
            CustomsCode = _customsCode,
            Quantity    = _quantity,
            UnitPrice   = _unitPrice,
        };

        if (_declaration.Goods.Count > 0)
            _declaration.Goods[0] = item;
        else
            _declaration.Goods.Add(item);

        _trace.EmitNavigationOccurred("GoodsEditor", "PaymentsCalculator");
        _navigate(new PaymentsCalculatorPage(_declaration, _trace, _navigate));
    }
}
