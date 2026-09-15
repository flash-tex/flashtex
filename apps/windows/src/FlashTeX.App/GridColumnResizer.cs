// name: GridColumnResizer.cs
// purpose: A minimal draggable-divider behavior for a Grid's pixel-width
//   ColumnDefinition (the neighboring column stays "*" and fills whatever
//   remains), used for the project-tree | editor+preview split and the
//   editor | preview split (MainWindow.xaml's LeftSplitter/CenterSplitter).
//   WinUI3 has no built-in GridSplitter control (unlike WPF/UWP's Community
//   Toolkit); this avoids pulling in an extra NuGet dependency for one small
//   behavior. Width persistence (PaneSettings.cs) is the caller's concern via
//   onDragCompleted, not this class's.
// author: Claude Sonnet 5
// date: 2026-09-14

using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;

namespace FlashTeX.App;

/// <summary>Attaches drag-to-resize behavior to a splitter element over one pixel-width <see cref="ColumnDefinition"/>.</summary>
internal static class GridColumnResizer
{
    private const double HandleHitTestWidth = 6;

    /// <summary>
    /// Makes <paramref name="handle"/> drag <paramref name="column"/>'s width between
    /// <paramref name="minWidth"/> and <paramref name="maxWidth"/>, starting from
    /// <paramref name="initialWidth"/>. <paramref name="onDragCompleted"/> (if given) is
    /// called with the final width once the drag ends, for the caller to persist.
    /// </summary>
    public static void Attach(Border handle, ColumnDefinition column, double minWidth, double maxWidth,
        double initialWidth, Action<double>? onDragCompleted = null)
    {
        column.Width = new GridLength(Math.Clamp(initialWidth, minWidth, maxWidth));

        double? dragStartX = null;
        double dragStartWidth = 0;

        // Note: WinUI3's cursor-changing API (UIElement.ChangeCursor) is `protected`, only
        // reachable from within a custom control subclass, so this splitter does not swap
        // in a resize cursor on hover; the drag behavior itself works regardless.
        handle.PointerPressed += (sender, e) =>
        {
            dragStartX = e.GetCurrentPoint(handle).Position.X;
            dragStartWidth = column.ActualWidth;
            ((UIElement)sender).CapturePointer(e.Pointer);
        };
        handle.PointerMoved += (_, e) =>
        {
            if (dragStartX is not { } startX)
            {
                return;
            }
            var deltaX = e.GetCurrentPoint(handle).Position.X - startX;
            column.Width = new GridLength(Math.Clamp(dragStartWidth + deltaX, minWidth, maxWidth));
        };
        handle.PointerReleased += (sender, e) =>
        {
            dragStartX = null;
            ((UIElement)sender).ReleasePointerCapture(e.Pointer);
            onDragCompleted?.Invoke(column.ActualWidth);
        };
        handle.Width = HandleHitTestWidth;
    }
}
