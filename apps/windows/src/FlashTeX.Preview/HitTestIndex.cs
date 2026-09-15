// name: HitTestIndex.cs
// purpose: Per-page spatial index over rendering-v2 hit rectangles, for click-to-source
//   navigation: given a page-tick point, find the topmost/most-specific item whose hit
//   rectangle contains it and return its source provenance.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RenderingV2;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Preview;

/// <summary>
/// One hit-testable rectangle: a glyph run cluster's hit rect, or a rule/image item's own
/// bounding box (vector paths carry no hit-rect primitive on the wire, so they are not
/// indexed here — see <see cref="HitTestIndex.Build"/>).
/// </summary>
public sealed record HitTestEntry(
    Rect Bounds,
    int PaintOrder,
    IReadOnlyList<SourceRange>? Sources,
    string? SyntheticReason)
{
    /// <summary>The first declared source range, if any (a synthetic entry has none).</summary>
    public SourceRange? PrimarySource => this.Sources is { Count: > 0 } sources ? sources[0] : null;
}

/// <summary>
/// A spatial index over one page's hit-testable items. Built once per page and queried
/// once per pointer press: entries are kept sorted by their rectangle's left edge so a
/// query can stop scanning as soon as it passes the query point (nothing further left
/// could contain it), which is enough for the few-hundred-to-low-thousands items a page
/// realistically carries without needing an R-tree.
/// </summary>
public sealed class HitTestIndex
{
    private readonly List<HitTestEntry> entriesByLeftEdge;

    private HitTestIndex(List<HitTestEntry> entriesByLeftEdge)
    {
        this.entriesByLeftEdge = entriesByLeftEdge;
    }

    public int Count => this.entriesByLeftEdge.Count;

    /// <summary>Builds an index over every hit-testable rectangle in <paramref name="page"/>'s items, in paint order.</summary>
    public static HitTestIndex Build(FlashTeX.Protocol.RenderingV2.Page page)
    {
        ArgumentNullException.ThrowIfNull(page);
        var entries = new List<HitTestEntry>();
        for (int itemIndex = 0; itemIndex < page.Items.Count; itemIndex++)
        {
            CollectEntries(page.Items[itemIndex], itemIndex, entries);
        }
        entries.Sort((a, b) => a.Bounds.X.Value.CompareTo(b.Bounds.X.Value));
        return new HitTestIndex(entries);
    }

    /// <summary>
    /// Finds the topmost (highest paint order) hit rectangle containing (<paramref name="x"/>, <paramref name="y"/>),
    /// breaking ties between same-order overlapping rectangles by preferring the smaller
    /// (more specific) area. Returns null when no rectangle contains the point.
    /// </summary>
    public HitTestEntry? Query(long x, long y)
    {
        HitTestEntry? best = null;
        foreach (var entry in this.entriesByLeftEdge)
        {
            if (entry.Bounds.X.Value > x)
            {
                // Sorted ascending by left edge: every remaining entry starts further
                // right than the query point, so none of them can contain it either.
                break;
            }
            if (!entry.Bounds.Contains(x, y))
            {
                continue;
            }
            if (best is null || IsMoreSpecific(entry, best))
            {
                best = entry;
            }
        }
        return best;
    }

    private static bool IsMoreSpecific(HitTestEntry candidate, HitTestEntry current)
    {
        if (candidate.PaintOrder != current.PaintOrder)
        {
            return candidate.PaintOrder > current.PaintOrder;
        }
        return Area(candidate.Bounds) < Area(current.Bounds);
    }

    private static long Area(Rect bounds) => bounds.Width.Value * bounds.Height.Value;

    private static void CollectEntries(Item item, int itemIndex, List<HitTestEntry> entries)
    {
        switch (item)
        {
            case Item.OfGlyphRun glyphRun:
                CollectClusterEntries(glyphRun.Value, itemIndex, entries);
                break;
            case Item.OfRule rule:
                entries.Add(new HitTestEntry(
                    new Rect(rule.Value.X, rule.Value.Top, rule.Value.Width, rule.Value.Height),
                    itemIndex,
                    rule.Value.Sources,
                    rule.Value.SyntheticReason));
                break;
            case Item.OfImage image:
                entries.Add(new HitTestEntry(
                    new Rect(image.Value.X, image.Value.Top, image.Value.Width, image.Value.Height),
                    itemIndex,
                    image.Value.Sources,
                    image.Value.SyntheticReason));
                break;
            case Item.OfPath:
                // path_fill/path_stroke items carry no rect hit-testing primitive on the
                // wire (only clusters/rules/images do); out of scope for this index.
                break;
        }
    }

    private static void CollectClusterEntries(GlyphRun run, int itemIndex, List<HitTestEntry> entries)
    {
        foreach (var cluster in run.Clusters)
        {
            foreach (var rect in cluster.HitRects)
            {
                entries.Add(new HitTestEntry(rect, itemIndex, cluster.Sources, cluster.SyntheticReason));
            }
        }
    }
}
