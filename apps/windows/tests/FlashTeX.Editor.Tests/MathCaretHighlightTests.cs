// name: MathCaretHighlightTests.cs
// purpose: Unit tests for FlashTeX.Editor.MathCaretHighlight (cluster/rule
//   caret-containment matching and the enclosing formula box for a fanned-out
//   source span), ported behavior from
//   apps/mac/Sources/FlashTeXMac/MathCaretHighlight.swift and the
//   V2Geometry.clusters(containing:) primitive in GlyphRunRenderer.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RenderingV2;
using SourceRange = FlashTeX.Protocol.RuntimeV1.SourceRange;
// The enclosing FlashTeX.Editor namespace also declares a plain `Rect` (RuleGeometry.cs)
// which shadows an unqualified `Rect` here (enclosing-namespace lookup wins over a
// using-alias), so every rendering-v2 rect below is spelled out as `V2Rect`.
using V2Rect = FlashTeX.Protocol.RenderingV2.Rect;

namespace FlashTeX.Editor.Tests;

public class MathCaretHighlightTests
{
    private const string Path = "main.tex";

    private static V2Rect HitRect(long x, long width) => new(x, 0, width, 10);

    private static Cluster GlyphCluster(int textStart, int textEnd, SourceRange source, IReadOnlyList<Caret>? carets = null, IReadOnlyList<V2Rect>? rects = null) =>
        new(textStart, textEnd, rects ?? [HitRect(textStart, textEnd - textStart)], carets ?? [], [source]);

    private static Item GlyphRunItem(params Cluster[] clusters) =>
        new Item.OfGlyphRun(new GlyphRun("font1", 10, new string('x', clusters.Length), [], clusters, Paint.Black));

    private static Item RuleItem(SourceRange source, long x = 0, long width = 10) =>
        new Item.OfRule(new Rule(x, 0, width, 1, Paint.Black, [source]));

    private static Page MakePage(params Item[] items) => new(1, 1000, 1000, items);

    // MARK: Clusters (V2Geometry.clusters(containing:))

    [Fact]
    public void ClustersFindsAOneToOneClusterAndDerivesTheExactCaret()
    {
        var source = new SourceRange(Path, 10, 12); // 2 bytes, matches cluster's 2 text bytes: 1:1
        var caret = new Caret(TextByte: 0, X: 5, Top: 0, Height: 10);
        var cluster = GlyphCluster(0, 2, source, carets: [caret]);
        var page = MakePage(GlyphRunItem(cluster));

        var matches = MathCaretHighlight.Clusters(10, Path, page);
        var match = Assert.Single(matches);
        Assert.Equal(0, match.ItemIndex);
        Assert.Equal(0, match.ClusterIndex);
        Assert.NotNull(match.Caret);
        Assert.Equal(0, match.Caret!.TextByte);
    }

    [Fact]
    public void ClustersReturnsNoCaretForANonOneToOneFannedOutSpan()
    {
        // The whole formula ($x+y$, 5 source bytes) maps to a 1-byte cluster: not 1:1.
        var source = new SourceRange(Path, 100, 105);
        var cluster = GlyphCluster(0, 1, source);
        var page = MakePage(GlyphRunItem(cluster));

        var matches = MathCaretHighlight.Clusters(102, Path, page);
        var match = Assert.Single(matches);
        Assert.Null(match.Caret);
        Assert.NotEmpty(match.HitRects);
    }

    [Fact]
    public void ClustersIgnoresClustersOfADifferentPathOrByteOutsideTheirRange()
    {
        var cluster = GlyphCluster(0, 2, new SourceRange("other.tex", 0, 2));
        var page = MakePage(GlyphRunItem(cluster));
        Assert.Empty(MathCaretHighlight.Clusters(0, Path, page));

        var farCluster = GlyphCluster(0, 2, new SourceRange(Path, 0, 2));
        var farPage = MakePage(GlyphRunItem(farCluster));
        Assert.Empty(MathCaretHighlight.Clusters(5, Path, farPage));
    }

    [Fact]
    public void ClustersMatchesAnEmptySourceRangeOnlyAtItsOwnOffset()
    {
        var cluster = GlyphCluster(0, 1, new SourceRange(Path, 7, 7));
        var page = MakePage(GlyphRunItem(cluster));
        Assert.Single(MathCaretHighlight.Clusters(7, Path, page));
        Assert.Empty(MathCaretHighlight.Clusters(8, Path, page));
    }

    // MARK: BoxFor

    [Fact]
    public void BoxForUnitesEveryItemCarryingExactlyTheGivenSpan()
    {
        var span = new SourceRange(Path, 100, 110);
        var c1 = GlyphCluster(0, 1, span, rects: [new V2Rect(0, 0, 10, 20)]);
        var c2 = GlyphCluster(1, 2, span, rects: [new V2Rect(20, 5, 10, 10)]);
        var page = MakePage(GlyphRunItem(c1, c2), RuleItem(span, x: 0, width: 20));

        var box = MathCaretHighlight.BoxFor(span, page);
        Assert.NotNull(box);
        Assert.Equal(2, box!.ClusterCount);
        Assert.Equal(1, box.RuleCount);
        Assert.Equal([0, 1], box.ItemIndices);
        Assert.Equal(3, box.Rects.Count);
        // Bounds: union of (0,0,10,20), (20,5,10,10) and the rule rect (0,0,20,1);
        // the widest right edge is the second cluster's (20+10=30), the tallest
        // bottom edge is the first cluster's (0+20=20).
        Assert.Equal(0, box.Bounds.X.Value);
        Assert.Equal(0, box.Bounds.Top.Value);
        Assert.Equal(30, box.Bounds.Width.Value);
        Assert.Equal(20, box.Bounds.Height.Value);
    }

    [Fact]
    public void BoxForReturnsNullWhenNoItemCarriesTheSpan()
    {
        var span = new SourceRange(Path, 100, 110);
        var other = new SourceRange(Path, 200, 210);
        var page = MakePage(GlyphRunItem(GlyphCluster(0, 1, other)));
        Assert.Null(MathCaretHighlight.BoxFor(span, page));
    }

    [Fact]
    public void BoxForIgnoresAPartialSourceMatchNotEqualToTheWholeSpan()
    {
        // The cluster's source is a *different* range, even though it overlaps; only an exact Contains match counts.
        var span = new SourceRange(Path, 100, 110);
        var narrower = new SourceRange(Path, 100, 105);
        var page = MakePage(GlyphRunItem(GlyphCluster(0, 1, narrower)));
        Assert.Null(MathCaretHighlight.BoxFor(span, page));
    }

    // MARK: CaretHighlights

    [Fact]
    public void CaretHighlightsPrefersExactOneToOneClustersOverAnyFannedOutBox()
    {
        var source = new SourceRange(Path, 10, 12);
        var caret = new Caret(0, 5, 0, 10);
        var cluster = GlyphCluster(0, 2, source, carets: [caret]);
        var page = MakePage(GlyphRunItem(cluster));

        var highlights = MathCaretHighlight.CaretHighlights(10, Path, page);
        var only = Assert.Single(highlights);
        Assert.IsType<CaretHighlight.OfCluster>(only);
    }

    [Fact]
    public void CaretHighlightsReturnsAFormulaBoxWhenTheFannedOutSpanCoversMultipleItems()
    {
        var span = new SourceRange(Path, 100, 110);
        var c1 = GlyphCluster(0, 1, span);
        var c2 = GlyphCluster(1, 2, span);
        var page = MakePage(GlyphRunItem(c1, c2));

        var highlights = MathCaretHighlight.CaretHighlights(105, Path, page);
        var only = Assert.Single(highlights);
        var formula = Assert.IsType<CaretHighlight.OfFormula>(only);
        Assert.Equal(2, formula.Box.ClusterCount);
    }

    [Fact]
    public void CaretHighlightsFallsBackToClusterWhenTheFannedOutSpanIsCarriedByOnlyOneCluster()
    {
        // A single non-1:1 cluster (e.g. "\'e" -> "e" via an accent macro): the
        // documented whole-cluster fallback, not a formula box.
        var span = new SourceRange(Path, 100, 103);
        var cluster = GlyphCluster(0, 1, span);
        var page = MakePage(GlyphRunItem(cluster));

        var highlights = MathCaretHighlight.CaretHighlights(101, Path, page);
        var only = Assert.Single(highlights);
        Assert.IsType<CaretHighlight.OfCluster>(only);
    }

    [Fact]
    public void CaretHighlightsReturnsEmptyWhenNoClusterContainsTheByte()
    {
        var page = MakePage(GlyphRunItem(GlyphCluster(0, 1, new SourceRange(Path, 0, 1))));
        Assert.Empty(MathCaretHighlight.CaretHighlights(50, Path, page));
    }
}
