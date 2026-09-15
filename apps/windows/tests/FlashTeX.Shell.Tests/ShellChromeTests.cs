// name: ShellChromeTests.cs
// purpose: Tests for FlashTeX.Shell.ShellChrome's coalescing and change-only
//   publish behavior (the entire point of the class, per
//   apps/mac/Sources/FlashTeXMac/ShellChrome.swift's FT-071 rationale): rapid
//   updates coalesce into one scheduled refresh, and a refresh fires
//   PropertyChanged only for keys whose value actually differs from what was
//   last published. Uses ManualChromeScheduler so no real timer/sleep is involved.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

public class ShellChromeTests
{
    [Fact]
    public void NotifyPossibleChange_CoalescesRapidCallsIntoOneScheduledRefresh()
    {
        var scheduler = new ManualChromeScheduler();
        var chrome = new ShellChrome(scheduler);
        chrome.Register("value", () => 1);

        chrome.NotifyPossibleChange();
        chrome.NotifyPossibleChange();
        chrome.NotifyPossibleChange();

        Assert.Equal(1, scheduler.PendingCount);
    }

    [Fact]
    public void Refresh_PublishesOnlyTheCoalescedFinalValue_NotIntermediateOnes()
    {
        var scheduler = new ManualChromeScheduler();
        var chrome = new ShellChrome(scheduler);
        var current = "A";
        var raisedValues = new List<string>();
        chrome.Register("value", () => current);
        chrome.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName == "value")
            {
                raisedValues.Add(chrome.Get<string>("value") ?? string.Empty);
            }
        };

        chrome.NotifyPossibleChange();
        current = "B";
        current = "C"; // rapid-fire changes within the same window
        scheduler.Fire();

        Assert.Equal(new[] { "C" }, raisedValues); // only the coalesced final value, one event
    }

    [Fact]
    public void Refresh_ChangedValue_FiresExactlyOnePropertyChangedEvent()
    {
        var scheduler = new ManualChromeScheduler();
        var chrome = new ShellChrome(scheduler);
        var value = 1;
        var raiseCount = 0;
        chrome.Register("value", () => value);
        chrome.PropertyChanged += (_, e) => { if (e.PropertyName == "value") raiseCount++; };

        chrome.NotifyPossibleChange();
        scheduler.Fire(); // first publish: nothing published before, always fires
        Assert.Equal(1, raiseCount);

        value = 2;
        chrome.NotifyPossibleChange();
        scheduler.Fire();

        Assert.Equal(2, raiseCount);
        Assert.Equal(2, chrome.Get<int>("value"));
    }

    [Fact]
    public void Refresh_UnchangedValue_FiresNoPropertyChangedEvent()
    {
        var scheduler = new ManualChromeScheduler();
        var chrome = new ShellChrome(scheduler);
        const int value = 42;
        var raiseCount = 0;
        chrome.Register("value", () => value);
        chrome.PropertyChanged += (_, e) => { if (e.PropertyName == "value") raiseCount++; };

        chrome.NotifyPossibleChange();
        scheduler.Fire();
        Assert.Equal(1, raiseCount); // the initial publish always fires

        chrome.NotifyPossibleChange();
        scheduler.Fire();
        Assert.Equal(1, raiseCount); // unchanged since the last publish: no second event
    }

    [Fact]
    public void Refresh_OnlyFiresForKeysThatActuallyChanged()
    {
        var scheduler = new ManualChromeScheduler();
        var chrome = new ShellChrome(scheduler);
        var stable = 1;
        var changing = 1;
        var changedKeys = new List<string>();
        chrome.Register("stable", () => stable);
        chrome.Register("changing", () => changing);
        chrome.PropertyChanged += (_, e) => changedKeys.Add(e.PropertyName!);

        chrome.NotifyPossibleChange();
        scheduler.Fire();
        changedKeys.Clear();

        changing = 2;
        chrome.NotifyPossibleChange();
        scheduler.Fire();

        Assert.Equal(new[] { "changing" }, changedKeys);
    }

    [Fact]
    public void NotifyPossibleChange_AfterARefreshRan_SchedulesANewWindow()
    {
        var scheduler = new ManualChromeScheduler();
        var chrome = new ShellChrome(scheduler);
        chrome.Register("value", () => 1);

        chrome.NotifyPossibleChange();
        Assert.Equal(1, scheduler.PendingCount);
        scheduler.Fire();

        chrome.NotifyPossibleChange();
        Assert.Equal(1, scheduler.PendingCount); // a fresh window, not still coalesced with the first
    }

    [Fact]
    public void Get_BeforeAnyRefresh_ReturnsTheFallback()
    {
        var chrome = new ShellChrome(new ManualChromeScheduler());
        chrome.Register("value", () => 99);

        Assert.Equal(0, chrome.Get<int>("value"));
        Assert.Equal(-1, chrome.Get("value", -1));
    }
}
