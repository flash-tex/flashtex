// name: MainWindow.CommandPalette.cs
// purpose: Native WinUI command palette over Shell's single CommandRegistry. This is
// deliberately a ContentDialog rather than a WebView popup: command discovery remains
// part of the native shell even though the text editor is a dedicated CodeMirror host.

using FlashTeX.Shell;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Windows.System;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private bool _commandPaletteOpen;

    private bool TryStartCommandPalette()
    {
        if (_commandPaletteOpen)
        {
            return true;
        }
        _ = ShowCommandPaletteAsync();
        return true;
    }

    private async Task ShowCommandPaletteAsync()
    {
        _commandPaletteOpen = true;
        try
        {
            var query = new TextBox
            {
                PlaceholderText = "Type a command…",
                Margin = new Thickness(0, 0, 0, 8),
            };
            var list = new ListView
            {
                SelectionMode = ListViewSelectionMode.Single,
                MaxHeight = 440,
                MinWidth = 520,
            };
            var content = new StackPanel();
            content.Children.Add(query);
            content.Children.Add(list);

            var dialog = new ContentDialog
            {
                Title = "Command Palette",
                Content = content,
                CloseButtonText = "Close",
                XamlRoot = Content.XamlRoot,
            };

            void Rebuild(string text)
            {
                list.Items.Clear();
                foreach (var command in CommandRegistry.Search(text).Where(command => command.CanExecute(_shell)))
                {
                    var row = new Grid();
                    row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
                    row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
                    row.Children.Add(new TextBlock { Text = command.Title });
                    var shortcut = new TextBlock { Text = command.DefaultShortcut.ToString(), Opacity = 0.65 };
                    Grid.SetColumn(shortcut, 1);
                    row.Children.Add(shortcut);
                    list.Items.Add(new ListViewItem { Content = row, Tag = command });
                }
            }

            void RunSelected()
            {
                if (list.SelectedItem is not ListViewItem { Tag: Command command })
                {
                    return;
                }
                dialog.Hide();
                DispatcherQueue.TryEnqueue(() => ExecuteCommand(command.Id));
            }

            query.TextChanged += (_, _) => Rebuild(query.Text);
            list.DoubleTapped += (_, _) => RunSelected();
            query.KeyDown += (_, args) =>
            {
                if (args.Key == VirtualKey.Down && list.Items.Count > 0)
                {
                    list.SelectedIndex = Math.Min(list.SelectedIndex + 1, list.Items.Count - 1);
                    args.Handled = true;
                }
                else if (args.Key == VirtualKey.Up && list.Items.Count > 0)
                {
                    list.SelectedIndex = Math.Max(list.SelectedIndex - 1, 0);
                    args.Handled = true;
                }
                else if (args.Key == VirtualKey.Enter)
                {
                    if (list.SelectedIndex < 0 && list.Items.Count > 0)
                    {
                        list.SelectedIndex = 0;
                    }
                    RunSelected();
                    args.Handled = true;
                }
            };

            Rebuild(string.Empty);
            _ = DispatcherQueue.TryEnqueue(() => query.Focus(FocusState.Programmatic));
            await dialog.ShowAsync();
        }
        finally
        {
            _commandPaletteOpen = false;
        }
    }
}
