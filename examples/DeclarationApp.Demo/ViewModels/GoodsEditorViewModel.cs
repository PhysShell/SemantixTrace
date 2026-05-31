using System;
using System.Diagnostics;
using System.Threading.Tasks;
using System.Windows.Controls;
using System.Windows.Input;

using SemantxTrace.Abstractions;
using SemantxTrace.Wpf;

using DeclarationApp.Demo.Models;
using DeclarationApp.Demo.Views;

namespace DeclarationApp.Demo.ViewModels
{
    /// <summary>
    /// ViewModel for <see cref="GoodsEditorPage"/>.
    ///
    /// BUG 3: <see cref="AsyncCustomsCodeValidator"/> runs on a background thread
    /// via <c>Task.Run</c>. The validator may complete AFTER
    /// <see cref="SubmitCommand"/> fires: the <c>ValidationFailed</c> event then
    /// arrives in the trace AFTER the <c>CommandExecuted</c> event for the
    /// submit, producing an out-of-order signal that the SemantxTrace analyser
    /// flags as a race condition.
    /// </summary>
    public class GoodsEditorViewModel : ViewModelBase
    {
        private readonly ITraceContext _trace;
        private readonly Frame _navigationFrame;
        private readonly Declaration _declaration;

        // Tracks the background validation task so the bug can manifest.
        private Task _pendingValidationTask;

        public GoodsEditorViewModel(ITraceContext trace, Frame navigationFrame, Declaration declaration)
        {
            _trace = trace ?? throw new ArgumentNullException(nameof(trace));
            _navigationFrame = navigationFrame ?? throw new ArgumentNullException(nameof(navigationFrame));
            _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

            // Seed from the first goods item if present.
            var first = declaration.Goods.Count > 0 ? declaration.Goods[0] : null;
            _customsCode = first?.CustomsCode ?? string.Empty;
            _quantity    = first?.Quantity ?? 1;
            _unitPrice   = first?.UnitPrice ?? 0m;

            var submitInner = new RelayCommand(ExecuteSubmit);
            SubmitCommand = new TracedRelayCommand(submitInner, "GoodsDeclaration.Submit", _trace);
        }

        // ----------------------------------------------------------------
        // Bound properties
        // ----------------------------------------------------------------

        private string _customsCode;
        public string CustomsCode
        {
            get => _customsCode;
            set
            {
                if (SetProperty(ref _customsCode, value))
                {
                    // Kick off async validation whenever the code changes.
                    StartAsyncValidation(value);
                }
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

        private string _validationStatus;
        public string ValidationStatus
        {
            get => _validationStatus;
            set => SetProperty(ref _validationStatus, value);
        }

        // ----------------------------------------------------------------
        // Commands
        // ----------------------------------------------------------------

        [TraceCommand("GoodsDeclaration.Submit")]
        public ICommand SubmitCommand { get; }

        // ----------------------------------------------------------------
        // Async validation (BUG 3 source)
        // ----------------------------------------------------------------

        /// <summary>
        /// Starts the background customs-code validator. The validator
        /// intentionally uses a delay to simulate a real I/O-bound lookup
        /// so that it can complete AFTER <see cref="ExecuteSubmit"/> has already
        /// emitted its <c>CommandExecuted</c> event.
        /// </summary>
        private void StartAsyncValidation(string code)
        {
            // Keep a reference so the submit handler can observe the race.
            _pendingValidationTask = Task.Run(() => AsyncCustomsCodeValidator(code));
        }

        private void AsyncCustomsCodeValidator(string code)
        {
            var sw = Stopwatch.StartNew();

            // Simulate a slow external lookup (200 ms) so that submit can fire first.
            System.Threading.Thread.Sleep(200);

            // Validate: codes must be exactly 7 characters in HS format "NNNN.NN".
            bool isValid = code != null && code.Length == 7 && code[4] == '.';

            sw.Stop();

            if (!isValid)
            {
                // BUG 3: async validation result arrives after submit fires —
                // this ValidationFailed event may be emitted AFTER the
                // GoodsDeclaration.Submit CommandExecuted event, causing an
                // out-of-order trace that signals a race condition.
                _trace.EmitValidationFailed(
                    "AsyncCustomsCodeValidator",
                    "GoodsEditor.CustomsCode",
                    "***");
            }

            // Update UI status (fire-and-forget, marshalled to UI thread via Dispatcher).
            System.Windows.Application.Current?.Dispatcher?.BeginInvoke(
                new Action(() =>
                {
                    ValidationStatus = isValid
                        ? "Customs code: OK"
                        : "Customs code: INVALID (must be HS format NNNN.NN)";
                }));
        }

        private void ExecuteSubmit(object parameter)
        {
            // BUG 3: async validation result arrives after submit fires —
            // we do NOT await _pendingValidationTask here. The
            // TracedRelayCommand will emit CommandExecuted("GoodsDeclaration.Submit", ...)
            // immediately, but the AsyncCustomsCodeValidator may still be
            // running and will emit ValidationFailed AFTER this point.
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
            _navigationFrame.Navigate(new PaymentsCalculatorPage(_declaration));
        }
    }
}
