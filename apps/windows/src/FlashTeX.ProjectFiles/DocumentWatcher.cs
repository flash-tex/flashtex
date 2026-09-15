// name: DocumentWatcher.cs
// purpose: Live file-system change notification for one document. Replaces
//   the Mac original's macOS-only vnode source
//   (apps/mac/Sources/FlashTeXMac/DocumentWatcher.swift,
//   DispatchSource.makeFileSystemObjectSource) with a Windows-appropriate
//   System.IO.FileSystemWatcher, debounced ~250ms to match. Nothing here ever
//   reloads or writes anything — a coalesced OnChange is only a hint that a
//   caller should re-check disk status (the same explicit-conflict path the
//   Mac original documents), exactly like the source it replaces.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.ProjectFiles;

/// <summary>
/// Watches one file's containing directory and delivers a single, debounced
/// <see cref="OnChange"/> per coalesced burst of activity on that exact file.
/// </summary>
/// <remarks>
/// Deliberately watches the whole containing directory rather than setting
/// <see cref="FileSystemWatcher.Filter"/> to the target's own name. The
/// durable-save pattern this exists for (write to a temp file, then an atomic
/// <c>File.Move(overwrite: true)</c> onto the target — the same pattern
/// <c>crates/project-files</c>'s save layer uses) was observed on this
/// platform (NTFS) to raise, for the target's own name: a
/// <see cref="FileSystemWatcher.Deleted"/> event, immediately followed by a
/// <see cref="FileSystemWatcher.Renamed"/> event whose <em>old</em> name is
/// the temp file and whose <em>new</em> name is the target — never one plain
/// <c>Changed</c> event (see <c>DocumentWatcherTests.RawFileSystemWatcher_OnAnAtomicRenameOverAnExistingFile_DoesNotRaiseASingleCleanChangedEvent</c>
/// for the exact sequence captured). Whether a <c>Filter</c> set to the
/// target's own name would still surface that Renamed event depends on
/// whether the platform matches a rename's old or new name against it — an
/// unreliable, version-dependent detail this design avoids depending on by
/// watching unfiltered and matching both <see cref="RenamedEventArgs.FullPath"/>
/// and <see cref="RenamedEventArgs.OldFullPath"/> against the target itself.
/// </remarks>
public sealed class DocumentWatcher : IDisposable
{
    /// <summary>Matches the Mac original's <c>debounce = 0.25</c> (DocumentWatcher.swift).</summary>
    public static readonly TimeSpan DefaultDebounce = TimeSpan.FromMilliseconds(250);

    private readonly object _gate = new();
    private FileSystemWatcher? _watcher;
    private Timer? _debounceTimer;
    private string? _watchedFullPath;

    public TimeSpan Debounce { get; set; } = DefaultDebounce;

    /// <summary>Invoked (never on the caller's thread) once per coalesced burst.</summary>
    public Action? OnChange { get; set; }

    /// <summary>Raw file-system events seen, before coalescing (tests).</summary>
    public int Events { get; private set; }

    /// <summary>Coalesced deliveries to <see cref="OnChange"/> (tests).</summary>
    public int Deliveries { get; private set; }

    /// <summary>The path last passed to <see cref="Watch"/> (null once stopped without <c>keepPath</c>).</summary>
    public string? Path { get; private set; }

    public string Status { get; private set; } = "not watching";

    public bool IsWatching => _watcher is not null;

    /// <summary>Watches <paramref name="path"/> (replacing any previous watch). False when the containing directory does not exist.</summary>
    public bool Watch(string path)
    {
        Stop(keepPath: true);
        Path = path;
        return Arm();
    }

    /// <summary>Re-opens the watch on the current <see cref="Path"/> (e.g. after its directory reappeared).</summary>
    public bool Rearm()
    {
        if (Path is null)
        {
            return false;
        }
        Stop(keepPath: true);
        return Arm();
    }

    private bool Arm()
    {
        if (Path is null)
        {
            return false;
        }
        string fullPath = System.IO.Path.GetFullPath(Path);
        string? directory = System.IO.Path.GetDirectoryName(fullPath);
        if (directory is null || !Directory.Exists(directory))
        {
            Status = $"not watching {System.IO.Path.GetFileName(Path)}: containing directory does not exist";
            return false;
        }

        _watchedFullPath = fullPath;
        _watcher = CreateWatcher(directory);
        Status = $"watching {System.IO.Path.GetFileName(Path)}";
        return true;
    }

    private FileSystemWatcher CreateWatcher(string directory)
    {
        var watcher = new FileSystemWatcher(directory)
        {
            NotifyFilter = NotifyFilters.LastWrite | NotifyFilters.Size | NotifyFilters.FileName | NotifyFilters.CreationTime,
            IncludeSubdirectories = false,
        };
        watcher.Changed += OnFileSystemEvent;
        watcher.Created += OnFileSystemEvent;
        watcher.Deleted += OnFileSystemEvent;
        watcher.Renamed += OnRenamed;
        watcher.Error += OnWatcherError;
        watcher.EnableRaisingEvents = true;
        return watcher;
    }

    private void OnFileSystemEvent(object sender, FileSystemEventArgs e)
    {
        if (string.Equals(e.FullPath, _watchedFullPath, StringComparison.OrdinalIgnoreCase))
        {
            RecordEvent();
        }
    }

    private void OnRenamed(object sender, RenamedEventArgs e)
    {
        bool relevant = string.Equals(e.FullPath, _watchedFullPath, StringComparison.OrdinalIgnoreCase)
            || string.Equals(e.OldFullPath, _watchedFullPath, StringComparison.OrdinalIgnoreCase);
        if (relevant)
        {
            RecordEvent();
        }
    }

    /// <summary>
    /// A buffer overflow or the watched directory itself disappearing: retry
    /// on a pool thread rather than the watcher's own callback thread, so a
    /// recovered directory resumes being watched instead of going silent.
    /// </summary>
    private void OnWatcherError(object sender, ErrorEventArgs e)
    {
        Status = $"watch error: {e.GetException().Message}";
        ThreadPool.QueueUserWorkItem(_ => Rearm());
    }

    private void RecordEvent()
    {
        lock (_gate)
        {
            Events++;
            _debounceTimer?.Dispose();
            _debounceTimer = new Timer(_ => Deliver(), null, Debounce, Timeout.InfiniteTimeSpan);
        }
    }

    private void Deliver()
    {
        lock (_gate)
        {
            Deliveries++;
        }
        OnChange?.Invoke();
    }

    /// <summary>Stops watching; <see cref="Path"/> is forgotten unless <paramref name="keepPath"/>.</summary>
    public void Stop(bool keepPath = false)
    {
        lock (_gate)
        {
            _debounceTimer?.Dispose();
            _debounceTimer = null;
        }
        DetachWatcher();
        if (!keepPath)
        {
            Path = null;
            _watchedFullPath = null;
            Status = "not watching";
        }
    }

    private void DetachWatcher()
    {
        if (_watcher is null)
        {
            return;
        }
        _watcher.EnableRaisingEvents = false;
        _watcher.Changed -= OnFileSystemEvent;
        _watcher.Created -= OnFileSystemEvent;
        _watcher.Deleted -= OnFileSystemEvent;
        _watcher.Renamed -= OnRenamed;
        _watcher.Error -= OnWatcherError;
        _watcher.Dispose();
        _watcher = null;
    }

    public void Dispose() => Stop();
}
