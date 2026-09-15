// name: EditorHost.Bridge.cs
// purpose: The native-side half of the WebView2 <-> CodeMirror bridge
//   protocol (envelope shape `{v, id, type, payload}`, UTF-8 byte offsets):
//   outgoing message senders (set_document/apply_edit/set_diagnostics/
//   set_theme/set_font_size) and incoming message dispatch (edit_made/
//   selection_changed/caret_moved/completion_request/set_caret_screen_rect).
//   Field names and shapes mirror src/FlashTeX.Editor/web/src/bridge.ts
//   exactly -- see that file's own doc comment and test/bridge.test.ts for
//   the contract this must match byte-for-byte.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;
using System.Text.Json.Serialization;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;
using Microsoft.Web.WebView2.Core;

namespace FlashTeX.App;

public sealed partial class EditorHost
{
    private const int ProtocolVersion = 1;

    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);

    // MARK: - Outgoing (native -> JS)

    private void SendSetDocument(string text, int revision) =>
        Post("set_document", new SetDocumentPayload(text, revision));

    private void SendApplyEdit(int startByte, int endByte, string replacement, int revision) =>
        Post("apply_edit", new ApplyEditPayload(startByte, endByte, replacement, revision));

    private void SendRevealRange(int startByte, int endByte) =>
        Post("reveal_range", new RevealRangePayload(startByte, endByte));

    /// <summary>Reveals and selects a source span requested by the native preview.</summary>
    public void RevealSourceRange(SourceRange source)
    {
        ArgumentNullException.ThrowIfNull(source);
        SendRevealRange(source.StartByte, source.EndByte);
    }

    private void SendDiagnostics()
    {
        if (_mirroredPath is not { } path)
        {
            return;
        }
        var items = new List<DiagnosticPayload>();
        foreach (Diagnostic diagnostic in _shell.Diagnostics)
        {
            if (diagnostic.Source is not { } source || source.Path != path)
            {
                continue;
            }
            items.Add(new DiagnosticPayload(
                source.StartByte,
                source.EndByte,
                diagnostic.Severity == Severity.error ? "error" : "warning",
                diagnostic.Message,
                diagnostic.Code));
        }
        Post("set_diagnostics", items);
    }

    private void SendTheme()
    {
        bool dark = ActualTheme == Microsoft.UI.Xaml.ElementTheme.Dark;
        Post("set_theme", new SetThemePayload(dark ? "dark" : "light"));
    }

    /// <summary>Converts ShellModel's point-size font (matching the rest of this port's UI, which
    /// is otherwise measured in points/DIPs) to CSS pixels at the standard 96 DPI reference.</summary>
    private void SendFontSize()
    {
        double fontSizePx = _shell.EditorFontSize * 96.0 / 72.0;
        Post("set_font_size", new SetFontSizePayload(fontSizePx));
    }

    /// <summary>Native -> JS `completion_reply`: acknowledges a `completion_request` for
    /// protocol completeness. JS's handler for this message is a deliberate no-op (see main.ts's
    /// header comment) -- the actual suggestions render as a native Flyout
    /// (EditorHost.Completion.cs), never inside the WebView2 page itself.</summary>
    private void SendCompletionReplyAck(string requestId)
    {
        Post("completion_reply", new CompletionReplyPayload(requestId, Array.Empty<CompletionItemPayload>()));
    }

    private void Post(string type, object payload)
    {
        if (WebView.CoreWebView2 is not { } core)
        {
            return;
        }
        var envelope = new OutgoingEnvelope(ProtocolVersion, $"native-{Guid.NewGuid():N}", type, payload);
        core.PostWebMessageAsJson(JsonSerializer.Serialize(envelope, JsonOptions));
    }

    // MARK: - Incoming (JS -> native)

    private void OnWebMessageReceived(CoreWebView2 sender, CoreWebView2WebMessageReceivedEventArgs args)
    {
        IncomingEnvelope? envelope;
        try
        {
            envelope = JsonSerializer.Deserialize<IncomingEnvelope>(args.WebMessageAsJson, JsonOptions);
        }
        catch (JsonException)
        {
            return; // malformed message from the page; nothing native can do with it.
        }
        if (envelope is not { V: ProtocolVersion } message)
        {
            return;
        }

        switch (message.Type)
        {
            case "edit_made":
                if (Deserialize<EditMadeWire>(message.Payload) is { } editMade)
                {
                    HandleEditMade(editMade);
                }
                break;
            case "selection_changed":
                if (Deserialize<SelectionChangedWire>(message.Payload) is { } selectionChanged)
                {
                    HandleSelectionChanged(selectionChanged);
                }
                break;
            case "caret_moved":
                if (Deserialize<CaretMovedWire>(message.Payload) is { } caretMoved)
                {
                    HandleCaretMoved(caretMoved);
                }
                break;
            case "completion_request":
                if (Deserialize<CompletionRequestWire>(message.Payload) is { } completionRequest)
                {
                    HandleCompletionRequest(message.Id, completionRequest);
                }
                break;
            case "set_caret_screen_rect":
                if (Deserialize<SetCaretScreenRectWire>(message.Payload) is { } caretRect)
                {
                    HandleSetCaretScreenRect(caretRect);
                }
                break;
        }
    }

    private static T? Deserialize<T>(JsonElement payload)
        where T : class
    {
        try
        {
            return payload.Deserialize<T>(JsonOptions);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    private void HandleEditMade(EditMadeWire edit)
    {
        if (_mirroredPath is not { } path)
        {
            return;
        }
        (int Utf16Start, int Utf16End)? range = ByteOffsets.Utf16RangeForUtf8Bytes(_mirroredText, edit.StartByte, edit.EndByte);
        if (range is not (int utf16Start, int utf16End))
        {
            return; // out-of-sync byte offsets; drop rather than corrupt the mirror.
        }

        string newText = _mirroredText[..utf16Start] + edit.Replacement + _mirroredText[utf16End..];
        _mirroredText = newText;

        _applyingRemoteEdit = true;
        try
        {
            _shell.UpdateDocumentText(path, newText);
        }
        finally
        {
            _applyingRemoteEdit = false;
        }
    }

    private void HandleSelectionChanged(SelectionChangedWire selection)
    {
        _lastAnchorByte = selection.AnchorByte;
        _lastCaretByte = selection.HeadByte;
    }

    private void HandleCaretMoved(CaretMovedWire caret)
    {
        _lastCaretByte = caret.ByteOffset;
        UpdateSignatureHelp();
    }

    private void HandleSetCaretScreenRect(SetCaretScreenRectWire rect)
    {
        _lastCaretRect = (rect.X, rect.Y, rect.Width, rect.Height);
    }

    // MARK: - Wire envelope shapes (field names/order match bridge.ts exactly)

    private sealed record OutgoingEnvelope(
        [property: JsonPropertyName("v")] int V,
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("type")] string Type,
        [property: JsonPropertyName("payload")] object Payload);

    private sealed record IncomingEnvelope(
        [property: JsonPropertyName("v")] int V,
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("type")] string Type,
        [property: JsonPropertyName("payload")] JsonElement Payload);

    private sealed record SetDocumentPayload(
        [property: JsonPropertyName("text")] string Text,
        [property: JsonPropertyName("revision")] int Revision);

    private sealed record ApplyEditPayload(
        [property: JsonPropertyName("start_byte")] int StartByte,
        [property: JsonPropertyName("end_byte")] int EndByte,
        [property: JsonPropertyName("replacement")] string Replacement,
        [property: JsonPropertyName("revision")] int Revision);

    private sealed record RevealRangePayload(
        [property: JsonPropertyName("start_byte")] int StartByte,
        [property: JsonPropertyName("end_byte")] int EndByte);

    private sealed record DiagnosticPayload(
        [property: JsonPropertyName("start_byte")] int StartByte,
        [property: JsonPropertyName("end_byte")] int EndByte,
        [property: JsonPropertyName("severity")] string Severity,
        [property: JsonPropertyName("message")] string Message,
        [property: JsonPropertyName("code")] string? Code);

    private sealed record SetThemePayload([property: JsonPropertyName("theme")] string Theme);

    private sealed record SetFontSizePayload([property: JsonPropertyName("font_size_px")] double FontSizePx);

    private sealed record CompletionItemPayload(
        [property: JsonPropertyName("label")] string Label,
        [property: JsonPropertyName("detail")] string? Detail,
        [property: JsonPropertyName("insert_text")] string? InsertText);

    private sealed record CompletionReplyPayload(
        [property: JsonPropertyName("request_id")] string RequestId,
        [property: JsonPropertyName("items")] IReadOnlyList<CompletionItemPayload> Items);

    private sealed record EditMadeWire(
        [property: JsonPropertyName("start_byte")] int StartByte,
        [property: JsonPropertyName("end_byte")] int EndByte,
        [property: JsonPropertyName("removed_text")] string RemovedText,
        [property: JsonPropertyName("replacement")] string Replacement,
        [property: JsonPropertyName("new_revision")] int NewRevision);

    private sealed record SelectionChangedWire(
        [property: JsonPropertyName("anchor_byte")] int AnchorByte,
        [property: JsonPropertyName("head_byte")] int HeadByte);

    private sealed record CaretMovedWire([property: JsonPropertyName("byte_offset")] int ByteOffset);

    private sealed record CompletionRequestWire(
        [property: JsonPropertyName("position_byte")] int PositionByte,
        [property: JsonPropertyName("prefix")] string Prefix);

    private sealed record SetCaretScreenRectWire(
        [property: JsonPropertyName("x")] double X,
        [property: JsonPropertyName("y")] double Y,
        [property: JsonPropertyName("width")] double Width,
        [property: JsonPropertyName("height")] double Height);
}
