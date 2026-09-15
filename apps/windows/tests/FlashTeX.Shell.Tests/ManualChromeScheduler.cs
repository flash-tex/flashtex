// name: ManualChromeScheduler.cs
// purpose: Deterministic IChromeScheduler test double: records scheduled
//   callbacks instead of running them on a real timer, so ShellChrome tests
//   can drive the coalescing window explicitly (Fire()) without sleeping.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

internal sealed class ManualChromeScheduler : IChromeScheduler
{
    private readonly List<(Action Callback, TimeSpan Delay)> _pending = new();

    /// <summary>How many times <see cref="Schedule"/> has been called since construction (or the last <see cref="Fire"/>).</summary>
    public int PendingCount => _pending.Count;

    public void Schedule(Action callback, TimeSpan delay)
    {
        _pending.Add((callback, delay));
    }

    /// <summary>Runs every callback scheduled so far, in scheduling order, then clears the queue.</summary>
    public void Fire()
    {
        var toRun = _pending.ToArray();
        _pending.Clear();
        foreach (var (callback, _) in toRun)
        {
            callback();
        }
    }
}
