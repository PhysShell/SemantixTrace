using System;
using System.Collections.ObjectModel;
using System.Windows.Input;

using Avalonia.Controls;

using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.Views;

using SemantxTrace.Abstractions;
using SemantxTrace.Avalonia;

namespace DeclarationApp.Demo.Avalonia.ViewModels;

/// <summary>ViewModel for <see cref="InvoiceListPage"/>.</summary>
public class InvoiceListViewModel : ViewModelBase
{
    private readonly ITraceContext _trace;
    private readonly Action<Control> _navigate;

    public InvoiceListViewModel(ITraceContext trace, Action<Control> navigate)
    {
        _trace = trace ?? throw new ArgumentNullException(nameof(trace));
        _navigate = navigate ?? throw new ArgumentNullException(nameof(navigate));

        Declarations = new ObservableCollection<Declaration>
        {
            new() { Id = Guid.Parse("11111111-0000-0000-0000-000000000001"), Number = "KZ-2024-000001", ExporterIin = "123456789012", TotalDuty = 1250.00m },
            new() { Id = Guid.Parse("11111111-0000-0000-0000-000000000002"), Number = "KZ-2024-000002", ExporterIin = "987654321098", TotalDuty = 4780.50m },
            new() { Id = Guid.Parse("11111111-0000-0000-0000-000000000003"), Number = "KZ-2024-000003", ExporterIin = "555000111222", TotalDuty = 320.75m },
        };

        NewDeclarationCommand  = new TracedRelayCommand(new RelayCommand(ExecuteNewDeclaration),  "Declaration.New",  _trace);
        OpenDeclarationCommand = new TracedRelayCommand(new RelayCommand(ExecuteOpenDeclaration), "Declaration.Open", _trace);
    }

    public ObservableCollection<Declaration> Declarations { get; }

    private Declaration? _selectedDeclaration;
    public Declaration? SelectedDeclaration
    {
        get => _selectedDeclaration;
        set => SetProperty(ref _selectedDeclaration, value);
    }

    [TraceCommand("Declaration.New")]
    public ICommand NewDeclarationCommand { get; }

    [TraceCommand("Declaration.Open")]
    public ICommand OpenDeclarationCommand { get; }

    private void ExecuteNewDeclaration(object? _)
    {
        var declaration = new Declaration
        {
            Id     = Guid.NewGuid(),
            Number = $"KZ-2024-{Declarations.Count + 1:D6}",
        };
        _trace.EmitNavigationOccurred("InvoiceList", "InvoiceEditor");
        _navigate(new InvoiceEditorPage(declaration, _trace, _navigate));
    }

    private void ExecuteOpenDeclaration(object? _)
    {
        if (_selectedDeclaration is not { } declaration) return;
        _trace.EmitNavigationOccurred("InvoiceList", "InvoiceEditor");
        _navigate(new InvoiceEditorPage(declaration, _trace, _navigate));
    }
}
