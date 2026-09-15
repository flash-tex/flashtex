// name: MathCaretHighlight.cs
// purpose: Source -> preview for a caret inside math. Ported from
//   apps/mac/Sources/FlashTeXMac/MathCaretHighlight.swift. A formula is laid out
//   as ordinary `glyph_run` items (math font, script sizes) plus `rule` items
//   (fraction bars, the `\sqrt` overbar), and EVERY cluster and rule of the
//   formula carries one source range: the whole formula including its
//   delimiters (`$...$`, `\[...\]`). No cluster maps its bytes 1:1, so the
//   cluster-matching primitive below returns every glyph cluster of the formula
//   with no exact caret and the rules not at all.
//
//   `CaretHighlights` keeps the exact caret / whole-cluster contract for text
//   and adds the enclosing formula box: when no cluster containing the caret
//   byte maps 1:1 and one source span fans out over several items, the
//   highlight is the union of every item on the page with exactly that span
//   (glyph clusters AND rules). A single non-1:1 cluster keeps the documented
//   whole-cluster fallback.
//
//   The cluster-matching primitive (`CaretMatch`/`Clusters`) is ported from the
//   Swift source's `V2Geometry.clusters(containing:)`
//   (apps/mac/Sources/FlashTeXMac/GlyphRunRenderer.swift), included here because
//   `MathCaretHighlight.swift` depends on it directly and it is the same pure
//   byte-range/geometry family this port's instructions describe.
// author: Claude Sonnet 5
// date: 2026-09-14

using TickRect = FlashTeX.Protocol.RenderingV2.Rect;
using Item = FlashTeX.Protocol.RenderingV2.Item;
using Cluster = FlashTeX.Protocol.RenderingV2.Cluster;
using Caret = FlashTeX.Protocol.RenderingV2.Caret;
using Page = FlashTeX.Protocol.RenderingV2.Page;
using SourceRange = FlashTeX.Protocol.RuntimeV1.SourceRange;

namespace FlashTeX.Editor;

/// <summary>One glyph cluster whose source contains a queried caret byte, with exact caret geometry when derivable.</summary>
public readonly record struct CaretMatch(int ItemIndex, int ClusterIndex, Caret? Caret, IReadOnlyList<TickRect> HitRects);

/// <summary>The enclosing box of one fanned-out source span (a whole formula, or a sub-expression sharing one source range).</summary>
public sealed record FormulaBox(
    SourceRange Source,
    IReadOnlyList<int> ItemIndices,
    int ClusterCount,
    int RuleCount,
    IReadOnlyList<TickRect> Rects,
    TickRect Bounds);

/// <summary>Either an exact/whole-cluster caret, or the enclosing box of a formula the caret is inside.</summary>
public abstract record CaretHighlight
{
    private CaretHighlight()
    {
    }

    /// <summary>Exact caret bar (1:1 cluster with a caret at that byte) or the whole-cluster fallback.</summary>
    public sealed record OfCluster(CaretMatch Match) : CaretHighlight;

    /// <summary>The caret is inside a formula: the enclosing box of every item carrying that span.</summary>
    public sealed record OfFormula(FormulaBox Box) : CaretHighlight;
}

public static class MathCaretHighlight
{
    /// <summary>
    /// Clusters whose sources contain <paramref name="byteOffset"/> of
    /// <paramref name="path"/> (`start &lt;= byte &lt; end`; an empty range
    /// matches only its own offset), with exact caret geometry when derivable.
    /// </summary>
    public static IReadOnlyList<CaretMatch> Clusters(int byteOffset, string path, Page page)
    {
        var output = new List<CaretMatch>();
        for (int index = 0; index < page.Items.Count; index++)
        {
            if (page.Items[index] is not Item.OfGlyphRun { Value: var run })
            {
                continue;
            }
            for (int clusterIndex = 0; clusterIndex < run.Clusters.Count; clusterIndex++)
            {
                var cluster = run.Clusters[clusterIndex];
                var source = FirstSourceContaining(cluster.Sources, path, byteOffset);
                if (source is null)
                {
                    continue;
                }
                Caret? caret = null;
                if (source.EndByte - source.StartByte == cluster.TextEndByte - cluster.TextStartByte)
                {
                    int textByte = cluster.TextStartByte + (byteOffset - source.StartByte);
                    caret = cluster.Carets.FirstOrDefault(c => c.TextByte == textByte);
                }
                output.Add(new CaretMatch(index, clusterIndex, caret, cluster.HitRects));
            }
        }
        return output;
    }

    /// <summary>Every item of <paramref name="page"/> whose sources contain exactly <paramref name="span"/>, as one box.</summary>
    public static FormulaBox? BoxFor(SourceRange span, Page page)
    {
        var indices = new List<int>();
        var rects = new List<TickRect>();
        int clusterCount = 0, ruleCount = 0;
        for (int index = 0; index < page.Items.Count; index++)
        {
            switch (page.Items[index])
            {
                case Item.OfRule { Value: var rule }:
                    if (rule.Sources is null || !rule.Sources.Contains(span))
                    {
                        continue;
                    }
                    ruleCount += 1;
                    indices.Add(index);
                    rects.Add(new TickRect(rule.X, rule.Top, rule.Width, rule.Height));
                    break;
                case Item.OfGlyphRun { Value: var run }:
                    bool member = false;
                    foreach (var cluster in run.Clusters)
                    {
                        if (cluster.Sources is null || !cluster.Sources.Contains(span))
                        {
                            continue;
                        }
                        clusterCount += 1;
                        member = true;
                        rects.AddRange(cluster.HitRects);
                    }
                    if (member)
                    {
                        indices.Add(index);
                    }
                    break;
                default:
                    continue; // images and TikZ paths are never part of a formula box
            }
        }
        if (rects.Count == 0)
        {
            return null;
        }
        long minX = rects[0].X.Value, minTop = rects[0].Top.Value;
        long maxX = rects[0].X.Value + rects[0].Width.Value, maxBottom = rects[0].Top.Value + rects[0].Height.Value;
        for (int i = 1; i < rects.Count; i++)
        {
            var r = rects[i];
            minX = Math.Min(minX, r.X.Value);
            minTop = Math.Min(minTop, r.Top.Value);
            maxX = Math.Max(maxX, r.X.Value + r.Width.Value);
            maxBottom = Math.Max(maxBottom, r.Top.Value + r.Height.Value);
        }
        var bounds = new TickRect(minX, minTop, maxX - minX, maxBottom - minTop);
        return new FormulaBox(span, indices, clusterCount, ruleCount, rects, bounds);
    }

    /// <summary>
    /// Highlights for the caret at <paramref name="byteOffset"/> of <paramref name="path"/> on <paramref name="page"/>:
    /// every 1:1 cluster containing the byte, as <see cref="CaretHighlight.OfCluster"/>; otherwise, for each distinct
    /// fanned-out span containing the byte (carried by two or more items, or by any rule), one
    /// <see cref="CaretHighlight.OfFormula"/> box; a span carried by a single glyph cluster stays
    /// <see cref="CaretHighlight.OfCluster"/> (fallback).
    /// </summary>
    public static IReadOnlyList<CaretHighlight> CaretHighlights(int byteOffset, string path, Page page)
    {
        var matches = Clusters(byteOffset, path, page);
        var exact = new List<CaretHighlight>();
        var fanned = new List<SourceRange>();
        var single = new List<CaretHighlight>();
        foreach (var match in matches)
        {
            if (page.Items[match.ItemIndex] is not Item.OfGlyphRun { Value: var run })
            {
                continue;
            }
            var cluster = run.Clusters[match.ClusterIndex];
            var source = FirstSourceContaining(cluster.Sources, path, byteOffset);
            if (source is null)
            {
                continue;
            }
            if (IsOneToOne(cluster, source))
            {
                exact.Add(new CaretHighlight.OfCluster(match));
            }
            else if (!fanned.Contains(source))
            {
                fanned.Add(source);
                var box = BoxFor(source, page);
                single.Add(box is not null && box.ClusterCount + box.RuleCount > 1
                    ? new CaretHighlight.OfFormula(box)
                    : new CaretHighlight.OfCluster(match));
            }
            // else: another cluster of a span already boxed; covered by the box.
        }
        return exact.Count > 0 ? exact : single;
    }

    /// <summary>Whether <paramref name="cluster"/> maps its source bytes 1:1 onto its text bytes.</summary>
    private static bool IsOneToOne(Cluster cluster, SourceRange source) =>
        source.EndByte - source.StartByte == cluster.TextEndByte - cluster.TextStartByte;

    private static SourceRange? FirstSourceContaining(IReadOnlyList<SourceRange>? sources, string path, int byteOffset)
    {
        if (sources is null)
        {
            return null;
        }
        foreach (var source in sources)
        {
            if (source.Path != path)
            {
                continue;
            }
            bool contains = source.StartByte == source.EndByte
                ? byteOffset == source.StartByte
                : source.StartByte <= byteOffset && byteOffset < source.EndByte;
            if (contains)
            {
                return source;
            }
        }
        return null;
    }
}
