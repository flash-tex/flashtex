// name: MainWindow.Problems.cs
// purpose: The collapsible Problems panel: a VS Code/Xcode-style list of the
//   active compile's diagnostics, grouped by source file, with severity
//   icon/color and a "file:line:column" location computed from each
//   diagnostic's byte-offset SourceRange against the matching open document's
//   text. Clicking a row switches to its document (ShellModel.SwitchActiveDocument,
//   the same pattern MainWindow.ProjectFiles.cs's project-tree buttons and
//   MainWindow.Preview.cs's preview-click navigation already use) and reveals
//   the span in the shared CodeMirror editor host via EditorHost.RevealSourceRange
//   (EditorHost.Bridge.cs) -- no parallel reveal mechanism. Wired to
//   CommandIds.ToggleProblems in MainWindow.Menu.cs's ExecuteCommand, and
//   (optional polish) to a click on the status bar's diagnostics count
//   (MainWindow.StatusBar.cs).
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;
using FlashTeX.Shell;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private const double ProblemsPanelHeight = 220;

    private bool _problemsPanelVisible;

    private void WireProblemsPanel()
    {
        _shell.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName == nameof(ShellModel.Diagnostics))
            {
                DispatcherQueue.TryEnqueue(RebuildProblemsPanel);
            }
        };
        RebuildProblemsPanel();
    }

    /// <summary>Toggles the panel's visibility; always returns true (CommandIds.ToggleProblems has no CanExecute gate).</summary>
    private bool ToggleProblemsPanel()
    {
        SetProblemsPanelVisible(!_problemsPanelVisible);
        return true;
    }

    private void SetProblemsPanelVisible(bool visible)
    {
        _problemsPanelVisible = visible;
        ProblemsPanelRow.Height = new GridLength(visible ? ProblemsPanelHeight : 0);
        ProblemsPanelHost.Visibility = visible ? Visibility.Visible : Visibility.Collapsed;
        if (visible)
        {
            RebuildProblemsPanel();
        }
    }

    private void RebuildProblemsPanel()
    {
        var root = new Grid();
        root.RowDefinitions.Add(new RowDefinition { Height = GridLength.Auto });
        root.RowDefinitions.Add(new RowDefinition { Height = new GridLength(1, GridUnitType.Star) });
        root.Children.Add(BuildProblemsHeader());

        // A plain ScrollViewer over a StackPanel of rows, not a ListView: this app already
        // avoids ListView-style selection/virtualization machinery in favor of directly
        // clickable Buttons for this kind of simple row list (MainWindow.ProjectFiles.cs's
        // project-tree buttons are the precedent) -- diagnostic counts here are always small
        // (one compile's worth), so virtualization buys nothing and a Button's Click event is
        // simpler to reason about and to drive from tests/UI automation than ListView.ItemClick
        // over raw (non-data-bound) item elements.
        var list = new StackPanel();
        foreach (var element in BuildProblemsListContent())
        {
            list.Children.Add(element);
        }
        var scroller = new ScrollViewer
        {
            Content = list,
            VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
            HorizontalScrollBarVisibility = ScrollBarVisibility.Disabled,
        };
        Grid.SetRow(scroller, 1);
        root.Children.Add(scroller);

        ProblemsPanelHost.Child = root;
    }

    private UIElement BuildProblemsHeader()
    {
        var header = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 10, Padding = new Thickness(10, 6, 10, 6) };
        header.Children.Add(new TextBlock { Text = "Problems", FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
        header.Children.Add(new TextBlock
        {
            Text = _shell.ErrorCount == 0 && _shell.WarningCount == 0
                ? "no diagnostics"
                : $"{_shell.ErrorCount} errors, {_shell.WarningCount} warnings",
            Opacity = 0.7,
            VerticalAlignment = VerticalAlignment.Center,
        });

        var closeButton = new Button
        {
            Content = "", // Segoe Fluent Icons: Cancel/X glyph.
            FontFamily = new FontFamily("Segoe Fluent Icons"),
            Padding = new Thickness(6, 2, 6, 2),
            HorizontalAlignment = HorizontalAlignment.Right,
        };
        ToolTipService.SetToolTip(closeButton, "Close Problems panel");
        closeButton.Click += (_, _) => SetProblemsPanelVisible(false);

        var row = new Grid();
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(1, GridUnitType.Star) });
        row.ColumnDefinitions.Add(new ColumnDefinition { Width = GridLength.Auto });
        Grid.SetColumn(header, 0);
        Grid.SetColumn(closeButton, 1);
        row.Children.Add(header);
        row.Children.Add(closeButton);
        return row;
    }

    /// <summary>
    /// One list item per group header (a source file with at least one diagnostic, or
    /// "(no location)" for diagnostics without a SourceRange) followed by its diagnostic
    /// rows, in <see cref="ShellModel.Diagnostics"/> order within each group.
    /// </summary>
    private IEnumerable<UIElement> BuildProblemsListContent()
    {
        if (_shell.Diagnostics.Count == 0)
        {
            yield return new TextBlock { Text = "No diagnostics.", Opacity = 0.65, Margin = new Thickness(10, 6, 10, 6) };
            yield break;
        }

        foreach (var group in _shell.Diagnostics.GroupBy(d => d.Source?.Path ?? "(no location)"))
        {
            yield return new TextBlock
            {
                Text = group.Key,
                FontWeight = Microsoft.UI.Text.FontWeights.SemiBold,
                Opacity = 0.75,
                Margin = new Thickness(10, 8, 10, 2),
            };
            foreach (var diagnostic in group)
            {
                yield return BuildDiagnosticRow(diagnostic);
            }
        }
    }

    private FrameworkElement BuildDiagnosticRow(Diagnostic diagnostic)
    {
        var icon = new FontIcon
        {
            FontFamily = new FontFamily("Segoe Fluent Icons"),
            Glyph = diagnostic.Severity == Severity.error ? "" : "", // ErrorBadge / Warning
            Foreground = new SolidColorBrush(diagnostic.Severity == Severity.error
                ? Colors.Firebrick
                : Colors.DarkGoldenrod),
            FontSize = 14,
            VerticalAlignment = VerticalAlignment.Top,
            Margin = new Thickness(0, 2, 0, 0),
        };

        var messageText = new TextBlock
        {
            Text = diagnostic.Message,
            TextWrapping = TextWrapping.Wrap,
        };
        var body = new StackPanel { Spacing = 2 };
        body.Children.Add(messageText);
        if (DescribeLocation(diagnostic.Source) is { } location)
        {
            body.Children.Add(new TextBlock { Text = location, Opacity = 0.6, FontSize = 12 });
        }

        var content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        content.Children.Add(icon);
        content.Children.Add(body);

        // A Button rather than a StackPanel+ListView.ItemClick: this app's proven pattern for a
        // clickable row in a plain list (MainWindow.ProjectFiles.cs's project-tree buttons) --
        // simpler to reason about and to drive from tests/UI automation than relying on
        // ListView's hit-testing over raw (non-data-bound) item elements.
        var row = new Button
        {
            Content = content,
            HorizontalAlignment = HorizontalAlignment.Stretch,
            HorizontalContentAlignment = HorizontalAlignment.Left,
            Padding = new Thickness(10, 4, 10, 4),
            Background = new SolidColorBrush(Colors.Transparent),
            BorderThickness = new Thickness(0),
        };
        if (diagnostic.Source is null)
        {
            row.IsEnabled = false; // nothing to navigate to without a SourceRange.
        }
        else
        {
            row.Click += (_, _) => NavigateToDiagnostic(diagnostic);
        }
        return row;
    }

    /// <summary>
    /// "line:column" (1-based) computed against the matching open document's current text, or
    /// null if that document is not open (e.g. an include file never opened in this session) or
    /// the byte offset no longer lands on a UTF-8 boundary against it.
    /// </summary>
    private string? DescribeLocation(SourceRange? source)
    {
        if (source is null)
        {
            return null;
        }
        var document = _shell.Documents.FirstOrDefault(d => d.Path == source.Path);
        if (document is null)
        {
            return $"{source.Path} (byte {source.StartByte})";
        }
        if (ByteOffsets.Utf16RangeForUtf8Bytes(document.Text, source.StartByte, source.EndByte) is not (int utf16Start, _))
        {
            return source.Path;
        }

        int line = 1;
        int lastNewline = -1;
        for (int i = 0; i < utf16Start; i++)
        {
            if (document.Text[i] == '\n')
            {
                line++;
                lastNewline = i;
            }
        }
        int column = utf16Start - lastNewline;
        return $"{source.Path}:{line}:{column}";
    }

    private void NavigateToDiagnostic(Diagnostic diagnostic)
    {
        if (diagnostic.Source is not { } source)
        {
            return;
        }
        if (_shell.SwitchActiveDocument(source.Path))
        {
            _editorHost?.RevealSourceRange(source);
        }
    }
}
