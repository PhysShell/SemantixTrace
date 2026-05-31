using System;
using System.IO;
using System.Windows;

using SemantxTrace.Wpf;
using SemantxTrace.Abstractions;

namespace DeclarationApp.Demo
{
    /// <summary>
    /// Application entry point. Creates the <see cref="FileJsonlTraceContext"/>
    /// that writes all trace events to <c>trace.jsonl</c> in the application
    /// directory, and disposes it (flushing the final events) on exit.
    /// </summary>
    public partial class App : Application
    {
        /// <summary>
        /// The application-wide trace context. All ViewModels receive this via
        /// constructor injection from their code-behind pages.
        /// </summary>
        public static ITraceContext TraceContext { get; private set; }

        protected override void OnStartup(StartupEventArgs e)
        {
            base.OnStartup(e);

            string traceFilePath = Path.Combine(
                AppDomain.CurrentDomain.BaseDirectory,
                "trace.jsonl");

            TraceContext = new FileJsonlTraceContext(traceFilePath);
        }

        protected override void OnExit(ExitEventArgs e)
        {
            TraceContext?.Dispose();
            base.OnExit(e);
        }
    }
}
