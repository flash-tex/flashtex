// name: ShellModel.Documents.cs
// purpose: Document/tab lifecycle for ShellModel: opening an in-memory
//   document as a new tab (or activating one already open), switching the
//   active tab, closing a tab, and dirty tracking against the text each
//   document had when it was opened. Real file I/O (reading/saving/watching a
//   path on disk) belongs to FlashTeX.ProjectFiles and is not wired in here;
//   this only accepts text the caller already has in memory. Adapted from the
//   shape of apps/mac/Sources/FlashTeXMac/ShellModel.swift's
//   `documents`/`activePath`/`replaceProject`/`updateActiveText`: the Swift
//   source has one entry document per project, but this port has no project
//   concept yet, so every opened document is its own independent tab.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Shell;

public partial class ShellModel
{
    private const int InitialDocumentRevision = 1;

    /// <summary>The text each open document had when it was opened (or last matched by an undo/redo/replace), for dirty tracking.</summary>
    private readonly Dictionary<string, string> _baselineText = new(StringComparer.Ordinal);

    /// <summary>
    /// Opens <paramref name="text"/> as a new tab at <paramref name="path"/> and makes it
    /// active. If a tab at <paramref name="path"/> is already open, this only activates it;
    /// its in-memory text and edit-ledger state are left untouched (call
    /// <see cref="UpdateDocumentText"/> to replace an already-open document's content).
    /// Starts a durable edit-ledger store for the new tab when this model was constructed
    /// with an edit-ledger executable path.
    /// </summary>
    public async Task OpenDocumentAsync(string path, string text, CancellationToken cancellationToken = default)
    {
        ArgumentException.ThrowIfNullOrEmpty(path);
        if (Documents.Any(d => d.Path == path))
        {
            ActiveDocumentPath = path;
            return;
        }

        _baselineText[path] = text;
        Documents.Add(new ShellDocument(path, text, InitialDocumentRevision, IsDirty: false));
        ActiveDocumentPath = path;
        await InitializeEditLedgerAsync(path, text, cancellationToken).ConfigureAwait(true);
    }

    /// <summary>
    /// Closes the tab at <paramref name="path"/>, disposing its edit-ledger store if it has
    /// one, and re-activates a neighboring tab (or none, if it was the last one open). A
    /// no-op if <paramref name="path"/> is not open.
    /// </summary>
    public async Task CloseDocumentAsync(string path, CancellationToken cancellationToken = default)
    {
        var index = IndexOf(path);
        if (index < 0)
        {
            return;
        }

        Documents.RemoveAt(index);
        _baselineText.Remove(path);
        await DisposeEditLedgerAsync(path).ConfigureAwait(true);

        if (ActiveDocumentPath == path)
        {
            ActiveDocumentPath = Documents.Count == 0 ? null : Documents[Math.Min(index, Documents.Count - 1)].Path;
        }
    }

    /// <summary>Switches the active tab to <paramref name="path"/>; returns false (a no-op) if it is not open.</summary>
    public bool SwitchActiveDocument(string path)
    {
        if (!Documents.Any(d => d.Path == path))
        {
            return false;
        }
        ActiveDocumentPath = path;
        return true;
    }

    /// <summary>
    /// Replaces the tab's text (a keystroke, an applied capture, a quick fix), bumps its
    /// shell-local <see cref="ShellDocument.Revision"/>, recomputes <see cref="ShellDocument.IsDirty"/>
    /// against the text it had when opened, schedules the debounced auto-compile
    /// (ShellModel.Compile.cs), and queues the change for that document's edit-ledger store
    /// (ShellModel.EditLedger.cs). A no-op if the text is unchanged or the path is not open.
    /// </summary>
    public void UpdateDocumentText(string path, string text)
    {
        var index = IndexOf(path);
        if (index < 0 || Documents[index].Text == text)
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
        ScheduleLedgerReplace(path, text);
    }

    /// <summary>
    /// Records the document's current text as its persisted baseline after a successful save.
    /// This intentionally does not write a file itself: the caller owns the file-system
    /// transaction and must invoke this only after it has received a durable save receipt.
    /// </summary>
    public bool MarkDocumentSaved(string path)
    {
        var index = IndexOf(path);
        if (index < 0)
        {
            return false;
        }

        var document = Documents[index];
        _baselineText[path] = document.Text;
        if (document.IsDirty)
        {
            Documents[index] = document with { IsDirty = false };
        }
        return true;
    }

    private bool IsDirtyAgainstBaseline(string path, string text) =>
        !_baselineText.TryGetValue(path, out var baseline) || baseline != text;

    private int IndexOf(string path)
    {
        for (var i = 0; i < Documents.Count; i++)
        {
            if (Documents[i].Path == path)
            {
                return i;
            }
        }
        return -1;
    }
}
