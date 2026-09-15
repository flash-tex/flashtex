// name: MainWindow.Toolbar.cs
// purpose: Builds the toolbar for the most-used actions (compile, undo/redo,
//   export placeholder), backed by the same FlashTeX.Shell.CommandRegistry
//   table as the menu bar (MainWindow.Menu.cs) rather than a second
//   hand-written command list. Uses a plain StackPanel of Buttons
//   (MainWindow.xaml's Toolbar) instead of a CommandBar: see that XAML
//   comment for why (CommandBar's overflow layout silently dropped buttons
//   without a generated PRI file, which this unpackaged build does not
//   produce). Also owns RefreshCommandEnabledState, the one place that
//   re-evaluates every command's CanExecute against the live ShellModel and
//   applies it to both the menu items and these toolbar buttons.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;
using Microsoft.UI.Xaml.Controls;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    /// <summary>Commands shown as toolbar buttons, in display order.</summary>
    private static readonly string[] ToolbarCommandIds =
    {
        CommandIds.Compile, CommandIds.Undo, CommandIds.Redo, CommandIds.ExportPdf,
    };

    /// <summary>
    /// Segoe Fluent Icons glyphs (the font WinUI3 ships by default) for each toolbar
    /// command, as \uXXXX escapes rather than literal characters: these are Private Use
    /// Area code points (icon glyph slots), which are invisible/unreliable as literal
    /// source text across editors and encodings.
    /// </summary>
    private static readonly IReadOnlyDictionary<string, string> ToolbarGlyphsByCommandId = new Dictionary<string, string>(StringComparer.Ordinal)
    {
        [CommandIds.Compile] = "", // Play
        [CommandIds.Undo] = "", // Undo
        [CommandIds.Redo] = "", // Redo
        [CommandIds.ExportPdf] = "", // Save
    };

    private readonly Dictionary<string, Button> _toolbarButtonsById = new(StringComparer.Ordinal);

    private void BuildToolbar()
    {
        foreach (var commandId in ToolbarCommandIds)
        {
            var command = CommandRegistry.All.First(c => c.Id == commandId);
            var button = CreateToolbarButton(command);
            _toolbarButtonsById[commandId] = button;
            Toolbar.Children.Add(button);
        }
    }

    private Button CreateToolbarButton(Command command)
    {
        var content = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6 };
        content.Children.Add(new FontIcon { Glyph = ToolbarGlyphsByCommandId[command.Id], FontSize = 14 });
        content.Children.Add(new TextBlock { Text = command.Title, VerticalAlignment = Microsoft.UI.Xaml.VerticalAlignment.Center });

        var button = new Button { Content = content, Padding = new Microsoft.UI.Xaml.Thickness(10, 6, 10, 6) };
        ToolTipService.SetToolTip(button, command.Description);
        button.Click += (_, _) => ExecuteCommand(command.Id);
        return button;
    }

    /// <summary>
    /// Re-evaluates every command's enabled state against the live ShellModel and applies it
    /// to its menu item and (if shown) toolbar button. Called after state-changing operations
    /// rather than on a menu-opening event (MenuBarItem exposes none in WinUI3) or a full data
    /// binding (ShellModel/ICommandContext predicates are plain C# funcs, not bindable
    /// properties).
    /// </summary>
    private void RefreshCommandEnabledState()
    {
        foreach (var command in CommandRegistry.All)
        {
            var canExecute = command.CanExecute(_shell);
            _menuItemsById[command.Id].IsEnabled = canExecute;
            if (_toolbarButtonsById.TryGetValue(command.Id, out var button))
            {
                button.IsEnabled = canExecute;
            }
        }
    }
}
