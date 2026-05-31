using System.Windows;
using System.Windows.Controls;
using System.Windows.Navigation;

using SemantxTrace.Abstractions;

using DeclarationApp.Demo.ViewModels;

namespace DeclarationApp.Demo.Views
{
    /// <summary>
    /// Code-behind for <see cref="InvoiceListPage"/>.
    /// Creates <see cref="InvoiceListViewModel"/> with constructor-injected
    /// <see cref="ITraceContext"/> from <see cref="App.TraceContext"/>.
    /// </summary>
    [ScreenId("InvoiceList")]
    public partial class InvoiceListPage : Page
    {
        public InvoiceListPage()
        {
            InitializeComponent();

            var frame = NavigationService?.Content as Frame
                        ?? FindNavigationFrame();

            DataContext = new InvoiceListViewModel(App.TraceContext, GetFrame());

            Loaded += InvoiceListPage_Loaded;
        }

        private void InvoiceListPage_Loaded(object sender, RoutedEventArgs e)
        {
            App.TraceContext.EmitScreenOpened("InvoiceList");
        }

        /// <summary>
        /// Walks up the visual tree to find the containing <see cref="Frame"/>.
        /// Falls back to using <see cref="NavigationService"/> if the frame
        /// is not yet in the visual tree.
        /// </summary>
        private Frame GetFrame()
        {
            // The Frame is set as the NavigationService host once the page
            // is navigated into it.  At construction time we pre-resolve it
            // from the MainWindow.
            var mainWindow = System.Windows.Application.Current?.MainWindow as MainWindow;
            return mainWindow?.MainFrame;
        }

        private Frame FindNavigationFrame()
        {
            var mainWindow = System.Windows.Application.Current?.MainWindow as MainWindow;
            return mainWindow?.MainFrame;
        }
    }
}
