using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;

namespace DeclarationApp.Demo.ViewModels
{
    /// <summary>
    /// Base class for all ViewModels. Provides <see cref="INotifyPropertyChanged"/>
    /// via <see cref="SetProperty{T}"/> and a lightweight <see cref="RelayCommand"/>
    /// inner class.
    /// </summary>
    public abstract class ViewModelBase : INotifyPropertyChanged
    {
        public event PropertyChangedEventHandler PropertyChanged;

        /// <summary>
        /// Sets <paramref name="field"/> to <paramref name="value"/> and raises
        /// <see cref="PropertyChanged"/> if the value actually changed.
        /// </summary>
        /// <typeparam name="T">Property type.</typeparam>
        /// <param name="field">Backing field reference.</param>
        /// <param name="value">New value.</param>
        /// <param name="propertyName">
        /// Caller property name; filled automatically by <see cref="CallerMemberNameAttribute"/>.
        /// </param>
        /// <returns><see langword="true"/> if the value changed and the event was raised.</returns>
        protected bool SetProperty<T>(ref T field, T value, [CallerMemberName] string propertyName = null)
        {
            if (EqualityComparer<T>.Default.Equals(field, value))
                return false;

            field = value;
            OnPropertyChanged(propertyName);
            return true;
        }

        protected void OnPropertyChanged([CallerMemberName] string propertyName = null)
        {
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        }

        // ----------------------------------------------------------------
        // Lightweight relay command — no third-party dependencies.
        // ----------------------------------------------------------------

        /// <summary>
        /// Minimal <see cref="ICommand"/> implementation backed by delegates.
        /// Wraps the <see cref="CommandManager.RequerySuggested"/> event for
        /// automatic <see cref="CanExecute"/> re-evaluation.
        /// </summary>
        protected sealed class RelayCommand : ICommand
        {
            private readonly Action<object> _execute;
            private readonly Func<object, bool> _canExecute;

            public RelayCommand(Action<object> execute, Func<object, bool> canExecute = null)
            {
                _execute = execute ?? throw new ArgumentNullException(nameof(execute));
                _canExecute = canExecute;
            }

            public RelayCommand(Action execute, Func<bool> canExecute = null)
                : this(
                    _ => execute(),
                    canExecute == null ? (Func<object, bool>)null : _ => canExecute())
            {
            }

            public bool CanExecute(object parameter)
                => _canExecute == null || _canExecute(parameter);

            public void Execute(object parameter) => _execute(parameter);

            public event EventHandler CanExecuteChanged
            {
                add => CommandManager.RequerySuggested += value;
                remove => CommandManager.RequerySuggested -= value;
            }

            public void RaiseCanExecuteChanged()
                => CommandManager.InvalidateRequerySuggested();
        }
    }
}
