using System;
using System.Collections.ObjectModel;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

using SemantxTrace.Abstractions;
using SemantxTrace.Wpf;

using DeclarationApp.Demo.Models;
using DeclarationApp.Demo.Views;

namespace DeclarationApp.Demo.ViewModels
{
    /// <summary>
    /// ViewModel for <see cref="InvoiceListPage"/>.
    /// Displays three seed declarations and exposes commands to create or open them.
    /// </summary>
    public class InvoiceListViewModel : ViewModelBase
    {
        private readonly ITraceContext _trace;
        private readonly Frame _navigationFrame;

        public InvoiceListViewModel(ITraceContext trace, Frame navigationFrame)
        {
            _trace = trace ?? throw new ArgumentNullException(nameof(trace));
            _navigationFrame = navigationFrame ?? throw new ArgumentNullException(nameof(navigationFrame));

            Declarations = new ObservableCollection<Declaration>
            {
                new Declaration
                {
                    Id           = Guid.Parse("11111111-0000-0000-0000-000000000001"),
                    Number       = "KZ-2024-000001",
                    ExporterIin  = "123456789012",
                    TotalDuty    = 1250.00m,
                },
                new Declaration
                {
                    Id           = Guid.Parse("11111111-0000-0000-0000-000000000002"),
                    Number       = "KZ-2024-000002",
                    ExporterIin  = "987654321098",
                    TotalDuty    = 4780.50m,
                },
                new Declaration
                {
                    Id           = Guid.Parse("11111111-0000-0000-0000-000000000003"),
                    Number       = "KZ-2024-000003",
                    ExporterIin  = "555000111222",
                    TotalDuty    = 320.75m,
                },
            };

            var newInner  = new RelayCommand(ExecuteNewDeclaration);
            var openInner = new RelayCommand(ExecuteOpenDeclaration);

            NewDeclarationCommand  = new TracedRelayCommand(newInner,  "Declaration.New",  _trace);
            OpenDeclarationCommand = new TracedRelayCommand(openInner, "Declaration.Open", _trace);
        }

        public ObservableCollection<Declaration> Declarations { get; }

        private Declaration _selectedDeclaration;
        public Declaration SelectedDeclaration
        {
            get => _selectedDeclaration;
            set => SetProperty(ref _selectedDeclaration, value);
        }

        [TraceCommand("Declaration.New")]
        public ICommand NewDeclarationCommand { get; }

        [TraceCommand("Declaration.Open")]
        public ICommand OpenDeclarationCommand { get; }

        private void ExecuteNewDeclaration(object parameter)
        {
            var declaration = new Declaration
            {
                Id     = Guid.NewGuid(),
                Number = string.Format("KZ-2024-{0:D6}", Declarations.Count + 1),
            };

            _trace.EmitNavigationOccurred("InvoiceList", "InvoiceEditor");
            _navigationFrame.Navigate(new InvoiceEditorPage(declaration));
        }

        private void ExecuteOpenDeclaration(object parameter)
        {
            var declaration = SelectedDeclaration;
            if (declaration == null)
                return;

            _trace.EmitNavigationOccurred("InvoiceList", "InvoiceEditor");
            _navigationFrame.Navigate(new InvoiceEditorPage(declaration));
        }
    }
}
