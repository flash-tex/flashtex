// name: EditorHost.Completion.cs
// purpose: Native WinUI3 Flyouts for completion and signature help -- the
//   Windows port's deliberate design choice (already reflected in
//   FlashTeX.Editor/Completion.cs and SignatureHelp.cs, and in main.ts's own
//   header comment) that these popups are native controls positioned from
//   the caret's WebView2-relative screen rect (`set_caret_screen_rect`),
//   never CodeMirror's own tooltip/panel widgets. Completion candidates come
//   from FlashTeX.Editor.Completion.CommandSuggestions (pure ranking over the
//   compiler's command vocabulary); signature help from
//   FlashTeX.Editor.SignatureHelp.Info (pure scan of the mirrored document
//   text). Picking a completion item applies it through
//   EditorHost.ApplyNativeEdit so ShellModel and the live CM6 view never
//   disagree about the document's contents.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Editor;
using FlashTeX.Protocol;
using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Documents;
using Windows.Foundation;

namespace FlashTeX.App;

public sealed partial class EditorHost
{
    private int? _lastAnchorByte;
    private int? _lastCaretByte;

    /// <summary>The caret's last-reported client-area rect (WebView2-control-relative pixels,
    /// per `set_caret_screen_rect` -- see bridge.ts's doc comment on that message), used to
    /// position both Flyouts below (completion) or above (signature help) the caret.</summary>
    private (double X, double Y, double Width, double Height)? _lastCaretRect;

    private string? _completionRequestPrefix;
    private int? _completionRequestPositionByte;
    private Flyout? _completionFlyout;
    private Flyout? _signatureFlyout;

    private void HandleCompletionRequest(string requestId, CompletionRequestWire request)
    {
        _completionRequestPrefix = request.Prefix;
        _completionRequestPositionByte = request.PositionByte;

        IReadOnlyList<CompletionSuggestion> suggestions = Completion.CommandSuggestions(request.Prefix);
        SendCompletionReplyAck(requestId);
        ShowCompletionFlyout(suggestions);
    }

    private void ShowCompletionFlyout(IReadOnlyList<CompletionSuggestion> suggestions)
    {
        if (suggestions.Count == 0)
        {
            _completionFlyout?.Hide();
            return;
        }

        var listView = new ListView { SelectionMode = ListViewSelectionMode.Single, MaxHeight = 260, MinWidth = 220 };
        foreach (CompletionSuggestion suggestion in suggestions)
        {
            listView.Items.Add(new ListViewItem { Content = CompletionRow(suggestion), Tag = suggestion });
        }
        listView.SelectionChanged += (_, _) =>
        {
            if (listView.SelectedItem is ListViewItem { Tag: CompletionSuggestion picked })
            {
                AcceptCompletion(picked);
                _completionFlyout?.Hide();
            }
        };

        _completionFlyout ??= new Flyout();
        _completionFlyout.Content = listView;
        ShowFlyout(_completionFlyout, below: true);
    }

    private static StackPanel CompletionRow(CompletionSuggestion suggestion)
    {
        var stack = new StackPanel { Orientation = Orientation.Vertical, Spacing = 2, Padding = new Thickness(2) };
        stack.Children.Add(new TextBlock { Text = suggestion.Label, FontWeight = FontWeights.SemiBold });
        stack.Children.Add(new TextBlock
        {
            Text = suggestion.Detail,
            FontSize = 11,
            Foreground = (Microsoft.UI.Xaml.Media.Brush)Application.Current.Resources["TextFillColorSecondaryBrush"],
        });
        return stack;
    }

    private void AcceptCompletion(CompletionSuggestion suggestion)
    {
        if (_completionRequestPositionByte is not int positionByte || _completionRequestPrefix is not { } prefix)
        {
            return;
        }
        int prefixByteLength = ByteOffsets.Utf8ByteCount(prefix);
        int backslashByteLength = 1; // the triggering '\' is always one ASCII byte
        int startByte = Math.Max(0, positionByte - prefixByteLength - backslashByteLength);
        ApplyNativeEdit(startByte, positionByte, suggestion.InsertText);
    }

    private void UpdateSignatureHelp()
    {
        if (_lastCaretByte is not int caretByte)
        {
            _signatureFlyout?.Hide();
            return;
        }
        (int Utf16Start, int Utf16End)? range = ByteOffsets.Utf16RangeForUtf8Bytes(_mirroredText, caretByte, caretByte);
        if (range is not (int caretUtf16, _) || SignatureHelp.Info(_mirroredText, caretUtf16) is not { } info)
        {
            _signatureFlyout?.Hide();
            return;
        }
        ShowSignatureHelpFlyout(info);
    }

    private void ShowSignatureHelpFlyout(SignatureHelpInfo info)
    {
        (string text, Utf16Range? active) = info.Display;

        var signatureText = new TextBlock();
        if (active is { } activeRange)
        {
            signatureText.Inlines.Add(new Run { Text = text[..activeRange.Location] });
            signatureText.Inlines.Add(new Run
            {
                Text = text.Substring(activeRange.Location, activeRange.Length),
                FontWeight = FontWeights.Bold,
            });
            signatureText.Inlines.Add(new Run { Text = text[activeRange.End..] });
        }
        else
        {
            signatureText.Inlines.Add(new Run { Text = text });
        }

        var description = new TextBlock
        {
            Text = info.Description,
            FontSize = 11,
            Margin = new Thickness(0, 4, 0, 0),
            Foreground = (Microsoft.UI.Xaml.Media.Brush)Application.Current.Resources["TextFillColorSecondaryBrush"],
        };
        var stack = new StackPanel { Orientation = Orientation.Vertical, Padding = new Thickness(4) };
        stack.Children.Add(signatureText);
        stack.Children.Add(description);

        _signatureFlyout ??= new Flyout();
        _signatureFlyout.Content = stack;
        ShowFlyout(_signatureFlyout, below: false);
    }

    private void ShowFlyout(Flyout flyout, bool below)
    {
        var options = new FlyoutShowOptions
        {
            Placement = below ? FlyoutPlacementMode.BottomEdgeAlignedLeft : FlyoutPlacementMode.TopEdgeAlignedLeft,
        };
        if (_lastCaretRect is { } rect)
        {
            options.Position = new Point(rect.X, below ? rect.Y + rect.Height : rect.Y);
        }
        flyout.ShowAt(WebView, options);
    }
}
