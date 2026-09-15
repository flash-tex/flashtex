// name: RuleGeometry.cs
// purpose: Rectangle mapping for typed `rules-v1` page items, shared by a future
//   preview renderer and PDF export so both draw the same geometry. Ported from
//   apps/mac/Sources/FlashTeXMac/RuleGeometry.swift. The contract's `(x_pt, y_pt)`
//   is the rectangle's TOP-LEFT corner in a y-down page space; it is never
//   reinterpreted as a text baseline. This library has no UI dependency, so the
//   Mac source's CoreGraphics `CGRect` is replaced by the plain `Rect` below
//   rather than a WinUI/System.Drawing type.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Editor;

/// <summary>A plain, UI-framework-agnostic rectangle in PDF points (y down, top-left origin).</summary>
public readonly record struct Rect(double X, double Y, double Width, double Height);

/// <summary>
/// Rectangle mapping for `RuntimeV1.PageItem.RuleItem`. Only the typed-rule route
/// is ported: the legacy `rules-v1`-not-negotiated approximation
/// (`RuleGeometry.legacyPDFRect`, which infers a bar from a run of U+2500 text
/// segments via a `RuleConvention` helper) is not part of this file's scope and
/// is not ported here.
/// </summary>
public static class RuleGeometry
{
    /// <summary>A hairline rule never collapses below this many preview points.</summary>
    private const double MinimumPreviewHeight = 0.5;

    /// <summary>Preview rectangle in view points (y down, origin at the page's top-left), scaled for display.</summary>
    public static Rect PreviewRect(PageItem.RuleItem rule, double scale) =>
        new(rule.XPt * scale, rule.YPt * scale, rule.WidthPt * scale, Math.Max(MinimumPreviewHeight, rule.HeightPt * scale));

    /// <summary>PDF user-space rectangle (y up, origin at the page's bottom-left): the top edge lands at `height_pt - y_pt`.</summary>
    public static Rect PdfRect(Page page, PageItem.RuleItem rule) =>
        new(rule.XPt, page.HeightPt - rule.YPt - rule.HeightPt, rule.WidthPt, rule.HeightPt);
}
