// name: Capture.cs
// purpose: Capture and insertion messages from runtime v1 ("Capture and insertion"
//   section of docs/contracts/runtime-v1.md). Ported from
//   apps/mac/Sources/FlashTeXProtocol/Capture.swift.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text.Json;
using System.Text.Json.Serialization;

namespace FlashTeX.Protocol.RuntimeV1;

/// <summary>A captured image: MIME type plus base64-encoded bytes.</summary>
public sealed record CaptureImage(
    [property: JsonPropertyName("mime_type")] string MimeType,
    [property: JsonPropertyName("data_base64")] string DataBase64);

/// <summary>A <c>capture_submit</c> request payload.</summary>
public sealed record CaptureSubmit(
    [property: JsonPropertyName("capture_id")] string CaptureId,
    [property: JsonPropertyName("destination_id")] string DestinationId,
    [property: JsonPropertyName("base_revision")] int BaseRevision,
    [property: JsonPropertyName("image")] CaptureImage Image,
    [property: JsonPropertyName("instructions")] string Instructions);

/// <summary>Acknowledges only durable receipt of a capture (runtime-v1 shape, no bridge fields).</summary>
public sealed record CaptureReceived([property: JsonPropertyName("capture_id")] string CaptureId);

/// <summary>
/// Proposed LaTeX for a capture. Never inserted automatically. <see cref="ContextRevision"/>
/// (transfer-v1) is the source revision the bridge assembled context from; absent in
/// plain runtime-v1 proposal files.
/// </summary>
public sealed record CaptureProposal(
    [property: JsonPropertyName("capture_id")] string CaptureId,
    [property: JsonPropertyName("latex")] string Latex,
    [property: JsonPropertyName("ambiguities")] IReadOnlyList<string> Ambiguities,
    [property: JsonPropertyName("required_dependencies")] IReadOnlyList<string> RequiredDependencies,
    [property: JsonPropertyName("context_revision")]
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    int? ContextRevision = null);

/// <summary>Capture and insertion protocol constants and decode helpers.</summary>
public static class CaptureProtocol
{
    public static readonly IReadOnlyCollection<string> AcceptedMimeTypes = new HashSet<string>(StringComparer.Ordinal)
    {
        "image/png",
        "image/jpeg",
    };

    public static Envelope<CaptureSubmit> DecodeCaptureSubmit(ReadOnlySpan<byte> data, JsonSerializerOptions options) =>
        Protocol.Decode<CaptureSubmit>(data, "capture_submit", options);

    public static Envelope<CaptureProposal> DecodeCaptureProposal(ReadOnlySpan<byte> data, JsonSerializerOptions options) =>
        Protocol.Decode<CaptureProposal>(data, "capture_proposal", options);
}
