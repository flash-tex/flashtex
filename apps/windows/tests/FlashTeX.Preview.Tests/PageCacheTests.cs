// name: PageCacheTests.cs
// purpose: Unit tests for FlashTeX.Preview.PageCache<T>: reuse of unchanged pages, rebuild
//   of changed pages, and least-recently-used eviction once the entry limit is exceeded.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Preview;
using FlashTeX.Protocol.RenderingV2;

namespace FlashTeX.Preview.Tests;

public class PageCacheTests
{
    private sealed record FakeDrawable(int BuildSequence);

    private static Page MakePage(int number, long ruleWidthTicks = 100)
    {
        var rule = new Rule(
            X: 0,
            Top: 0,
            Width: ruleWidthTicks,
            Height: 10,
            Paint: Paint.Black,
            Sources: null,
            SyntheticReason: "test rule");
        return new Page(number, Width: 1000, Height: 2000, Items: new Item[] { new Item.OfRule(rule) });
    }

    [Fact]
    public void GetOrBuild_FirstCallForAPage_IsAMissAndCallsBuild()
    {
        var cache = new PageCache<FakeDrawable>();
        int buildCalls = 0;
        var page = MakePage(1);

        var drawable = cache.GetOrBuild(page, p => new FakeDrawable(++buildCalls));

        Assert.Equal(1, buildCalls);
        Assert.Equal(1, drawable.BuildSequence);
        Assert.Equal(1, cache.MissCount);
        Assert.Equal(0, cache.HitCount);
    }

    [Fact]
    public void GetOrBuild_SameContentAgain_IsAHitAndDoesNotCallBuild()
    {
        var cache = new PageCache<FakeDrawable>();
        int buildCalls = 0;
        var page = MakePage(1);

        var first = cache.GetOrBuild(page, p => new FakeDrawable(++buildCalls));
        var second = cache.GetOrBuild(MakePage(1), p => new FakeDrawable(++buildCalls));

        Assert.Same(first, second);
        Assert.Equal(1, buildCalls);
        Assert.Equal(1, cache.HitCount);
    }

    [Fact]
    public void GetOrBuild_ChangedContentForSamePageNumber_RebuildsAndReplaces()
    {
        var cache = new PageCache<FakeDrawable>();
        int buildCalls = 0;

        var first = cache.GetOrBuild(MakePage(1, ruleWidthTicks: 100), p => new FakeDrawable(++buildCalls));
        var second = cache.GetOrBuild(MakePage(1, ruleWidthTicks: 200), p => new FakeDrawable(++buildCalls));

        Assert.NotSame(first, second);
        Assert.Equal(2, buildCalls);
        Assert.Equal(2, cache.MissCount);
    }

    [Fact]
    public void IsDirty_ReportsTrueUntilBuilt_ThenFalseForUnchangedContent_ThenTrueAgainAfterChange()
    {
        var cache = new PageCache<FakeDrawable>();
        var unchanged = MakePage(1, ruleWidthTicks: 100);

        Assert.True(cache.IsDirty(unchanged));
        cache.GetOrBuild(unchanged, p => new FakeDrawable(1));
        Assert.False(cache.IsDirty(MakePage(1, ruleWidthTicks: 100)));
        Assert.True(cache.IsDirty(MakePage(1, ruleWidthTicks: 999)));
    }

    [Fact]
    public void GetOrBuild_UnrelatedPageChanging_DoesNotDirtyOtherPages()
    {
        var cache = new PageCache<FakeDrawable>();
        int buildCalls = 0;
        cache.GetOrBuild(MakePage(1), p => new FakeDrawable(++buildCalls));
        cache.GetOrBuild(MakePage(2), p => new FakeDrawable(++buildCalls));

        // Page 1 changes; page 2's cached entry must survive untouched.
        cache.GetOrBuild(MakePage(1, ruleWidthTicks: 500), p => new FakeDrawable(++buildCalls));

        Assert.False(cache.IsDirty(MakePage(2)));
        Assert.Equal(3, buildCalls);
    }

    [Fact]
    public void GetOrBuild_ExceedingMaxEntries_EvictsLeastRecentlyUsedPage()
    {
        var cache = new PageCache<FakeDrawable>(maxEntries: 2);
        int buildCalls = 0;
        cache.GetOrBuild(MakePage(1), p => new FakeDrawable(++buildCalls));
        cache.GetOrBuild(MakePage(2), p => new FakeDrawable(++buildCalls));
        // Touch page 1 again so page 2 becomes the least recently used.
        cache.GetOrBuild(MakePage(1), p => new FakeDrawable(++buildCalls));
        cache.GetOrBuild(MakePage(3), p => new FakeDrawable(++buildCalls));

        Assert.Equal(2, cache.Count);
        Assert.True(cache.IsDirty(MakePage(2)), "page 2 should have been evicted as least-recently-used");
        Assert.False(cache.IsDirty(MakePage(1)));
        Assert.False(cache.IsDirty(MakePage(3)));
    }

    [Fact]
    public void Invalidate_DropsOnePageOnly()
    {
        var cache = new PageCache<FakeDrawable>();
        cache.GetOrBuild(MakePage(1), p => new FakeDrawable(1));
        cache.GetOrBuild(MakePage(2), p => new FakeDrawable(2));

        cache.Invalidate(1);

        Assert.True(cache.IsDirty(MakePage(1)));
        Assert.False(cache.IsDirty(MakePage(2)));
    }
}
