// name: MainWindow.TitleBar.cs
// purpose: Extends window content into the title bar (Window.ExtendsContentIntoTitleBar)
//   and hosts the menu bar (MainWindow.Menu.cs's AppMenuBar) inside that
//   extended row, Fluent-style — the pattern Windows Terminal, File Explorer
//   and Notepad use, and the closest Windows analogue to the Mac original
//   putting its menu in the system-wide menu bar (FlashTeXMacApp.swift's
//   `.commands{}`) rather than a second row under a separate native title
//   bar. A MicaBackdrop lets the Fluent material show through the now-
//   transparent title bar and (system-drawn) caption buttons. The one
//   genuinely fiddly part: WinUI3 does not automatically know the MenuBar is
//   interactive once its parent is marked as the drag region (SetTitleBar) —
//   InputNonClientPointerSource.SetRegionRects(Passthrough, ...) must be told
//   the menu bar's exact rectangle, in raw (not XAML-effective) pixels, or
//   clicks on File/Edit/View/Navigate would just drag the window instead of
//   opening their dropdowns. Recomputed on every resize since that rectangle
//   moves/resizes with the window.
// author: Claude Sonnet 5
// date: 2026-09-14

using Microsoft.UI;
using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml.Media;
using Windows.Graphics;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    /// <summary>Standard Fluent title-bar row height (Windows Terminal/File Explorer/Notepad all use ~40px).</summary>
    private const int TitleBarRowHeight = 40;

    private void SetupTitleBar()
    {
        ExtendsContentIntoTitleBar = true;
        TrySetMicaBackdrop();
        MakeTitleBarTransparentForMica();
        SetTitleBar(TitleBarRow);

        TitleBarRow.SizeChanged += (_, _) => UpdateTitleBarPassthroughRegion();
        AppMenuBar.SizeChanged += (_, _) => UpdateTitleBarPassthroughRegion();
    }

    /// <summary>Mica requires Windows 11 (22000+); older Windows 10 hosts fall back to the default backdrop rather than failing.</summary>
    private void TrySetMicaBackdrop()
    {
        if (MicaController.IsSupported())
        {
            SystemBackdrop = new MicaBackdrop();
        }
    }

    /// <summary>Lets the Mica backdrop (or the default background on older hosts) show through instead of an opaque system title-bar color, in both light and dark theme.</summary>
    private void MakeTitleBarTransparentForMica()
    {
        var titleBar = AppWindow.TitleBar;
        titleBar.BackgroundColor = Colors.Transparent;
        titleBar.InactiveBackgroundColor = Colors.Transparent;
        titleBar.ButtonBackgroundColor = Colors.Transparent;
        titleBar.ButtonInactiveBackgroundColor = Colors.Transparent;
        TitleBarRow.Background = new SolidColorBrush(Colors.Transparent);
        AppMenuBar.Background = new SolidColorBrush(Colors.Transparent);
    }

    /// <summary>
    /// Marks the menu bar's current on-screen rectangle as a "passthrough" non-client
    /// region: normal hit-testing (so its dropdowns open) inside a row that
    /// <see cref="SetTitleBar"/> otherwise treats entirely as a window-drag handle.
    /// </summary>
    private void UpdateTitleBarPassthroughRegion()
    {
        if (Content.XamlRoot is not { } xamlRoot || AppMenuBar.ActualWidth == 0)
        {
            return;
        }

        var scale = xamlRoot.RasterizationScale;
        var transform = AppMenuBar.TransformToVisual(Content);
        var bounds = transform.TransformBounds(new Windows.Foundation.Rect(0, 0, AppMenuBar.ActualWidth, AppMenuBar.ActualHeight));
        var rawRect = new RectInt32(
            (int)(bounds.X * scale), (int)(bounds.Y * scale),
            (int)(bounds.Width * scale), (int)(bounds.Height * scale));

        InputNonClientPointerSource.GetForWindowId(AppWindow.Id)
            .SetRegionRects(NonClientRegionKind.Passthrough, new[] { rawRect });
    }
}
