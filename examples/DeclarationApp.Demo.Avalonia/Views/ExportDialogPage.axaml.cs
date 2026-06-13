using Avalonia.Controls;
using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.ViewModels;
using SemantxTrace.Abstractions;

namespace DeclarationApp.Demo.Avalonia.Views;

[ScreenId("ExportDialog")]
public partial class ExportDialogPage : UserControl
{
    public ExportDialogPage(Declaration declaration, ITraceContext trace)
    {
        InitializeComponent();
        DataContext = new ExportDialogViewModel(trace, declaration);
        trace.EmitScreenOpened("ExportDialog");
    }
}
