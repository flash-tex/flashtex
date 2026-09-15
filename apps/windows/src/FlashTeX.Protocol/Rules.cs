// name: Rules.cs
// purpose: Fraction-bar (and similar rule) text-item rendering convention shared by
//   the preview and both PDF writers. Ported from
//   apps/mac/Sources/FlashTeXProtocol/Rules.swift.
// author: Claude Sonnet 5
// date: 2026-09-13

using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Protocol;

/// <summary>
/// The compiler emits fraction bars (and similar rules) as text items made only of
/// U+2500 BOX DRAWINGS LIGHT HORIZONTAL, each assumed 0.5 em wide, with the rule
/// hugging the baseline from above. Renderers draw such items as filled rectangles
/// rather than glyphs, so the bar never depends on font coverage.
/// </summary>
public static class RuleConvention
{
    public const int RuleScalarCodepoint = 0x2500;
    public const double AdvanceEm = 0.5;

    /// <summary>0.06 x parent size at child size 0.7 x parent.</summary>
    public const double ThicknessEm = 0.0857;

    /// <summary>
    /// Returns the rule rectangle in top-left page coordinates, or <c>null</c> if the
    /// item is not a pure rule.
    /// </summary>
    public static (double X, double Y, double Width, double Height)? Rect(PageItem.TextItem item)
    {
        var scalars = item.Text.EnumerateRunes().ToList();
        if (scalars.Count == 0 || scalars.Any(r => r.Value != RuleScalarCodepoint))
        {
            return null;
        }
        double width = scalars.Count * AdvanceEm * item.FontSizePt;
        double height = ThicknessEm * item.FontSizePt;
        return (item.XPt, item.BaselineYPt - height, width, height);
    }
}
