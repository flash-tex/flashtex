// name: PreviewControllerV1.cs
// purpose: Wire DTOs for the flashtex-preview-controller JSON Lines helper
//   (crates/preview-controller/STDIO.md — the authoritative source for every
//   shape below). The wire envelope is `{protocol_version,session_id,id,type,
//   payload}` on requests; every synchronous reply is uniformly `{...,id,
//   type:"result"|"error",payload}` regardless of operation (unlike runtime-v1's
//   per-operation reply `type`, e.g. `compile_result`), and unsolicited frames
//   (`id` null) are `type:"ready"` (once, at startup) or `type:"update"` with a
//   `payload.kind` discriminating preview/stale/discarded/superseded/cancelled/
//   failed/completed_snapshot/display_candidate. `search_literal` (served by
//   this same binary, delegating to crates/project-index) is included here too.
//   Session/request-id identity, `configure_display_candidates`/
//   `configure_completed_snapshots` negotiation and metadata-vs-full document
//   shapes are read field-for-field from STDIO.md and crates/preview-controller/
//   src/main.rs's `handle()`, not paraphrased.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;
using System.Text.Json.Serialization;
using FlashTeX.Protocol.EditLedgerV1;
using FlashTeX.Protocol.RuntimeV1;
using FlashTeX.Protocol.TransferV1;

namespace FlashTeX.Protocol.PreviewControllerV1;

/// <summary>Protocol size/identifier limits (STDIO.md).</summary>
public static class PreviewControllerProtocol
{
    /// <summary>Incoming request frames are bounded to 1MiB; outgoing (including large compiler results) to 16MiB.</summary>
    public const int MaxLineBytes = 16 * 1024 * 1024;

    /// <summary>A request <c>id</c> is 1-128 bytes, matching the helper's own validation.</summary>
    public const int MaxRequestIdBytes = 128;

    public const int MaxSearchMatches = 1000;
    public const int MaxSearchWork = 1_000_000;
    public const int MaxCompletionLimit = 100;
    public const int MaxPlanBytes = 8 * 1024 * 1024;
    public const int MaxProjectStatusDocuments = 256;

    /// <summary><c>configure_display_candidates</c>'s negotiated capability when the helper started with the default (non-raw) display transport.</summary>
    public const string DisplayCandidatesCapability = "display-candidates-v1";

    /// <summary><c>configure_display_candidates</c>'s negotiated capability when the helper started with <c>display_transport:"raw-prototype"</c>.</summary>
    public const string DisplayCandidatesRawCapability = "display-candidates-raw-v1";

    /// <summary><c>configure_completed_snapshots</c>'s negotiated capability (crates/preview-controller/src/completed_protocol.rs's <c>CAPABILITY</c>).</summary>
    public const string CompletedSnapshotsCapability = "completed-snapshots-v1";
}

// MARK: request/reply envelope

/// <summary>
/// Every request's outer shape: <c>protocol_version</c>, <c>session_id</c>,
/// <c>id</c>, <c>type</c> (the operation name), <c>payload</c>. Replies echo the
/// same shape with <c>type</c> collapsed to <c>result</c>/<c>error</c>; unsolicited
/// frames (<c>ready</c>/<c>update</c>) carry a null <c>id</c> instead.
/// </summary>
public sealed record Envelope<TPayload>(
    [property: JsonPropertyName("protocol_version")] int ProtocolVersion,
    [property: JsonPropertyName("session_id")] string SessionId,
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("type")] string Type,
    [property: JsonPropertyName("payload")] TPayload Payload);

/// <summary>The <c>error</c> reply's payload: a single human-readable message, no stable code.</summary>
public sealed record ErrorPayload([property: JsonPropertyName("message")] string Message);

// MARK: launch configuration (written to a config file, never sent over stdio)

/// <summary>
/// The JSON config file passed as this helper's sole CLI argument. Exactly one
/// of <see cref="StorePaths"/> or (<see cref="ProjectRoot"/>, <see cref="PrivateLedgerRoot"/>)
/// must be set — store-backed and file-backed startup are mutually exclusive.
/// </summary>
public sealed record LaunchConfig
{
    public const int MinStorePaths = 1;
    public const int MaxStorePaths = 256;

    [JsonConstructor]
    public LaunchConfig(
        string sessionId,
        string projectId,
        string entryPath,
        IReadOnlyList<string>? storePaths = null,
        string? projectRoot = null,
        string? privateLedgerRoot = null,
        string? compilerPath = null,
        IReadOnlyList<string>? bibliographyPaths = null,
        string? displayTransport = null,
        bool? diagnosticTimings = null,
        int? compilerMaxFrameBytes = null)
    {
        if (sessionId.Length == 0 || ByteOffsets.Utf8ByteCount(sessionId) > PreviewControllerProtocol.MaxRequestIdBytes)
        {
            throw new ArgumentException($"session_id must be 1-{PreviewControllerProtocol.MaxRequestIdBytes} UTF-8 bytes", nameof(sessionId));
        }
        bool storeBacked = storePaths is not null;
        bool fileBacked = projectRoot is not null || privateLedgerRoot is not null;
        if (storeBacked == fileBacked)
        {
            throw new ArgumentException("choose exactly one of store_paths or (project_root, private_ledger_root)");
        }
        if (storePaths is not null && storePaths.Count is < MinStorePaths or > MaxStorePaths)
        {
            throw new ArgumentException($"store_paths needs {MinStorePaths}-{MaxStorePaths} entries", nameof(storePaths));
        }
        SessionId = sessionId;
        ProjectId = projectId;
        EntryPath = entryPath;
        StorePaths = storePaths;
        ProjectRoot = projectRoot;
        PrivateLedgerRoot = privateLedgerRoot;
        CompilerPath = compilerPath;
        BibliographyPaths = bibliographyPaths;
        DisplayTransport = displayTransport;
        DiagnosticTimings = diagnosticTimings;
        CompilerMaxFrameBytes = compilerMaxFrameBytes;
    }

    [JsonPropertyName("session_id")]
    public string SessionId { get; }

    [JsonPropertyName("project_id")]
    public string ProjectId { get; }

    [JsonPropertyName("entry_path")]
    public string EntryPath { get; }

    [JsonPropertyName("store_paths")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public IReadOnlyList<string>? StorePaths { get; }

    [JsonPropertyName("project_root")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? ProjectRoot { get; }

    [JsonPropertyName("private_ledger_root")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? PrivateLedgerRoot { get; }

    [JsonPropertyName("compiler_path")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? CompilerPath { get; }

    [JsonPropertyName("bibliography_paths")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public IReadOnlyList<string>? BibliographyPaths { get; }

    /// <summary><c>value</c> (default) or <c>raw-prototype</c>; fixed for the process lifetime.</summary>
    [JsonPropertyName("display_transport")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? DisplayTransport { get; }

    [JsonPropertyName("diagnostic_timings")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public bool? DiagnosticTimings { get; }

    [JsonPropertyName("compiler_max_frame_bytes")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public int? CompilerMaxFrameBytes { get; }
}

// MARK: shared document/source-span shapes

/// <summary>
/// The <c>document</c> field shared by many replies. A metadata-mode response
/// (<c>response_mode:"metadata"</c>) omits <see cref="Text"/> and carries
/// <see cref="ByteLength"/> instead; a full response is the reverse. Exactly one
/// of the two is populated on any given wire value.
/// </summary>
public sealed record PreviewDocument(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] ulong Revision,
    [property: JsonPropertyName("source_sha256")] string SourceSha256,
    [property: JsonPropertyName("text")] string? Text = null,
    [property: JsonPropertyName("byte_length")] int? ByteLength = null);

/// <summary>A lexical source location: <c>{path,revision,start_byte,end_byte}</c> (<c>source_json</c> in main.rs).</summary>
public sealed record SourceSpan(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] ulong Revision,
    [property: JsonPropertyName("start_byte")] int StartByte,
    [property: JsonPropertyName("end_byte")] int EndByte);

// MARK: request payloads

public sealed record PathPayload([property: JsonPropertyName("path")] string Path);

public sealed record EditPayload
{
    public EditPayload(string path, ulong expectedRevision, string expectedSha256, string text,
        string? responseMode = null, string? sourceBindingToken = null)
    {
        Path = path;
        ExpectedRevision = expectedRevision;
        ExpectedSha256 = expectedSha256;
        Text = text;
        ResponseMode = responseMode;
        SourceBindingToken = sourceBindingToken;
    }

    [JsonPropertyName("path")]
    public string Path { get; }

    [JsonPropertyName("expected_revision")]
    public ulong ExpectedRevision { get; }

    [JsonPropertyName("expected_sha256")]
    public string ExpectedSha256 { get; }

    [JsonPropertyName("text")]
    public string Text { get; }

    /// <summary>Set to <c>"metadata"</c> to receive <see cref="PreviewDocument.ByteLength"/> instead of full text back.</summary>
    [JsonPropertyName("response_mode")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? ResponseMode { get; }

    [JsonPropertyName("source_binding_token")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public string? SourceBindingToken { get; }
}

/// <summary>Shared by <c>compile</c>: empty except for the optional negotiated submission binding.</summary>
public sealed record CompilePayload(
    [property: JsonPropertyName("source_binding_token")]
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? SourceBindingToken = null);

public sealed record ReviewPayload([property: JsonPropertyName("edit")] CaptureEdit Edit);

public sealed record ApplyReviewedPayload(
    [property: JsonPropertyName("approval_token")] string ApprovalToken,
    [property: JsonPropertyName("user_approved")] bool UserApproved);

public sealed record ApprovalTokenPayload([property: JsonPropertyName("approval_token")] string ApprovalToken);

public sealed record ConfirmReceiptPayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("receipt")] CaptureApplied Receipt);

public sealed record CompletePayload
{
    public CompletePayload(IReadOnlyDictionary<string, ulong> sourceVersions, string category, string prefix, int limit)
    {
        if (limit is < 1 or > PreviewControllerProtocol.MaxCompletionLimit)
        {
            throw new ArgumentException($"limit must be 1-{PreviewControllerProtocol.MaxCompletionLimit}", nameof(limit));
        }
        SourceVersions = sourceVersions;
        Category = category;
        Prefix = prefix;
        Limit = limit;
    }

    [JsonPropertyName("source_versions")]
    public IReadOnlyDictionary<string, ulong> SourceVersions { get; }

    /// <summary><c>label</c>, <c>citation</c>, or <c>command</c>.</summary>
    [JsonPropertyName("category")]
    public string Category { get; }

    [JsonPropertyName("prefix")]
    public string Prefix { get; }

    [JsonPropertyName("limit")]
    public int Limit { get; }
}

public sealed record NavigatePayload(
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("byte_offset")] int ByteOffset);

public sealed record ApplyGroupPayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("command")] GroupedEdit Command,
    [property: JsonPropertyName("response_mode")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? ResponseMode = null,
    [property: JsonPropertyName("source_binding_token")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? SourceBindingToken = null);

public sealed record HistoryMovePayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("command")] HistoryMove Command,
    [property: JsonPropertyName("response_mode")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? ResponseMode = null);

public sealed record ConfigureLayoutPayload(
    [property: JsonPropertyName("layout_capabilities")] IReadOnlyList<string> LayoutCapabilities,
    [property: JsonPropertyName("renderer_support_confirmed")] bool RendererSupportConfirmed);

public sealed record ExportPayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("expected_revision")] ulong ExpectedRevision,
    [property: JsonPropertyName("expected_sha256")] string ExpectedSha256,
    // Deliberately no [JsonIgnore]: an explicit JSON `null` (new file) is required
    // wire content here, distinct from the field being entirely absent.
    [property: JsonPropertyName("expected_disk_sha256")] string? ExpectedDiskSha256);

public sealed record ReloadPayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("expected_revision")] ulong ExpectedRevision,
    [property: JsonPropertyName("expected_sha256")] string ExpectedSha256,
    [property: JsonPropertyName("expected_disk_sha256")] string ExpectedDiskSha256,
    [property: JsonPropertyName("user_approved")] bool UserApproved,
    [property: JsonPropertyName("source_binding_token")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? SourceBindingToken = null);

/// <summary>Shared by <c>open_document</c>/<c>detach_document</c>; <c>document_kind</c> is meaningful only for open.</summary>
public sealed record MembershipPayload(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("membership_generation")] ulong MembershipGeneration,
    [property: JsonPropertyName("document_kind")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] string? DocumentKind = null);

public sealed record ProjectStatusPayload(
    [property: JsonPropertyName("max_documents")] [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] int? MaxDocuments = null);

public sealed record SearchLiteralPayload
{
    public SearchLiteralPayload(
        IReadOnlyDictionary<string, ulong> sourceVersions, string literal, int maxMatches, int maxWork,
        IReadOnlyList<string>? documents = null)
    {
        if (maxMatches is < 1 || maxMatches > PreviewControllerProtocol.MaxSearchMatches)
        {
            throw new ArgumentException($"max_matches must be 1-{PreviewControllerProtocol.MaxSearchMatches}", nameof(maxMatches));
        }
        if (maxWork is < 1 || maxWork > PreviewControllerProtocol.MaxSearchWork)
        {
            throw new ArgumentException($"max_work must be 1-{PreviewControllerProtocol.MaxSearchWork}", nameof(maxWork));
        }
        SourceVersions = sourceVersions;
        Literal = literal;
        MaxMatches = maxMatches;
        MaxWork = maxWork;
        Documents = documents;
    }

    [JsonPropertyName("source_versions")]
    public IReadOnlyDictionary<string, ulong> SourceVersions { get; }

    [JsonPropertyName("literal")]
    public string Literal { get; }

    [JsonPropertyName("max_matches")]
    public int MaxMatches { get; }

    [JsonPropertyName("max_work")]
    public int MaxWork { get; }

    /// <summary>Restricts the search to these project-relative paths; null/omitted searches every document.</summary>
    [JsonPropertyName("documents")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public IReadOnlyList<string>? Documents { get; }
}

/// <summary>Shared snapshot-identity fields required by every <c>plan_*</c> request (source_plans.rs).</summary>
public abstract record PlanPayloadBase
{
    private protected PlanPayloadBase(IReadOnlyDictionary<string, ulong> sourceVersions, ulong membershipGeneration, int maxBytes)
    {
        if (maxBytes is < 1 || maxBytes > PreviewControllerProtocol.MaxPlanBytes)
        {
            throw new ArgumentException($"max_bytes must be 1-{PreviewControllerProtocol.MaxPlanBytes}", nameof(maxBytes));
        }
        SourceVersions = sourceVersions;
        MembershipGeneration = membershipGeneration;
        MaxBytes = maxBytes;
    }

    [JsonPropertyName("source_versions")]
    public IReadOnlyDictionary<string, ulong> SourceVersions { get; }

    [JsonPropertyName("membership_generation")]
    public ulong MembershipGeneration { get; }

    [JsonPropertyName("max_bytes")]
    public int MaxBytes { get; }
}

public sealed record PlanLiteralReplacementPayload : PlanPayloadBase
{
    public PlanLiteralReplacementPayload(
        IReadOnlyDictionary<string, ulong> sourceVersions, ulong membershipGeneration, int maxBytes,
        string literal, string replacement, int maxMatches, int maxWork, IReadOnlyList<string>? documents = null)
        : base(sourceVersions, membershipGeneration, maxBytes)
    {
        Literal = literal;
        Replacement = replacement;
        MaxMatches = maxMatches;
        MaxWork = maxWork;
        Documents = documents;
    }

    [JsonPropertyName("literal")]
    public string Literal { get; }

    [JsonPropertyName("replacement")]
    public string Replacement { get; }

    [JsonPropertyName("max_matches")]
    public int MaxMatches { get; }

    [JsonPropertyName("max_work")]
    public int MaxWork { get; }

    [JsonPropertyName("documents")]
    [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
    public IReadOnlyList<string>? Documents { get; }
}

public sealed record PlanCitationRenamePayload : PlanPayloadBase
{
    public PlanCitationRenamePayload(
        IReadOnlyDictionary<string, ulong> sourceVersions, ulong membershipGeneration, int maxBytes,
        string oldName, string newName)
        : base(sourceVersions, membershipGeneration, maxBytes)
    {
        OldName = oldName;
        NewName = newName;
    }

    [JsonPropertyName("old_name")]
    public string OldName { get; }

    [JsonPropertyName("new_name")]
    public string NewName { get; }
}

public sealed record PlanCitationRenameAtPayload : PlanPayloadBase
{
    public PlanCitationRenameAtPayload(
        IReadOnlyDictionary<string, ulong> sourceVersions, ulong membershipGeneration, int maxBytes,
        string path, int startByte, int endByte, string newName)
        : base(sourceVersions, membershipGeneration, maxBytes)
    {
        Path = path;
        StartByte = startByte;
        EndByte = endByte;
        NewName = newName;
    }

    [JsonPropertyName("path")]
    public string Path { get; }

    [JsonPropertyName("start_byte")]
    public int StartByte { get; }

    [JsonPropertyName("end_byte")]
    public int EndByte { get; }

    [JsonPropertyName("new_name")]
    public string NewName { get; }
}

public sealed record ConfigureDisplayCandidatesPayload(
    [property: JsonPropertyName("capability")] string Capability,
    [property: JsonPropertyName("enabled")] bool Enabled,
    [property: JsonPropertyName("renderer_support_confirmed")]
    [property: JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)] bool? RendererSupportConfirmed = null);

public sealed record ConfigureCompletedSnapshotsPayload(
    [property: JsonPropertyName("capability")] string Capability,
    [property: JsonPropertyName("enabled")] bool Enabled);

// MARK: reply payloads

public sealed record DocumentResult([property: JsonPropertyName("document")] PreviewDocument Document);

public sealed record EditResult(
    [property: JsonPropertyName("document")] PreviewDocument Document,
    [property: JsonPropertyName("compile_request_id")] string? CompileRequestId,
    [property: JsonPropertyName("compile_revision")] ulong? CompileRevision,
    [property: JsonPropertyName("preview_error")] string? PreviewError,
    [property: JsonPropertyName("save_and_submit_ms")] double SaveAndSubmitMs,
    [property: JsonPropertyName("response_mode")] string? ResponseMode = null);

/// <summary><c>compile</c>/<c>restart</c>/<c>configure_layout</c> share this trivial acknowledgement shape.</summary>
public sealed record SubmittedResult([property: JsonPropertyName("submitted")] bool Submitted);

public sealed record ClosedResult([property: JsonPropertyName("closed")] bool Closed);

public sealed record ReviewResult(
    [property: JsonPropertyName("approval_token")] string ApprovalToken,
    [property: JsonPropertyName("edit")] CaptureEdit Edit,
    [property: JsonPropertyName("requires_explicit_user_approval")] bool RequiresExplicitUserApproval);

public sealed record ApplyReviewedResult(
    [property: JsonPropertyName("receipt")] CaptureApplied Receipt,
    [property: JsonPropertyName("document")] PreviewDocument Document,
    [property: JsonPropertyName("preview_error")] string? PreviewError);

public sealed record RetiredResult([property: JsonPropertyName("retired")] bool Retired);

public sealed record RecoveryResult([property: JsonPropertyName("transactions")] IReadOnlyList<CaptureApplied> Transactions);

public sealed record ConfirmedResult([property: JsonPropertyName("confirmed")] bool Confirmed);

public sealed record SnapshotResult(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("membership_generation")] ulong MembershipGeneration,
    [property: JsonPropertyName("document_kinds")] IReadOnlyDictionary<string, string> DocumentKinds);

public sealed record CompletionItem(
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("definitions")] IReadOnlyList<SourceSpan> Definitions,
    [property: JsonPropertyName("occurrences")] IReadOnlyList<SourceSpan> Occurrences,
    [property: JsonPropertyName("locations_truncated")] bool LocationsTruncated);

public sealed record CompleteResult(
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("completions")] IReadOnlyList<CompletionItem> Completions);

public sealed record NavigationHit(
    [property: JsonPropertyName("origin")] SourceSpan Origin,
    [property: JsonPropertyName("name")] string Name,
    [property: JsonPropertyName("definitions")] IReadOnlyList<SourceSpan> Definitions,
    [property: JsonPropertyName("definitions_truncated")] bool DefinitionsTruncated);

public sealed record NavigateResult(
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("navigation")] NavigationHit? Navigation);

public sealed record HistoryLimits(
    [property: JsonPropertyName("history_bytes")] int HistoryBytes,
    [property: JsonPropertyName("history_entries")] int HistoryEntries,
    [property: JsonPropertyName("permanent_command_ids")] int PermanentCommandIds);

public sealed record HistoryStatusDocument(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] ulong Revision,
    [property: JsonPropertyName("source_sha256")] string SourceSha256);

public sealed record HistoryStatusReply(
    [property: JsonPropertyName("history")] EditLedgerV1.HistoryStatusResult History,
    [property: JsonPropertyName("document")] HistoryStatusDocument Document,
    [property: JsonPropertyName("limits")] HistoryLimits Limits);

public sealed record PreviewHistoryResult(
    [property: JsonPropertyName("document")] PreviewDocument Document,
    [property: JsonPropertyName("command_revision")] ulong CommandRevision,
    [property: JsonPropertyName("replayed_command")] bool ReplayedCommand,
    [property: JsonPropertyName("can_undo")] bool CanUndo,
    [property: JsonPropertyName("can_redo")] bool CanRedo);

/// <summary><c>apply_group</c>/<c>undo</c>/<c>redo</c> reply; <see cref="CompileRequestId"/>/<see cref="CompileRevision"/> are only ever set for <c>apply_group</c>.</summary>
public sealed record ApplyHistoryResult(
    [property: JsonPropertyName("history")] PreviewHistoryResult History,
    [property: JsonPropertyName("preview_error")] string? PreviewError,
    [property: JsonPropertyName("save_and_submit_ms")] double SaveAndSubmitMs,
    [property: JsonPropertyName("response_mode")] string? ResponseMode = null,
    [property: JsonPropertyName("compile_request_id")] string? CompileRequestId = null,
    [property: JsonPropertyName("compile_revision")] ulong? CompileRevision = null);

/// <summary>
/// <c>file_status</c>'s <c>disk</c> field, internally tagged on <c>state</c>:
/// <c>matches_source</c>/<c>differs_from_source</c>/<c>missing</c>/<c>unavailable</c>
/// (crates/preview-controller/src/file_project.rs's <c>DiskState</c>).
/// </summary>
[JsonConverter(typeof(FileDiskStateJsonConverter))]
public abstract record FileDiskState
{
    private FileDiskState()
    {
    }

    public sealed record MatchesSource(string Sha256) : FileDiskState;

    public sealed record DiffersFromSource(string DiskSha256, string SourceSha256) : FileDiskState;

    public sealed record Missing : FileDiskState;

    public sealed record Unavailable(string Reason) : FileDiskState;
}

/// <summary>Reads/writes <see cref="FileDiskState"/>'s <c>state</c>-tagged wire shape.</summary>
public sealed class FileDiskStateJsonConverter : JsonConverter<FileDiskState>
{
    public override FileDiskState Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using JsonDocument document = JsonDocument.ParseValue(ref reader);
        JsonElement root = document.RootElement;
        string state = root.TryGetProperty("state", out JsonElement stateElement)
            ? stateElement.GetString() ?? throw new JsonException("disk state 'state' must be a string")
            : throw new JsonException("disk state missing 'state'");
        return state switch
        {
            "matches_source" => new FileDiskState.MatchesSource(root.GetProperty("sha256").GetString()!),
            "differs_from_source" => new FileDiskState.DiffersFromSource(
                root.GetProperty("disk_sha256").GetString()!, root.GetProperty("source_sha256").GetString()!),
            "missing" => new FileDiskState.Missing(),
            "unavailable" => new FileDiskState.Unavailable(root.GetProperty("reason").GetString()!),
            _ => throw new JsonException($"unknown disk state '{state}'"),
        };
    }

    public override void Write(Utf8JsonWriter writer, FileDiskState value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        switch (value)
        {
            case FileDiskState.MatchesSource matches:
                writer.WriteString("state", "matches_source");
                writer.WriteString("sha256", matches.Sha256);
                break;
            case FileDiskState.DiffersFromSource differs:
                writer.WriteString("state", "differs_from_source");
                writer.WriteString("disk_sha256", differs.DiskSha256);
                writer.WriteString("source_sha256", differs.SourceSha256);
                break;
            case FileDiskState.Missing:
                writer.WriteString("state", "missing");
                break;
            case FileDiskState.Unavailable unavailable:
                writer.WriteString("state", "unavailable");
                writer.WriteString("reason", unavailable.Reason);
                break;
            default:
                throw new JsonException($"unhandled FileDiskState case {value.GetType()}");
        }
        writer.WriteEndObject();
    }
}

public sealed record FileStatusResult(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("disk")] FileDiskState Disk,
    [property: JsonPropertyName("discovery_diagnostics")] JsonElement DiscoveryDiagnostics,
    [property: JsonPropertyName("export_available")] bool ExportAvailable);

public sealed record ExportResult(
    [property: JsonPropertyName("exported")] bool Exported,
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("sha256")] string Sha256,
    [property: JsonPropertyName("bytes")] int Bytes);

public sealed record ReloadResult(
    [property: JsonPropertyName("document")] PreviewDocument Document,
    [property: JsonPropertyName("preview_error")] string? PreviewError,
    [property: JsonPropertyName("save_and_submit_ms")] double SaveAndSubmitMs);

public sealed record MembershipResult(
    [property: JsonPropertyName("document")] PreviewDocument? Document,
    [property: JsonPropertyName("preview_error")] string? PreviewError,
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("membership_generation")] ulong MembershipGeneration);

public sealed record ProjectStatusDocument(
    [property: JsonPropertyName("path")] string Path,
    [property: JsonPropertyName("revision")] ulong Revision,
    [property: JsonPropertyName("sha256")] string Sha256,
    [property: JsonPropertyName("bytes")] int Bytes);

public sealed record ProjectStatusResult(
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("membership_generation")] ulong MembershipGeneration,
    [property: JsonPropertyName("documents")] IReadOnlyList<ProjectStatusDocument> Documents,
    [property: JsonPropertyName("total_documents")] int TotalDocuments,
    [property: JsonPropertyName("truncated")] bool Truncated,
    [property: JsonPropertyName("scope")] string Scope,
    [property: JsonPropertyName("disk_tree_enumerated")] bool DiskTreeEnumerated);

/// <summary><c>search_literal</c>'s explicit termination reason. Never treat a partial result as exhaustive.</summary>
[JsonConverter(typeof(JsonStringEnumConverter<SearchTermination>))]
public enum SearchTermination
{
    complete,
    match_limit,
    work_limit,
    cancelled,
}

public sealed record SearchResult(
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("matches")] IReadOnlyList<SourceSpan> Matches,
    [property: JsonPropertyName("termination")] SearchTermination Termination,
    [property: JsonPropertyName("work_used")] int WorkUsed);

/// <summary>
/// Shared by <c>plan_literal_replacement</c>/<c>plan_citation_rename</c>/
/// <c>plan_citation_rename_at</c>. <see cref="Plan"/>'s inner shape differs per
/// operation (docs/source-plans.md); kept opaque and round-tripped rather than
/// reconstructed, matching this codebase's existing precedent for forward-compat
/// payload data (e.g. <c>PageItem.Unknown.RawJson</c>).
/// </summary>
public sealed record PlanResult(
    [property: JsonPropertyName("source_versions")] IReadOnlyDictionary<string, ulong> SourceVersions,
    [property: JsonPropertyName("membership_generation")] ulong MembershipGeneration,
    [property: JsonPropertyName("plan")] JsonElement Plan);

public sealed record DisplayCandidatesResult(
    [property: JsonPropertyName("capability")] string Capability,
    [property: JsonPropertyName("enabled")] bool Enabled,
    [property: JsonPropertyName("preview_error")] string? PreviewError);

public sealed record CompletedSnapshotsResult(
    [property: JsonPropertyName("capability")] string Capability,
    [property: JsonPropertyName("enabled")] bool Enabled);

// MARK: unsolicited frames (`id` null): `ready` once at startup, then `update`s

public sealed record ReadyPayload(
    [property: JsonPropertyName("compiler_error")] string? CompilerError,
    [property: JsonPropertyName("compiler_max_frame_bytes")] int CompilerMaxFrameBytes,
    [property: JsonPropertyName("helper_max_output_bytes")] int HelperMaxOutputBytes);

/// <summary>
/// One <c>update</c> frame's payload, internally tagged on <c>kind</c>. Compare
/// <c>compile_revision</c>/<c>request_id</c>/<c>source_versions</c> against the
/// current editor state before painting anything derived from these — a stale
/// update must be suppressed, never assumed superseded automatically.
/// </summary>
[JsonConverter(typeof(PreviewUpdateJsonConverter))]
public abstract record PreviewUpdate
{
    private PreviewUpdate()
    {
    }

    public sealed record Preview(
        string RequestId, ulong CompileRevision, IReadOnlyDictionary<string, ulong> SourceVersions,
        IReadOnlyList<string> MissingLayoutCapabilities, double RuntimeTotalMs, double ControllerTotalMs,
        RuntimeV1.Envelope<RuntimeV1.CompileResult> Result) : PreviewUpdate;

    public sealed record Discarded(string RequestId, ulong CompileRevision) : PreviewUpdate;

    public sealed record Superseded(string RequestId, string ByRequestId) : PreviewUpdate;

    public sealed record Stale(string RequestId, ulong CompileRevision) : PreviewUpdate;

    public sealed record Cancelled(string RequestId) : PreviewUpdate;

    public sealed record Failed(string RequestId, string Reason) : PreviewUpdate;

    /// <summary>A negotiated (<c>configure_completed_snapshots</c>) preview for a superseded revision, never the current one.</summary>
    public sealed record CompletedSnapshot(
        string ProjectId, string SessionId, IReadOnlyDictionary<string, ulong> SourceVersions, string RequestId,
        ulong CompileRevision, ulong CurrentCompileRevision, bool IsCurrent, bool SourceActionsEnabled,
        string SourceBindingToken, RuntimeV1.Envelope<RuntimeV1.CompileResult> Result) : PreviewUpdate;

    /// <summary>An untrusted, negotiated (<c>configure_display_candidates</c>) intermediate rendering payload; opaque per-renderer <see cref="DisplayList"/>.</summary>
    public sealed record DisplayCandidate(
        bool Untrusted, bool SourceActionsEnabled, string RequestId, string ProjectId, ulong CompileRevision,
        IReadOnlyDictionary<string, ulong> SourceVersions, ulong MembershipGeneration, JsonElement DisplayList) : PreviewUpdate;
}

/// <summary>Reads <see cref="PreviewUpdate"/>'s <c>kind</c>-tagged wire shape (never written; this client never emits updates).</summary>
public sealed class PreviewUpdateJsonConverter : JsonConverter<PreviewUpdate>
{
    public override PreviewUpdate Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using JsonDocument document = JsonDocument.ParseValue(ref reader);
        JsonElement root = document.RootElement;
        string kind = root.TryGetProperty("kind", out JsonElement kindElement)
            ? kindElement.GetString() ?? throw new JsonException("update 'kind' must be a string")
            : throw new JsonException("update missing 'kind'");
        return kind switch
        {
            "preview" => ReadPreview(root, options),
            "discarded" => new PreviewUpdate.Discarded(RequestId(root), root.GetProperty("compile_revision").GetUInt64()),
            "superseded" => new PreviewUpdate.Superseded(RequestId(root), root.GetProperty("by_id").GetString()!),
            "stale" => new PreviewUpdate.Stale(RequestId(root), root.GetProperty("compile_revision").GetUInt64()),
            "cancelled" => new PreviewUpdate.Cancelled(RequestId(root)),
            "failed" => new PreviewUpdate.Failed(RequestId(root), root.GetProperty("reason").GetString()!),
            "completed_snapshot" => ReadCompletedSnapshot(root, options),
            "display_candidate" => ReadDisplayCandidate(root, options),
            _ => throw new JsonException($"unknown update kind '{kind}'"),
        };
    }

    private static string RequestId(JsonElement root) => root.GetProperty("request_id").GetString()!;

    private static PreviewUpdate.Preview ReadPreview(JsonElement root, JsonSerializerOptions options) =>
        new(
            RequestId(root),
            root.GetProperty("compile_revision").GetUInt64(),
            root.GetProperty("source_versions").Deserialize<IReadOnlyDictionary<string, ulong>>(options)!,
            root.GetProperty("missing_layout_capabilities").Deserialize<IReadOnlyList<string>>(options)!,
            root.GetProperty("runtime_total_ms").GetDouble(),
            root.GetProperty("controller_total_ms").GetDouble(),
            root.GetProperty("result").Deserialize<RuntimeV1.Envelope<RuntimeV1.CompileResult>>(options)!);

    private static PreviewUpdate.CompletedSnapshot ReadCompletedSnapshot(JsonElement root, JsonSerializerOptions options) =>
        new(
            root.GetProperty("project_id").GetString()!,
            root.GetProperty("session_id").GetString()!,
            root.GetProperty("source_versions").Deserialize<IReadOnlyDictionary<string, ulong>>(options)!,
            RequestId(root),
            root.GetProperty("compile_revision").GetUInt64(),
            root.GetProperty("current_compile_revision").GetUInt64(),
            root.GetProperty("is_current").GetBoolean(),
            root.GetProperty("source_actions_enabled").GetBoolean(),
            root.GetProperty("source_binding_token").GetString()!,
            root.GetProperty("result").Deserialize<RuntimeV1.Envelope<RuntimeV1.CompileResult>>(options)!);

    private static PreviewUpdate.DisplayCandidate ReadDisplayCandidate(JsonElement root, JsonSerializerOptions options) =>
        new(
            root.GetProperty("untrusted").GetBoolean(),
            root.GetProperty("source_actions_enabled").GetBoolean(),
            RequestId(root),
            root.GetProperty("project_id").GetString()!,
            root.GetProperty("compile_revision").GetUInt64(),
            root.GetProperty("source_versions").Deserialize<IReadOnlyDictionary<string, ulong>>(options)!,
            root.GetProperty("membership_generation").GetUInt64(),
            root.GetProperty("display_list").Clone());

    public override void Write(Utf8JsonWriter writer, PreviewUpdate value, JsonSerializerOptions options) =>
        throw new NotSupportedException("this client only ever reads update frames, never writes them");
}
