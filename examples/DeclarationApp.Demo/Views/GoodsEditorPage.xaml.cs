using System.Windows;
using System.Windows.Controls;

using SemantxTrace.Abstractions;

using DeclarationApp.Demo.Models;
using DeclarationApp.Demo.ViewModels;

namespace DeclarationApp.Demo.Views
{
    /// <summary>
    /// Code-behind for <see cref="GoodsEditorPage"/>.
    /// Emits <c>ScreenOpened("GoodsEditor")</c> on load and creates
    /// <see cref="GoodsEditorViewModel"/> with constructor-injected
    /// <see cref="ITraceContext"/>.
    /// </summary>
    [ScreenId("GoodsEditor")]
    public partial class GoodsEditorPage : Page
    {
        private readonly Declaration _declaration;

        public GoodsEditorPage(Declaration declaration)
        {
            InitializeComponent();
            _declaration = declaration;

            var frame = GetFrame();
            DataContext = new GoodsEditorViewModel(App.TraceContext, frame, declaration);

            Loaded += GoodsEditorPage_Loaded;
        }

        private void GoodsEditorPage_Loaded(object sender, RoutedEventArgs e)
        {
            App.TraceContext.EmitScreenOpened("GoodsEditor");
        }

        private Frame GetFrame()
        {
            var mainWindow = System.Windows.Application.Current?.MainWindow as MainWindow;
            return mainWindow?.MainFrame;
        }
    }
}
