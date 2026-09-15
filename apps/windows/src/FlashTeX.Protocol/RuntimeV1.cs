// name: RuntimeV1.cs
// purpose: Compile request/result envelopes, diagnostics, page items and layout
//   capability negotiation for FlashTeX runtime protocol v1 (docs/contracts/runtime-v1.md,
//   docs/contracts/runtime-v1-layout-capabilities.md). Ported from
//   apps/mac/Sources/FlashTeXProtocol/RuntimeV1.swift.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text.Json;
using System.Text.Json.Serialization;

namespace FlashTeX.Protocol.RuntimeV1;

/// <summary>
/// Envelope shared by every runtime-v1 message: <c>protocol_version</c>, <c>id</c>,
/// <c>type</c>, <c>payload</c>. Replies preserve the request's <c>id</c>.
/// </summary>
public sealed record Envelope<TPayload>(
    [property: JsonPropertyName("protocol_version")] int ProtocolVersion,
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("type")] string Type,
    [property: JsonPropertyName("payload")] TPayload Payload);

/// <summary>Constants and decode helpers shared by every runtime-v1 message.</summary>
public static class Protocol
{
    public const int Version = 1;

    public static Envelope<CompileRequest> DecodeCompileRequest(ReadOnlySpan<byte> data, JsonSerializerOptions options) =>
        Decode<CompileRequest>(data, "compile", options);

    public static Envelope<CompileResult> DecodeCompileResult(ReadOnlySpan<byte> data, JsonSerializerOptions options) =>
        Decode<CompileResult>(data, "compile_result", options);

    /// <summary>Decodes an envelope and checks its protocol version and message type.</summary>
    public static Envelope<TPayload> Decode<TPayload>(ReadOnlySpan<byte> data, string expectedType, JsonSerializerOptions options)
    {
        var envelope = JsonSerializer.Deserialize<Envelope<TPayload>>(data, options)
            ?? throw new RuntimeV1DecodeException("envelope decoded to null");
        if (envelope.ProtocolVersion != Version)
        {
            throw RuntimeV1DecodeException.UnsupportedVersion(envelope.ProtocolVersion);
        }
        if (envelope.Type != expectedType)
        {
            throw RuntimeV1DecodeException.UnexpectedType(expectedType, envelope.Type);
        }
        return envelope;
    }
}

/// <summary>Decode-time failures mirroring the Swift source's <c>RuntimeV1.DecodeError</c>.</summary>
public sealed class RuntimeV1DecodeException : Exception
{
    public RuntimeV1DecodeException(string message) : base(message)
    {
    }

    public static RuntimeV1DecodeException UnsupportedVersion(int version) =>
        new($"unsupported protocol_version {version}");

    public static RuntimeV1DecodeException UnexpectedType(string expected, string actual) =>
        new($"expected message type '{expected}' but found '{actual}'");

    public static RuntimeV1DecodeException InvalidLayoutCapabilities(string reason) =>
        new($"invalid layout_capabilities: {reason}");
}

/// <summary>One project-relative source document supplied with a compile request.</summary>
public sealed record Document(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("text")] string Text);

/// <summary>
/// Zero-based, end-exclusive UTF-8 byte offsets into the input revision named by
/// <see cref="Path"/>. Swift/​.NET code must convert explicitly to editor coordinates
/// via <see cref="ByteOffsets"/>.
/// </summary>
public sealed record SourceRange(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("start_byte")] int StartByte,
    [property: JsonPropertyName("end_byte")] int EndByte);

/// <summary>
/// Negotiated layout capabilities (runtime-v1-layout-capabilities.md): a request's
/// list is at most <see cref="MaxCount"/> unique, nonempty strings of at most
/// <see cref="MaxBytes"/> UTF-8 bytes each.
/// </summary>
public static class LayoutCapabilities
{
    public const string RulesV1 = "rules-v1";
    public const string FontHintsV1 = "font-hints-v1";

    /// <summary>Capabilities this client knows how to consume.</summary>
    public static readonly IReadOnlyList<string> Supported = new[] { RulesV1, FontHintsV1 };

    public const int MaxCount = 16;
    public const int MaxBytes = 64;

    public static void Validate(IReadOnlyList<string> capabilities)
    {
        if (capabilities.Count > MaxCount)
        {
            throw RuntimeV1DecodeException.InvalidLayoutCapabilities(
                $"{capabilities.Count} capabilities exceed the limit of {MaxCount}");
        }
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var capability in capabilities)
        {
            ValidateOne(capability, seen);
        }
    }

    private static void ValidateOne(string capability, HashSet<string> seen)
    {
        if (capability.Length == 0)
        {
            throw RuntimeV1DecodeException.InvalidLayoutCapabilities("empty capability string");
        }
        int byteCount = ByteOffsets.Utf8ByteCount(capability);
        if (byteCount > MaxBytes)
        {
            throw RuntimeV1DecodeException.InvalidLayoutCapabilities($"capability of {byteCount} bytes exceeds {MaxBytes}");
        }
        if (!seen.Add(capability))
        {
            throw RuntimeV1DecodeException.InvalidLayoutCapabilities($"duplicate capability {capability}");
        }
    }
}

/// <summary>
/// A <c>compile</c> request: project/revision identity, the entry document, its
/// full document set, and optional negotiated-capability fields.
/// </summary>
public sealed record CompileRequest
{
    [JsonConstructor]
    public CompileRequest(
        string projectId,
        int revision,
        string entryPath,
        IReadOnlyList<Document> documents,
        IReadOnlyList<string>? layoutCapabilities = null,
        DisplayListBaseInfo? displayListBase = null,
        string? projectRoot = null)
    {
        if (layoutCapabilities is not null)
        {
            // Fully qualified: the `LayoutCapabilities` property declared below on this
            // same record would otherwise shadow the static `LayoutCapabilities` type.
            FlashTeX.Protocol.RuntimeV1.LayoutCapabilities.Validate(layoutCapabilities);
        }
        ProjectId = projectId;
        Revision = revision;
        EntryPath = entryPath;
        Documents = documents;
        LayoutCapabilities = layoutCapabilities;
        DisplayListBase = displayListBase;
        ProjectRoot = projectRoot;
    }

    [JsonPropertyName("project_id")]
    public string ProjectId { get; }

    [JsonPropertyName("revision")]
    public int Revision { get; }

    [JsonPropertyName("entry_path")]
    public string EntryPath { get; }

    [JsonPropertyName("documents")]
    public IReadOnlyList<Document> Documents { get; }

    /// <summary>Requested layout capabilities. Omitted from the wire when null.</summary>
    [JsonPropertyName("layout_capabilities")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public IReadOnlyList<string>? LayoutCapabilities { get; }

    /// <summary>
    /// <c>display-list-v2-delta</c> installed-base acknowledgement (proposal r5 §3).
    /// Isolated feature: sent only when the delta capability is requested; omitted
    /// from the wire when null.
    /// </summary>
    [JsonPropertyName("display_list_base")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public DisplayListBaseInfo? DisplayListBase { get; }

    /// <summary>
    /// Absolute directory <c>\includegraphics</c> files are read from by the producer
    /// (display-list-v2-images proposal §2). Omitted from the wire when null; old
    /// producers ignore it.
    /// </summary>
    [JsonPropertyName("project_root")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? ProjectRoot { get; }

    public sealed record DisplayListBaseInfo(
        [property: JsonPropertyName("request_id")] string RequestId,
        [property: JsonPropertyName("project_id")] string ProjectId,
        [property: JsonPropertyName("revision")] int Revision,
        [property: JsonPropertyName("page_count")] int PageCount,
        [property: JsonPropertyName("list_digest")] string ListDigest);
}

/// <summary>Compile status: <c>ok</c>, <c>recovered</c>, or <c>failed</c>.</summary>
[JsonConverter(typeof(JsonStringEnumConverter<Status>))]
public enum Status
{
    ok,
    recovered,
    failed,
}

/// <summary>Diagnostic severity: <c>error</c> or <c>warning</c>.</summary>
[JsonConverter(typeof(JsonStringEnumConverter<Severity>))]
public enum Severity
{
    error,
    warning,
}

/// <summary>
/// A <c>compile_result</c> diagnostic. <c>Code</c>/<c>Suggestion</c> are the issue #76
/// additive fields documented in docs/contracts/runtime-v1.md (absent, never null,
/// when unset) — NOTE: the Swift source this file otherwise mirrors does not yet
/// declare these two fields; per this port's instructions the contract doc is the
/// more authoritative source for wire shape, so they are included here for a human
/// to reconcile with the Swift package.
/// </summary>
public sealed record Diagnostic(
    [property: JsonPropertyName("severity")] Severity Severity,
    [property: JsonPropertyName("message")] string Message,
    [property: JsonPropertyName("source")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] SourceRange? Source,
    [property: JsonPropertyName("recovery")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? Recovery,
    [property: JsonPropertyName("code")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? Code = null,
    [property: JsonPropertyName("suggestion")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? Suggestion = null);

/// <summary>One rendered page of a compile result.</summary>
public sealed record Page(
    [property: JsonPropertyName("number")] int Number,
    [property: JsonPropertyName("width_pt")] double WidthPt,
    [property: JsonPropertyName("height_pt")] double HeightPt,
    [property: JsonPropertyName("items")] IReadOnlyList<PageItem> Items);

/// <summary>
/// A page item: <c>kind: text</c> is base v1; <c>kind: rule</c> is decoded only as a
/// typed rule (negotiated <c>rules-v1</c>) and fails decoding when malformed. Unknown
/// kinds decode as <see cref="Unknown"/> (with their <c>source</c> when present and the
/// raw JSON) so a newer contract revision does not crash the shell.
/// </summary>
[JsonConverter(typeof(PageItemJsonConverter))]
public abstract record PageItem
{
    private PageItem()
    {
    }

    public sealed record OfText(TextItem Item) : PageItem;

    public sealed record OfRule(RuleItem Item) : PageItem;

    public sealed record Unknown(string Kind, SourceRange? Source, JsonElement RawJson) : PageItem;

    /// <summary>A base-v1 text item: shaped position, size, source, and an optional font hint.</summary>
    public sealed record TextItem
    {
        [JsonConstructor]
        public TextItem(string text, double xPt, double baselineYPt, double fontSizePt, SourceRange? source, FontHint? font = null)
        {
            Text = text;
            XPt = xPt;
            BaselineYPt = baselineYPt;
            FontSizePt = fontSizePt;
            Source = source;
            Font = font;
        }

        [JsonPropertyName("text")]
        public string Text { get; }

        [JsonPropertyName("x_pt")]
        public double XPt { get; }

        [JsonPropertyName("baseline_y_pt")]
        public double BaselineYPt { get; }

        [JsonPropertyName("font_size_pt")]
        public double FontSizePt { get; }

        [JsonPropertyName("source")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public SourceRange? Source { get; }

        [JsonPropertyName("font")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public FontHint? Font { get; }
    }

    /// <summary>
    /// <c>font-hints-v1</c>: family/weight/style intent. Not font bytes, GIDs, or
    /// advances — a consumer that cannot resolve <see cref="Family"/> must report
    /// substitution rather than claim the requested face was preserved.
    /// </summary>
    public sealed record FontHint
    {
        public const int MaxFamilyBytes = 128;

        public FontHint(string family, FontWeight weight = FontWeight.normal, FontStyle style = FontStyle.normal)
        {
            if (family.Length == 0 || ByteOffsets.Utf8ByteCount(family) > MaxFamilyBytes)
            {
                throw new ArgumentException($"font family must be 1...{MaxFamilyBytes} UTF-8 bytes", nameof(family));
            }
            foreach (var rune in family.EnumerateRunes())
            {
                if (System.Globalization.CharUnicodeInfo.GetUnicodeCategory(rune.Value) == System.Globalization.UnicodeCategory.Control)
                {
                    throw new ArgumentException("font family must not contain control characters", nameof(family));
                }
            }
            Family = family;
            Weight = weight;
            Style = style;
        }

        [JsonPropertyName("family")]
        public string Family { get; }

        [JsonPropertyName("weight")]
        public FontWeight Weight { get; }

        [JsonPropertyName("style")]
        public FontStyle Style { get; }

        [JsonConverter(typeof(JsonStringEnumConverter<FontWeight>))]
        public enum FontWeight
        {
            normal,
            bold,
        }

        [JsonConverter(typeof(JsonStringEnumConverter<FontStyle>))]
        public enum FontStyle
        {
            normal,
            italic,
        }
    }

    /// <summary>
    /// <c>rules-v1</c>: a filled rectangle whose top-left corner is <c>(x_pt, y_pt)</c>
    /// in page coordinates (y downward from the page top). <c>y_pt</c> is not a
    /// baseline. Dimensions are positive and finite; magnitudes at most
    /// <see cref="MaxMagnitude"/>.
    /// </summary>
    public sealed record RuleItem
    {
        public const double MaxMagnitude = 1_000_000.0;

        [JsonConstructor]
        public RuleItem(double xPt, double yPt, double widthPt, double heightPt, SourceRange? source)
        {
            foreach (var (value, name) in new[] { (xPt, nameof(xPt)), (yPt, nameof(yPt)), (widthPt, nameof(widthPt)), (heightPt, nameof(heightPt)) })
            {
                if (!double.IsFinite(value) || Math.Abs(value) > MaxMagnitude)
                {
                    throw new ArgumentException($"rule {name} must be finite with magnitude <= {MaxMagnitude}");
                }
            }
            if (!(widthPt > 0) || !(heightPt > 0))
            {
                throw new ArgumentException("rule width_pt and height_pt must be positive");
            }
            XPt = xPt;
            YPt = yPt;
            WidthPt = widthPt;
            HeightPt = heightPt;
            Source = source;
        }

        [JsonPropertyName("x_pt")]
        public double XPt { get; }

        [JsonPropertyName("y_pt")]
        public double YPt { get; }

        [JsonPropertyName("width_pt")]
        public double WidthPt { get; }

        [JsonPropertyName("height_pt")]
        public double HeightPt { get; }

        [JsonPropertyName("source")]
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public SourceRange? Source { get; }
    }
}

/// <summary>A <c>compile_result</c> message payload.</summary>
public sealed record CompileResult
{
    [JsonConstructor]
    public CompileResult(
        string projectId,
        int revision,
        Status status,
        IReadOnlyList<Page> pages,
        IReadOnlyList<Diagnostic> diagnostics,
        string? pdfPath,
        IReadOnlyList<string>? layoutCapabilities = null)
    {
        if (layoutCapabilities is not null)
        {
            // Fully qualified: the `LayoutCapabilities` property declared below on this
            // same record would otherwise shadow the static `LayoutCapabilities` type.
            FlashTeX.Protocol.RuntimeV1.LayoutCapabilities.Validate(layoutCapabilities);
        }
        ProjectId = projectId;
        Revision = revision;
        Status = status;
        Pages = pages;
        Diagnostics = diagnostics;
        PdfPath = pdfPath;
        LayoutCapabilities = layoutCapabilities;
    }

    [JsonPropertyName("project_id")]
    public string ProjectId { get; }

    [JsonPropertyName("revision")]
    public int Revision { get; }

    [JsonPropertyName("status")]
    public Status Status { get; }

    [JsonPropertyName("pages")]
    public IReadOnlyList<Page> Pages { get; }

    [JsonPropertyName("diagnostics")]
    public IReadOnlyList<Diagnostic> Diagnostics { get; }

    /// <summary>Null until a real artifact exists; always written, never omitted.</summary>
    [JsonPropertyName("pdf_path")]
    public string? PdfPath { get; }

    /// <summary>
    /// Capabilities the producer accepted for this result (a subset of the request's
    /// <c>layout_capabilities</c>). Null/omitted means none.
    /// </summary>
    [JsonPropertyName("layout_capabilities")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public IReadOnlyList<string>? LayoutCapabilities { get; }
}

/// <summary>
/// Discriminates <see cref="PageItem"/> on its <c>kind</c> field. An unrecognized
/// kind decodes to <see cref="PageItem.Unknown"/> carrying the raw JSON rather than
/// throwing, so a newer contract revision does not crash this consumer.
/// </summary>
public sealed class PageItemJsonConverter : JsonConverter<PageItem>
{
    public override PageItem Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var root = document.RootElement;
        string kind = root.TryGetProperty("kind", out var kindElement)
            ? kindElement.GetString() ?? throw new JsonException("page item 'kind' must be a string")
            : throw new JsonException("page item missing 'kind'");
        return kind switch
        {
            "text" => new PageItem.OfText(root.Deserialize<PageItem.TextItem>(options)!),
            "rule" => new PageItem.OfRule(root.Deserialize<PageItem.RuleItem>(options)!),
            _ => new PageItem.Unknown(kind, ReadOptionalSource(root, options), root.Clone()),
        };
    }

    private static SourceRange? ReadOptionalSource(JsonElement root, JsonSerializerOptions options) =>
        root.TryGetProperty("source", out var source) && source.ValueKind != JsonValueKind.Null
            ? source.Deserialize<SourceRange>(options)
            : null;

    public override void Write(Utf8JsonWriter writer, PageItem value, JsonSerializerOptions options)
    {
        switch (value)
        {
            case PageItem.OfText text:
                WriteWithKind(writer, "text", text.Item, options);
                break;
            case PageItem.OfRule rule:
                WriteWithKind(writer, "rule", rule.Item, options);
                break;
            case PageItem.Unknown unknown:
                unknown.RawJson.WriteTo(writer);
                break;
            default:
                throw new JsonException($"unhandled PageItem case {value.GetType()}");
        }
    }

    private static void WriteWithKind<T>(Utf8JsonWriter writer, string kind, T item, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        writer.WriteString("kind", kind);
        using var document = JsonSerializer.SerializeToDocument(item, options);
        foreach (var property in document.RootElement.EnumerateObject())
        {
            property.WriteTo(writer);
        }
        writer.WriteEndObject();
    }
}
