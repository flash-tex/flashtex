// name: ShellModel.EditLedger.cs
// purpose: Durable undo/redo for ShellModel, one flashtex-edit-ledger store
//   per open document (in a temp directory for now; real store-path
//   management belongs to a project/workspace concept not yet built).
//   Initializes a store when a document opens, serializes its text changes
//   through replace_document (queuing the newest buffer behind an in-flight
//   call rather than racing two requests against the same expected
//   revision/hash), and routes Undo/Redo through the ledger's own history
//   move. Also exposes the session-staleness guard EditLedgerClient requires
//   (crates/edit-ledger/README.md: "native consumers must compare the
//   current session identity before updating UI and reject older
//   sequence/revision observations") so a caller holding an observation from
//   before a helper restart gets an explicit, unswallowed exception rather
//   than silently stale state.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Ipc;
using FlashTeX.Protocol.EditLedgerV1;

namespace FlashTeX.Shell;

public partial class ShellModel
{
    private const string EditLedgerProjectId = "flashtex-shell";
    private const ulong InitialLedgerRevision = 1;

    private readonly Dictionary<string, EditLedgerClient> _ledgerClients = new(StringComparer.Ordinal);
    private readonly Dictionary<string, EditLedgerDocument> _ledgerDocuments = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> _ledgerStoreDirectories = new(StringComparer.Ordinal);
    private readonly Dictionary<string, bool> _ledgerReplaceInFlight = new(StringComparer.Ordinal);
    private readonly Dictionary<string, string> _ledgerQueuedText = new(StringComparer.Ordinal);
    private readonly Dictionary<string, (bool CanUndo, bool CanRedo)> _ledgerHistoryState = new(StringComparer.Ordinal);

    /// <summary>The most recently started undo/redo move, for tests to await; null before the first one.</summary>
    public Task? LastUndoRedoTask { get; private set; }

    /// <summary>The most recently started edit-ledger replace-document call, for tests to await; null before the first edit.</summary>
    public Task? LastLedgerReplaceTask { get; private set; }

    /// <summary>Whether the active document has a durable edit to undo (false when it has no edit-ledger store).</summary>
    public bool CanUndoActiveDocument =>
        ActiveDocumentPath is { } path && _ledgerHistoryState.TryGetValue(path, out var state) && state.CanUndo;

    /// <summary>Whether the active document has a durable edit to redo (false when it has no edit-ledger store).</summary>
    public bool CanRedoActiveDocument =>
        ActiveDocumentPath is { } path && _ledgerHistoryState.TryGetValue(path, out var state) && state.CanRedo;

    /// <summary>Moves the active document's edit-ledger history one entry backward. A no-op if it has no ledger store.</summary>
    public Task UndoActiveDocumentAsync(CancellationToken cancellationToken = default)
    {
        var task = MoveHistoryAsync(isUndo: true, cancellationToken);
        LastUndoRedoTask = task;
        return task;
    }

    /// <summary>Moves the active document's edit-ledger history one entry forward. A no-op if it has no ledger store.</summary>
    public Task RedoActiveDocumentAsync(CancellationToken cancellationToken = default)
    {
        var task = MoveHistoryAsync(isUndo: false, cancellationToken);
        LastUndoRedoTask = task;
        return task;
    }

    /// <summary>
    /// Simulates a helper-process restart for <paramref name="path"/>'s edit-ledger store:
    /// disposes the current client and starts a fresh one against the same store
    /// directory, then re-reads the durable document. The fresh client begins a brand
    /// new session (crates/edit-ledger/README.md), so any <see cref="EditLedgerObservation"/>
    /// captured before this call is no longer current — see <see cref="AssertLedgerObservationIsCurrent"/>.
    /// </summary>
    public async Task ReattachEditLedgerStoreAsync(string path, CancellationToken cancellationToken = default)
    {
        if (_editLedgerExecutablePath is not { Length: > 0 } executablePath)
        {
            throw new InvalidOperationException("this shell was constructed without an edit-ledger executable path");
        }
        if (!_ledgerStoreDirectories.TryGetValue(path, out var storeDirectory))
        {
            throw new InvalidOperationException($"no edit-ledger store is tracked for '{path}'");
        }

        if (_ledgerClients.Remove(path, out var oldClient))
        {
            await oldClient.DisposeAsync().ConfigureAwait(true);
        }

        var client = EditLedgerClient.Start(executablePath, storeDirectory, _ => { });
        StatusResult status = await client
            .StatusAsync($"status-{Guid.NewGuid():N}", cancellationToken)
            .ConfigureAwait(true);
        _ledgerClients[path] = client;
        if (status.Document is { } document)
        {
            _ledgerDocuments[path] = document;
        }
    }

    /// <summary>
    /// The safety-critical guard itself: throws <see cref="EditLedgerStaleSessionException"/>
    /// (never silently swallowed) when <paramref name="observation"/> does not belong to
    /// <paramref name="path"/>'s current edit-ledger session — e.g. one captured before a
    /// call to <see cref="ReattachEditLedgerStoreAsync"/>, or before a real helper crash/restart.
    /// Throws <see cref="InvalidOperationException"/> if <paramref name="path"/> has no
    /// edit-ledger client attached at all.
    /// </summary>
    public void AssertLedgerObservationIsCurrent(string path, EditLedgerObservation observation)
    {
        if (!_ledgerClients.TryGetValue(path, out var client))
        {
            throw new InvalidOperationException($"no edit-ledger client is attached for '{path}'");
        }
        client.AssertObservationIsCurrent(observation);
    }

    /// <summary>The most recently observed <c>(session_id, sequence, ...)</c> for <paramref name="path"/>'s edit-ledger client, or null if it has none.</summary>
    public EditLedgerObservation? LastLedgerObservation(string path) =>
        _ledgerClients.TryGetValue(path, out var client) ? client.LastObservation : null;

    /// <summary>Whether <paramref name="path"/> has a durable edit-ledger store attached (false for a document opened without one, e.g. no ledger executable configured).</summary>
    public bool HasEditLedgerStore(string path) => _ledgerClients.ContainsKey(path);

    /// <summary>
    /// Reports the active document's durable undo/redo history (retained labels,
    /// permanent-command-id count, payload bytes) for an edit-history panel — null
    /// if there is no active document or it has no edit-ledger store attached.
    /// </summary>
    public async Task<HistoryStatusResult?> GetActiveDocumentHistoryStatusAsync(CancellationToken cancellationToken = default)
    {
        if (ActiveDocumentPath is not { } path || !_ledgerClients.TryGetValue(path, out var client))
        {
            return null;
        }
        return await client.HistoryStatusAsync($"history-status-{Guid.NewGuid():N}", cancellationToken).ConfigureAwait(true);
    }

    private async Task InitializeEditLedgerAsync(string path, string text, CancellationToken cancellationToken)
    {
        if (_editLedgerExecutablePath is not { Length: > 0 } executablePath)
        {
            return; // no ledger backing configured; Undo/Redo simply have nothing to call for this document
        }

        DirectoryInfo storeDirectory = Directory.CreateTempSubdirectory("flashtex-shell-ledger-");
        var client = EditLedgerClient.Start(executablePath, storeDirectory.FullName, _ => { });
        var seed = EditLedgerDocument.Create(EditLedgerProjectId, path, InitialLedgerRevision, text);
        InitializeResult initialized = await client
            .InitializeAsync($"init-{Guid.NewGuid():N}", seed, cancellationToken)
            .ConfigureAwait(true);

        _ledgerClients[path] = client;
        _ledgerStoreDirectories[path] = storeDirectory.FullName;
        _ledgerDocuments[path] = initialized.Document;
        _ledgerHistoryState[path] = (CanUndo: false, CanRedo: false);
    }

    private async Task DisposeEditLedgerAsync(string path)
    {
        if (_ledgerClients.Remove(path, out var client))
        {
            await client.DisposeAsync().ConfigureAwait(true);
        }
        _ledgerDocuments.Remove(path);
        _ledgerHistoryState.Remove(path);
        _ledgerReplaceInFlight.Remove(path);
        _ledgerQueuedText.Remove(path);

        if (_ledgerStoreDirectories.Remove(path, out var directory))
        {
            try
            {
                Directory.Delete(directory, recursive: true);
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                // Best-effort temp-directory cleanup only; a lingering handle (e.g. the
                // helper's own process teardown still in flight) never fails closing a tab.
            }
        }
    }

    /// <summary>
    /// Queues <paramref name="text"/> for <paramref name="path"/>'s edit-ledger store. A
    /// replace already in flight is left to finish; this only records that the newest text
    /// should go out next, so two calls never race with the same expected revision/hash.
    /// </summary>
    private void ScheduleLedgerReplace(string path, string text)
    {
        if (!_ledgerClients.ContainsKey(path))
        {
            return;
        }
        if (_ledgerReplaceInFlight.GetValueOrDefault(path))
        {
            _ledgerQueuedText[path] = text;
            return;
        }
        _ledgerReplaceInFlight[path] = true;
        LastLedgerReplaceTask = RunLedgerReplaceAsync(path, text);
    }

    private async Task RunLedgerReplaceAsync(string path, string text)
    {
        try
        {
            while (_ledgerDocuments.TryGetValue(path, out var current) && current.Text != text)
            {
                EditLedgerClient client = _ledgerClients[path];
                ReplaceDocumentResult result = await client
                    .ReplaceDocumentAsync($"replace-{Guid.NewGuid():N}", current.Revision, current.SourceSha256, text)
                    .ConfigureAwait(true);
                _ledgerDocuments[path] = result.Document;
                // replace_document always records a new "Source edit" undo entry and clears
                // redo when the text actually changed (crates/edit-ledger/src/history.rs's
                // `record`); ReplaceDocumentResult itself carries no can_undo/can_redo fields
                // (unlike apply_group/undo/redo's HistoryResult), so this is set directly.
                _ledgerHistoryState[path] = (CanUndo: true, CanRedo: false);

                if (!_ledgerQueuedText.Remove(path, out var queued))
                {
                    break;
                }
                text = queued;
            }
        }
        finally
        {
            _ledgerReplaceInFlight[path] = false;
        }
    }

    private async Task MoveHistoryAsync(bool isUndo, CancellationToken cancellationToken)
    {
        if (ActiveDocumentPath is not { } path ||
            !_ledgerClients.TryGetValue(path, out var client) ||
            !_ledgerDocuments.TryGetValue(path, out var current))
        {
            return;
        }

        string label = isUndo ? "undo" : "redo";
        var move = new HistoryMove($"{label}-{Guid.NewGuid():N}", current.Revision, current.SourceSha256);
        string requestId = $"{label}-request-{Guid.NewGuid():N}";
        HistoryResult result = isUndo
            ? await client.UndoAsync(requestId, move, cancellationToken).ConfigureAwait(true)
            : await client.RedoAsync(requestId, move, cancellationToken).ConfigureAwait(true);

        _ledgerDocuments[path] = result.Document;
        _ledgerHistoryState[path] = (result.CanUndo, result.CanRedo);
        ApplyLedgerTextToTab(path, result.Document.Text);
    }

    private void ApplyLedgerTextToTab(string path, string text)
    {
        var index = IndexOf(path);
        if (index < 0)
        {
            return;
        }
        Documents[index] = Documents[index] with
        {
            Text = text,
            Revision = Documents[index].Revision + 1,
            IsDirty = IsDirtyAgainstBaseline(path, text),
        };
        ScheduleAutoCompile();
    }
}
