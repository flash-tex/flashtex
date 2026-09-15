// name: ShellModelDocumentsTests.cs
// purpose: Tests for ShellModel.Documents.cs's tab lifecycle: opening several
//   in-memory documents, activating an already-open one instead of replacing
//   it, switching the active tab, closing a tab (including re-activating a
//   neighbor, and leaving no active document when the last tab closes), and
//   dirty-flag transitions against the text each document had when opened.
//   Pure in-memory: no worker or edit-ledger executable is configured, so
//   none of this spawns a real child process.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

public class ShellModelDocumentsTests
{
    [Fact]
    public async Task OpenDocumentAsync_MultipleDocuments_TracksTabsAndActivatesTheNewestOne()
    {
        var model = new ShellModel();

        await model.OpenDocumentAsync("a.tex", "alpha");
        await model.OpenDocumentAsync("b.tex", "beta");

        Assert.Equal(new[] { "a.tex", "b.tex" }, model.Documents.Select(d => d.Path));
        Assert.Equal("b.tex", model.ActiveDocumentPath);
    }

    [Fact]
    public async Task OpenDocumentAsync_AlreadyOpenPath_OnlyActivatesItWithoutReplacingItsText()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");
        await model.OpenDocumentAsync("b.tex", "beta");

        await model.OpenDocumentAsync("a.tex", "should not replace the open text");

        Assert.Equal("a.tex", model.ActiveDocumentPath);
        Assert.Equal("alpha", model.Documents.Single(d => d.Path == "a.tex").Text);
        Assert.Equal(2, model.Documents.Count);
    }

    [Fact]
    public void SwitchActiveDocument_UnknownPath_ReturnsFalseAndLeavesActiveUnchanged()
    {
        var model = new ShellModel();

        Assert.False(model.SwitchActiveDocument("missing.tex"));
        Assert.Null(model.ActiveDocumentPath);
    }

    [Fact]
    public async Task SwitchActiveDocument_OpenPath_ActivatesItAndReturnsTrue()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");
        await model.OpenDocumentAsync("b.tex", "beta");

        Assert.True(model.SwitchActiveDocument("a.tex"));
        Assert.Equal("a.tex", model.ActiveDocumentPath);
    }

    [Fact]
    public async Task CloseDocumentAsync_ActiveTab_ActivatesANeighboringTab()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");
        await model.OpenDocumentAsync("b.tex", "beta");
        await model.OpenDocumentAsync("c.tex", "gamma");
        model.SwitchActiveDocument("b.tex");

        await model.CloseDocumentAsync("b.tex");

        Assert.DoesNotContain(model.Documents, d => d.Path == "b.tex");
        Assert.Equal("c.tex", model.ActiveDocumentPath);
    }

    [Fact]
    public async Task CloseDocumentAsync_LastOpenTab_LeavesNoActiveDocument()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");

        await model.CloseDocumentAsync("a.tex");

        Assert.Empty(model.Documents);
        Assert.Null(model.ActiveDocumentPath);
    }

    [Fact]
    public async Task CloseDocumentAsync_UnknownPath_IsANoOp()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");

        await model.CloseDocumentAsync("missing.tex");

        Assert.Single(model.Documents);
    }

    [Fact]
    public async Task UpdateDocumentText_TogglesDirtyAgainstTheOpenedBaselineAndBumpsRevision()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");
        Assert.False(model.Documents.Single().IsDirty);
        Assert.Equal(1, model.Documents.Single().Revision);

        model.UpdateDocumentText("a.tex", "alpha beta");
        Assert.True(model.Documents.Single().IsDirty);
        Assert.Equal(2, model.Documents.Single().Revision);

        model.UpdateDocumentText("a.tex", "alpha"); // back to exactly the text it was opened with
        Assert.False(model.Documents.Single().IsDirty);
        Assert.Equal(3, model.Documents.Single().Revision);
    }

    [Fact]
    public async Task UpdateDocumentText_SameTextAsAlreadyStored_IsANoOp()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");

        model.UpdateDocumentText("a.tex", "alpha");

        Assert.Equal(1, model.Documents.Single().Revision);
        Assert.False(model.Documents.Single().IsDirty);
    }

    [Fact]
    public void UpdateDocumentText_UnknownPath_IsANoOp()
    {
        var model = new ShellModel();

        model.UpdateDocumentText("missing.tex", "text"); // must not throw

        Assert.Empty(model.Documents);
    }

    [Fact]
    public async Task MarkDocumentSaved_ChangesTheDirtyBaselineOnlyAfterTheCallerHasPersistedIt()
    {
        var model = new ShellModel();
        await model.OpenDocumentAsync("a.tex", "alpha");
        model.UpdateDocumentText("a.tex", "alpha beta");

        Assert.True(model.Documents.Single().IsDirty);
        Assert.True(model.MarkDocumentSaved("a.tex"));
        Assert.False(model.Documents.Single().IsDirty);

        model.UpdateDocumentText("a.tex", "alpha gamma");
        Assert.True(model.Documents.Single().IsDirty);
        model.UpdateDocumentText("a.tex", "alpha beta");
        Assert.False(model.Documents.Single().IsDirty);
    }

    [Fact]
    public void MarkDocumentSaved_UnknownPathReturnsFalse()
    {
        var model = new ShellModel();

        Assert.False(model.MarkDocumentSaved("missing.tex"));
    }
}
