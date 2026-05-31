using System;
using System.Collections.Generic;
using System.Windows.Input;

using SemantxTrace.Abstractions;
using SemantxTrace.Wpf;

using DeclarationApp.Demo.Models;

namespace DeclarationApp.Demo.ViewModels
{
    /// <summary>
    /// ViewModel for <see cref="Views.ExportDialogPage"/>.
    ///
    /// BUG 2: <see cref="ValidateIinCommand"/> emits a
    /// <c>CommandExecuted("Export.ValidateIin", Success)</c> event and then
    /// opens the <c>DuplicateIinErrorModal</c> screen if the IIN is a duplicate.
    /// However, it forgets to set <c>CanSubmit = false</c> and does NOT call
    /// <c>CommandManager.InvalidateRequerySuggested()</c>, so the Export button
    /// remains enabled even though the IIN failed validation.  The
    /// <c>CommandExecuted</c> outcome also incorrectly says <c>Success</c>
    /// instead of <c>Failure</c>, hiding the problem from dashboards.
    /// </summary>
    public class ExportDialogViewModel : ViewModelBase
    {
        private readonly ITraceContext _trace;
        private readonly Declaration _declaration;

        // Simulated "database" of already-used IINs for the duplicate check.
        private static readonly HashSet<string> KnownDuplicateIins = new HashSet<string>(StringComparer.Ordinal)
        {
            "123456789012",  // Seed declaration #1 — will trigger BUG 2.
            "555000111222",  // Seed declaration #3 — also a duplicate.
        };

        public ExportDialogViewModel(ITraceContext trace, Declaration declaration)
        {
            _trace = trace ?? throw new ArgumentNullException(nameof(trace));
            _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

            _exporterIin = declaration.ExporterIin ?? string.Empty;
            _canSubmit   = true;

            var validateInner = new RelayCommand(ExecuteValidateIin);
            var submitInner   = new RelayCommand(ExecuteSubmit, CanExecuteSubmit);

            ValidateIinCommand = new TracedRelayCommand(validateInner, "Export.ValidateIin", _trace);
            SubmitCommand      = new TracedRelayCommand(submitInner,   "Export.Submit",      _trace);
        }

        // ----------------------------------------------------------------
        // Bound properties
        // ----------------------------------------------------------------

        private string _exporterIin;

        [TraceField("ExportDialog.ExporterIin", Policy = ValuePolicyKind.Hashed)]
        public string ExporterIin
        {
            get => _exporterIin;
            set
            {
                if (SetProperty(ref _exporterIin, value))
                {
                    _trace.EmitFieldChanged(
                        "ExportDialog.ExporterIin",
                        ValuePolicy.Masked("***"),
                        ValuePolicy.Masked("***"));
                }
            }
        }

        private bool _canSubmit;
        public bool CanSubmit
        {
            get => _canSubmit;
            set => SetProperty(ref _canSubmit, value);
        }

        private string _validationMessage;
        public string ValidationMessage
        {
            get => _validationMessage;
            set => SetProperty(ref _validationMessage, value);
        }

        private string _exportStatus;
        public string ExportStatus
        {
            get => _exportStatus;
            set => SetProperty(ref _exportStatus, value);
        }

        // ----------------------------------------------------------------
        // Commands
        // ----------------------------------------------------------------

        [TraceCommand("Export.ValidateIin")]
        public ICommand ValidateIinCommand { get; }

        [TraceCommand("Export.Submit")]
        public ICommand SubmitCommand { get; }

        private void ExecuteValidateIin(object parameter)
        {
            bool isDuplicate = KnownDuplicateIins.Contains(_exporterIin);

            if (isDuplicate)
            {
                // Show the error modal by emitting ScreenOpened.
                _trace.EmitScreenOpened("DuplicateIinErrorModal");

                ValidationMessage = "ERROR: Duplicate IIN detected. Export blocked.";

                // BUG 2: CanSubmit not updated after error modal — Export button stays enabled.
                // The correct fix would be:
                //   CanSubmit = false;
                //   CommandManager.InvalidateRequerySuggested();
                // Instead we leave CanSubmit = true and do NOT invalidate the command manager,
                // so the Export button stays enabled even though the IIN is invalid.
            }
            else
            {
                ValidationMessage = "IIN validated OK.";
                CanSubmit = true;
            }
        }

        private bool CanExecuteSubmit(object parameter) => _canSubmit;

        private void ExecuteSubmit(object parameter)
        {
            ExportStatus = string.Format(
                "Declaration {0} submitted for export at {1:u}.",
                _declaration.Number,
                DateTime.UtcNow);

            _trace.EmitNavigationOccurred("ExportDialog", "ExportComplete");
        }
    }
}
