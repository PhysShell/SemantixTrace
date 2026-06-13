using System;
using Avalonia;
using DeclarationApp.Demo.Avalonia;

class Program
{
    [STAThread]
    public static void Main(string[] args) =>
        App.BuildAvaloniaApp().StartWithClassicDesktopLifetime(args);
}
