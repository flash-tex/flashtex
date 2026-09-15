// name: V2Frame.cs
// purpose: Turns a validated rendering-v2 display list into everything the Win2D painter
//   needs, ONCE per frame: resolved DirectWrite font faces (content-addressed by SHA-256),
//   per-page paint-ordered draw items in device-independent pixels, and a per-page
//   hit-test index for click-to-source. Ports the design of
//   apps/mac/Sources/FlashTeXMac/GlyphRunRenderer.swift's `V2PreparedPage`/`V2Frame`
//   (CoreGraphics there, Direct2D/DirectWrite here).
//
//   Fail-closed like the Swift source: the first font that does not resolve aborts the
//   whole frame, so the caller falls back to the runtime-v1 renderer instead of painting a
//   page with substituted or missing glyphs. Coordinates come straight from the wire's
//   `bp_2pow20` ticks through Ticks.ToDips(); glyph positions are the producer's ABSOLUTE
//   baseline origins and are never re-derived from advances.
// author: Claude Opus 5
// date: 2026-09-14

using System.Numerics;
using FlashTeX.Preview;
using FlashTeX.Protocol.RenderingV2;
using Microsoft.Graphics.Canvas.Text;
using Windows.UI;
// `Path` is both a display-list vector item and System.IO.Path (an implicit using).
using V2Path = FlashTeX.Protocol.RenderingV2.Path;

namespace FlashTeX.App;

/// <summary>One paint-ordered item of a prepared page, in device-independent pixels.</summary>
internal abstract record V2PreparedItem
{
    private V2PreparedItem()
    {
    }

    /// <summary>
    /// A whole glyph run as one DirectWrite draw call. <see cref="Origin"/> is the first
    /// glyph's absolute baseline origin and every <see cref="CanvasGlyph"/> carries a zero
    /// advance plus the offset from that origin, so DirectWrite places each glyph exactly
    /// where the producer put it rather than accumulating advances of its own.
    /// </summary>
    public sealed record GlyphRunItem(CanvasFontFace Face, float FontSizeDip, Vector2 Origin, CanvasGlyph[] Glyphs, Color Color) : V2PreparedItem;

    public sealed record RuleItem(double X, double Top, double Width, double Height, Color Color) : V2PreparedItem;

    /// <summary>
    /// A vector path (TikZ). Geometry is built at draw time from these commands because a
    /// <c>CanvasGeometry</c> needs a device, and rebuilding it costs nothing in practice:
    /// a <c>CanvasControl</c> only redraws on invalidation, resize or device loss, not while
    /// the pane is panned or scrolled.
    /// </summary>
    public sealed record PathItem(V2Path Value, Color Color) : V2PreparedItem;

    /// <summary>
    /// An `image` item this consumer did not paint, carrying the reason. A refused image is
    /// NOT a frame failure (the rest of the page is complete and correct); it surfaces as a
    /// non-modal notice, exactly as the Mac source treats an unverifiable image.
    /// </summary>
    public sealed record ImageNoticeItem(string Notice) : V2PreparedItem;
}

/// <summary>One page prepared for painting, plus its click-to-source index.</summary>
internal sealed record V2PreparedPage(
    int Number,
    double WidthDip,
    double HeightDip,
    IReadOnlyList<V2PreparedItem> Items,
    HitTestIndex HitTest,
    int GlyphCount);

/// <summary>
/// A display list whose fonts all resolved and whose pages are all prepared: either fully
/// paintable or absent, never partial.
/// </summary>
internal sealed record V2Frame(DisplayList List, IReadOnlyList<V2PreparedPage> Pages, IReadOnlyList<string> Notices)
{
    public int GlyphCount => this.Pages.Sum(p => p.GlyphCount);

    /// <summary>
    /// Resolves every font a glyph run references and prepares every page. Throws
    /// <see cref="RenderingV2ValidationException"/> on the first refusal; the caller treats
    /// that as "no v2 frame this revision" and keeps the v1 renderer.
    /// </summary>
    public static V2Frame Prepare(DisplayList list, FontFileStore fonts, IPreparedPageCache? cache = null)
    {
        ArgumentNullException.ThrowIfNull(list);
        ArgumentNullException.ThrowIfNull(fonts);

        var faces = ResolveFaces(list, fonts);
        var pages = new List<V2PreparedPage>(list.Pages.Count);
        var notices = new List<string>();
        foreach (Page page in list.Pages)
        {
            V2PreparedPage prepared = cache is null
                ? PreparePage(page, faces)
                : cache.GetOrBuild(page, p => PreparePage(p, faces));
            pages.Add(prepared);
            foreach (V2PreparedItem item in prepared.Items)
            {
                if (item is V2PreparedItem.ImageNoticeItem notice && !notices.Contains(notice.Notice))
                {
                    notices.Add(notice.Notice);
                }
            }
        }
        return new V2Frame(list, pages, notices);
    }

    /// <summary>Loads exactly the fonts at least one glyph run references, keyed by the wire's <c>font_id</c>.</summary>
    private static Dictionary<string, CanvasFontFace> ResolveFaces(DisplayList list, FontFileStore fonts)
    {
        var referenced = new List<string>();
        foreach (Page page in list.Pages)
        {
            foreach (Item item in page.Items)
            {
                if (item is Item.OfGlyphRun run && !referenced.Contains(run.Value.FontId))
                {
                    referenced.Add(run.Value.FontId);
                }
            }
        }

        var faces = new Dictionary<string, CanvasFontFace>(StringComparer.Ordinal);
        foreach (string fontId in referenced)
        {
            FontResource resource = list.Font(fontId)
                ?? throw new RenderingV2ValidationException("invalid_resource", $"font resource '{fontId}' is referenced by a glyph run but is not declared in fonts");
            ResolvedFontFile resolved = fonts.Resolve(resource);
            faces[fontId] = FontFaceLoader.Load(resolved);
        }
        return faces;
    }

    private static V2PreparedPage PreparePage(Page page, IReadOnlyDictionary<string, CanvasFontFace> faces)
    {
        var items = new List<V2PreparedItem>(page.Items.Count);
        int glyphs = 0;
        foreach (Item item in page.Items)
        {
            switch (item)
            {
                case Item.OfRule rule:
                    items.Add(new V2PreparedItem.RuleItem(
                        rule.Value.X.ToDips(), rule.Value.Top.ToDips(), rule.Value.Width.ToDips(), rule.Value.Height.ToDips(), ToColor(rule.Value.Paint)));
                    break;
                case Item.OfPath path:
                    items.Add(new V2PreparedItem.PathItem(path.Value, ToColor(path.Value.Paint)));
                    break;
                case Item.OfGlyphRun run:
                    items.Add(PrepareGlyphRun(run.Value, faces));
                    glyphs += run.Value.Glyphs.Count;
                    break;
                case Item.OfImage image:
                    // This consumer never requests `display-list-v2-images`, so a producer
                    // should never emit one; if one appears anyway it is reported, not faked.
                    items.Add(new V2PreparedItem.ImageNoticeItem(
                        $"image '{image.Value.ImageResource.Path}' was not painted: this preview does not implement display-list-v2 images yet"));
                    break;
            }
        }
        return new V2PreparedPage(page.Number, page.Width.ToDips(), page.Height.ToDips(), items, HitTestIndex.Build(page), glyphs);
    }

    private static V2PreparedItem PrepareGlyphRun(GlyphRun run, IReadOnlyDictionary<string, CanvasFontFace> faces)
    {
        CanvasFontFace face = faces[run.FontId];
        Glyph first = run.Glyphs[0];
        double originX = first.OriginX.ToDips();
        double originY = first.BaselineY.ToDips();
        var glyphs = new CanvasGlyph[run.Glyphs.Count];
        for (int i = 0; i < run.Glyphs.Count; i++)
        {
            Glyph glyph = run.Glyphs[i];
            glyphs[i] = new CanvasGlyph
            {
                Index = glyph.Gid,
                // Zero advance: every glyph is placed by its own offset from the run origin,
                // so DirectWrite never accumulates a position of its own. The producer's
                // advance_x/advance_y are metadata for the exporter, not a layout input here.
                Advance = 0,
                AdvanceOffset = (float)(glyph.OriginX.ToDips() - originX),
                // The ascender direction is UP, i.e. the negative y of this top-left,
                // y-down page space.
                AscenderOffset = (float)-(glyph.BaselineY.ToDips() - originY),
            };
        }
        return new V2PreparedItem.GlyphRunItem(
            face,
            (float)run.FontSize.ToDips(),
            new Vector2((float)originX, (float)originY),
            glyphs,
            ToColor(run.Paint));
    }

    /// <summary>sRGB components on the wire are 0...1 doubles; the validator already bounded them.</summary>
    internal static Color ToColor(Paint paint) => Color.FromArgb(
        Channel(paint.A), Channel(paint.R), Channel(paint.G), Channel(paint.B));

    private static byte Channel(double value) => (byte)Math.Clamp(Math.Round(value * 255.0), 0, 255);
}

/// <summary>
/// The per-page reuse the painter wants from <see cref="PageCache{TDrawable}"/>, expressed as an
/// interface so <see cref="V2Frame.Prepare"/> stays testable without one.
/// </summary>
internal interface IPreparedPageCache
{
    V2PreparedPage GetOrBuild(Page page, Func<Page, V2PreparedPage> build);
}

/// <summary>Adapts FlashTeX.Preview's content-digest-keyed LRU page cache to <see cref="IPreparedPageCache"/>.</summary>
internal sealed class PreparedPageCache : IPreparedPageCache
{
    private readonly PageCache<V2PreparedPage> cache = new();

    public V2PreparedPage GetOrBuild(Page page, Func<Page, V2PreparedPage> build) => this.cache.GetOrBuild(page, build);

    public int HitCount => this.cache.HitCount;

    public int MissCount => this.cache.MissCount;

    public void Clear() => this.cache.Clear();
}
