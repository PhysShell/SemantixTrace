using System.Windows;
using System.Windows.Controls;

using SemantxTrace.Abstractions;

using DeclarationApp.Demo.Models;
using DeclarationApp.Demo.ViewModels;

namespace DeclarationApp.Demo.Views
{
    /// <summary>
    /// Code-behind for <see cref="PaymentsCalculatorPage"/>.
    /// Emits <c>ScreenOpened("PaymentsCalculator")</c> on load and creates
    /// <see cref="PaymentsCalculatorViewModel"/> with constructor-injected
    /// <see cref="ITraceContext"/>.
    /// </summary>
    [ScreenId("PaymentsCalculator")]
    public partial class PaymentsCalculatorPage : Page
    {
        private readonly Declaration _declaration;

        public PaymentsCalculatorPage(Declaration declaration)
        {
            InitializeComponent();
            _declaration = declaration;

            var frame = GetFrame();
            DataContext = new PaymentsCalculatorViewModel(App.TraceContext, frame, declaration);

            Loaded += PaymentsCalculatorPage_Loaded;
        }

        private void PaymentsCalculatorPage_Loaded(object sender, RoutedEventArgs e)
        {
            App.TraceContext.EmitScreenOpened("PaymentsCalculator");
        }

        private Frame GetFrame()
        {
            var mainWindow = System.Windows.Application.Current?.MainWindow as MainWindow;
            return mainWindow?.MainFrame;
        }
    }
}
