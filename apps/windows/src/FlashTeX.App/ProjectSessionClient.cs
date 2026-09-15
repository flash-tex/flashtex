// name: ProjectSessionClient.cs
// purpose: Lazily starts and caches one flashtex-preview-controller helper
//   process (file-backed: project_root + a private, per-session ledger
//   scratch directory, never store_paths) for whichever project root/entry
//   document is currently active. Both the project-wide search UI
//   (MainWindow.Search.cs, via FlashTeX.Ipc.ProjectSearchClient) and the
//   citation-rename UI (MainWindow.CitationRename.cs) share one instance of
//   this rather than each launching its own helper process, since both are
//   "project navigation" operations against the same live project. Restarts
//   the helper (disposing the old one first) whenever the bound root or entry
//   document changes, e.g. because a different project was opened.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Ipc;
using FlashTeX.Protocol.PreviewControllerV1;

namespace FlashTeX.App;

/// <summary>Thrown when the flashtex-preview-controller tool binary cannot be found on disk.</summary>
public sealed class PreviewControllerToolMissingException(string message) : Exception(message);

/// <summary>One lazily-started, project-root-bound <see cref="PreviewControllerClient"/>, reused across calls while the binding is unchanged.</summary>
internal sealed class ProjectSessionClient : IAsyncDisposable
{
    private PreviewControllerClient? _client;
    private string? _boundRoot;
    private string? _boundEntryPath;

    /// <summary>
    /// Returns the current client if it is already bound to <paramref name="projectRoot"/>/
    /// <paramref name="entryPath"/>, else disposes any previous one and starts a fresh helper
    /// process against a new, private per-session ledger scratch directory.
    /// </summary>
    public async Task<PreviewControllerClient> GetOrStartAsync(
        string projectRoot, string entryPath, Action<string> onStderrLine, CancellationToken cancellationToken = default)
    {
        if (_client is not null
            && string.Equals(_boundRoot, projectRoot, StringComparison.OrdinalIgnoreCase)
            && string.Equals(_boundEntryPath, entryPath, StringComparison.OrdinalIgnoreCase))
        {
            return _client;
        }

        if (_client is not null)
        {
            await _client.DisposeAsync().ConfigureAwait(true);
            _client = null;
            _boundRoot = null;
            _boundEntryPath = null;
        }

        string? executable = PreviewControllerToolLocator.FindExecutable();
        if (executable is null || !File.Exists(executable))
        {
            throw new PreviewControllerToolMissingException(
                "flashtex-preview-controller.exe was not found. Build it with `cargo build --release` in crates/preview-controller.");
        }

        string ledgerRoot = Directory.CreateTempSubdirectory("flashtex-preview-controller-ledger-").FullName;
        var config = new LaunchConfig(
            sessionId: $"windows-session-{Guid.NewGuid():N}",
            projectId: "flashtex-shell",
            entryPath: entryPath,
            projectRoot: projectRoot,
            privateLedgerRoot: ledgerRoot);

        _client = await PreviewControllerClient
            .StartAsync(executable, config, Path.GetTempPath(), onStderrLine, cancellationToken: cancellationToken)
            .ConfigureAwait(true);
        _boundRoot = projectRoot;
        _boundEntryPath = entryPath;
        return _client;
    }

    public async ValueTask DisposeAsync()
    {
        if (_client is not null)
        {
            await _client.DisposeAsync().ConfigureAwait(true);
            _client = null;
            _boundRoot = null;
            _boundEntryPath = null;
        }
    }
}
