// name: MainWindow.Outline.cs
// purpose: Appends an "OUTLINE" section (sections, environments, labels) below
//   the "PROJECT" section MainWindow.ProjectFiles.cs's RebuildProjectTree already
//   builds into the same left-side pane, scanning the active document with
//   FlashTeX.ProjectFiles.DocumentOutline.Scan. Clicking an item reveals its
//   source span in the shared CodeMirror editor host via EditorHost.RevealSourceRange
//   — the same native-preview-click navigation MainWindow.Preview.cs already uses,
//   not a parallel mechanism.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.ProjectFiles;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;
using FlashTeX.Shell;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private const int MaxOutlineItems = 300;

    /// <summary>Adds the "OUTLINE" header plus one row per scanned item of <paramref name="active"/>'s text (nothing when there is no active document, or it is too large to scan).</summary>
    private void AppendOutlineSection(StackPanel panel, ShellDocument? active)
    {
        panel.Children.Add(new TextBlock
        {
            Text = "OUTLINE",
            FontSize = 12,
            Opacity = 0.65,
            Margin = new Thickness(6, 14, 6, 6),
        });

        if (active is null)
        {
            panel.Children.Add(new TextBlock { Text = "No active document", Opacity = 0.65, Margin = new Thickness(6) });
            return;
        }

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(active.Text);
        if (items.Count == 0)
        {
            panel.Children.Add(new TextBlock { Text = "No sections, environments or labels found", Opacity = 0.65, Margin = new Thickness(6) });
            return;
        }

        string path = active.Path;
        string text = active.Text;
        foreach (DocumentOutline.Item item in items.Take(MaxOutlineItems))
        {
            double indent = 8 + (Math.Max(0, item.Level) * 14);
            string glyph = item.Kind switch
            {
                DocumentOutline.Kind.Section => "§",
                DocumentOutline.Kind.Environment => "▤",
                DocumentOutline.Kind.Label => "🏷",
                _ => "•",
            };
            var row = new Button
            {
                Content = $"{glyph} {item.DisplayTitle}",
                HorizontalAlignment = HorizontalAlignment.Stretch,
                HorizontalContentAlignment = HorizontalAlignment.Left,
                Padding = new Thickness(indent, 3, 8, 3),
                Opacity = 0.9,
            };
            DocumentOutline.Item capturedItem = item;
            row.Click += (_, _) => RevealOutlineItem(path, text, capturedItem);
            panel.Children.Add(row);
        }

        if (items.Count > MaxOutlineItems)
        {
            panel.Children.Add(new TextBlock
            {
                Text = $"…and {items.Count - MaxOutlineItems} more",
                Opacity = 0.55,
                Margin = new Thickness(8, 2, 6, 2),
            });
        }
    }

    /// <summary>
    /// Converts the outline item's UTF-16 range back to UTF-8 byte offsets (the
    /// wire/editor-bridge coordinate space) and reveals it, only if the active
    /// document is still the one the outline was built from (the user could have
    /// switched tabs since the last RebuildProjectTree).
    /// </summary>
    private void RevealOutlineItem(string path, string scannedText, DocumentOutline.Item item)
    {
        if (_shell.ActiveDocumentPath != path)
        {
            return;
        }
        var current = ActiveDocumentOrNull();
        if (current is null || current.Text != scannedText)
        {
            return; // the buffer changed since this outline was scanned; stale offsets would misfire
        }
        (int StartByte, int EndByte)? byteRange = ByteOffsets.Utf8ByteRangeForUtf16Range(scannedText, item.Utf16.Location, item.Utf16.End);
        if (byteRange is not (int startByte, int endByte))
        {
            return;
        }
        _editorHost?.RevealSourceRange(new SourceRange(path, startByte, endByte));
    }
}
