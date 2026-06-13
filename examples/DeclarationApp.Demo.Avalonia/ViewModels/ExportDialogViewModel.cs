using System;
using System.Collections.Generic;
using System.Windows.Input;

using DeclarationApp.Demo.Avalonia.Models;

using SemantxTrace.Abstractions;
using SemantxTrace.Avalonia;

namespace DeclarationApp.Demo.Avalonia.ViewModels;

/// <summary>
/// ViewModel for <see cref="Views.ExportDialogPage"/>.
///
/// BUG 2: <see cref="ValidateIinCommand"/> emits
/// <c>CommandExecuted("Export.ValidateIin", Success)</c> and opens the
/// <c>DuplicateIinErrorModal</c> screen when the IIN is a duplicate, but
/// forgets to set <c>CanSubmit = false</c> so the Export button stays enabled.
/// The <c>CommandExecuted</c> outcome also incorrectly says <c>Success</c>
/// instead of <c>Failure</c>, hiding the problem from dashboards.
/// </summary>
public class ExportDialogViewModel : ViewModelBase
{
    private readonly ITraceContext _trace;
    private readonly Declaration _declaration;

    private static readonly HashSet<string> KnownDuplicateIins = new(StringComparer.Ordinal)
    {
        "123456789012",
        "555000111222",
    };

    public ExportDialogViewModel(ITraceContext trace, Declaration declaration)
    {
        _trace = trace ?? throw new ArgumentNullException(nameof(trace));
        _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

        _exporterIin = declaration.ExporterIin ?? string.Empty;
        _canSubmit   = true;

        var submitInner = new RelayCommand(ExecuteSubmit, _ => _canSubmit);
        ValidateIinCommand = new TracedRelayCommand(new RelayCommand(ExecuteValidateIin), "Export.ValidateIin",    _trace);
        SubmitCommand      = new TracedRelayCommand(submitInner,                          "Export.Submit",         _trace);
        DismissModalCommand = new TracedRelayCommand(new RelayCommand(ExecuteDismissModal), "Modal.Dismiss",       _trace);
    }

    private string _exporterIin;
    [TraceField("ExportDialog.ExporterIin", Policy = ValuePolicyKind.Hashed)]
    public string ExporterIin
    {
        get => _exporterIin;
        set
        {
            if (SetProperty(ref _exporterIin, value))
                _trace.EmitFieldChanged("ExportDialog.ExporterIin", ValuePolicy.Masked("***"), ValuePolicy.Masked("***"));
        }
    }

    private bool _canSubmit;
    public bool CanSubmit
    {
        get => _canSubmit;
        set => SetProperty(ref _canSubmit, value);
    }

    private string? _validationMessage;
    public string? ValidationMessage
    {
        get => _validationMessage;
        set => SetProperty(ref _validationMessage, value);
    }

    private string? _exportStatus;
    public string? ExportStatus
    {
        get => _exportStatus;
        set => SetProperty(ref _exportStatus, value);
    }

    private bool _isDuplicateModalVisible;
    public bool IsDuplicateModalVisible
    {
        get => _isDuplicateModalVisible;
        set => SetProperty(ref _isDuplicateModalVisible, value);
    }

    [TraceCommand("Export.ValidateIin")]
    public ICommand ValidateIinCommand { get; }

    [TraceCommand("Export.Submit")]
    public ICommand SubmitCommand { get; }

    [TraceCommand("Modal.Dismiss")]
    public ICommand DismissModalCommand { get; }

    private void ExecuteValidateIin(object? _)
    {
        bool isDuplicate = KnownDuplicateIins.Contains(_exporterIin);
        if (isDuplicate)
        {
            _trace.EmitScreenOpened("DuplicateIinErrorModal");
            IsDuplicateModalVisible = true;
            ValidationMessage = "ERROR: Duplicate IIN detected. Export blocked.";

            // BUG 2: CanSubmit not updated — Export button stays enabled.
            // Correct fix would be: CanSubmit = false; ((RelayCommand)SubmitCommand).RaiseCanExecuteChanged();
        }
        else
        {
            ValidationMessage = "IIN validated OK.";
            CanSubmit = true;
        }
    }

    private void ExecuteDismissModal(object? _)
    {
        IsDuplicateModalVisible = false;
    }

    private void ExecuteSubmit(object? _)
    {
        ExportStatus = $"Declaration {_declaration.Number} submitted for export at {DateTime.UtcNow:u}.";
        _trace.EmitNavigationOccurred("ExportDialog", "ExportComplete");
    }
}
