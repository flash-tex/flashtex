// name: CommandRegistryTests.cs
// purpose: Tests for FlashTeX.Shell.CommandRegistry: table content and
//   default shortcuts, the ranked search's tier ordering (title > menu item >
//   category/shortcut > description, ported from CommandPaletteModel.rows(matching:)
//   in CommandPalette.swift), CanExecute gating, and Execute dispatch.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

public class CommandRegistryTests
{
    private sealed class FakeCommandContext : ICommandContext
    {
        public bool HasActiveDocument { get; set; }

        public bool HasCompileResult { get; set; }

        public bool HasWorkerAttached { get; set; }

        public List<string> PerformedActions { get; } = new();

        public void PerformAction(string commandId)
        {
            PerformedActions.Add(commandId);
        }
    }

    [Fact]
    public void All_ContainsExpectedCommandsWithDefaultShortcuts()
    {
        var byId = CommandRegistry.All.ToDictionary(c => c.Id);

        Assert.Equal(new KeyboardShortcut(ShortcutModifiers.Ctrl, "B"), byId[CommandIds.Compile].DefaultShortcut);
        Assert.Equal(new KeyboardShortcut(ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "P"), byId[CommandIds.CommandPalette].DefaultShortcut);
        Assert.Equal(new KeyboardShortcut(ShortcutModifiers.Ctrl, "Z"), byId[CommandIds.Undo].DefaultShortcut);
        Assert.Equal(new KeyboardShortcut(ShortcutModifiers.Ctrl, "Y"), byId[CommandIds.Redo].DefaultShortcut);
        Assert.Equal(new KeyboardShortcut(ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "M"), byId[CommandIds.ToggleProblems].DefaultShortcut);
        Assert.Equal(new KeyboardShortcut(ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "I"), byId[CommandIds.ToggleCaptures].DefaultShortcut);
        Assert.Equal(KeyboardShortcut.None, byId[CommandIds.ExportPdfExact].DefaultShortcut);
        Assert.Equal("Ctrl+B", byId[CommandIds.Compile].DefaultShortcut.ToString());
    }

    [Fact]
    public void All_HasNoDuplicateIds()
    {
        var ids = CommandRegistry.All.Select(c => c.Id).ToList();
        Assert.Equal(ids.Count, ids.Distinct(StringComparer.Ordinal).Count());
    }

    [Fact]
    public void Search_EmptyQuery_ReturnsEveryCommandInTableOrder()
    {
        var results = CommandRegistry.Search(string.Empty);
        Assert.Equal(CommandRegistry.All, results);

        var whitespaceOnly = CommandRegistry.Search("   ");
        Assert.Equal(CommandRegistry.All, whitespaceOnly);
    }

    [Fact]
    public void Search_TitleTierRanksAboveCategoryTier()
    {
        // "edit" appears in the Title of the three editor-font-size commands
        // ("Editor" contains "edit"), in the Category ("Edit") of Undo/Redo/Find
        // (whose titles do not contain "edit"), and nowhere else.
        var results = CommandRegistry.Search("edit").Select(c => c.Id).ToList();

        Assert.Equal(6, results.Count);
        Assert.Equal(
            new[] { CommandIds.IncreaseEditorFontSize, CommandIds.DecreaseEditorFontSize, CommandIds.ResetEditorFontSize },
            results.Take(3));
        Assert.Equal(new[] { CommandIds.Undo, CommandIds.Redo, CommandIds.Find }, results.Skip(3));
    }

    [Fact]
    public void Search_MenuItemOnlyMatch_IsIncludedAtMenuItemTier()
    {
        // "begin" occurs in GoToMatching's menu-item text ("\begin/\end…") but
        // not its title, and in no other command at all.
        var results = CommandRegistry.Search("begin");
        Assert.Single(results);
        Assert.Equal(CommandIds.GoToMatching, results[0].Id);
    }

    [Fact]
    public void Search_DescriptionOnlyMatch_IsIncludedAtWeakestTier()
    {
        // "buffers" occurs only in Compile's description ("the current
        // buffers"), not its title, menu item, category or shortcut, and in
        // no other command's fields at all.
        var results = CommandRegistry.Search("buffers");
        Assert.Single(results);
        Assert.Equal(CommandIds.Compile, results[0].Id);
    }

    [Fact]
    public void Search_TermMatchingNothing_ExcludesTheRow()
    {
        var results = CommandRegistry.Search("xyznonexistentterm");
        Assert.Empty(results);
    }

    [Fact]
    public void Search_EveryTermMustMatch()
    {
        // "compile" alone also matches OpenLatexFile's description ("compiles
        // it when a worker is attached"), but "sends" occurs only in
        // Compile's description, so requiring both terms excludes OpenLatexFile.
        var results = CommandRegistry.Search("compile sends");
        Assert.Single(results);
        Assert.Equal(CommandIds.Compile, results[0].Id);
    }

    [Theory]
    [InlineData(CommandIds.Compile, false, false, false, false)]
    [InlineData(CommandIds.Compile, false, false, true, true)]
    [InlineData(CommandIds.ExportPdf, false, false, false, false)]
    [InlineData(CommandIds.ExportPdf, false, true, false, true)]
    [InlineData(CommandIds.NewFile, false, false, false, false)]
    [InlineData(CommandIds.NewFile, true, false, false, true)]
    [InlineData(CommandIds.CommandPalette, false, false, false, true)]
    public void CanExecute_GatesOnContextState(string commandId, bool hasActiveDocument, bool hasCompileResult, bool hasWorkerAttached, bool expected)
    {
        var command = CommandRegistry.All.Single(c => c.Id == commandId);
        var context = new FakeCommandContext
        {
            HasActiveDocument = hasActiveDocument,
            HasCompileResult = hasCompileResult,
            HasWorkerAttached = hasWorkerAttached,
        };

        Assert.Equal(expected, command.CanExecute(context));
    }

    [Fact]
    public void Execute_UnknownCommandId_ReturnsFalseAndPerformsNothing()
    {
        var context = new FakeCommandContext();
        Assert.False(CommandRegistry.Execute("not-a-real-command", context));
        Assert.Empty(context.PerformedActions);
    }

    [Fact]
    public void Execute_NotExecutable_ReturnsFalseAndPerformsNothing()
    {
        var context = new FakeCommandContext { HasWorkerAttached = false };
        Assert.False(CommandRegistry.Execute(CommandIds.Compile, context));
        Assert.Empty(context.PerformedActions);
    }

    [Fact]
    public void Execute_Executable_ForwardsToPerformActionWithTheCommandId()
    {
        var context = new FakeCommandContext { HasWorkerAttached = true };
        Assert.True(CommandRegistry.Execute(CommandIds.Compile, context));
        Assert.Equal(new[] { CommandIds.Compile }, context.PerformedActions);
    }
}
