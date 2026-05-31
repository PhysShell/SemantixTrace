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
    /// Thrown when the computed duty amount is negative, which indicates a
    /// data-entry error (e.g. zero or negative quantity combined with a duty
    /// rate rounding artefact).
    /// </summary>
    public class NegativePaymentException : Exception
    {
        public NegativePaymentException(string message) : base(message) { }
        public NegativePaymentException(string message, Exception inner) : base(message, inner) { }
    }

    /// <summary>
    /// ViewModel for <see cref="PaymentsCalculatorPage"/>.
    ///
    /// BUG 1: <see cref="RecalculateCommand"/> uses integer division to compute
    /// the effective quantity from <see cref="TotalQuantity"/>. When
    /// <c>TotalQuantity &gt; 1000</c>, the expression
    /// <c>(TotalQuantity / 1000) * 1000</c> silently truncates by up to 999
    /// units, producing a duty amount that is lower than the correct value.
    /// If the result goes negative (possible when <see cref="DutyRate"/> is
    /// negative due to an input error), a <see cref="NegativePaymentException"/>
    /// is thrown; <see cref="TracedRelayCommand"/> catches it and emits an
    /// <c>ExceptionThrown</c> event.
    /// </summary>
    public class PaymentsCalculatorViewModel : ViewModelBase
    {
        private readonly ITraceContext _trace;
        private readonly Frame _navigationFrame;
        private readonly Declaration _declaration;

        public PaymentsCalculatorViewModel(ITraceContext trace, Frame navigationFrame, Declaration declaration)
        {
            _trace = trace ?? throw new ArgumentNullException(nameof(trace));
            _navigationFrame = navigationFrame ?? throw new ArgumentNullException(nameof(navigationFrame));
            _declaration = declaration ?? throw new ArgumentNullException(nameof(declaration));

            // Seed from declaration goods.
            foreach (var item in declaration.Goods)
                _totalQuantity += item.Quantity;

            _dutyRate = 0.15m; // 15% default rate.

            var recalcInner    = new RelayCommand(ExecuteRecalculate);
            var exportInner    = new RelayCommand(ExecuteNavigateToExport);

            RecalculateCommand    = new TracedRelayCommand(recalcInner,  "Graph47.Recalculate", _trace);
            NavigateToExportCommand = new TracedRelayCommand(exportInner, "PaymentsCalculator.GoToExport", _trace);
        }

        // ----------------------------------------------------------------
        // Bound properties
        // ----------------------------------------------------------------

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

        private string _calculationStatus;
        public string CalculationStatus
        {
            get => _calculationStatus;
            set => SetProperty(ref _calculationStatus, value);
        }

        // ----------------------------------------------------------------
        // Commands
        // ----------------------------------------------------------------

        [TraceCommand("Graph47.Recalculate")]
        public ICommand RecalculateCommand { get; }

        [TraceCommand("PaymentsCalculator.GoToExport")]
        public ICommand NavigateToExportCommand { get; }

        private void ExecuteRecalculate(object parameter)
        {
            // BUG 1: integer division truncates quantity > 1000 by 1 unit —
            // (TotalQuantity / 1000) performs integer division, so any quantity
            // in the range [1001, 1999] is treated as 1000, [2001, 2999] as 2000,
            // etc. The product (/ 1000) * 1000 therefore rounds DOWN by up to 999
            // units, making the duty amount smaller than it should be.
            // When TotalQuantity is exactly a multiple of 1000 this matches
            // the correct result, masking the bug in simple test cases.
            int effectiveQuantity = (TotalQuantity / 1000) * 1000;

            decimal dutyAmount = effectiveQuantity * DutyRate;

            if (dutyAmount < 0)
            {
                // Negative payment indicates a data error — throw so that
                // TracedRelayCommand can capture and emit ExceptionThrown.
                throw new NegativePaymentException(
                    string.Format("Computed duty {0:F2} is negative; check quantity and rate.", dutyAmount));
            }

            DutyAmount = dutyAmount;
            _declaration.TotalDuty = dutyAmount;
            CalculationStatus = string.Format("Duty: {0:C} (effective qty: {1})", dutyAmount, effectiveQuantity);
        }

        private void ExecuteNavigateToExport(object parameter)
        {
            _trace.EmitNavigationOccurred("PaymentsCalculator", "ExportDialog");
            _navigationFrame.Navigate(new ExportDialogPage(_declaration));
        }
    }
}
