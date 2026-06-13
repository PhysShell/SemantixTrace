using Avalonia.Controls;
using DeclarationApp.Demo.Avalonia.Views;
using SemantxTrace.Avalonia;

namespace DeclarationApp.Demo.Avalonia;

public partial class MainWindow : Window
{
    private readonly SemantxTrace.Abstractions.ITraceContext _trace;

    public MainWindow()
    {
        _trace = new FileJsonlTraceContext("trace-output.jsonl");
        InitializeComponent();
        NavigateTo(new InvoiceListPage(_trace, NavigateTo));
    }

    private void NavigateTo(Control page)
    {
        this.FindControl<ContentControl>("MainContent")!.Content = page;
    }
}
