// name: ShellChrome.cs
// purpose: Throttled, change-only-fires mirror of whatever state the window
//   chrome (status bar, tab bar, toolbar) binds to, so a future UI does not
//   re-render on every keystroke. C# analogue of
//   apps/mac/Sources/FlashTeXMac/ShellChrome.swift, which exists because the
//   Mac app measured that reading ShellModel state directly from those views
//   caused 3-4 whole-window re-layouts per keystroke (FT-071, 2026-09-13: main
//   thread 71% busy while typing, 73% of that SwiftUI graph updates). Not a
//   field-for-field port: the real ShellModel this will mirror doesn't exist
//   yet in this port, so the registration surface is generic (a source is any
//   named, typed accessor) rather than the ~20 hardcoded Mac fields.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.ComponentModel;

namespace FlashTeX.Shell;

/// <summary>
/// A generic key/value mirror: callers <see cref="Register{T}"/> named
/// accessors (e.g. "wordCount" -> () => model.WordCount), then call
/// <see cref="NotifyPossibleChange"/> whenever a registered source might have
/// changed (typically from a model property setter). Rapid-fire calls within
/// one <see cref="_interval"/> window coalesce into a single scheduled
/// refresh; when that refresh runs, each source is re-read and
/// <see cref="PropertyChanged"/> fires for a key only if its value actually
/// differs from what was last published for that key — the point being that
/// a value that returned to its previous state during the window, or a
/// tracked-but-unrelated field, produces no event at all.
/// </summary>
public sealed class ShellChrome : INotifyPropertyChanged
{
    /// <summary>Default coalescing window, matching the Mac source's <c>FLASHTEX_CHROME_MS</c> default.</summary>
    public static readonly TimeSpan DefaultInterval = TimeSpan.FromMilliseconds(100);

    private readonly IChromeScheduler _scheduler;
    private readonly TimeSpan _interval;
    private readonly Dictionary<string, Func<object?>> _sources = new(StringComparer.Ordinal);
    private readonly Dictionary<string, object?> _published = new(StringComparer.Ordinal);
    private bool _isRefreshPending;

    public event PropertyChangedEventHandler? PropertyChanged;

    /// <param name="scheduler">Defaults to a real one-shot timer; tests inject a deterministic fake.</param>
    /// <param name="interval">Defaults to <see cref="DefaultInterval"/>.</param>
    public ShellChrome(IChromeScheduler? scheduler = null, TimeSpan? interval = null)
    {
        _scheduler = scheduler ?? new RealTimeChromeScheduler();
        _interval = interval ?? DefaultInterval;
    }

    /// <summary>
    /// Registers (or replaces) the accessor for <paramref name="key"/>. Does
    /// not itself schedule a refresh or publish a value — the first value is
    /// published on the next refresh after a <see cref="NotifyPossibleChange"/> call.
    /// </summary>
    public void Register<T>(string key, Func<T> read)
    {
        ArgumentException.ThrowIfNullOrEmpty(key);
        ArgumentNullException.ThrowIfNull(read);
        _sources[key] = () => read();
    }

    /// <summary>
    /// The last-published value for <paramref name="key"/>, or
    /// <paramref name="fallback"/> if nothing has been published for it yet
    /// (no source registered, or no refresh has run since registration).
    /// </summary>
    public T? Get<T>(string key, T? fallback = default) =>
        _published.TryGetValue(key, out var value) && value is T typed ? typed : fallback;

    /// <summary>
    /// Signals that a registered source may have changed. The first call in a
    /// coalescing window schedules one refresh <see cref="_interval"/> later;
    /// further calls before that refresh runs are no-ops, so a burst of edits
    /// (keystrokes, rapid IPC replies) triggers at most one refresh per window.
    /// </summary>
    public void NotifyPossibleChange()
    {
        if (_isRefreshPending)
        {
            return;
        }

        _isRefreshPending = true;
        _scheduler.Schedule(PublishPendingChanges, _interval);
    }

    /// <summary>
    /// Re-reads every registered source and fires <see cref="PropertyChanged"/>
    /// only for keys whose freshly read value differs from the last one
    /// published for that key.
    /// </summary>
    private void PublishPendingChanges()
    {
        _isRefreshPending = false;
        foreach (var (key, read) in _sources)
        {
            var value = read();
            var hadPrevious = _published.TryGetValue(key, out var previous);
            if (hadPrevious && Equals(previous, value))
            {
                continue;
            }

            _published[key] = value;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(key));
        }
    }
}
