using System.Windows;
using System.Windows.Controls;

using SemantxTrace.Abstractions;

using DeclarationApp.Demo.Models;
using DeclarationApp.Demo.ViewModels;

namespace DeclarationApp.Demo.Views
{
    /// <summary>
    /// Code-behind for <see cref="InvoiceEditorPage"/>.
    /// Receives a <see cref="Declaration"/> to edit and creates the
    /// <see cref="InvoiceEditorViewModel"/> with constructor-injected
    /// <see cref="ITraceContext"/>.
    /// </summary>
    [ScreenId("InvoiceEditor")]
    public partial class InvoiceEditorPage : Page
    {
        private readonly Declaration _declaration;

        public InvoiceEditorPage(Declaration declaration)
        {
            InitializeComponent();
            _declaration = declaration;

            var frame = GetFrame();
            DataContext = new InvoiceEditorViewModel(App.TraceContext, frame, declaration);

            Loaded += InvoiceEditorPage_Loaded;
        }

        private void InvoiceEditorPage_Loaded(object sender, RoutedEventArgs e)
        {
            App.TraceContext.EmitScreenOpened("InvoiceEditor");
        }

        private Frame GetFrame()
        {
            var mainWindow = System.Windows.Application.Current?.MainWindow as MainWindow;
            return mainWindow?.MainFrame;
        }
    }
}
