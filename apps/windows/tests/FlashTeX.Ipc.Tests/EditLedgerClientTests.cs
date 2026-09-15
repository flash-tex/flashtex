// name: EditLedgerClientTests.cs
// purpose: End-to-end tests of EditLedgerClient against the REAL flashtex-edit-ledger
//   binary (cargo build --release from crates/edit-ledger), driving the documented
//   initialize -> apply -> confirm -> undo/redo sequence, a durable command
//   failure's error payload, and — the safety-critical case — that a helper
//   process restart starts a fresh session whose observation a caller must
//   explicitly re-validate rather than trust blindly.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Ipc;
using FlashTeX.Protocol.EditLedgerV1;
using FlashTeX.Protocol.TransferV1;
using Xunit;

namespace FlashTeX.Ipc.Tests;

public class EditLedgerClientTests
{
    private const string ProjectId = "proj";
    private const string DocumentPath = "main.tex";

    [Fact]
    public async Task InitializeApplyConfirmUndoRedo_RoundTripsAgainstTheRealBinary()
    {
        Assert.True(
            File.Exists(TestPaths.EditLedgerExePath),
            $"expected a release build of flashtex-edit-ledger at '{TestPaths.EditLedgerExePath}' " +
            "(run `cargo build --release` from crates/edit-ledger first)");

        DirectoryInfo store = Directory.CreateTempSubdirectory("flashtex-edit-ledger-test-");
        try
        {
            var stderrLines = new List<string>();
            await using EditLedgerClient client = EditLedgerClient.Start(
                TestPaths.EditLedgerExePath, store.FullName, line => stderrLines.Add(line));

            EditLedgerDocument seed = EditLedgerDocument.Create(ProjectId, DocumentPath, 1, "hello world");
            InitializeResult initialized = await client.InitializeAsync("init", seed).WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal("hello world", initialized.Document.Text);
            Assert.Equal(1UL, initialized.Document.Revision);

            var edit = new CaptureEdit(
                CaptureId: "capture-1",
                EditId: "edit-1",
                ProjectId: ProjectId,
                Path: DocumentPath,
                ExpectedRevision: 1,
                StartByte: 6,
                EndByte: 11,
                RemovedText: "world",
                Replacement: "there",
                DocumentBeforeSha256: seed.SourceSha256);
            ApplyResult applied = await client.ApplyAsync("apply", edit).WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal("hello there", applied.Document.Text);
            Assert.Equal(2UL, applied.Document.Revision);
            Assert.Equal(2, applied.Receipt.NewRevision);

            ConfirmResult confirmed = await client.ConfirmAsync("confirm", applied.Receipt).WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal(applied.Receipt.EditId, confirmed.Confirmed.EditId);

            HistoryStatusResult history = await client.HistoryStatusAsync("history-1").WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Single(history.UndoLabels);
            Assert.Empty(history.RedoLabels);

            HistoryResult undone = await client
                .UndoAsync("undo-1", new HistoryMove("undo-1", applied.Document.Revision, applied.Document.SourceSha256))
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal("hello world", undone.Document.Text);
            Assert.True(undone.CanRedo);
            Assert.False(undone.CanUndo);

            HistoryResult redone = await client
                .RedoAsync("redo-1", new HistoryMove("redo-1", undone.Document.Revision, undone.Document.SourceSha256))
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal("hello there", redone.Document.Text);
            Assert.True(redone.CanUndo);
            Assert.False(redone.CanRedo);

            StatusResult status = await client.StatusAsync("status-1").WaitAsync(TimeSpan.FromSeconds(30));
            Assert.NotNull(status.Document);
            Assert.Equal("hello there", status.Document!.Text);
            // The single applied edit was confirmed above, so nothing remains pending.
            Assert.Empty(status.PendingReceipts);
        }
        finally
        {
            store.Delete(recursive: true);
        }
    }

    [Fact]
    public async Task Apply_WithAStaleSourceHash_ThrowsEditLedgerErrorException()
    {
        Assert.True(File.Exists(TestPaths.EditLedgerExePath));

        DirectoryInfo store = Directory.CreateTempSubdirectory("flashtex-edit-ledger-test-");
        try
        {
            await using EditLedgerClient client = EditLedgerClient.Start(TestPaths.EditLedgerExePath, store.FullName, _ => { });

            EditLedgerDocument seed = EditLedgerDocument.Create(ProjectId, DocumentPath, 1, "hello world");
            await client.InitializeAsync("init", seed).WaitAsync(TimeSpan.FromSeconds(30));

            var staleEdit = new CaptureEdit(
                CaptureId: "capture-stale",
                EditId: "edit-stale",
                ProjectId: ProjectId,
                Path: DocumentPath,
                ExpectedRevision: 1,
                StartByte: 6,
                EndByte: 11,
                RemovedText: "world",
                Replacement: "there",
                DocumentBeforeSha256: new string('0', 64));

            EditLedgerErrorException ex = await Assert.ThrowsAsync<EditLedgerErrorException>(async () =>
                await client.ApplyAsync("apply-stale", staleEdit).WaitAsync(TimeSpan.FromSeconds(30)));
            Assert.Equal("source_hash_conflict", ex.Code);
            Assert.Equal("apply-stale", ex.RequestId);
        }
        finally
        {
            store.Delete(recursive: true);
        }
    }

    /// <summary>
    /// The safety-critical guard: a restart of the underlying helper process
    /// (here, disposing one client and starting a fresh one against the same
    /// store directory) begins a new session. An observation captured from the
    /// old session must be rejected by the new client, rather than silently
    /// treated as still current — exactly the hazard crates/edit-ledger/README.md
    /// warns "native consumers must" guard against.
    /// </summary>
    [Fact]
    public async Task AssertObservationIsCurrent_RejectsAnObservationFromBeforeAHelperRestart()
    {
        Assert.True(File.Exists(TestPaths.EditLedgerExePath));

        DirectoryInfo store = Directory.CreateTempSubdirectory("flashtex-edit-ledger-test-");
        try
        {
            EditLedgerObservation observationBeforeRestart;
            EditLedgerDocument seed = EditLedgerDocument.Create(ProjectId, DocumentPath, 1, "hello world");

            await using (EditLedgerClient first = EditLedgerClient.Start(TestPaths.EditLedgerExePath, store.FullName, _ => { }))
            {
                await first.InitializeAsync("init", seed).WaitAsync(TimeSpan.FromSeconds(30));
                var edit = new CaptureEdit(
                    CaptureId: "capture-1",
                    EditId: "edit-1",
                    ProjectId: ProjectId,
                    Path: DocumentPath,
                    ExpectedRevision: 1,
                    StartByte: 6,
                    EndByte: 11,
                    RemovedText: "world",
                    Replacement: "there",
                    DocumentBeforeSha256: seed.SourceSha256);
                await first.ApplyAsync("apply", edit).WaitAsync(TimeSpan.FromSeconds(30));

                observationBeforeRestart = first.LastObservation
                    ?? throw new InvalidOperationException("expected an observation after a successful apply");
            }
            // `first` is now disposed: its helper process was killed, simulating
            // an actual crash/restart of the durable-ledger host for this store.

            await using EditLedgerClient second = EditLedgerClient.Start(TestPaths.EditLedgerExePath, store.FullName, _ => { });
            StatusResult statusAfterRestart = await second.StatusAsync("status-after-restart").WaitAsync(TimeSpan.FromSeconds(30));

            // The durable document itself survives the restart intact...
            Assert.Equal("hello there", statusAfterRestart.Document!.Text);

            // ...but the new process is a brand new session: never the same ID.
            EditLedgerObservation observationAfterRestart = second.LastObservation!.Value;
            Assert.NotEqual(observationBeforeRestart.SessionId, observationAfterRestart.SessionId);

            // The guard must actively reject the stale, pre-restart observation...
            EditLedgerStaleSessionException ex = Assert.Throws<EditLedgerStaleSessionException>(
                () => second.AssertObservationIsCurrent(observationBeforeRestart));
            Assert.Contains(observationBeforeRestart.SessionId, ex.Message, StringComparison.Ordinal);

            // ...while the new client's own current observation is, of course, still trusted.
            second.AssertObservationIsCurrent(observationAfterRestart);
        }
        finally
        {
            store.Delete(recursive: true);
        }
    }
}
