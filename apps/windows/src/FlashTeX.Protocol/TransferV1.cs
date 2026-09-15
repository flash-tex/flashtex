// name: TransferV1.cs
// purpose: The Nearby/bridge capture-transfer protocol — documents, anchors,
//   proposals, capture lifecycle (docs/contracts/transfer-v1.md). Ported from
//   apps/mac/Sources/FlashTeXProtocol/TransferV1.swift. Additive to runtime v1: the
//   same envelope, snake_case keys, zero-based end-exclusive UTF-8 byte offsets, and
//   `error {code, message}` replies. `capture_submit` reuses `RuntimeV1.CaptureSubmit`.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text.Json.Serialization;

namespace FlashTeX.Protocol.TransferV1;

/// <summary>Bridge protocol size/identifier limits (transfer-v1.md).</summary>
public static class TransferV1Protocol
{
    /// <summary>Each line, including its newline, is at most this many bytes in either direction.</summary>
    public const int MaxLineBytes = 12 * 1024 * 1024;
    public const int MaxRequestIdBytes = 128;
    public const int MaxDocumentBytes = 8 * 1024 * 1024;
    public const int MaxImageBytes = 8 * 1024 * 1024;
    public const int MaxInstructionBytes = 4096;

    /// <summary><c>capture_convert</c>'s <c>supported_features</c>: at most this many entries.</summary>
    public const int MaxSupportedFeatures = 64;
    public const int MaxSupportedFeatureBytes = 128;

    /// <summary><c>capture_proposal</c>/journaled proposal <c>latex</c> byte bounds.</summary>
    public const int MinLatexBytes = 1;
    public const int MaxLatexBytes = 65536;

    /// <summary><c>ambiguities</c>/<c>required_dependencies</c> list and entry bounds.</summary>
    public const int MaxProposalListEntries = 32;
    public const int MaxProposalEntryBytes = 2048;

    /// <summary>1-128 ASCII alphanumerics, <c>-</c> or <c>_</c>.</summary>
    public static bool IsValidIdentifier(string id)
    {
        if (id.Length == 0 || ByteOffsets.Utf8ByteCount(id) > MaxRequestIdBytes)
        {
            return false;
        }
        foreach (char c in id)
        {
            bool ok = (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') || c == '-' || c == '_';
            if (!ok)
            {
                return false;
            }
        }
        return true;
    }

    /// <summary>Normalized relative path: no leading <c>/</c>, no <c>\</c>, <c>:</c>, NUL, empty, <c>.</c> or <c>..</c> segments.</summary>
    public static bool IsValidRelativePath(string path)
    {
        if (path.Length == 0 || path.StartsWith('/') || path.Contains('\\') || path.Contains(':') || path.Contains('\0'))
        {
            return false;
        }
        foreach (var segment in path.Split('/'))
        {
            if (segment.Length == 0 || segment == "." || segment == "..")
            {
                return false;
            }
        }
        return true;
    }
}

/// <summary>Request type to reply type, exactly as <c>crates/bridge/src/main.rs</c> dispatches.</summary>
public enum Request
{
    DocumentOpen,
    DocumentEdit,
    DestinationPin,
    CaptureSubmit,
    CaptureConvert,
    CapturePrepareInsert,
    CaptureApplied,
    CaptureStatus,
    CaptureReject,
}

/// <summary>Wire strings for <see cref="Request"/> and its reply type.</summary>
public static class RequestExtensions
{
    public static string WireName(this Request request) => request switch
    {
        Request.DocumentOpen => "document_open",
        Request.DocumentEdit => "document_edit",
        Request.DestinationPin => "destination_pin",
        Request.CaptureSubmit => "capture_submit",
        Request.CaptureConvert => "capture_convert",
        Request.CapturePrepareInsert => "capture_prepare_insert",
        Request.CaptureApplied => "capture_applied",
        Request.CaptureStatus => "capture_status",
        Request.CaptureReject => "capture_reject",
        _ => throw new ArgumentOutOfRangeException(nameof(request)),
    };

    public static string ReplyType(this Request request) => request switch
    {
        Request.DocumentOpen => "document_opened",
        Request.DocumentEdit => "document_updated",
        Request.DestinationPin => "destination_pinned",
        Request.CaptureSubmit => "capture_received",
        Request.CaptureConvert => "capture_proposal",
        Request.CapturePrepareInsert => "capture_edit",
        Request.CaptureApplied => "capture_application_received",
        Request.CaptureStatus => "capture_status",
        Request.CaptureReject => "capture_rejected",
        _ => throw new ArgumentOutOfRangeException(nameof(request)),
    };
}

/// <summary>
/// <c>error</c> reply payload. Codes are stable strings such as <c>provider_disabled</c>.
/// Deliberately a plain DTO (not an <see cref="Exception"/>): the Swift source's
/// <c>ErrorPayload</c> conforms to <c>Error</c> only so it can be thrown/caught in
/// Swift, and an <see cref="Exception"/> subtype here would leak base-class members
/// (StackTrace, Data, ...) into the wire shape via default JSON contract resolution.
/// </summary>
public sealed record ErrorPayload(
    [property: JsonPropertyName("code")] string Code,
    [property: JsonPropertyName("message")] string Message);

/// <summary>An empty payload, e.g. <c>document_opened</c>'s reply.</summary>
public sealed record Empty;

// MARK: documents and destinations

public sealed record DocumentOpen(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] int Revision,
    [property: JsonPropertyName("text")] string Text);

public sealed record DocumentEdit(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("base_revision")] int BaseRevision,
    [property: JsonPropertyName("revision")] int Revision,
    [property: JsonPropertyName("start_byte")] int StartByte,
    [property: JsonPropertyName("end_byte")] int EndByte,
    [property: JsonPropertyName("replacement")] string Replacement);

public sealed record DocumentUpdated([property: JsonPropertyName("revision")] int Revision);

public sealed record DestinationPin(
    [property: JsonPropertyName("destination_id")] string DestinationId,
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] int Revision,
    [property: JsonPropertyName("start_byte")] int StartByte,
    [property: JsonPropertyName("end_byte")] int EndByte);

/// <summary>Immutable binding of a destination to the exact source it was pinned in.</summary>
public sealed record AnchorBinding(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] int Revision,
    [property: JsonPropertyName("start_byte")] int StartByte,
    [property: JsonPropertyName("end_byte")] int EndByte,
    [property: JsonPropertyName("source_sha256")] string SourceSha256);

public sealed record Anchor(
    [property: JsonPropertyName("destination_id")] string DestinationId,
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("pinned_revision")] int PinnedRevision,
    [property: JsonPropertyName("current_revision")] int CurrentRevision,
    [property: JsonPropertyName("start_byte")] int StartByte,
    [property: JsonPropertyName("end_byte")] int EndByte,
    [property: JsonPropertyName("valid")] bool IsValid,
    [property: JsonPropertyName("binding")] AnchorBinding Binding);

// MARK: capture lifecycle

public sealed record CaptureReceived(
    [property: JsonPropertyName("capture_id")] string CaptureId,
    [property: JsonPropertyName("durable")] bool Durable,
    [property: JsonPropertyName("has_proposal")] bool HasProposal,
    [property: JsonPropertyName("applied")] bool Applied);

/// <summary><c>capture_convert</c> request payload.</summary>
public sealed record CaptureConvert
{
    [JsonConstructor]
    public CaptureConvert(string captureId, IReadOnlyList<string> supportedFeatures)
    {
        if (supportedFeatures.Count > TransferV1Protocol.MaxSupportedFeatures)
        {
            throw new ArgumentException(
                $"supported_features carries {supportedFeatures.Count} entries (limit {TransferV1Protocol.MaxSupportedFeatures})");
        }
        foreach (var feature in supportedFeatures)
        {
            if (ByteOffsets.Utf8ByteCount(feature) > TransferV1Protocol.MaxSupportedFeatureBytes)
            {
                throw new ArgumentException($"supported feature '{feature}' exceeds {TransferV1Protocol.MaxSupportedFeatureBytes} bytes");
            }
        }
        CaptureId = captureId;
        SupportedFeatures = supportedFeatures;
    }

    [JsonPropertyName("capture_id")]
    public string CaptureId { get; }

    [JsonPropertyName("supported_features")]
    public IReadOnlyList<string> SupportedFeatures { get; }
}

public sealed record CaptureId([property: JsonPropertyName("capture_id")] string Value);

public sealed record CapturePrepareInsert(
    [property: JsonPropertyName("capture_id")] string CaptureId,
    [property: JsonPropertyName("expected_revision")] int ExpectedRevision,
    [property: JsonPropertyName("approved")] bool Approved);

/// <summary>
/// A prepared, persisted edit. Preparation never edits source; the Mac verifies
/// every field against its buffer before applying it once.
/// </summary>
public sealed record CaptureEdit(
    [property: JsonPropertyName("capture_id")] string CaptureId,
    [property: JsonPropertyName("edit_id")] string EditId,
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("expected_revision")] int ExpectedRevision,
    [property: JsonPropertyName("start_byte")] int StartByte,
    [property: JsonPropertyName("end_byte")] int EndByte,
    [property: JsonPropertyName("removed_text")] string RemovedText,
    [property: JsonPropertyName("replacement")] string Replacement,
    [property: JsonPropertyName("document_before_sha256")] string DocumentBeforeSha256);

public sealed record CaptureApplied(
    [property: JsonPropertyName("capture_id")] string CaptureId,
    [property: JsonPropertyName("edit_id")] string EditId,
    [property: JsonPropertyName("new_revision")] int NewRevision);

/// <summary>Journaled proposal as it appears inside <c>capture_status</c> (no capture_id).</summary>
public sealed record Proposal
{
    [JsonConstructor]
    public Proposal(string latex, IReadOnlyList<string> ambiguities, IReadOnlyList<string> requiredDependencies)
    {
        int latexBytes = ByteOffsets.Utf8ByteCount(latex);
        if (latexBytes < TransferV1Protocol.MinLatexBytes || latexBytes > TransferV1Protocol.MaxLatexBytes)
        {
            throw new ArgumentException($"latex must be {TransferV1Protocol.MinLatexBytes}...{TransferV1Protocol.MaxLatexBytes} UTF-8 bytes");
        }
        ValidateEntryList(ambiguities, nameof(ambiguities));
        ValidateEntryList(requiredDependencies, nameof(requiredDependencies));
        Latex = latex;
        Ambiguities = ambiguities;
        RequiredDependencies = requiredDependencies;
    }

    private static void ValidateEntryList(IReadOnlyList<string> entries, string fieldName)
    {
        if (entries.Count > TransferV1Protocol.MaxProposalListEntries)
        {
            throw new ArgumentException($"{fieldName} carries {entries.Count} entries (limit {TransferV1Protocol.MaxProposalListEntries})");
        }
        foreach (var entry in entries)
        {
            if (ByteOffsets.Utf8ByteCount(entry) > TransferV1Protocol.MaxProposalEntryBytes)
            {
                throw new ArgumentException($"{fieldName} entry exceeds {TransferV1Protocol.MaxProposalEntryBytes} bytes");
            }
        }
    }

    [JsonPropertyName("latex")]
    public string Latex { get; }

    [JsonPropertyName("ambiguities")]
    public IReadOnlyList<string> Ambiguities { get; }

    [JsonPropertyName("required_dependencies")]
    public IReadOnlyList<string> RequiredDependencies { get; }
}

public sealed record AppliedEdit(
    [property: JsonPropertyName("edit_id")] string EditId,
    [property: JsonPropertyName("new_revision")] int NewRevision);

public sealed record CaptureStatus(
    [property: JsonPropertyName("capture_id")] string CaptureId,
    [property: JsonPropertyName("proposal")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] Proposal? Proposal,
    [property: JsonPropertyName("prepared")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] CaptureEdit? Prepared,
    [property: JsonPropertyName("applied")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] AppliedEdit? Applied,
    [property: JsonPropertyName("rejected")] bool Rejected);
