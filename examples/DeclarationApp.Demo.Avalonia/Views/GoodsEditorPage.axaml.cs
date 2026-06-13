using System;
using Avalonia.Controls;
using DeclarationApp.Demo.Avalonia.Models;
using DeclarationApp.Demo.Avalonia.ViewModels;
using SemantxTrace.Abstractions;

namespace DeclarationApp.Demo.Avalonia.Views;

[ScreenId("GoodsEditor")]
public partial class GoodsEditorPage : UserControl
{
    public GoodsEditorPage(Declaration declaration, ITraceContext trace, Action<Control> navigate)
    {
        InitializeComponent();
        DataContext = new GoodsEditorViewModel(trace, navigate, declaration);
        trace.EmitScreenOpened("GoodsEditor");
    }
}
