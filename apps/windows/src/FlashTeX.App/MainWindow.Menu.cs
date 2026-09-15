// name: MainWindow.Menu.cs
// purpose: Builds the MenuBar (File/Edit/View/Navigate) and window-scoped
//   KeyboardAccelerators from FlashTeX.Shell.CommandRegistry's shared command
//   table, so the menu, the toolbar (MainWindow.Toolbar.cs) and a future
//   command palette all read the same source of truth rather than duplicating
//   a hand-written menu. Ported from the DESIGN of FlashTeXMacApp.swift's
//   `.commands{}` block (File/Edit/View/Navigate groupings), not copied
//   verbatim: the Mac source hand-writes ~40 SwiftUI Button/CommandGroup
//   entries against AppKit menu commands, where this reads CommandRegistry.All
//   (the ~30-command core already shared with the palette) and generates one
//   MenuFlyoutItem/KeyboardAccelerator pair per command.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private readonly Dictionary<string, MenuFlyoutItem> _menuItemsById = new(StringComparer.Ordinal);

    private void BuildMenuBar()
    {
        foreach (var group in CommandRegistry.All.GroupBy(c => c.Category))
        {
            var menuBarItem = new MenuBarItem { Title = group.Key };
            foreach (var command in group)
            {
                var item = CreateMenuFlyoutItem(command);
                _menuItemsById[command.Id] = item;
                menuBarItem.Items.Add(item);
            }
            AppMenuBar.Items.Add(menuBarItem);
        }
    }

    private MenuFlyoutItem CreateMenuFlyoutItem(Command command)
    {
        var item = new MenuFlyoutItem
        {
            Text = command.Title,
            KeyboardAcceleratorTextOverride = command.DefaultShortcut.HasKey ? command.DefaultShortcut.ToString() : null,
        };
        item.Click += (_, _) => ExecuteCommand(command.Id);
        return item;
    }

    private void BuildKeyboardAccelerators()
    {
        foreach (var command in CommandRegistry.All)
        {
            if (!KeyboardShortcutTranslator.TryTranslate(command.DefaultShortcut, out var modifiers, out var key))
            {
                continue;
            }

            var accelerator = new KeyboardAccelerator { Key = key, Modifiers = modifiers };
            accelerator.Invoked += (_, args) => args.Handled = ExecuteCommand(command.Id);
            RootGrid.KeyboardAccelerators.Add(accelerator);
        }
    }

    /// <summary>
    /// The one dispatch point every menu item, toolbar button and keyboard accelerator calls
    /// through; also refreshes enabled state immediately since a run can change what else is now
    /// allowed (e.g. Compile enables Export once a result exists). Export commands (MainWindow.Export.cs)
    /// are special-cased here rather than forwarded to <see cref="CommandRegistry.Execute"/>/
    /// <c>ShellModel.PerformAction</c>: they need a file-save picker and a result dialog, both
    /// UI/window concerns ShellModel deliberately has no dependency on.
    /// </summary>
    private bool ExecuteCommand(string commandId)
    {
        var didRun = IsExportCommand(commandId)
            ? TryStartExport(commandId)
            : IsProjectFileCommand(commandId)
                ? TryStartProjectFileCommand(commandId)
                : commandId == CommandIds.CommandPalette
                    ? TryStartCommandPalette()
                : commandId == CommandIds.ToggleProblems
                    ? ToggleProblemsPanel()
                : commandId == CommandIds.ProjectSearch
                    ? TryStartProjectSearch()
                : commandId == CommandIds.CitationRename
                    ? TryStartCitationRename()
                : commandId == CommandIds.ToggleEditHistory
                    ? TryToggleEditHistory()
                : CommandRegistry.Execute(commandId, _shell);
        RefreshCommandEnabledState();
        return didRun;
    }
}
