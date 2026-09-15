// name: MainWindow.Watch.cs
// purpose: Wires FlashTeX.ProjectFiles.DocumentWatcher to every open, disk-backed
//   document so external changes (another editor, git checkout, ...) are noticed
//   while the app is running. A watcher firing never reloads or overwrites either
//   side by itself: it only triggers a fresh DocumentFilesClient.StatusAsync check,
//   and — only if that confirms a real on-disk difference — a ContentDialog asking
//   the user to reload from disk or keep the in-memory buffer, following the same
//   ShowProjectFileDialogAsync pattern MainWindow.ProjectFiles.cs already uses for
//   save conflicts. Reconciliation (arming a watcher for a newly bound document,
//   disposing one for a closed/renamed-away document) runs off the same
//   _shell.Documents.CollectionChanged signal MainWindow.ProjectFiles.cs already
//   uses to rebuild the project tree, plus after every project-file command, so no
//   extra call site needs to remember to invoke it.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.ProjectFiles;
using FlashTeX.Protocol.ProjectFilesV1;
using Microsoft.UI.Xaml.Controls;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private readonly Dictionary<string, DocumentWatcher> _watchersByDocument = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Documents with a reload-or-keep prompt currently open, so a burst of file-system events never stacks up duplicate dialogs.</summary>
    private readonly HashSet<string> _reloadPromptOpenForDocument = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>Called once from <see cref="WireProjectTree"/> (MainWindow.ProjectFiles.cs) at startup.</summary>
    private void WireDocumentWatchers()
    {
        _shell.Documents.CollectionChanged += (_, _) => ReconcileDocumentWatchers();
    }

    /// <summary>
    /// Arms a watcher for every open document that has a file binding (on-disk path)
    /// and lacks one yet, refreshes one whose bound path moved (e.g. Save As), and
    /// disposes any watcher/binding left over from a tab that is no longer open.
    /// </summary>
    private void ReconcileDocumentWatchers()
    {
        var openPaths = new HashSet<string>(_shell.Documents.Select(d => d.Path), StringComparer.OrdinalIgnoreCase);

        foreach (string stalePath in _watchersByDocument.Keys.Where(p => !openPaths.Contains(p)).ToList())
        {
            _watchersByDocument[stalePath].Dispose();
            _watchersByDocument.Remove(stalePath);
            _fileBindingsByDocument.Remove(stalePath);
            _reloadPromptOpenForDocument.Remove(stalePath);
        }

        foreach (var document in _shell.Documents)
        {
            if (!_fileBindingsByDocument.TryGetValue(document.Path, out var binding))
            {
                continue; // in-memory only (never saved yet): nothing on disk to watch
            }

            string absolutePath = Path.Combine(binding.Root, binding.RelativePath);
            if (!_watchersByDocument.TryGetValue(document.Path, out var watcher))
            {
                watcher = new DocumentWatcher();
                string capturedPath = document.Path;
                watcher.OnChange = () => DispatcherQueue.TryEnqueue(() => OnExternalFileChanged(capturedPath));
                _watchersByDocument[document.Path] = watcher;
            }
            if (!string.Equals(watcher.Path, absolutePath, StringComparison.OrdinalIgnoreCase))
            {
                watcher.Watch(absolutePath);
            }
        }
    }

    /// <summary>
    /// A coalesced file-system hint for <paramref name="path"/> — never assumed to
    /// mean anything by itself (our own save also touches the file). Confirms with a
    /// real <see cref="DocumentFilesClient.StatusAsync"/> round trip before ever
    /// prompting, and never reloads or discards anything without the user's choice.
    /// </summary>
    private void OnExternalFileChanged(string path)
    {
        if (!_reloadPromptOpenForDocument.Add(path))
        {
            return; // a prompt for this document is already up; the user hasn't answered it yet
        }
        _ = HandleExternalChangeAsync(path);
    }

    private async Task HandleExternalChangeAsync(string path)
    {
        try
        {
            if (!_fileBindingsByDocument.TryGetValue(path, out var binding))
            {
                return; // closed/rebound since the watcher fired
            }
            var document = _shell.Documents.FirstOrDefault(d => d.Path == path);
            if (document is null)
            {
                return;
            }

            StatusPayload status = await binding.Client.StatusAsync(binding.RelativePath, binding.Sha256).ConfigureAwait(true);
            if (status.State == DiskState.unchanged)
            {
                return; // e.g. our own save's rename-over-target, or a rewrite with identical bytes
            }

            await PromptExternalChangeAsync(path, document.IsDirty, status).ConfigureAwait(true);
        }
        catch (Exception ex)
        {
            await ShowProjectFileDialogAsync("Could not check external change", ex.Message).ConfigureAwait(true);
        }
        finally
        {
            _reloadPromptOpenForDocument.Remove(path);
        }
    }

    private async Task PromptExternalChangeAsync(string path, bool isDirty, StatusPayload status)
    {
        string headline = status.State switch
        {
            DiskState.deleted => "File deleted on disk",
            DiskState.modified => "File changed on disk",
            DiskState.created => "File appeared on disk",
            _ => "File changed on disk",
        };
        string body = status.State == DiskState.deleted
            ? $"“{path}” no longer exists on disk.{(isDirty ? " Your in-memory buffer still has unsaved changes." : string.Empty)}"
            : $"“{path}” changed outside FlashTeX.{(isDirty ? " You also have unsaved changes in this buffer." : string.Empty)} Reload it from disk, or keep what is currently open here?";

        var dialog = new ContentDialog
        {
            Title = headline,
            Content = body,
            PrimaryButtonText = status.State == DiskState.deleted ? "Keep My Version" : "Reload from Disk",
            SecondaryButtonText = status.State == DiskState.deleted ? null : "Keep My Version",
            CloseButtonText = "Cancel",
            XamlRoot = Content.XamlRoot,
        };

        ContentDialogResult result = await dialog.ShowAsync();
        bool shouldReload = status.State != DiskState.deleted && result == ContentDialogResult.Primary;
        if (!shouldReload)
        {
            return; // user kept the in-memory buffer; a later Save will still hit the normal conflict path
        }

        if (!_fileBindingsByDocument.TryGetValue(path, out var binding))
        {
            return;
        }
        ReadPayload read = await binding.Client.ReadAsync(binding.RelativePath).ConfigureAwait(true);
        if (!read.Exists || read.Text is null)
        {
            await ShowProjectFileDialogAsync("Reload failed", $"“{path}” no longer exists on disk.").ConfigureAwait(true);
            return;
        }

        _shell.UpdateDocumentText(path, read.Text);
        _shell.MarkDocumentSaved(path);
        _fileBindingsByDocument[path] = binding with { Sha256 = read.Sha256 };
    }
}
