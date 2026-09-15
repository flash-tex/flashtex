// name: IChromeScheduler.cs
// purpose: Injectable "run this later" abstraction for ShellChrome's
//   coalescing timer, so production code uses a real timer while tests drive
//   the callback deterministically without sleeping (see
//   FlashTeX.Shell.Tests' ManualChromeScheduler).
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Shell;

/// <summary>Schedules a single callback to run after a delay.</summary>
public interface IChromeScheduler
{
    /// <summary>Runs <paramref name="callback"/> once, approximately <paramref name="delay"/> from now.</summary>
    void Schedule(Action callback, TimeSpan delay);
}

/// <summary>Default production scheduler: a one-shot, non-repeating <see cref="System.Timers.Timer"/>.</summary>
public sealed class RealTimeChromeScheduler : IChromeScheduler
{
    public void Schedule(Action callback, TimeSpan delay)
    {
        ArgumentNullException.ThrowIfNull(callback);
        if (delay <= TimeSpan.Zero)
        {
            callback();
            return;
        }

        var timer = new System.Timers.Timer(delay.TotalMilliseconds) { AutoReset = false };
        timer.Elapsed += (_, _) =>
        {
            timer.Dispose();
            callback();
        };
        timer.Start();
    }
}
