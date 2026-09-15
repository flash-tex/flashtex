// name: RuleGeometryTests.cs
// purpose: Unit tests for FlashTeX.Editor.RuleGeometry (typed rules-v1
//   rectangle mapping for preview and PDF export), ported behavior from
//   apps/mac/Sources/FlashTeXMac/RuleGeometry.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Editor.Tests;

public class RuleGeometryTests
{
    private static PageItem.RuleItem Rule(double x, double y, double width, double height) =>
        new(x, y, width, height, null);

    [Fact]
    public void PreviewRectScalesEveryDimensionByTheGivenFactor()
    {
        var rule = Rule(10, 20, 30, 4);
        var rect = RuleGeometry.PreviewRect(rule, scale: 2.0);
        Assert.Equal(20, rect.X);
        Assert.Equal(40, rect.Y);
        Assert.Equal(60, rect.Width);
        Assert.Equal(8, rect.Height);
    }

    [Fact]
    public void PreviewRectNeverCollapsesAHairlineBelowTheMinimumHeight()
    {
        // A 0.1pt rule scaled down to 0.01 would be invisible; it must clamp to 0.5.
        var rule = Rule(0, 0, 100, 0.1);
        var rect = RuleGeometry.PreviewRect(rule, scale: 0.1);
        Assert.Equal(0.5, rect.Height);
    }

    [Fact]
    public void PreviewRectKeepsHeightWhenItAlreadyExceedsTheMinimum()
    {
        var rule = Rule(0, 0, 100, 10);
        var rect = RuleGeometry.PreviewRect(rule, scale: 1.0);
        Assert.Equal(10, rect.Height);
    }

    [Fact]
    public void PdfRectFlipsYFromTopLeftToBottomLeftOrigin()
    {
        // A page 792pt tall; a rule at y_pt=100 with height 2 (top-left origin, y down).
        var page = new Page(1, 612, 792, []);
        var rule = Rule(x: 50, y: 100, width: 200, height: 2);
        var rect = RuleGeometry.PdfRect(page, rule);
        Assert.Equal(50, rect.X);
        Assert.Equal(200, rect.Width);
        Assert.Equal(2, rect.Height);
        // Top edge at y_pt=100 in a y-down page becomes y=792-100-2=690 in PDF's y-up space.
        Assert.Equal(690, rect.Y);
    }

    [Fact]
    public void PdfRectPlacesARuleAtThePageTopAtTheHighestPdfYCoordinate()
    {
        var page = new Page(1, 612, 792, []);
        var rule = Rule(x: 0, y: 0, width: 100, height: 1);
        var rect = RuleGeometry.PdfRect(page, rule);
        Assert.Equal(791, rect.Y); // height_pt - y_pt - height_pt = 792 - 0 - 1
    }
}
