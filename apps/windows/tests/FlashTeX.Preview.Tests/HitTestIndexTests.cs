// name: HitTestIndexTests.cs
// purpose: Unit tests for FlashTeX.Preview.HitTestIndex: correct hits, correct misses, and
//   topmost/most-specific-wins behavior for overlapping hit rectangles.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Preview;
using FlashTeX.Protocol.RenderingV2;
using FlashTeX.Protocol.RuntimeV1;
using Page = FlashTeX.Protocol.RenderingV2.Page;

namespace FlashTeX.Preview.Tests;

public class HitTestIndexTests
{
    private static Cluster MakeCluster(int startByte, int endByte, Rect rect, string sourcePath, int sourceStart, int sourceEnd) =>
        new(
            TextStartByte: startByte,
            TextEndByte: endByte,
            HitRects: new[] { rect },
            Carets: Array.Empty<Caret>(),
            Sources: new[] { new SourceRange(sourcePath, sourceStart, sourceEnd) });

    /// <summary>
    /// Page laid out as:
    ///  - item 0 (glyph run): cluster0 rect [0,200)x[0,50) "big", cluster1 rect [80,100)x[0,50) "small", same paint order (item 0).
    ///  - item 1 (rule): rect [150,250)x[0,50), paints after (on top of) item 0.
    /// </summary>
    private static Page MakeTestPage()
    {
        var big = MakeCluster(0, 10, new Rect(0, 0, 200, 50), "doc.tex", 0, 10);
        var small = MakeCluster(10, 12, new Rect(80, 0, 20, 50), "doc.tex", 10, 12);
        var glyphRun = new GlyphRun(
            FontId: "f1",
            FontSize: 1000,
            Text: "abcdefghijkl",
            Glyphs: Array.Empty<Glyph>(),
            Clusters: new[] { big, small },
            Paint: Paint.Black);

        var rule = new Rule(
            X: 150,
            Top: 0,
            Width: 100,
            Height: 50,
            Paint: Paint.Black,
            Sources: null,
            SyntheticReason: "background rule");

        return new Page(1, Width: 1000, Height: 1000, Items: new Item[] { new Item.OfGlyphRun(glyphRun), new Item.OfRule(rule) });
    }

    [Fact]
    public void Query_PointOnlyInsideOneRect_ReturnsThatEntry()
    {
        var index = HitTestIndex.Build(MakeTestPage());

        var hit = index.Query(10, 10);

        Assert.NotNull(hit);
        Assert.Equal("doc.tex", hit!.PrimarySource!.Path);
        Assert.Equal(0, hit.PrimarySource!.StartByte);
        Assert.Equal(10, hit.PrimarySource!.EndByte);
    }

    [Fact]
    public void Query_PointOutsideEveryRect_ReturnsNull()
    {
        var index = HitTestIndex.Build(MakeTestPage());

        Assert.Null(index.Query(900, 900));
    }

    [Fact]
    public void Query_OverlappingRectsSamePaintOrder_PrefersSmallerMoreSpecificRect()
    {
        var index = HitTestIndex.Build(MakeTestPage());

        // (90, 10) is inside both the "big" cluster rect [0,200) and the "small" cluster
        // rect [80,100); both belong to item 0, so the smaller (more specific) one wins.
        var hit = index.Query(90, 10);

        Assert.NotNull(hit);
        Assert.Equal(10, hit!.PrimarySource!.StartByte);
        Assert.Equal(12, hit.PrimarySource!.EndByte);
    }

    [Fact]
    public void Query_OverlappingRectsDifferentPaintOrder_PrefersTopmostRegardlessOfArea()
    {
        var index = HitTestIndex.Build(MakeTestPage());

        // (160, 10) is inside the "big" cluster rect [0,200) (item 0) and the rule's rect
        // [150,250) (item 1, painted later/on top); the rule must win despite being a
        // similarly sized rect, because it has the higher paint order.
        var hit = index.Query(160, 10);

        Assert.NotNull(hit);
        Assert.Null(hit!.Sources);
        Assert.Equal("background rule", hit.SyntheticReason);
    }

    [Fact]
    public void Build_CountsEveryHitRectAcrossItems()
    {
        var index = HitTestIndex.Build(MakeTestPage());

        // 2 cluster hit rects + 1 rule rect.
        Assert.Equal(3, index.Count);
    }
}
