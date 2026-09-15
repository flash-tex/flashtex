// name: ProjectFilesV1.cs
// purpose: Wire DTOs for the private flashtex-project-files JSON Lines helper
//   (protocol project-files-v1): rooted, symlink-refusing read/status/save on
//   one project directory. Requests are `{id, operation, ...fields}`; replies
//   are `{id, payload}` or `{id, error: {code, message}}` — a save *conflict*
//   is a payload, never an error: nothing was written and the caller must show
//   it and keep its buffer. Authoritative source:
//   crates/project-files/src/bin/flashtex-project-files.rs. Ported from the
//   responsibility of apps/mac/Sources/FlashTeXMac/DocumentFilesClient.swift's
//   `ProjectFilesV1` enum.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json.Serialization;

namespace FlashTeX.Protocol.ProjectFilesV1;

/// <summary>Protocol identifier and size limits (flashtex-project-files.rs).</summary>
public static class ProjectFilesV1Protocol
{
    public const string ProtocolName = "project-files-v1";

    /// <summary>Each line, including its newline, is at most this many bytes in either direction.</summary>
    public const int MaxLineBytes = 12 * 1024 * 1024;

    /// <summary>The helper's own read bound (<c>DEFAULT_READ_LIMIT</c>, crates/project-files/src/save.rs); informational only, not enforced client-side.</summary>
    public const long DefaultReadLimitBytes = 64 * 1024 * 1024;
}

// MARK: requests

/// <summary><c>ping</c> request: no fields beyond the envelope.</summary>
public sealed record PingRequest
{
    public PingRequest(string id) => Id = id;

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "ping";
}

/// <summary><c>read</c> request: the rooted, project-relative path to read.</summary>
public sealed record ReadRequest
{
    public ReadRequest(string id, string path)
    {
        Id = id;
        Path = path;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "read";

    [JsonPropertyName("path")]
    public string Path { get; }
}

/// <summary>
/// <c>status</c> request. <see cref="ExpectedSha256"/> is sent as an explicit
/// JSON <c>null</c> (never omitted) to mean "the caller expects no file yet" —
/// System.Text.Json already serializes a null property by default, so unlike
/// the Swift source's hand-written <c>encode(to:)</c> no custom writer is needed.
/// </summary>
public sealed record StatusRequest
{
    public StatusRequest(string id, string path, string? expectedSha256)
    {
        Id = id;
        Path = path;
        ExpectedSha256 = expectedSha256;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "status";

    [JsonPropertyName("path")]
    public string Path { get; }

    [JsonPropertyName("expected_sha256")]
    public string? ExpectedSha256 { get; }
}

/// <summary><c>save</c> request. <see cref="Expected"/> is <see cref="ProjectFilesV1.Expected.Wire"/>: <c>"new"</c>, <c>"any"</c>, or a SHA-256 hex digest.</summary>
public sealed record SaveRequest
{
    public SaveRequest(string id, string path, string text, string expected, bool force)
    {
        Id = id;
        Path = path;
        Text = text;
        Expected = expected;
        Force = force;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "save";

    [JsonPropertyName("path")]
    public string Path { get; }

    [JsonPropertyName("text")]
    public string Text { get; }

    [JsonPropertyName("expected")]
    public string Expected { get; }

    [JsonPropertyName("force")]
    public bool Force { get; }
}

/// <summary>
/// What the caller believes is on disk before a <c>save</c>. Never itself
/// serialized — a client passes one to <c>DocumentFilesClient.SaveAsync</c>,
/// which reads <see cref="Wire"/> into <see cref="SaveRequest.Expected"/>.
/// </summary>
public abstract record Expected
{
    private Expected()
    {
    }

    /// <summary>No file is expected to exist yet (wire <c>"new"</c>).</summary>
    public sealed record NoFileYet : Expected;

    /// <summary>Overwrite whatever is there, if anything (wire <c>"any"</c>).</summary>
    public sealed record AnyContent : Expected;

    /// <summary>The file must currently hash to <see cref="Sha256Hex"/> (wire: the hex digest itself).</summary>
    public sealed record MatchesHash(string Sha256Hex) : Expected;

    public static readonly Expected NewFile = new NoFileYet();
    public static readonly Expected Any = new AnyContent();

    public static Expected Hash(string sha256Hex) => new MatchesHash(sha256Hex);

    public string Wire => this switch
    {
        NoFileYet => "new",
        AnyContent => "any",
        MatchesHash hash => hash.Sha256Hex,
        _ => throw new NotSupportedException($"unknown {nameof(ProjectFilesV1.Expected)} variant {GetType()}"),
    };
}

// MARK: replies

/// <summary><c>ping</c> reply payload.</summary>
public sealed record PingPayload(
    [property: JsonPropertyName("protocol")] string Protocol,
    [property: JsonPropertyName("root")] string Root,
    [property: JsonPropertyName("pid")] long Pid);

/// <summary>One rooted read. <see cref="Exists"/> false carries only <see cref="Path"/>.</summary>
public sealed record ReadPayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("exists")] bool Exists,
    [property: JsonPropertyName("text")] string? Text,
    [property: JsonPropertyName("sha256")] string? Sha256,
    [property: JsonPropertyName("bytes")] long? Bytes,
    [property: JsonPropertyName("mtime_unix_ms")] long? MtimeUnixMs);

/// <summary>On-disk state relative to the hash the caller last saw (<c>status</c>'s <c>expected_sha256</c>).</summary>
[JsonConverter(typeof(JsonStringEnumConverter<DiskState>))]
public enum DiskState
{
    unchanged,
    modified,
    deleted,
    created,
}

/// <summary><c>status</c> reply payload.</summary>
public sealed record StatusPayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("exists")] bool Exists,
    [property: JsonPropertyName("state")] DiskState State,
    [property: JsonPropertyName("sha256")] string? Sha256,
    [property: JsonPropertyName("bytes")] long? Bytes,
    [property: JsonPropertyName("mtime_unix_ms")] long? MtimeUnixMs);

/// <summary>Successful <c>save</c> receipt: the file now holds exactly the saved bytes.</summary>
public sealed record SaveReceipt(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("bytes")] long Bytes,
    [property: JsonPropertyName("sha256")] string Sha256,
    [property: JsonPropertyName("mtime_unix_ms")] long MtimeUnixMs);

/// <summary>Why a <c>save</c> was refused without writing anything.</summary>
[JsonConverter(typeof(JsonStringEnumConverter<ConflictKind>))]
public enum ConflictKind
{
    /// <summary>The file exists with content other than <c>expected</c>.</summary>
    modified_externally,

    /// <summary><c>expected</c> named a hash but the file is gone.</summary>
    deleted_externally,

    /// <summary>A new file was expected but something already sits at the path.</summary>
    already_exists,

    /// <summary>An out-of-contract writer interfered between check and rename.</summary>
    modified_during_save,
}

/// <summary>
/// A refused <c>save</c>. Never an error: the on-disk file no longer matches
/// what the caller last saw (or now exists/is gone unexpectedly), nothing was
/// written, and the caller must show this and keep its buffer.
/// </summary>
public sealed record SaveConflict(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("kind")] ConflictKind Kind,
    [property: JsonPropertyName("ours")] string? Ours,
    [property: JsonPropertyName("theirs")] string? Theirs,
    [property: JsonPropertyName("mtime_unix_ms")] long? MtimeUnixMs,
    [property: JsonPropertyName("size")] long? Size);

/// <summary>Raw <c>save</c> reply shape, before <see cref="SaveOutcome"/> narrows it to exactly one case.</summary>
public sealed record SaveOutcomeWire(
    [property: JsonPropertyName("outcome")] string Outcome,
    [property: JsonPropertyName("receipt")] SaveReceipt? Receipt,
    [property: JsonPropertyName("conflict")] SaveConflict? Conflict);

/// <summary>Narrowed <c>save</c> result: exactly a receipt or a conflict, never both.</summary>
public abstract record SaveOutcome
{
    private SaveOutcome()
    {
    }

    public sealed record Saved(SaveReceipt Receipt) : SaveOutcome;

    public sealed record Conflict(SaveConflict Details) : SaveOutcome;
}

/// <summary>project-files-v1's <c>error {code, message}</c> reply payload.</summary>
public sealed record ErrorPayload(
    [property: JsonPropertyName("code")] string Code,
    [property: JsonPropertyName("message")] string Message);
