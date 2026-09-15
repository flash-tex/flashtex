// name: ShellModelEditLedgerTests.cs
// purpose: Tests for ShellModel.EditLedger.cs against the REAL
//   flashtex-edit-ledger binary (crates/edit-ledger, `cargo build --release`):
//   opening a document seeds its own durable store, an edit round-trips
//   through replace_document, Undo/Redo move real ledger history, and closing
//   a document disposes its store. Also covers the safety-critical
//   session-staleness guard: after simulating a helper restart
//   (ReattachEditLedgerStoreAsync), an observation captured beforehand must be
//   rejected rather than silently trusted, mirroring
//   FlashTeX.Ipc.Tests/EditLedgerClientTests.cs's own restart test.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Ipc;
using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

public class ShellModelEditLedgerTests
{
    private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(30);

    private static void RequireEditLedgerBuilt() =>
        Assert.True(
            File.Exists(TestPaths.EditLedgerExePath),
            $"expected a release build of flashtex-edit-ledger at '{TestPaths.EditLedgerExePath}' " +
            "(run `cargo build --release` from crates/edit-ledger first)");

    [Fact]
    public async Task UpdateDocumentText_RoundTripsThroughRealEditLedgerAndSupportsUndoRedo()
    {
        RequireEditLedgerBuilt();
        var model = new ShellModel(editLedgerExecutablePath: TestPaths.EditLedgerExePath);
        await model.OpenDocumentAsync("main.tex", "hello world");

        model.UpdateDocumentText("main.tex", "hello there");
        Assert.NotNull(model.LastLedgerReplaceTask);
        await model.LastLedgerReplaceTask!.WaitAsync(Timeout);

        Assert.True(model.CanUndoActiveDocument);
        Assert.False(model.CanRedoActiveDocument);

        await model.UndoActiveDocumentAsync().WaitAsync(Timeout);
        Assert.Equal("hello world", model.Documents.Single(d => d.Path == "main.tex").Text);
        Assert.False(model.CanUndoActiveDocument);
        Assert.True(model.CanRedoActiveDocument);
        Assert.False(model.Documents.Single(d => d.Path == "main.tex").IsDirty); // back to exactly the opened text

        await model.RedoActiveDocumentAsync().WaitAsync(Timeout);
        Assert.Equal("hello there", model.Documents.Single(d => d.Path == "main.tex").Text);
        Assert.True(model.CanUndoActiveDocument);
        Assert.False(model.CanRedoActiveDocument);

        await model.CloseDocumentAsync("main.tex");
    }

    [Fact]
    public async Task UndoActiveDocumentAsync_WithNoEditLedgerConfigured_IsANoOp()
    {
        var model = new ShellModel(); // no editLedgerExecutablePath: documents open without a store
        await model.OpenDocumentAsync("main.tex", "hello world");

        await model.UndoActiveDocumentAsync().WaitAsync(Timeout); // must not throw

        Assert.Equal("hello world", model.Documents.Single().Text);
        Assert.False(model.CanUndoActiveDocument);
    }

    [Fact]
    public async Task CloseDocumentAsync_DisposesTheDocumentsEditLedgerStore()
    {
        RequireEditLedgerBuilt();
        var model = new ShellModel(editLedgerExecutablePath: TestPaths.EditLedgerExePath);
        await model.OpenDocumentAsync("main.tex", "hello world");
        Assert.NotNull(model.LastLedgerObservation("main.tex")); // Initialize already observed a reply

        await model.CloseDocumentAsync("main.tex");

        Assert.Null(model.LastLedgerObservation("main.tex"));
    }

    /// <summary>
    /// The safety-critical guard: simulating a helper-process restart (disposing the
    /// current client and starting a fresh one against the same store directory) begins
    /// a new session. An observation captured from before that restart must be actively
    /// rejected by <see cref="ShellModel.AssertLedgerObservationIsCurrent"/>, never
    /// silently treated as still current.
    /// </summary>
    [Fact]
    public async Task ReattachEditLedgerStoreAsync_StartsANewSessionAndRejectsAPreRestartObservation()
    {
        RequireEditLedgerBuilt();
        var model = new ShellModel(editLedgerExecutablePath: TestPaths.EditLedgerExePath);
        await model.OpenDocumentAsync("main.tex", "hello world");
        model.UpdateDocumentText("main.tex", "hello there");
        await model.LastLedgerReplaceTask!.WaitAsync(Timeout);

        EditLedgerObservation beforeRestart = model.LastLedgerObservation("main.tex")
            ?? throw new InvalidOperationException("expected an observation after a successful edit");

        await model.ReattachEditLedgerStoreAsync("main.tex").WaitAsync(Timeout);

        // The durable text survives the simulated restart intact...
        Assert.Equal("hello there", model.Documents.Single(d => d.Path == "main.tex").Text);

        // ...but the new client is a brand new session: never the same ID.
        EditLedgerObservation afterRestart = model.LastLedgerObservation("main.tex")
            ?? throw new InvalidOperationException("expected an observation after reattaching");
        Assert.NotEqual(beforeRestart.SessionId, afterRestart.SessionId);

        // The guard must actively reject the stale, pre-restart observation...
        EditLedgerStaleSessionException ex = Assert.Throws<EditLedgerStaleSessionException>(
            () => model.AssertLedgerObservationIsCurrent("main.tex", beforeRestart));
        Assert.Contains(beforeRestart.SessionId, ex.Message, StringComparison.Ordinal);

        // ...while the fresh session's own current observation is, of course, still trusted.
        model.AssertLedgerObservationIsCurrent("main.tex", afterRestart);

        await model.CloseDocumentAsync("main.tex");
    }

    [Fact]
    public void AssertLedgerObservationIsCurrent_NoLedgerAttached_ThrowsInvalidOperationException()
    {
        var model = new ShellModel();
        var fakeObservation = new EditLedgerObservation("session", 1, DocumentRevision: 1, DocumentSha256: null);

        Assert.Throws<InvalidOperationException>(
            () => model.AssertLedgerObservationIsCurrent("missing.tex", fakeObservation));
    }
}
