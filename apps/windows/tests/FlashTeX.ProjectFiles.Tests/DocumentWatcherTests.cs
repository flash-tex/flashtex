// name: DocumentWatcherTests.cs
// purpose: Tests for DocumentWatcher's coalesced change notification, and a
//   dedicated probe of the exact FileSystemWatcher behavior this platform
//   gives for an atomic rename-over-an-existing-file (the durable-save
//   pattern crates/project-files' save layer uses: write to a temp file, then
//   `File.Move(overwrite: true)` onto the target) — the known risk called out
//   in this port's task: such a save is commonly reported as a Deleted event
//   immediately followed by Created/Changed rather than one clean signal.
// author: Claude Sonnet 5
// date: 2026-09-14

using Xunit;

namespace FlashTeX.ProjectFiles.Tests;

public class DocumentWatcherTests
{
    private static async Task WaitUntilAsync(Func<bool> condition, TimeSpan timeout)
    {
        DateTime deadline = DateTime.UtcNow + timeout;
        while (!condition() && DateTime.UtcNow < deadline)
        {
            await Task.Delay(20);
        }
    }

    /// <summary>
    /// Probes the raw platform behavior with no coalescing involved, so the
    /// finding is visible independent of DocumentWatcher's own logic. This is
    /// the evidence behind the "watch unfiltered, match both old and new
    /// names" design documented on <see cref="DocumentWatcher"/> itself.
    /// </summary>
    [Fact]
    public async Task RawFileSystemWatcher_OnAnAtomicRenameOverAnExistingFile_DoesNotRaiseASingleCleanChangedEvent()
    {
        string dir = Directory.CreateTempSubdirectory("flashtex-watcher-raw-").FullName;
        string target = Path.Combine(dir, "doc.tex");
        await File.WriteAllTextAsync(target, "original\n");

        var events = new List<string>();
        using var watcher = new FileSystemWatcher(dir) { EnableRaisingEvents = true };
        watcher.Changed += (_, e) => events.Add($"Changed:{e.Name}");
        watcher.Created += (_, e) => events.Add($"Created:{e.Name}");
        watcher.Deleted += (_, e) => events.Add($"Deleted:{e.Name}");
        watcher.Renamed += (_, e) => events.Add($"Renamed:{e.OldName}->{e.Name}");

        string tmp = Path.Combine(dir, "doc.tex.tmp");
        await File.WriteAllTextAsync(tmp, "updated\n");
        File.Move(tmp, target, overwrite: true);

        await WaitUntilAsync(() => events.Count > 0, TimeSpan.FromSeconds(3));
        await Task.Delay(300); // let any trailing events land before asserting

        // FINDING (observed on this platform/filesystem, NTFS, this run):
        // Created:doc.tex.tmp | Changed:doc.tex.tmp | Deleted:doc.tex | Renamed:doc.tex.tmp->doc.tex
        // i.e. a Deleted event for the target's own name, immediately
        // followed by a Renamed event whose *old* name is the temp file and
        // whose *new* name is the target — never one plain `Changed:doc.tex`.
        // A watcher with `Filter = "doc.tex"` would still see the Deleted
        // event, but whether it also sees that Renamed event depends on
        // whether the platform matches Filter against a rename's old or new
        // name (documented as unreliable across .NET versions) — watching
        // unfiltered and matching both `RenamedEventArgs.FullPath` and
        // `.OldFullPath` ourselves (this file's `DocumentWatcher`) sidesteps
        // that question entirely rather than depending on it.
        Assert.NotEmpty(events);
        Assert.All(events, e => Assert.False(
            e == "Changed:doc.tex" && events.Count == 1,
            "if this ever fires, the platform started raising a single clean Changed event for this pattern; " +
            "the DocumentWatcher doc comment's rationale for watching unfiltered can be revisited"));
    }

    [Fact]
    public async Task Watch_AtomicRenameOverAnExistingFile_ProducesExactlyOneCoalescedNotification()
    {
        string dir = Directory.CreateTempSubdirectory("flashtex-watcher-").FullName;
        string target = Path.Combine(dir, "doc.tex");
        await File.WriteAllTextAsync(target, "original\n");

        using var watcher = new DocumentWatcher { Debounce = TimeSpan.FromMilliseconds(100) };
        var delivered = new TaskCompletionSource();
        watcher.OnChange = () => delivered.TrySetResult();
        Assert.True(watcher.Watch(target));

        string tmp = Path.Combine(dir, "doc.tex.tmp");
        await File.WriteAllTextAsync(tmp, "updated\n");
        File.Move(tmp, target, overwrite: true);

        await delivered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        await Task.Delay(400); // past the debounce window: catch any extra, un-coalesced delivery

        Assert.Equal(1, watcher.Deliveries);
        Assert.True(watcher.Events >= 1);
        Assert.Equal("updated\n", await File.ReadAllTextAsync(target));
    }

    [Fact]
    public async Task Watch_SeveralQuickInPlaceEditsCoalesceIntoOneNotification()
    {
        string dir = Directory.CreateTempSubdirectory("flashtex-watcher-inplace-").FullName;
        string target = Path.Combine(dir, "doc.tex");
        await File.WriteAllTextAsync(target, "v0\n");

        using var watcher = new DocumentWatcher { Debounce = TimeSpan.FromMilliseconds(150) };
        var delivered = new TaskCompletionSource();
        watcher.OnChange = () => delivered.TrySetResult();
        Assert.True(watcher.Watch(target));

        for (int i = 1; i <= 3; i++)
        {
            await File.WriteAllTextAsync(target, $"v{i}\n");
            await Task.Delay(20);
        }

        await delivered.Task.WaitAsync(TimeSpan.FromSeconds(5));
        await Task.Delay(400);

        Assert.Equal(1, watcher.Deliveries);
    }

    [Fact]
    public async Task Stop_PreventsFurtherNotifications()
    {
        string dir = Directory.CreateTempSubdirectory("flashtex-watcher-stop-").FullName;
        string target = Path.Combine(dir, "doc.tex");
        await File.WriteAllTextAsync(target, "v0\n");

        using var watcher = new DocumentWatcher { Debounce = TimeSpan.FromMilliseconds(100) };
        int deliveries = 0;
        watcher.OnChange = () => Interlocked.Increment(ref deliveries);
        Assert.True(watcher.Watch(target));
        watcher.Stop();

        await File.WriteAllTextAsync(target, "v1\n");
        await Task.Delay(400);

        Assert.Equal(0, deliveries);
        Assert.False(watcher.IsWatching);
    }

    [Fact]
    public void Watch_WithAMissingContainingDirectory_ReturnsFalse()
    {
        using var watcher = new DocumentWatcher();

        bool ok = watcher.Watch(Path.Combine(Path.GetTempPath(), "flashtex-does-not-exist-" + Guid.NewGuid(), "doc.tex"));

        Assert.False(ok);
        Assert.False(watcher.IsWatching);
        Assert.Contains("not watching", watcher.Status, StringComparison.Ordinal);
    }
}
