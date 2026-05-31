using System.Windows;

using DeclarationApp.Demo.Views;

namespace DeclarationApp.Demo
{
    /// <summary>
    /// Shell window. Hosts a <see cref="System.Windows.Controls.Frame"/> for
    /// page-based navigation and navigates to <see cref="InvoiceListPage"/> on load.
    /// </summary>
    public partial class MainWindow : Window
    {
        public MainWindow()
        {
            InitializeComponent();
            Loaded += MainWindow_Loaded;
        }

        private void MainWindow_Loaded(object sender, RoutedEventArgs e)
        {
            MainFrame.Navigate(new InvoiceListPage());
        }
    }
}
