// name: PreviewHost.xaml.cs
// purpose: Native WinUI renderer for runtime-v1 compile pages. It intentionally renders
// only the v1 contract's text/rule primitives; rendering-v2's embedded-font glyph painter
// remains a separate precision milestone rather than pretending system font substitution is exact.

using FlashTeX.Protocol.RuntimeV1;
using Microsoft.UI;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Microsoft.UI.Xaml.Shapes;
using Windows.UI;

namespace FlashTeX.App;

public sealed partial class PreviewHost : UserControl
{
    private const double PointsToDip = 96.0 / 72.0;

    public event EventHandler<SourceRange>? SourceRequested;

    public PreviewHost()
    {
        InitializeComponent();
    }

    public void Display(IReadOnlyList<FlashTeX.Protocol.RuntimeV1.Page>? pages)
    {
        PagesPanel.Children.Clear();
        EmptyText.Visibility = pages is { Count: > 0 } ? Visibility.Collapsed : Visibility.Visible;
        if (pages is not { Count: > 0 })
        {
            return;
        }

        foreach (FlashTeX.Protocol.RuntimeV1.Page page in pages)
        {
            PagesPanel.Children.Add(BuildPage(page));
        }
    }

    private Canvas BuildPage(FlashTeX.Protocol.RuntimeV1.Page page)
    {
        var canvas = new Canvas
        {
            Width = page.WidthPt * PointsToDip,
            Height = page.HeightPt * PointsToDip,
            Background = new SolidColorBrush(Colors.White),
        };
        canvas.Shadow = new ThemeShadow();
        foreach (PageItem item in page.Items)
        {
            switch (item)
            {
                case PageItem.OfText text:
                    AddText(canvas, text.Item);
                    break;
                case PageItem.OfRule rule:
                    AddRule(canvas, rule.Item);
                    break;
            }
        }
        return canvas;
    }

    private void AddText(Canvas canvas, PageItem.TextItem item)
    {
        var text = new TextBlock
        {
            Text = item.Text,
            FontSize = item.FontSizePt * PointsToDip,
            Foreground = new SolidColorBrush(Colors.Black),
            IsTextSelectionEnabled = true,
        };
        if (item.Font is { } font)
        {
            text.FontFamily = new FontFamily(font.Family);
            if (font.Weight == PageItem.FontHint.FontWeight.bold)
            {
                text.FontWeight = Microsoft.UI.Text.FontWeights.Bold;
            }
            if (font.Style == PageItem.FontHint.FontStyle.italic)
            {
                text.FontStyle = Windows.UI.Text.FontStyle.Italic;
            }
        }
        Canvas.SetLeft(text, item.XPt * PointsToDip);
        Canvas.SetTop(text, Math.Max(0, (item.BaselineYPt - item.FontSizePt) * PointsToDip));
        AddSourceGesture(text, item.Source);
        canvas.Children.Add(text);
    }

    private void AddRule(Canvas canvas, PageItem.RuleItem item)
    {
        var rule = new Rectangle
        {
            Width = item.WidthPt * PointsToDip,
            Height = item.HeightPt * PointsToDip,
            Fill = new SolidColorBrush(Colors.Black),
        };
        Canvas.SetLeft(rule, item.XPt * PointsToDip);
        Canvas.SetTop(rule, item.YPt * PointsToDip);
        AddSourceGesture(rule, item.Source);
        canvas.Children.Add(rule);
    }

    private void AddSourceGesture(UIElement element, SourceRange? source)
    {
        if (source is null)
        {
            return;
        }
        element.PointerPressed += (_, args) =>
        {
            if (args.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
            {
                SourceRequested?.Invoke(this, source);
                args.Handled = true;
            }
        };
    }
}
