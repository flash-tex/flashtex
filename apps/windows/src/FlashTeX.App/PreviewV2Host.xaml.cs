// name: PreviewV2Host.xaml.cs
// purpose: The rendering-v2 precision preview: paints a validated display-list-v2 frame
//   with Win2D/Direct2D, drawing glyphs by their ORIGINAL font glyph id at the producer's
//   absolute baseline origins through the embedded (content-addressed) font programs,
//   plus rules and path-v0 vector paths with their paint/stroke/dash/fill-rule/clip.
//   Click-to-source uses the wire's per-cluster hit rectangles through
//   FlashTeX.Preview.HitTestIndex and raises the same SourceRequested event the
//   runtime-v1 PreviewHost does, so MainWindow.Preview.cs routes both identically.
//
//   Unlike PreviewHost (which positions WinUI TextBlocks with substituted system fonts and
//   says so), nothing here is approximated: a frame whose fonts do not all resolve, or
//   whose geometry fails validation, is REFUSED whole and the caller keeps the v1 renderer.
// author: Claude Opus 5
// date: 2026-09-14

using System.Numerics;
using FlashTeX.Preview;
using FlashTeX.Protocol.RenderingV2;
using FlashTeX.Protocol.RuntimeV1;
using Microsoft.Graphics.Canvas;
using Microsoft.Graphics.Canvas.Brushes;
using Microsoft.Graphics.Canvas.Geometry;
using Microsoft.Graphics.Canvas.Text;
using Microsoft.Graphics.Canvas.UI.Xaml;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Foundation;
using Windows.UI;

namespace FlashTeX.App;

public sealed partial class PreviewV2Host : UserControl
{
    /// <summary>Win2D draws in device-independent pixels; the wire is in PDF points.</summary>
    private const double TicksPerDip = (double)Ticks.TicksPerPoint * 72.0 / 96.0;

    private readonly PreparedPageCache _pageCache = new();
    private readonly Dictionary<CanvasControl, V2PreparedPage> _pagesByCanvas = [];

    public PreviewV2Host()
    {
        InitializeComponent();
    }

    /// <summary>Raised when a click lands on an item whose wire provenance names a source range.</summary>
    public event EventHandler<SourceRange>? SourceRequested;

    /// <summary>The glyph count of the frame currently shown, for status/verification logging.</summary>
    public int LastFrameGlyphCount { get; private set; }

    /// <summary>Prepared pages reused from the cache rather than rebuilt, for the same reason.</summary>
    public int LastFrameReusedPages { get; private set; }

    /// <summary>
    /// Prepares and shows <paramref name="list"/>. Returns null on success, or the refusal
    /// message when the frame could not be prepared — in which case nothing was shown and
    /// the caller must fall back to the runtime-v1 renderer.
    /// </summary>
    public string? TryDisplay(DisplayList list, FontFileStore fonts)
    {
        ArgumentNullException.ThrowIfNull(list);
        ArgumentNullException.ThrowIfNull(fonts);
        V2Frame frame;
        try
        {
            frame = V2Frame.Prepare(list, fonts, this._pageCache);
        }
        catch (RenderingV2ValidationException ex)
        {
            return ex.ToString();
        }

        this.LastFrameGlyphCount = frame.GlyphCount;
        this.LastFrameReusedPages = this._pageCache.HitCount;
        Show(frame);
        return null;
    }

    /// <summary>Clears the surface (e.g. when the caller switches back to the v1 renderer).</summary>
    public void Clear()
    {
        foreach (CanvasControl canvas in this._pagesByCanvas.Keys)
        {
            canvas.RemoveFromVisualTree();
        }
        this._pagesByCanvas.Clear();
        this.PagesPanel.Children.Clear();
        this.EmptyText.Visibility = Visibility.Visible;
        this.NoticeBar.Visibility = Visibility.Collapsed;
    }

    private void Show(V2Frame frame)
    {
        Clear();
        this.EmptyText.Visibility = frame.Pages.Count > 0 ? Visibility.Collapsed : Visibility.Visible;
        foreach (V2PreparedPage page in frame.Pages)
        {
            var canvas = new CanvasControl
            {
                Width = page.WidthDip,
                Height = page.HeightDip,
                ClearColor = Colors.White,
            };
            this._pagesByCanvas[canvas] = page;
            canvas.Draw += OnPageDraw;
            canvas.PointerPressed += OnPagePointerPressed;
            this.PagesPanel.Children.Add(canvas);
        }
        if (frame.Notices.Count > 0)
        {
            this.NoticeText.Text = string.Join("  •  ", frame.Notices);
            this.NoticeBar.Visibility = Visibility.Visible;
        }
    }

    private void OnPageDraw(CanvasControl sender, CanvasDrawEventArgs args)
    {
        if (!this._pagesByCanvas.TryGetValue(sender, out V2PreparedPage? page))
        {
            return;
        }
        CanvasDrawingSession session = args.DrawingSession;
        session.Antialiasing = CanvasAntialiasing.Antialiased;
        // Grayscale rather than ClearType: the preview stands in for a printed page, and
        // subpixel-positioned colour fringes are not what the exporter will produce.
        session.TextAntialiasing = CanvasTextAntialiasing.Grayscale;
        // One brush per distinct colour on the page, not one per item: a body page is
        // thousands of glyph runs in a single black.
        var brushes = new Dictionary<Color, CanvasSolidColorBrush>();
        try
        {
            foreach (V2PreparedItem item in page.Items)
            {
                DrawItem(sender, session, item, brushes);
            }
        }
        finally
        {
            foreach (CanvasSolidColorBrush brush in brushes.Values)
            {
                brush.Dispose();
            }
        }
    }

    private static void DrawItem(
        ICanvasResourceCreator creator,
        CanvasDrawingSession session,
        V2PreparedItem item,
        Dictionary<Color, CanvasSolidColorBrush> brushes)
    {
        switch (item)
        {
            case V2PreparedItem.RuleItem rule:
                session.FillRectangle(new Windows.Foundation.Rect(rule.X, rule.Top, rule.Width, rule.Height), rule.Color);
                break;
            case V2PreparedItem.GlyphRunItem run:
                session.DrawGlyphRun(
                    run.Origin,
                    run.Face,
                    run.FontSizeDip,
                    run.Glyphs,
                    false,
                    0,
                    Brush(creator, brushes, run.Color),
                    CanvasTextMeasuringMode.Natural);
                break;
            case V2PreparedItem.PathItem path:
                DrawPath(creator, session, path);
                break;
            case V2PreparedItem.ImageNoticeItem:
                // Reported in the notice bar instead; nothing is painted in its place.
                break;
        }
    }

    private static CanvasSolidColorBrush Brush(ICanvasResourceCreator creator, Dictionary<Color, CanvasSolidColorBrush> brushes, Color color)
    {
        if (!brushes.TryGetValue(color, out CanvasSolidColorBrush? brush))
        {
            brush = new CanvasSolidColorBrush(creator, color);
            brushes[color] = brush;
        }
        return brush;
    }

    private static void DrawPath(ICanvasResourceCreator creator, CanvasDrawingSession session, V2PreparedItem.PathItem item)
    {
        var layers = new List<IDisposable>(item.Value.Clips.Count);
        try
        {
            foreach (ClipPath clip in item.Value.Clips)
            {
                using CanvasGeometry clipGeometry = BuildGeometry(creator, clip.Path, clip.FillRule);
                layers.Add(session.CreateLayer(1f, clipGeometry));
            }
            using CanvasGeometry geometry = BuildGeometry(creator, item.Value.Commands, item.Value.FillRule);
            if (item.Value.IsFill)
            {
                session.FillGeometry(geometry, item.Color);
            }
            else if (item.Value.Stroke is { } stroke)
            {
                float width = (float)stroke.Width.ToDips();
                session.DrawGeometry(geometry, item.Color, width, BuildStrokeStyle(stroke, width));
            }
        }
        finally
        {
            // Layers must close innermost-first.
            for (int i = layers.Count - 1; i >= 0; i--)
            {
                layers[i].Dispose();
            }
        }
    }

    /// <summary>Wire path-v0 commands (`m`/`l`/`c`/`z`, page ticks) to a Direct2D geometry in DIPs.</summary>
    private static CanvasGeometry BuildGeometry(ICanvasResourceCreator creator, IReadOnlyList<PathCommand> commands, FillRule fillRule)
    {
        using var builder = new CanvasPathBuilder(creator);
        builder.SetFilledRegionDetermination(fillRule == FillRule.evenodd
            ? CanvasFilledRegionDetermination.Alternate
            : CanvasFilledRegionDetermination.Winding);
        bool open = false;
        foreach (PathCommand command in commands)
        {
            switch (command)
            {
                case PathCommand.Move move:
                    if (open)
                    {
                        builder.EndFigure(CanvasFigureLoop.Open);
                    }
                    builder.BeginFigure(Point(move.X, move.Y));
                    open = true;
                    break;
                case PathCommand.Line line when open:
                    builder.AddLine(Point(line.X, line.Y));
                    break;
                case PathCommand.Cubic cubic when open:
                    builder.AddCubicBezier(Point(cubic.X1, cubic.Y1), Point(cubic.X2, cubic.Y2), Point(cubic.X, cubic.Y));
                    break;
                case PathCommand.Close when open:
                    builder.EndFigure(CanvasFigureLoop.Closed);
                    open = false;
                    break;
            }
        }
        if (open)
        {
            builder.EndFigure(CanvasFigureLoop.Open);
        }
        return CanvasGeometry.CreatePath(builder);
    }

    /// <summary>
    /// Direct2D expresses dash lengths as MULTIPLES of the stroke width, while the wire
    /// gives them in ticks, so both the array and the phase are divided by the width here.
    /// </summary>
    private static CanvasStrokeStyle BuildStrokeStyle(Stroke stroke, float widthDip)
    {
        var style = new CanvasStrokeStyle
        {
            StartCap = ToCapStyle(stroke.Cap),
            EndCap = ToCapStyle(stroke.Cap),
            DashCap = ToCapStyle(stroke.Cap),
            LineJoin = stroke.Join switch
            {
                LineJoin.round => CanvasLineJoin.Round,
                LineJoin.bevel => CanvasLineJoin.Bevel,
                _ => CanvasLineJoin.Miter,
            },
            MiterLimit = (float)stroke.MiterLimit,
        };
        if (stroke.Dash is { } dash && widthDip > 0)
        {
            style.CustomDashStyle = dash.Array.Select(t => (float)(t.ToDips() / widthDip)).ToArray();
            style.DashOffset = (float)(dash.Phase.ToDips() / widthDip);
        }
        return style;
    }

    private static CanvasCapStyle ToCapStyle(LineCap cap) => cap switch
    {
        LineCap.round => CanvasCapStyle.Round,
        LineCap.square => CanvasCapStyle.Square,
        _ => CanvasCapStyle.Flat,
    };

    private static Vector2 Point(Ticks x, Ticks y) => new((float)x.ToDips(), (float)y.ToDips());

    private void OnPagePointerPressed(object sender, Microsoft.UI.Xaml.Input.PointerRoutedEventArgs args)
    {
        if (sender is not CanvasControl canvas || !this._pagesByCanvas.TryGetValue(canvas, out V2PreparedPage? page))
        {
            return;
        }
        Microsoft.UI.Input.PointerPoint point = args.GetCurrentPoint(canvas);
        if (!point.Properties.IsLeftButtonPressed)
        {
            return;
        }
        HitTestEntry? entry = page.HitTest.Query(
            (long)Math.Round(point.Position.X * TicksPerDip),
            (long)Math.Round(point.Position.Y * TicksPerDip));
        if (entry?.PrimarySource is { } source)
        {
            this.SourceRequested?.Invoke(this, source);
            args.Handled = true;
        }
    }
}
