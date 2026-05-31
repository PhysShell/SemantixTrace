using System.Windows.Controls;
using DeclarationApp.Demo.ViewModels;

namespace DeclarationApp.Demo.Views
{
    public partial class ExportDialogPage : Page
    {
        public ExportDialogPage()
        {
            InitializeComponent();
            DataContext = new ExportDialogViewModel(App.TraceContext, NavigationService);
        }
    }
}
