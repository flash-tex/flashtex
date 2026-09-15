// name: RenderingV2.cs
// purpose: Codable model plus a fail-closed validator for the EXPERIMENTAL
//   rendering-v2 `display_list` envelope (docs/contracts/runtime-v1-display-list-v2.md;
//   not a negotiated production wire — runtime-v1 remains authoritative). Ported
//   from apps/mac/Sources/FlashTeXProtocol/RenderingV2.swift. This is the highest-value
//   file in the port: the live PDF preview painter (FlashTeX.Preview, a future
//   workstream) will consume these DTOs directly.
//
//   Coordinates are `bp_2pow20` ticks: signed integers, 1,048,576 per PDF point,
//   origin at the page's top-left, y down, represented by the dedicated `Ticks`
//   type below (never raw double math on wire tick values, per this port's design
//   requirements). Glyph origins are absolute baseline origins; advances are never
//   re-added. Glyph IDs index the ORIGINAL font's glyph order. Every cluster is an
//   end-exclusive UTF-8 byte range of the run's logical `text` with its own hit
//   rectangles, carets and source provenance.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Linq;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace FlashTeX.Protocol.RenderingV2;

/// <summary>
/// A `bp_2pow20` fixed-point coordinate: 1,048,576 (2^20) ticks per PDF point.
/// Dedicated wrapper type (rather than a raw `long`) so tick values are never
/// accidentally mixed into floating-point math before an explicit conversion.
/// </summary>
[JsonConverter(typeof(TicksJsonConverter))]
public readonly record struct Ticks(long Value)
{
    public const long TicksPerPoint = 1L << 20;

    /// <summary>Exact for |ticks| &lt; 2^53.</summary>
    public double ToPoints() => (double)Value / TicksPerPoint;

    /// <summary>Points converted to device-independent pixels (96 DIPs per 72 points, WPF/WinUI convention).</summary>
    public double ToDips() => ToPoints() * (96.0 / 72.0);

    public static implicit operator long(Ticks ticks) => ticks.Value;

    public static implicit operator Ticks(long value) => new(value);

    public override string ToString() => Value.ToString();
}

internal sealed class TicksJsonConverter : JsonConverter<Ticks>
{
    public override Ticks Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => new(reader.GetInt64());

    public override void Write(Utf8JsonWriter writer, Ticks value, JsonSerializerOptions options) => writer.WriteNumberValue(value.Value);
}

/// <summary>An inclusive numeric bound, matching the Swift source's <c>ClosedRange</c> bounds.</summary>
public readonly record struct BoundsRange(long Min, long Max)
{
    public bool Contains(long value) => value >= Min && value <= Max;
}

public sealed record Paint(
    [property: JsonPropertyName("r")] double R,
    [property: JsonPropertyName("g")] double G,
    [property: JsonPropertyName("b")] double B,
    [property: JsonPropertyName("a")] double A)
{
    public static readonly Paint Black = new(0, 0, 0, 1);
}

/// <summary>Hit rectangle: top-left anchored, y down, in ticks.</summary>
public sealed record Rect(
    [property: JsonPropertyName("x")] Ticks X,
    [property: JsonPropertyName("top")] Ticks Top,
    [property: JsonPropertyName("width")] Ticks Width,
    [property: JsonPropertyName("height")] Ticks Height)
{
    /// <summary>Half-open containment in ticks (<c>x &lt;= px &lt; x+width</c>, same for y).</summary>
    public bool Contains(long px, long py) =>
        px >= X.Value && px < X.Value + Width.Value && py >= Top.Value && py < Top.Value + Height.Value;
}

public sealed record Caret(
    [property: JsonPropertyName("text_byte")] int TextByte,
    [property: JsonPropertyName("x")] Ticks X,
    [property: JsonPropertyName("top")] Ticks Top,
    [property: JsonPropertyName("height")] Ticks Height);

public sealed record Cluster(
    [property: JsonPropertyName("text_start_byte")] int TextStartByte,
    [property: JsonPropertyName("text_end_byte")] int TextEndByte,
    [property: JsonPropertyName("hit_rects")] IReadOnlyList<Rect> HitRects,
    [property: JsonPropertyName("carets")] IReadOnlyList<Caret> Carets,
    [property: JsonPropertyName("sources")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] IReadOnlyList<RuntimeV1.SourceRange>? Sources,
    [property: JsonPropertyName("synthetic_reason")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? SyntheticReason = null);

public sealed record Glyph(
    [property: JsonPropertyName("gid")] int Gid,
    [property: JsonPropertyName("origin_x")] Ticks OriginX,
    [property: JsonPropertyName("baseline_y")] Ticks BaselineY,
    [property: JsonPropertyName("advance_x")] Ticks AdvanceX,
    [property: JsonPropertyName("advance_y")] Ticks AdvanceY,
    [property: JsonPropertyName("cluster")] int Cluster);

public sealed record GlyphRun(
    [property: JsonPropertyName("font_id")] string FontId,
    [property: JsonPropertyName("font_size")] Ticks FontSize,
    [property: JsonPropertyName("text")] string Text,
    [property: JsonPropertyName("glyphs")] IReadOnlyList<Glyph> Glyphs,
    [property: JsonPropertyName("clusters")] IReadOnlyList<Cluster> Clusters,
    [property: JsonPropertyName("paint")] Paint Paint)
{
    /// <summary>The logical text of cluster <paramref name="i"/> (its UTF-8 byte range of <see cref="Text"/>).</summary>
    public string ClusterText(int i)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(Text);
        if (i >= Clusters.Count || Clusters[i].TextStartByte > Clusters[i].TextEndByte || Clusters[i].TextEndByte > bytes.Length)
        {
            return "";
        }
        return Encoding.UTF8.GetString(bytes, Clusters[i].TextStartByte, Clusters[i].TextEndByte - Clusters[i].TextStartByte);
    }
}

public sealed record Rule(
    [property: JsonPropertyName("x")] Ticks X,
    [property: JsonPropertyName("top")] Ticks Top,
    [property: JsonPropertyName("width")] Ticks Width,
    [property: JsonPropertyName("height")] Ticks Height,
    [property: JsonPropertyName("paint")] Paint Paint,
    [property: JsonPropertyName("sources")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] IReadOnlyList<RuntimeV1.SourceRange>? Sources,
    [property: JsonPropertyName("synthetic_reason")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? SyntheticReason = null);

/// <summary>
/// One <c>\includegraphics</c> file as the producer sized it. Bytes are NOT on the
/// wire: the consumer reads <see cref="Path"/> through the rooted project reader and
/// must refuse bytes whose SHA-256/length differ.
/// </summary>
public sealed record ImageResource(
    [property: JsonPropertyName("image_id")] string ImageId,
    [property: JsonPropertyName("sha256")] string Sha256,
    [property: JsonPropertyName("byte_length")] long ByteLength,
    [property: JsonPropertyName("format")] string Format,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("pixel_width")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? PixelWidth = null,
    [property: JsonPropertyName("pixel_height")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? PixelHeight = null,
    [property: JsonPropertyName("pdf_page")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? PdfPage = null,
    [property: JsonPropertyName("pdf_box")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] IReadOnlyList<double>? PdfBox = null,
    [property: JsonPropertyName("pdf_rotate")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? PdfRotate = null);

/// <summary>
/// <c>kind: "image"</c>: the bounding box on the page (ticks, exact geometry, clip
/// to it) and the affine <see cref="Transform"/> <c>[a, b, c, d, e, f]</c> mapping the
/// image's unit square (u right, v up, origin lower-left) to page points with y
/// down: <c>page_x = e + a*u + c*v</c>, <c>page_y = f + b*u + d*v</c>.
/// </summary>
public sealed record Image(
    [property: JsonPropertyName("x")] Ticks X,
    [property: JsonPropertyName("top")] Ticks Top,
    [property: JsonPropertyName("width")] Ticks Width,
    [property: JsonPropertyName("height")] Ticks Height,
    [property: JsonPropertyName("transform")] IReadOnlyList<double> Transform,
    [property: JsonPropertyName("image")] ImageResource ImageResource,
    [property: JsonPropertyName("sources")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] IReadOnlyList<RuntimeV1.SourceRange>? Sources,
    [property: JsonPropertyName("synthetic_reason")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? SyntheticReason = null)
{
    /// <summary>The unrotated transform for a box at <c>(x, top, w, h)</c> points: <c>[w, 0, 0, -h, x, top + h]</c>.</summary>
    public static IReadOnlyList<double> Upright(double x, double top, double width, double height) =>
        new[] { width, 0, 0, -height, x, top + height };
}

/// <summary>One vector path command in page ticks (path-v0). Array shape on the wire: <c>["m",x,y]</c> etc.</summary>
[JsonConverter(typeof(PathCommandJsonConverter))]
public abstract record PathCommand
{
    private PathCommand()
    {
    }

    public sealed record Move(Ticks X, Ticks Y) : PathCommand;

    public sealed record Line(Ticks X, Ticks Y) : PathCommand;

    public sealed record Cubic(Ticks X1, Ticks Y1, Ticks X2, Ticks Y2, Ticks X, Ticks Y) : PathCommand;

    public sealed record Close : PathCommand;

    /// <summary>Every coordinate the command carries (none for <see cref="Close"/>).</summary>
    public IReadOnlyList<Ticks> Coordinates => this switch
    {
        Move m => new[] { m.X, m.Y },
        Line l => new[] { l.X, l.Y },
        Cubic c => new[] { c.X1, c.Y1, c.X2, c.Y2, c.X, c.Y },
        _ => Array.Empty<Ticks>(),
    };
}

[JsonConverter(typeof(JsonStringEnumConverter<FillRule>))]
public enum FillRule
{
    nonzero,
    evenodd,
}

[JsonConverter(typeof(JsonStringEnumConverter<LineCap>))]
public enum LineCap
{
    butt,
    round,
    square,
}

[JsonConverter(typeof(JsonStringEnumConverter<LineJoin>))]
public enum LineJoin
{
    miter,
    round,
    bevel,
}

/// <summary>Alternating on/off lengths in ticks (never empty on the wire).</summary>
public sealed record Dash(
    [property: JsonPropertyName("array")] IReadOnlyList<Ticks> Array,
    [property: JsonPropertyName("phase")] Ticks Phase);

public sealed record Stroke(
    [property: JsonPropertyName("width")] Ticks Width,
    [property: JsonPropertyName("cap")] LineCap Cap = LineCap.butt,
    [property: JsonPropertyName("join")] LineJoin Join = LineJoin.miter,
    [property: JsonPropertyName("miter_limit")] double MiterLimit = 10,
    [property: JsonPropertyName("dash")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] Dash? Dash = null);

/// <summary>One clip in page space; an item paints only inside every clip.</summary>
public sealed record ClipPath(
    [property: JsonPropertyName("path")] IReadOnlyList<PathCommand> Path,
    [property: JsonPropertyName("fill_rule")] FillRule FillRule = FillRule.nonzero);

/// <summary>A filled (<c>kind: "path_fill"</c>) or stroked (<c>kind: "path_stroke"</c>) vector path (TikZ).</summary>
[JsonConverter(typeof(PathJsonConverter))]
public sealed record Path(
    Path.Op PathOp,
    IReadOnlyList<PathCommand> Commands,
    IReadOnlyList<ClipPath> Clips,
    Paint Paint,
    IReadOnlyList<RuntimeV1.SourceRange>? Sources,
    string? SyntheticReason = null)
{
    public bool IsFill => PathOp is Op.Fill;

    public Stroke? Stroke => PathOp is Op.Stroke stroke ? stroke.Value : null;

    public FillRule FillRule => PathOp is Op.Fill fill ? fill.Rule : FillRule.nonzero;

    public string Kind => IsFill ? "path_fill" : "path_stroke";

    public abstract record Op
    {
        private Op()
        {
        }

        public sealed record Fill(FillRule Rule) : Op;

        public sealed record Stroke(RenderingV2.Stroke Value) : Op;
    }
}

/// <summary>
/// Paint-ordered page item. Decoding an unknown <c>kind</c> throws (see
/// <see cref="ItemJsonConverter"/>): nothing is skipped silently.
/// </summary>
[JsonConverter(typeof(ItemJsonConverter))]
public abstract record Item
{
    private Item()
    {
    }

    public sealed record OfGlyphRun(GlyphRun Value) : Item;

    public sealed record OfRule(Rule Value) : Item;

    public sealed record OfImage(Image Value) : Item;

    /// <summary><c>path_fill</c> and <c>path_stroke</c> alike (<c>Value.Op</c> tells them apart).</summary>
    public sealed record OfPath(Path Value) : Item;
}

public sealed record Page(
    [property: JsonPropertyName("number")] int Number,
    [property: JsonPropertyName("width")] Ticks Width,
    [property: JsonPropertyName("height")] Ticks Height,
    [property: JsonPropertyName("items")] IReadOnlyList<Item> Items)
{
    [JsonIgnore]
    public double WidthPt => Width.ToPoints();

    [JsonIgnore]
    public double HeightPt => Height.ToPoints();
}

public sealed record FontResource(
    [property: JsonPropertyName("font_id")] string FontId,
    [property: JsonPropertyName("sha256")] string Sha256,
    [property: JsonPropertyName("byte_length")] long ByteLength,
    [property: JsonPropertyName("format")] string Format,
    [property: JsonPropertyName("face_index")] int FaceIndex,
    [property: JsonPropertyName("units_per_em")] int UnitsPerEm,
    [property: JsonPropertyName("glyph_count")] int GlyphCount,
    [property: JsonPropertyName("postscript_name")] string PostscriptName)
{
    [JsonIgnore]
    public bool IsPaintable => RenderingV2Protocol.PaintableFontFormats.Contains(Format);
}

public sealed record DocumentResource(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] int Revision,
    [property: JsonPropertyName("sha256")] string Sha256,
    [property: JsonPropertyName("byte_length")] long ByteLength);

public sealed record Diagnostic(
    [property: JsonPropertyName("code")] string Code,
    [property: JsonPropertyName("message")] string Message,
    [property: JsonPropertyName("severity")] Diagnostic.Severity SeverityValue,
    [property: JsonPropertyName("sources")] IReadOnlyList<RuntimeV1.SourceRange> Sources)
{
    [JsonConverter(typeof(JsonStringEnumConverter<Severity>))]
    public enum Severity
    {
        warning,
        error,
    }

    /// <summary>The runtime-v1 shape the shell's diagnostics list already renders.</summary>
    public RuntimeV1.Diagnostic AsRuntimeV1() => new(
        SeverityValue == Severity.error ? RuntimeV1.Severity.error : RuntimeV1.Severity.warning,
        $"[{Code}] {Message}",
        Sources.Count > 0 ? Sources[0] : null,
        null);
}

public sealed record DisplayList(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("revision")] int Revision,
    [property: JsonPropertyName("required_features")] IReadOnlyList<string> RequiredFeatures,
    [property: JsonPropertyName("documents")] IReadOnlyList<DocumentResource> Documents,
    [property: JsonPropertyName("fonts")] IReadOnlyList<FontResource> Fonts,
    [property: JsonPropertyName("pages")] IReadOnlyList<Page> Pages,
    [property: JsonPropertyName("diagnostics")] IReadOnlyList<Diagnostic> Diagnostics,
    [property: JsonPropertyName("render_format")] string RenderFormat = RenderingV2Protocol.RenderFormat,
    [property: JsonPropertyName("coordinate_unit")] string CoordinateUnit = RenderingV2Protocol.CoordinateUnit,
    [property: JsonPropertyName("color_space")] string ColorSpace = RenderingV2Protocol.ColorSpace,
    [property: JsonPropertyName("text_extraction")] string TextExtraction = RenderingV2Protocol.TextExtraction)
{
    public FontResource? Font(string id) => Fonts.FirstOrDefault(f => f.FontId == id);
}

public sealed record Envelope(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("payload")] DisplayList Payload,
    [property: JsonPropertyName("protocol_version")] int ProtocolVersion = RenderingV2Protocol.Version,
    [property: JsonPropertyName("type")] string Type = RenderingV2Protocol.MessageType);

/// <summary>
/// A diagnostic-bearing refusal. The consumer never renders partially: any error
/// here means no frame is published for this display list.
/// </summary>
public sealed class RenderingV2ValidationException : Exception
{
    public RenderingV2ValidationException(string code, string message, RuntimeV1.SourceRange? source = null)
        : base(message)
    {
        Code = code;
        SourceRange = source;
    }

    public string Code { get; }

    /// <summary>Named to avoid colliding with the inherited <see cref="Exception.Source"/> property.</summary>
    public RuntimeV1.SourceRange? SourceRange { get; }

    public override string ToString() => $"{Code}: {Message}";

    public RuntimeV1.Diagnostic AsRuntimeV1() => new(RuntimeV1.Severity.error, $"[{Code}] {Message}", SourceRange, null);
}

/// <summary>Small array (de)serialization helpers shared by every hand-written converter in this file.</summary>
internal static class JsonArrayHelpers
{
    public static List<T> ReadArray<T>(JsonElement array, JsonSerializerOptions options)
    {
        var list = new List<T>(array.GetArrayLength());
        foreach (var element in array.EnumerateArray())
        {
            list.Add(element.Deserialize<T>(options)!);
        }
        return list;
    }

    public static void WriteArray<T>(Utf8JsonWriter writer, string propertyName, IReadOnlyList<T> items, JsonSerializerOptions options)
    {
        writer.WritePropertyName(propertyName);
        writer.WriteStartArray();
        foreach (var item in items)
        {
            JsonSerializer.Serialize(writer, item, options);
        }
        writer.WriteEndArray();
    }
}

/// <summary>
/// Reads/writes the wire's array-shaped path command: <c>["m",x,y]</c>, <c>["l",x,y]</c>,
/// <c>["c",x1,y1,x2,y2,x,y]</c> or <c>["z"]</c>.
/// </summary>
public sealed class PathCommandJsonConverter : JsonConverter<PathCommand>
{
    public override PathCommand Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType != JsonTokenType.StartArray)
        {
            throw new JsonException("path command must be a JSON array");
        }
        reader.Read();
        if (reader.TokenType != JsonTokenType.String)
        {
            throw new JsonException("path command must start with an operator string");
        }
        string op = reader.GetString()!;
        var numbers = new List<long>();
        reader.Read();
        while (reader.TokenType != JsonTokenType.EndArray)
        {
            numbers.Add(reader.GetInt64());
            reader.Read();
        }
        return Build(op, numbers);
    }

    private static PathCommand Build(string op, List<long> n) => (op, n.Count) switch
    {
        ("m", 2) => new PathCommand.Move(n[0], n[1]),
        ("l", 2) => new PathCommand.Line(n[0], n[1]),
        ("c", 6) => new PathCommand.Cubic(n[0], n[1], n[2], n[3], n[4], n[5]),
        ("z", 0) => new PathCommand.Close(),
        _ => throw new RenderingV2ValidationException("invalid_display_list", $"path command '{op}' is not one of m, l, c, z, or carries the wrong operand count"),
    };

    public override void Write(Utf8JsonWriter writer, PathCommand value, JsonSerializerOptions options)
    {
        writer.WriteStartArray();
        switch (value)
        {
            case PathCommand.Move m:
                writer.WriteStringValue("m");
                writer.WriteNumberValue(m.X.Value);
                writer.WriteNumberValue(m.Y.Value);
                break;
            case PathCommand.Line l:
                writer.WriteStringValue("l");
                writer.WriteNumberValue(l.X.Value);
                writer.WriteNumberValue(l.Y.Value);
                break;
            case PathCommand.Cubic c:
                writer.WriteStringValue("c");
                foreach (var v in new[] { c.X1, c.Y1, c.X2, c.Y2, c.X, c.Y })
                {
                    writer.WriteNumberValue(v.Value);
                }
                break;
            case PathCommand.Close:
                writer.WriteStringValue("z");
                break;
            default:
                throw new JsonException($"unhandled PathCommand case {value.GetType()}");
        }
        writer.WriteEndArray();
    }
}

/// <summary>Discriminates <see cref="Path"/> on its <c>kind</c> (<c>path_fill</c>/<c>path_stroke</c>); unknown kinds throw.</summary>
public sealed class PathJsonConverter : JsonConverter<Path>
{
    public override Path Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var root = document.RootElement;
        string kind = ReadKind(root, "path");
        var op = ReadOp(kind, root, options);
        var commands = JsonArrayHelpers.ReadArray<PathCommand>(root.GetProperty("path"), options);
        var clips = root.TryGetProperty("clips", out var clipsEl)
            ? JsonArrayHelpers.ReadArray<ClipPath>(clipsEl, options)
            : new List<ClipPath>();
        var paint = root.GetProperty("paint").Deserialize<Paint>(options)!;
        var sources = ReadOptionalSources(root, options);
        string? synthetic = root.TryGetProperty("synthetic_reason", out var synEl) && synEl.ValueKind != JsonValueKind.Null
            ? synEl.GetString()
            : null;
        return new Path(op, commands, clips, paint, sources, synthetic);
    }

    internal static string ReadKind(JsonElement root, string what) =>
        root.TryGetProperty("kind", out var kindEl)
            ? kindEl.GetString() ?? throw new JsonException($"{what} 'kind' must be a string")
            : throw new JsonException($"{what} missing 'kind'");

    private static Path.Op ReadOp(string kind, JsonElement root, JsonSerializerOptions options) => kind switch
    {
        "path_fill" => new Path.Op.Fill(
            root.TryGetProperty("fill_rule", out var fr) && fr.ValueKind != JsonValueKind.Null ? fr.Deserialize<FillRule>(options) : FillRule.nonzero),
        "path_stroke" => new Path.Op.Stroke(root.GetProperty("stroke").Deserialize<Stroke>(options)!),
        _ => throw new RenderingV2ValidationException("unknown_item_kind", $"display list item kind '{kind}' is not a path"),
    };

    private static List<RuntimeV1.SourceRange>? ReadOptionalSources(JsonElement root, JsonSerializerOptions options) =>
        root.TryGetProperty("sources", out var sourcesEl) && sourcesEl.ValueKind != JsonValueKind.Null
            ? JsonArrayHelpers.ReadArray<RuntimeV1.SourceRange>(sourcesEl, options)
            : null;

    public override void Write(Utf8JsonWriter writer, Path value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        writer.WriteString("kind", value.Kind);
        WriteOp(writer, value.PathOp, options);
        JsonArrayHelpers.WriteArray(writer, "path", value.Commands, options);
        if (value.Clips.Count > 0)
        {
            JsonArrayHelpers.WriteArray(writer, "clips", value.Clips, options);
        }
        writer.WritePropertyName("paint");
        JsonSerializer.Serialize(writer, value.Paint, options);
        if (value.Sources is not null)
        {
            JsonArrayHelpers.WriteArray(writer, "sources", value.Sources, options);
        }
        if (value.SyntheticReason is not null)
        {
            writer.WriteString("synthetic_reason", value.SyntheticReason);
        }
        writer.WriteEndObject();
    }

    private static void WriteOp(Utf8JsonWriter writer, Path.Op op, JsonSerializerOptions options)
    {
        switch (op)
        {
            case Path.Op.Fill fill:
                writer.WriteString("fill_rule", fill.Rule.ToString());
                break;
            case Path.Op.Stroke stroke:
                writer.WritePropertyName("stroke");
                JsonSerializer.Serialize(writer, stroke.Value, options);
                break;
            default:
                throw new JsonException($"unhandled Path.Op case {op.GetType()}");
        }
    }
}

/// <summary>
/// Discriminates <see cref="Item"/> on its <c>kind</c>. Decoding an unrecognized kind
/// throws: the display-list-v2 contract requires a fail-closed consumer that never
/// silently skips an unsupported primitive.
/// </summary>
public sealed class ItemJsonConverter : JsonConverter<Item>
{
    public override Item Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        var root = document.RootElement;
        string kind = PathJsonConverter.ReadKind(root, "display list item");
        return kind switch
        {
            "glyph_run" => new Item.OfGlyphRun(root.Deserialize<GlyphRun>(options)!),
            "rule" => new Item.OfRule(root.Deserialize<Rule>(options)!),
            "image" => new Item.OfImage(root.Deserialize<Image>(options)!),
            "path_fill" or "path_stroke" => new Item.OfPath(root.Deserialize<Path>(options)!),
            _ => throw new RenderingV2ValidationException("unknown_item_kind", $"display list item kind '{kind}' is not supported by this consumer"),
        };
    }

    public override void Write(Utf8JsonWriter writer, Item value, JsonSerializerOptions options)
    {
        switch (value)
        {
            case Item.OfGlyphRun glyphRun:
                WriteWithKind(writer, "glyph_run", glyphRun.Value, options);
                break;
            case Item.OfRule rule:
                WriteWithKind(writer, "rule", rule.Value, options);
                break;
            case Item.OfImage image:
                WriteWithKind(writer, "image", image.Value, options);
                break;
            case Item.OfPath path:
                JsonSerializer.Serialize(writer, path.Value, options); // Path writes its own kind
                break;
            default:
                throw new JsonException($"unhandled Item case {value.GetType()}");
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

/// <summary>
/// Constants, bounds and the fail-closed decode/validate pipeline for the
/// rendering-v2 <c>display_list</c> envelope. Decoding and validating are separate
/// steps (matching the Swift source): <see cref="Decode"/> parses the envelope and
/// checks its header; <see cref="Validate"/> then checks every structural rule
/// crates/rendering-core enforces (bounded collections, exact-integer ticks,
/// declared vs. used features, cluster/glyph cross-references, source provenance).
/// </summary>
public static class RenderingV2Protocol
{
    public const int Version = 2;
    public const string MessageType = "display_list";
    public const string RenderFormat = "display-list-v2";
    public const string CoordinateUnit = "bp_2pow20";
    public const string ColorSpace = "srgb";
    public const string TextExtraction = "cluster-actualtext";

    /// <summary>
    /// Largest integer JSON carries exactly (2^53 - 1); every tick, revision and byte
    /// count must stay within +/- this, and tick sums are checked.
    /// </summary>
    public const long MaxExactInteger = (1L << 53) - 1;

    /// <summary>Features this consumer understands.</summary>
    public static readonly IReadOnlyCollection<string> KnownFeatures = new HashSet<string>(StringComparer.Ordinal)
    {
        "glyph_run", "rule", "static-truetype", "rgba-srgb", "cluster-actualtext", "image", "path_fill", "path_stroke", "clip",
    };

    /// <summary>Vector path items (proposal path-v0; TikZ pictures).</summary>
    public static readonly IReadOnlyCollection<string> PathFeatures = new HashSet<string>(StringComparer.Ordinal)
    {
        "path_fill", "path_stroke", "clip",
    };

    /// <summary>Layout capability that lets the display_list line carry <c>image</c> items.</summary>
    public const string ImagesCapability = "display-list-v2-images";

    public static readonly IReadOnlyCollection<string> ImageFormats = new HashSet<string>(StringComparer.Ordinal) { "png", "jpeg", "pdf" };

    public const long MaxImageByteLength = 256L << 20;

    public static readonly IReadOnlyCollection<string> PaintableFontFormats = new HashSet<string>(StringComparer.Ordinal)
    {
        "static-truetype", "opentype-cff",
    };

    /// <summary>Formats the pipeline may declare that carry no program (never paintable).</summary>
    public static readonly IReadOnlyCollection<string> MetricsOnlyFontFormats = new HashSet<string>(StringComparer.Ordinal) { "core14-afm" };

    /// <summary>Bounded collection sizes: a list that exceeds one is refused, never truncated.</summary>
    public static class Bounds
    {
        public static readonly BoundsRange Documents = new(1, 4096);
        public static readonly BoundsRange Fonts = new(0, 256);
        public static readonly BoundsRange Pages = new(0, 10000);
        public static readonly BoundsRange PageItems = new(0, 100000);
        public static readonly BoundsRange Diagnostics = new(0, 10000);
        public static readonly BoundsRange RunTextBytes = new(1, 1_048_576);
        public static readonly BoundsRange Glyphs = new(1, 65536);
        public static readonly BoundsRange Clusters = new(1, 65536);
        public static readonly BoundsRange HitRects = new(1, 128);
        public static readonly BoundsRange Carets = new(0, 128);
        public static readonly BoundsRange SourceRanges = new(1, 128);
        public static readonly BoundsRange DiagnosticSources = new(0, 128);
        public static readonly BoundsRange DiagnosticMessageBytes = new(1, 4096);
        public static readonly BoundsRange SyntheticReasonBytes = new(1, 1024);
        public static readonly BoundsRange PostscriptNameBytes = new(1, 256);
        public static readonly BoundsRange FontByteLength = new(1, 67_108_864);
        public static readonly BoundsRange DocumentByteLength = new(0, 8_388_608);
        public static readonly BoundsRange PathCommands = new(1, 65536);
        public static readonly BoundsRange Clips = new(0, 16);
        public static readonly BoundsRange DashEntries = new(1, 32);
    }

    /// <summary>Ticks to PDF points (exact for |ticks| &lt; 2^53).</summary>
    public static double Points(long ticks) => (double)ticks / Ticks.TicksPerPoint;

    // MARK: decoding + validation

    /// <summary>
    /// Decodes and validates a <c>display_list</c> envelope. Fails closed: an unknown
    /// protocol version, message type, item kind, feature, font reference, or
    /// malformed geometry/cluster is an error, never a partial result.
    /// </summary>
    public static Envelope Decode(ReadOnlySpan<byte> data, JsonSerializerOptions options)
    {
        JsonDocument document;
        try
        {
            document = JsonDocument.Parse(data.ToArray());
        }
        catch (JsonException ex)
        {
            throw new RenderingV2ValidationException("malformed_json", $"not a JSON object: {ex.Message}");
        }
        using (document)
        {
            var root = document.RootElement;
            if (!root.TryGetProperty("protocol_version", out var versionElement))
            {
                throw new RenderingV2ValidationException("missing_protocol_version", "protocol_version is required");
            }
            if (!root.TryGetProperty("type", out var typeElement))
            {
                throw new RenderingV2ValidationException("missing_type", "type is required");
            }
            CheckHeader(versionElement.GetInt32(), typeElement.GetString() ?? "");
            Envelope envelope;
            try
            {
                envelope = root.Deserialize<Envelope>(options) ?? throw new RenderingV2ValidationException("malformed_payload", "envelope decoded to null");
            }
            catch (JsonException ex)
            {
                throw new RenderingV2ValidationException("malformed_payload", ex.Message);
            }
            Validate(envelope.Payload);
            return envelope;
        }
    }

    /// <summary>Version/type refusal shared by every decoding path.</summary>
    public static void CheckHeader(int version, string type)
    {
        if (version != Version)
        {
            throw new RenderingV2ValidationException(
                "unsupported_protocol_version",
                $"protocol version {version} is not supported; this consumer speaks rendering-v2 (protocol_version {Version})");
        }
        if (type != MessageType)
        {
            throw new RenderingV2ValidationException("unsupported_message_type", $"message type '{type}' is not a display_list");
        }
    }

    /// <summary>Semantic validation of a decoded payload (see <see cref="Decode"/>).</summary>
    public static void Validate(DisplayList list)
    {
        ValidateFormatStrings(list);
        ValidateTopLevelBounds(list);
        var documents = ValidateDocuments(list.Documents);
        var fontsById = ValidateFonts(list.Fonts);
        var usedFeatures = new HashSet<string>(StringComparer.Ordinal) { "rgba-srgb", "cluster-actualtext" };
        ValidatePages(list.Pages, documents, fontsById, usedFeatures);
        ValidateDiagnostics(list.Diagnostics, documents);
        CheckFeatureDeclarations(list.RequiredFeatures, usedFeatures);
    }

    private static RenderingV2ValidationException Fail(string code, string message, RuntimeV1.SourceRange? source = null) =>
        new(code, message, source);

    private static void ValidateFormatStrings(DisplayList list)
    {
        if (list.RenderFormat != RenderFormat)
        {
            throw Fail("unsupported_format", $"render_format '{list.RenderFormat}' is not {RenderFormat}");
        }
        if (list.CoordinateUnit != CoordinateUnit)
        {
            throw Fail("unsupported_coordinate_unit", $"coordinate_unit '{list.CoordinateUnit}' is not {CoordinateUnit}");
        }
        if (list.ColorSpace != ColorSpace)
        {
            throw Fail("unsupported_color_space", $"color_space '{list.ColorSpace}' is not {ColorSpace}");
        }
        if (list.TextExtraction != TextExtraction)
        {
            throw Fail("unsupported_text_extraction", $"text_extraction '{list.TextExtraction}' is not {TextExtraction}");
        }
        if (list.RequiredFeatures.Count == 0)
        {
            throw Fail("invalid_display_list", "required_features must list at least one feature");
        }
        foreach (var feature in list.RequiredFeatures.Where(f => !KnownFeatures.Contains(f)))
        {
            throw Fail("unsupported_feature", $"required feature '{feature}' is not supported by this consumer");
        }
    }

    private static void ValidateTopLevelBounds(DisplayList list)
    {
        // Revision is a 32-bit int (unlike Swift's 64-bit Int), so it can never exceed
        // MaxExactInteger (2^53 - 1) — only the nonnegativity bound is meaningful here.
        if (list.Revision < 0)
        {
            throw Fail("invalid_display_list", "revision must be a nonnegative exact integer");
        }
        if (!Bounds.Documents.Contains(list.Documents.Count))
        {
            throw Fail("invalid_display_list", $"documents must declare 1...{Bounds.Documents.Max} source documents (found {list.Documents.Count})");
        }
        if (!Bounds.Fonts.Contains(list.Fonts.Count))
        {
            throw Fail("invalid_display_list", $"fonts must declare at most {Bounds.Fonts.Max} resources (found {list.Fonts.Count})");
        }
        if (!Bounds.Pages.Contains(list.Pages.Count))
        {
            throw Fail("invalid_display_list", $"at most {Bounds.Pages.Max} pages (found {list.Pages.Count})");
        }
        if (!Bounds.Diagnostics.Contains(list.Diagnostics.Count))
        {
            throw Fail("invalid_display_list", $"at most {Bounds.Diagnostics.Max} diagnostics (found {list.Diagnostics.Count})");
        }
    }

    private static Dictionary<string, DocumentResource> ValidateDocuments(IReadOnlyList<DocumentResource> documents)
    {
        var byPath = new Dictionary<string, DocumentResource>(StringComparer.Ordinal);
        foreach (var d in documents)
        {
            if (!IsProjectPath(d.Path))
            {
                throw Fail("invalid_resource", $"document path '{d.Path}' must be project-relative: no empty, '.' or '..' components, no backslash, colon or NUL");
            }
            if (!byPath.TryAdd(d.Path, d))
            {
                throw Fail("invalid_resource", $"document '{d.Path}' is declared twice");
            }
            if (!IsHex64(d.Sha256))
            {
                throw Fail("invalid_resource", $"document '{d.Path}' sha256 is not 64 lowercase hex digits");
            }
            if (d.Revision < 0)
            {
                throw Fail("invalid_resource", $"document '{d.Path}' revision must be a nonnegative exact integer");
            }
            if (!Bounds.DocumentByteLength.Contains(d.ByteLength))
            {
                throw Fail("invalid_resource", $"document '{d.Path}' byte_length {d.ByteLength} is outside 0...{Bounds.DocumentByteLength.Max}");
            }
        }
        return byPath;
    }

    private static Dictionary<string, FontResource> ValidateFonts(IReadOnlyList<FontResource> fonts)
    {
        var byId = new Dictionary<string, FontResource>(StringComparer.Ordinal);
        foreach (var f in fonts)
        {
            ValidateFont(f, byId);
        }
        return byId;
    }

    private static void ValidateFont(FontResource f, Dictionary<string, FontResource> byId)
    {
        if (f.FontId.Length == 0 || !byId.TryAdd(f.FontId, f))
        {
            throw Fail("invalid_resource", $"font resource id '{f.FontId}' is empty or declared twice");
        }
        if (!IsHex64(f.Sha256))
        {
            throw Fail("invalid_resource", $"font resource {f.FontId} sha256 is not 64 lowercase hex digits");
        }
        if (!PaintableFontFormats.Contains(f.Format) && !MetricsOnlyFontFormats.Contains(f.Format))
        {
            throw Fail("unsupported_feature", $"font resource {f.FontId} format '{f.Format}' is not supported (paintable: {string.Join(", ", PaintableFontFormats.OrderBy(x => x, StringComparer.Ordinal))})");
        }
        if (f.FaceIndex != 0)
        {
            throw Fail("unsupported_feature", $"font resource {f.FontId} face_index {f.FaceIndex}: only face 0 is supported");
        }
        bool byteLengthOk = f.IsPaintable ? Bounds.FontByteLength.Contains(f.ByteLength) : f.ByteLength >= 0;
        if (!byteLengthOk)
        {
            throw Fail("invalid_resource", $"font resource {f.FontId} byte_length {f.ByteLength} is outside {Bounds.FontByteLength.Min}...{Bounds.FontByteLength.Max}");
        }
        if (f.UnitsPerEm < 16 || f.UnitsPerEm > 16384)
        {
            throw Fail("invalid_resource", $"font resource {f.FontId} units_per_em {f.UnitsPerEm} is out of range");
        }
        if (f.GlyphCount < 2 || f.GlyphCount > 65536)
        {
            throw Fail("invalid_resource", $"font resource {f.FontId} glyph_count {f.GlyphCount} is out of range");
        }
        if (!Bounds.PostscriptNameBytes.Contains(ByteOffsets.Utf8ByteCount(f.PostscriptName)))
        {
            throw Fail("invalid_resource", $"font resource {f.FontId} postscript_name must be 1...{Bounds.PostscriptNameBytes.Max} bytes");
        }
    }

    // MARK: pages and items

    private static void ValidatePages(
        IReadOnlyList<Page> pages,
        IReadOnlyDictionary<string, DocumentResource> documents,
        IReadOnlyDictionary<string, FontResource> fontsById,
        HashSet<string> usedFeatures)
    {
        int lastPage = 0;
        foreach (var page in pages)
        {
            if (page.Number != lastPage + 1)
            {
                throw Fail("invalid_display_list", $"page numbers must be contiguous from 1 (found {page.Number} after {lastPage})");
            }
            lastPage = page.Number;
            if (!IsPositiveTick(page.Width) || !IsPositiveTick(page.Height))
            {
                throw Fail("invalid_display_list", $"page {page.Number} must have positive exact width and height");
            }
            if (!Bounds.PageItems.Contains(page.Items.Count))
            {
                throw Fail("invalid_display_list", $"page {page.Number} has {page.Items.Count} items (limit {Bounds.PageItems.Max})");
            }
            for (int index = 0; index < page.Items.Count; index++)
            {
                ValidatePageItem(page, index, documents, fontsById, usedFeatures);
            }
        }
    }

    private static void ValidatePageItem(
        Page page,
        int index,
        IReadOnlyDictionary<string, DocumentResource> documents,
        IReadOnlyDictionary<string, FontResource> fontsById,
        HashSet<string> usedFeatures)
    {
        string at = $"page {page.Number} item {index}";
        switch (page.Items[index])
        {
            case Item.OfRule rule:
                ValidateRuleItem(page, at, rule.Value, documents, usedFeatures);
                break;
            case Item.OfImage image:
                ValidateImageItem(page, at, image.Value, documents, usedFeatures);
                break;
            case Item.OfPath path:
                usedFeatures.Add(path.Value.IsFill ? "path_fill" : "path_stroke");
                if (path.Value.Clips.Count > 0)
                {
                    usedFeatures.Add("clip");
                }
                ValidatePathValue(path.Value, at);
                ValidateProvenance(path.Value.Sources, path.Value.SyntheticReason, documents, at);
                break;
            case Item.OfGlyphRun glyphRun:
                usedFeatures.Add("glyph_run");
                ValidateGlyphRunItem(page, at, glyphRun.Value, fontsById, documents);
                break;
        }
    }

    private static void ValidateRuleItem(Page page, string at, Rule r, IReadOnlyDictionary<string, DocumentResource> documents, HashSet<string> usedFeatures)
    {
        usedFeatures.Add("rule");
        if (!IsTick(r.X) || !IsTick(r.Top) || !IsPositiveTick(r.Width) || !IsPositiveTick(r.Height)
            || !IsTick(r.X.Value + r.Width.Value) || !IsTick(r.Top.Value + r.Height.Value) || !IsTick(page.Height.Value - r.Top.Value - r.Height.Value))
        {
            throw Fail("invalid_display_list", $"{at}: rule needs positive width/height and exact-range coordinates");
        }
        ValidatePaint(r.Paint, at);
        ValidateProvenance(r.Sources, r.SyntheticReason, documents, at);
    }

    private static void ValidateImageItem(Page page, string at, Image i, IReadOnlyDictionary<string, DocumentResource> documents, HashSet<string> usedFeatures)
    {
        usedFeatures.Add("image");
        if (!IsTick(i.X) || !IsTick(i.Top) || !IsPositiveTick(i.Width) || !IsPositiveTick(i.Height)
            || !IsTick(i.X.Value + i.Width.Value) || !IsTick(i.Top.Value + i.Height.Value) || !IsTick(page.Height.Value - i.Top.Value - i.Height.Value))
        {
            throw Fail("invalid_display_list", $"{at}: image needs positive width/height and exact-range coordinates");
        }
        if (i.Transform.Count != 6 || i.Transform.Any(v => !double.IsFinite(v)))
        {
            throw Fail("invalid_display_list", $"{at}: image transform must be six finite numbers [a, b, c, d, e, f]");
        }
        ValidateImageResource(i.ImageResource, at);
        ValidateProvenance(i.Sources, i.SyntheticReason, documents, at);
    }

    private static void ValidateGlyphRunItem(
        Page page, string at, GlyphRun run, IReadOnlyDictionary<string, FontResource> fontsById, IReadOnlyDictionary<string, DocumentResource> documents)
    {
        if (!fontsById.TryGetValue(run.FontId, out var font))
        {
            throw Fail("invalid_resource", $"{at}: font resource '{run.FontId}' is not declared in fonts");
        }
        if (!IsPositiveTick(run.FontSize))
        {
            throw Fail("invalid_display_list", $"{at}: font_size must be a positive exact tick count");
        }
        if (!Bounds.RunTextBytes.Contains(ByteOffsets.Utf8ByteCount(run.Text)))
        {
            throw Fail("invalid_display_list", $"{at}: glyph run text must be 1...{Bounds.RunTextBytes.Max} bytes");
        }
        if (!Bounds.Glyphs.Contains(run.Glyphs.Count))
        {
            throw Fail("invalid_display_list", $"{at}: glyph run must carry 1...{Bounds.Glyphs.Max} glyphs (found {run.Glyphs.Count})");
        }
        if (!Bounds.Clusters.Contains(run.Clusters.Count))
        {
            throw Fail("invalid_display_list", $"{at}: glyph run must carry 1...{Bounds.Clusters.Max} clusters (found {run.Clusters.Count})");
        }
        ValidatePaint(run.Paint, at);
        ValidateGlyphs(page, at, run, font);
        ValidateClusters(at, run, documents);
    }

    private static void ValidateGlyphs(Page page, string at, GlyphRun run, FontResource font)
    {
        var referenced = new HashSet<int>();
        for (int gi = 0; gi < run.Glyphs.Count; gi++)
        {
            var g = run.Glyphs[gi];
            if (g.Gid < 1 || g.Gid >= font.GlyphCount)
            {
                throw Fail("invalid_display_list", $"{at} glyph {gi}: gid {g.Gid} is outside 1..<{font.GlyphCount} of font {font.PostscriptName}");
            }
            if (g.Cluster < 0 || g.Cluster >= run.Clusters.Count)
            {
                throw Fail("invalid_display_list", $"{at} glyph {gi}: cluster {g.Cluster} is outside 0..<{run.Clusters.Count}");
            }
            referenced.Add(g.Cluster);
            if (!IsTick(g.OriginX) || !IsTick(g.BaselineY) || !IsTick(g.AdvanceX) || !IsTick(g.AdvanceY)
                || !IsTick(g.OriginX.Value + g.AdvanceX.Value) || !IsTick(g.BaselineY.Value + g.AdvanceY.Value) || !IsTick(page.Height.Value - g.BaselineY.Value))
            {
                throw Fail("invalid_display_list", $"{at} glyph {gi}: origin/advance outside the exact tick range");
            }
        }
        if (referenced.Count != run.Clusters.Count)
        {
            throw OrphanedClusterFailure(at, run, referenced);
        }
    }

    /// <summary>
    /// Names the offending cluster(s): index, logical text and source span(s), so the
    /// refusal points at the source that produced them (a missing-glyph scalar glued
    /// to a word yields a cluster with no glyph).
    /// </summary>
    private static RenderingV2ValidationException OrphanedClusterFailure(string at, GlyphRun run, HashSet<int> referenced)
    {
        var orphaned = Enumerable.Range(0, run.Clusters.Count).Where(i => !referenced.Contains(i)).ToList();
        string named = string.Join("; ", orphaned.Select(ci =>
        {
            var c = run.Clusters[ci];
            string where = c.Sources is { } sources
                ? string.Join(", ", sources.Select(s => $"{s.Path} bytes {s.StartByte}..<{s.EndByte}"))
                : c.SyntheticReason is { } reason ? $"generated: {reason}" : "no provenance";
            return $"cluster {ci} “{run.ClusterText(ci)}” ({where})";
        }));
        var firstSource = orphaned.Count > 0 ? run.Clusters[orphaned[0]].Sources?.FirstOrDefault() : null;
        return Fail("invalid_display_list", $"{at}: {orphaned.Count} cluster(s) have no glyph (every cluster needs at least one glyph): {named}", firstSource);
    }

    private static void ValidateClusters(string at, GlyphRun run, IReadOnlyDictionary<string, DocumentResource> documents)
    {
        byte[] textBytes = Encoding.UTF8.GetBytes(run.Text);
        int expectedStart = 0;
        for (int ci = 0; ci < run.Clusters.Count; ci++)
        {
            var c = run.Clusters[ci];
            string cat = $"{at} cluster {ci}";
            if (c.TextStartByte != expectedStart || c.TextEndByte <= c.TextStartByte || c.TextEndByte > textBytes.Length)
            {
                throw Fail("invalid_display_list", $"{cat}: byte range {c.TextStartByte}..<{c.TextEndByte} does not partition the {textBytes.Length}-byte run text (expected a nonempty range starting at {expectedStart})");
            }
            if (!IsUtf8Boundary(textBytes, c.TextStartByte) || !IsUtf8Boundary(textBytes, c.TextEndByte))
            {
                throw Fail("invalid_display_list", $"{cat}: byte range {c.TextStartByte}..<{c.TextEndByte} splits a UTF-8 sequence");
            }
            expectedStart = c.TextEndByte;
            ValidateClusterRectsAndCarets(cat, c, textBytes);
            ValidateProvenance(c.Sources, c.SyntheticReason, documents, cat);
        }
        if (expectedStart != textBytes.Length)
        {
            throw Fail("invalid_display_list", $"{at}: clusters cover {expectedStart} of {textBytes.Length} text bytes");
        }
    }

    private static void ValidateClusterRectsAndCarets(string cat, Cluster c, byte[] textBytes)
    {
        if (!Bounds.HitRects.Contains(c.HitRects.Count))
        {
            throw Fail("invalid_display_list", $"{cat}: hit_rects must carry 1...{Bounds.HitRects.Max} rectangles (found {c.HitRects.Count})");
        }
        if (!Bounds.Carets.Contains(c.Carets.Count))
        {
            throw Fail("invalid_display_list", $"{cat}: at most {Bounds.Carets.Max} carets (found {c.Carets.Count})");
        }
        foreach (var r in c.HitRects)
        {
            if (!IsTick(r.X) || !IsTick(r.Top) || !IsTick(r.Width) || !IsTick(r.Height) || r.Width.Value < 0 || r.Height.Value < 0
                || !IsTick(r.X.Value + r.Width.Value) || !IsTick(r.Top.Value + r.Height.Value))
            {
                throw Fail("invalid_display_list", $"{cat}: hit rect has a negative size or coordinates outside the exact tick range");
            }
        }
        foreach (var k in c.Carets)
        {
            if (k.TextByte < c.TextStartByte || k.TextByte > c.TextEndByte || !IsUtf8Boundary(textBytes, k.TextByte))
            {
                throw Fail("invalid_display_list", $"{cat}: caret text_byte {k.TextByte} is outside the cluster or splits a UTF-8 sequence");
            }
            if (!IsTick(k.X) || !IsTick(k.Top) || !IsPositiveTick(k.Height) || !IsTick(k.Top.Value + k.Height.Value))
            {
                throw Fail("invalid_display_list", $"{cat}: caret needs a positive height and exact-range coordinates");
            }
        }
    }

    private static void ValidateDiagnostics(IReadOnlyList<Diagnostic> diagnostics, IReadOnlyDictionary<string, DocumentResource> documents)
    {
        foreach (var d in diagnostics)
        {
            if (d.Code.Length == 0 || !Bounds.DiagnosticMessageBytes.Contains(ByteOffsets.Utf8ByteCount(d.Message)))
            {
                throw Fail("invalid_display_list", $"diagnostics must carry a code and a 1...{Bounds.DiagnosticMessageBytes.Max}-byte message");
            }
            if (!Bounds.DiagnosticSources.Contains(d.Sources.Count))
            {
                throw Fail("invalid_display_list", $"diagnostic '{d.Code}' lists {d.Sources.Count} sources (limit {Bounds.DiagnosticSources.Max})");
            }
            foreach (var s in d.Sources)
            {
                ValidateSource(s, documents, $"diagnostic '{d.Code}'");
            }
        }
    }

    private static void CheckFeatureDeclarations(IReadOnlyList<string> requiredFeatures, HashSet<string> usedFeatures)
    {
        var declared = new HashSet<string>(requiredFeatures, StringComparer.Ordinal);
        var undeclared = usedFeatures.Where(f => !declared.Contains(f)).OrderBy(f => f, StringComparer.Ordinal).ToList();
        if (undeclared.Count > 0)
        {
            throw Fail("invalid_display_list", $"the list uses feature(s) {string.Join(", ", undeclared)} that required_features does not declare ({string.Join(", ", requiredFeatures)})");
        }
    }

    // MARK: shared low-level checks

    /// <summary>|t| &lt;= 2^53 - 1: representable exactly in JSON and in a Double.</summary>
    internal static bool IsTick(long t) => t >= -MaxExactInteger && t <= MaxExactInteger;

    internal static bool IsTick(Ticks t) => IsTick(t.Value);

    internal static bool IsPositiveTick(long t) => t > 0 && t <= MaxExactInteger;

    internal static bool IsPositiveTick(Ticks t) => IsPositiveTick(t.Value);

    /// <summary>crates/rendering-core `path`: no backslash, colon or NUL, and no empty, `.` or `..` component.</summary>
    internal static bool IsProjectPath(string p)
    {
        if (p.Length == 0 || p.Contains('\\') || p.Contains(':') || p.Contains('\0'))
        {
            return false;
        }
        foreach (var segment in p.Split('/'))
        {
            if (segment.Length == 0 || segment == "." || segment == "..")
            {
                return false;
            }
        }
        return true;
    }

    private static bool IsUtf8Boundary(byte[] bytes, int i) => i == bytes.Length || (i >= 0 && i < bytes.Length && (bytes[i] & 0xC0) != 0x80);

    private static bool IsHex64(string s)
    {
        if (s.Length != 64)
        {
            return false;
        }
        foreach (char c in s)
        {
            if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')))
            {
                return false;
            }
        }
        return true;
    }

    /// <summary>
    /// Proposal §3 resource shape: `image_id` is the SHA-256, a bounded positive byte
    /// length, a known format, a project-relative path, pixel dimensions for raster
    /// formats and a page/box/rotation for PDF.
    /// </summary>
    internal static void ValidateImageResource(ImageResource r, string at)
    {
        RenderingV2ValidationException Fail(string m) => new("invalid_resource", $"{at}: image resource {m}");
        if (!IsHex64(r.Sha256))
        {
            throw Fail("sha256 is not 64 lowercase hex digits");
        }
        if (r.ImageId != r.Sha256)
        {
            throw Fail($"image_id '{r.ImageId}' must equal sha256");
        }
        if (r.ByteLength < 1 || r.ByteLength > MaxImageByteLength)
        {
            throw Fail($"byte_length {r.ByteLength} is outside 1...{MaxImageByteLength}");
        }
        if (!ImageFormats.Contains(r.Format))
        {
            throw new RenderingV2ValidationException("unsupported_feature", $"{at}: image format '{r.Format}' is not supported ({string.Join(", ", ImageFormats.OrderBy(x => x, StringComparer.Ordinal))})");
        }
        if (!IsProjectPath(r.Path))
        {
            throw Fail($"path '{r.Path}' must be project-relative: no empty, '.' or '..' components, no backslash, colon or NUL");
        }
        if (r.Format == "pdf")
        {
            ValidatePdfImageResource(r, Fail);
        }
        else if (r.PixelWidth is not (> 0 and <= 1 << 20) || r.PixelHeight is not (> 0 and <= 1 << 20))
        {
            throw Fail($"pixel_width/pixel_height must be positive for {r.Format}");
        }
    }

    private static void ValidatePdfImageResource(ImageResource r, Func<string, RenderingV2ValidationException> fail)
    {
        if (r.PdfPage is not (>= 1 and <= 100_000))
        {
            throw fail("pdf_page must be a positive page number");
        }
        if (r.PdfBox is not { Count: 4 } box || !box.All(double.IsFinite) || !(box[2] > box[0]) || !(box[3] > box[1]))
        {
            throw fail("pdf_box must be [llx, lly, urx, ury] with positive extent");
        }
        if (!new[] { 0, 90, 180, 270 }.Contains(r.PdfRotate ?? 0))
        {
            throw fail("pdf_rotate must be 0, 90, 180 or 270");
        }
    }

    /// <summary>
    /// Commands: bounded count, every coordinate an exact tick, the first command a
    /// move, and nothing but a move after `z`. Strokes need a positive exact width, a
    /// finite miter limit &gt;= 1 and a dash whose entries are nonnegative exact ticks
    /// with at least one positive (an all-zero dash never advances).
    /// </summary>
    internal static void ValidateCommands(IReadOnlyList<PathCommand> commands, string at)
    {
        if (!Bounds.PathCommands.Contains(commands.Count))
        {
            throw Fail("invalid_display_list", $"{at}: path must carry 1...{Bounds.PathCommands.Max} commands (found {commands.Count})");
        }
        bool open = false;
        for (int i = 0; i < commands.Count; i++)
        {
            var c = commands[i];
            if (c.Coordinates.Any(v => !IsTick(v)))
            {
                throw Fail("invalid_display_list", $"{at}: path command {i} has a coordinate outside the exact tick range");
            }
            switch (c)
            {
                case PathCommand.Move:
                    open = true;
                    break;
                case PathCommand.Line or PathCommand.Cubic:
                    if (!open)
                    {
                        throw Fail("invalid_display_list", $"{at}: path command {i} needs a current point (no preceding move)");
                    }
                    break;
                case PathCommand.Close:
                    if (!open)
                    {
                        throw Fail("invalid_display_list", $"{at}: path command {i} closes without an open subpath");
                    }
                    open = false;
                    break;
            }
        }
    }

    internal static void ValidatePathValue(Path p, string at)
    {
        ValidateCommands(p.Commands, at);
        if (!Bounds.Clips.Contains(p.Clips.Count))
        {
            throw Fail("invalid_display_list", $"{at}: at most {Bounds.Clips.Max} clips (found {p.Clips.Count})");
        }
        for (int ci = 0; ci < p.Clips.Count; ci++)
        {
            ValidateCommands(p.Clips[ci].Path, $"{at} clip {ci}");
        }
        ValidatePaint(p.Paint, at);
        if (p.Stroke is { } stroke)
        {
            ValidateStroke(stroke, at);
        }
    }

    private static void ValidateStroke(Stroke stroke, string at)
    {
        if (!IsPositiveTick(stroke.Width))
        {
            throw Fail("invalid_display_list", $"{at}: stroke width must be a positive exact tick count");
        }
        if (!double.IsFinite(stroke.MiterLimit) || stroke.MiterLimit < 1)
        {
            throw Fail("invalid_display_list", $"{at}: stroke miter_limit must be a finite number >= 1");
        }
        if (stroke.Dash is { } dash)
        {
            if (!Bounds.DashEntries.Contains(dash.Array.Count))
            {
                throw Fail("invalid_display_list", $"{at}: dash array must carry 1...{Bounds.DashEntries.Max} entries (found {dash.Array.Count})");
            }
            if (!dash.Array.All(v => v.Value >= 0 && IsTick(v)) || !dash.Array.Any(v => v.Value > 0))
            {
                throw Fail("invalid_display_list", $"{at}: dash entries must be nonnegative exact ticks with at least one positive");
            }
            if (dash.Phase.Value < 0 || !IsTick(dash.Phase))
            {
                throw Fail("invalid_display_list", $"{at}: dash phase must be a nonnegative exact tick count");
            }
        }
    }

    private static void ValidatePaint(Paint p, string at)
    {
        foreach (var v in new[] { p.R, p.G, p.B, p.A })
        {
            if (!(v >= 0 && v <= 1))
            {
                throw Fail("invalid_display_list", $"{at}: paint components must be within 0...1");
            }
        }
    }

    /// <summary>A source range must name a declared document and lie within its declared byte length.</summary>
    private static void ValidateSource(RuntimeV1.SourceRange s, IReadOnlyDictionary<string, DocumentResource> documents, string at)
    {
        if (!documents.TryGetValue(s.Path, out var doc))
        {
            throw Fail("invalid_display_list", $"{at}: source path '{s.Path}' is not a declared document", s);
        }
        if (s.StartByte < 0 || s.EndByte < s.StartByte || s.EndByte > doc.ByteLength)
        {
            throw Fail("invalid_display_list", $"{at}: source range {s.StartByte}..<{s.EndByte} is malformed or outside {s.Path}'s {doc.ByteLength} bytes", s);
        }
    }

    private static void ValidateProvenance(
        IReadOnlyList<RuntimeV1.SourceRange>? sources, string? synthetic, IReadOnlyDictionary<string, DocumentResource> documents, string at)
    {
        if (sources is null && synthetic is null)
        {
            throw Fail("invalid_display_list", $"{at}: needs sources or synthetic_reason");
        }
        if (sources is not null && synthetic is not null)
        {
            throw Fail("invalid_display_list", $"{at}: sources and synthetic_reason are mutually exclusive");
        }
        if (synthetic is not null)
        {
            if (!Bounds.SyntheticReasonBytes.Contains(ByteOffsets.Utf8ByteCount(synthetic)))
            {
                throw Fail("invalid_display_list", $"{at}: synthetic_reason must be 1...{Bounds.SyntheticReasonBytes.Max} bytes");
            }
            return;
        }
        var ranges = sources!;
        if (!Bounds.SourceRanges.Contains(ranges.Count))
        {
            throw Fail("invalid_display_list", $"{at}: sources must list 1...{Bounds.SourceRanges.Max} ranges (found {ranges.Count})");
        }
        foreach (var s in ranges)
        {
            ValidateSource(s, documents, at);
        }
    }
}
