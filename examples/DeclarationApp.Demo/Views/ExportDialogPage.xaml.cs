using System.Windows;
using System.Windows.Controls;

using SemantxTrace.Abstractions;

using DeclarationApp.Demo.Models;
using DeclarationApp.Demo.ViewModels;

namespace DeclarationApp.Demo.Views
{
    /// <summary>
    /// Code-behind for <see cref="ExportDialogPage"/>.
    /// Emits <c>ScreenOpened("ExportDialog")</c> on load and creates
    /// <see cref="ExportDialogViewModel"/> with constructor-injected
    /// <see cref="ITraceContext"/>.
    /// </summary>
    [ScreenId("ExportDialog")]
    public partial class ExportDialogPage : Page
    {
        private readonly Declaration _declaration;

        public ExportDialogPage(Declaration declaration)
        {
            InitializeComponent();
            _declaration = declaration;

            DataContext = new ExportDialogViewModel(App.TraceContext, declaration);

            Loaded += ExportDialogPage_Loaded;
        }

        private void ExportDialogPage_Loaded(object sender, RoutedEventArgs e)
        {
            App.TraceContext.EmitScreenOpened("ExportDialog");
        }
    }
}
