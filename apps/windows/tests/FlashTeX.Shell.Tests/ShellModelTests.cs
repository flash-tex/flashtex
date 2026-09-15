// name: ShellModelTests.cs
// purpose: Integration test that FlashTeX.Shell.ShellModel's minimal skeleton
//   composes correctly with ShellChrome: word count and the active document's
//   dirty flag mirror through to the chrome exactly as ShellModel wires them.
//   This is not a test of real app behavior (there isn't any yet); it exists
//   to prove the composition, per the task's demonstration requirement.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

public class ShellModelTests
{
    private static (ShellModel Model, ManualChromeScheduler Scheduler) CreateModel()
    {
        var scheduler = new ManualChromeScheduler();
        var chrome = new ShellChrome(scheduler);
        return (new ShellModel(chrome), scheduler);
    }

    [Fact]
    public void WordCountChange_MirrorsThroughToChrome()
    {
        var (model, scheduler) = CreateModel();

        model.WordCount = 42;
        scheduler.Fire();

        Assert.Equal(42, model.Chrome.Get<int>(ShellChromeKeys.WordCount));
    }

    [Fact]
    public void ActiveDocumentDirtyFlag_MirrorsThroughToChrome()
    {
        var (model, scheduler) = CreateModel();
        model.Documents.Add(new ShellDocument("main.tex", "\\documentclass{article}", Revision: 1, IsDirty: true));
        model.ActiveDocumentPath = "main.tex";
        scheduler.Fire();

        Assert.True(model.Chrome.Get<bool>(ShellChromeKeys.ActiveDocumentDirty));

        model.Documents[0] = model.Documents[0] with { IsDirty = false };
        scheduler.Fire();

        Assert.False(model.Chrome.Get<bool>(ShellChromeKeys.ActiveDocumentDirty));
    }

    [Fact]
    public void NoActiveDocument_DirtyFlagMirrorsFalse()
    {
        var (model, scheduler) = CreateModel();

        model.ActiveDocumentPath = "missing.tex";
        scheduler.Fire();

        Assert.False(model.Chrome.Get<bool>(ShellChromeKeys.ActiveDocumentDirty));
    }

    [Fact]
    public void RapidWordCountChanges_CoalesceIntoOneRefresh()
    {
        var (model, scheduler) = CreateModel();

        model.WordCount = 1;
        model.WordCount = 2;
        model.WordCount = 3;

        Assert.Equal(1, scheduler.PendingCount);
        scheduler.Fire();
        Assert.Equal(3, model.Chrome.Get<int>(ShellChromeKeys.WordCount));
    }

    [Fact]
    public void DefaultConstructor_UsesARealShellChrome()
    {
        var model = new ShellModel();
        Assert.NotNull(model.Chrome);
        Assert.Equal(13.0, model.EditorFontSize);
    }
}
