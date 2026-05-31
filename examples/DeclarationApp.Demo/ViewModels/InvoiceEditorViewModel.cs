using System;
using System.Windows.Controls;
using System.Windows.Input;

using SemantxTrace.Abstractions;
using SemantxTrace.Wpf;

using DeclarationApp.Demo.Models;
using DeclarationApp.Demo.Views;

namespace DeclarationApp.Demo.ViewModels
{
    /// <summary>
    /// ViewModel for <see cref="InvoiceEditorPage"/>.
    /// Edits a Declaration's header fields. ExporterIin and DeclarationNumber are
    /// masked in trace events (ADR-0007).
    /// </summary>
    public class InvoiceEditorViewModel : ViewModelBase
    {
        private readonly ITraceContext _trace;
        private readonly Frame _navigationFrame;
        private readonly Declaration _declaration;

        public InvoiceEditorViewModel(ITraceContext trace, Frame navigationFrame, Declaration declaration)
        {
            _trace = trace ?? throw new ArgumentNullException(nameof(trace));
            _navigationFrame = navigationFrame ?? throw new ArgumentNullException(nameof(navigationFrame));
            _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

            _declarationNumber = declaration.Number ?? string.Empty;
            _exporterIin       = declaration.ExporterIin ?? string.Empty;

            var saveInner           = new RelayCommand(ExecuteSave);
            var navigateGoodsInner  = new RelayCommand(ExecuteNavigateToGoods);

            SaveCommand            = new TracedRelayCommand(saveInner,          "Declaration.Save",  _trace);
            NavigateToGoodsCommand = new TracedRelayCommand(navigateGoodsInner, "Declaration.GoToGoods", _trace);
        }

        // ----------------------------------------------------------------
        // Bound properties (masked in trace — ADR-0007)
        // ----------------------------------------------------------------

        private string _declarationNumber;

        [TraceField("InvoiceEditor.DeclarationNumber", Policy = ValuePolicyKind.Masked)]
        public string DeclarationNumber
        {
            get => _declarationNumber;
            set
            {
                string oldVal = _declarationNumber;
                if (SetProperty(ref _declarationNumber, value))
                {
                    _trace.EmitFieldChanged(
                        "InvoiceEditor.DeclarationNumber",
                        ValuePolicy.Masked("***"),
                        ValuePolicy.Masked("***"));
                }
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
                {
                    // IIN is a PII identifier — record as Hashed (ADR-0007).
                    _trace.EmitFieldChanged(
                        "InvoiceEditor.ExporterIin",
                        ValuePolicy.Masked("***"),
                        ValuePolicy.Masked("***"));
                }
            }
        }

        // ----------------------------------------------------------------
        // Commands
        // ----------------------------------------------------------------

        [TraceCommand("Declaration.Save")]
        public ICommand SaveCommand { get; }

        [TraceCommand("Declaration.GoToGoods")]
        public ICommand NavigateToGoodsCommand { get; }

        private void ExecuteSave(object parameter)
        {
            _declaration.Number      = _declarationNumber;
            _declaration.ExporterIin = _exporterIin;
        }

        private void ExecuteNavigateToGoods(object parameter)
        {
            // Persist edits before navigating.
            _declaration.Number      = _declarationNumber;
            _declaration.ExporterIin = _exporterIin;

            _trace.EmitNavigationOccurred("InvoiceEditor", "GoodsEditor");
            _navigationFrame.Navigate(new GoodsEditorPage(_declaration));
        }
    }
}
