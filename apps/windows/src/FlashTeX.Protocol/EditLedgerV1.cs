// name: EditLedgerV1.cs
// purpose: Wire DTOs for the private flashtex-edit-ledger JSON Lines helper
//   (crates/edit-ledger/README.md's "Private JSON Lines helper" section): the
//   durable per-document edit ledger, undo/redo history and checkpoint archive.
//   Requests are `{id, operation, ...fields}` (matching FlashTeX.Protocol.ProjectFilesV1's
//   framing, not runtime-v1/transfer-v1's `{protocol_version,id,type,payload}`
//   envelope); replies are one `ServiceReply<TPayload>` carrying
//   session_id/sequence/document_revision/document_sha256/command_succeeded
//   alongside `payload` or `error`, per crates/edit-ledger/src/service.rs's
//   `ServiceReply`/`Operation` (the authoritative source for every shape below,
//   read field-for-field, not paraphrased from the README). `PreparedEdit` and
//   `AppliedReceipt` are wire-compatible with transfer-v1 and reuse
//   FlashTeX.Protocol.TransferV1's `CaptureEdit`/`CaptureApplied` records.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;
using System.Text.Json.Serialization;
using FlashTeX.Protocol.TransferV1;

namespace FlashTeX.Protocol.EditLedgerV1;

/// <summary>Protocol size/identifier limits (crates/edit-ledger/src/service.rs, lib.rs).</summary>
public static class EditLedgerProtocol
{
    /// <summary>Each line, including its newline, is at most this many bytes in either direction (<c>MAX_FRAME_BYTES</c>).</summary>
    public const int MaxLineBytes = 12 * 1024 * 1024;

    /// <summary>A request <c>id</c> is 1-128 bytes (crates/edit-ledger/src/service.rs's <c>process_frame</c>).</summary>
    public const int MaxRequestIdBytes = 128;

    /// <summary>A single source edit's replacement is at most this many UTF-8 bytes (<c>MAX_REPLACEMENT_BYTES</c>).</summary>
    public const int MaxReplacementBytes = 64 * 1024;
}

// MARK: document and applied transactions

/// <summary>
/// The durable source document (<c>Document</c>, crates/edit-ledger/src/lib.rs).
/// <see cref="SourceSha256"/> must equal the lowercase hex SHA-256 of
/// <see cref="Text"/>'s UTF-8 bytes or the helper rejects it; use
/// <see cref="Create"/> to compute it the same way the Rust store does.
/// </summary>
public sealed record EditLedgerDocument
{
    [JsonConstructor]
    public EditLedgerDocument(string projectId, string path, ulong revision, string text, string sourceSha256)
    {
        if (!TransferV1Protocol.IsValidIdentifier(projectId))
        {
            throw new ArgumentException("project_id must be 1-128 ASCII alphanumeric/-/_ characters", nameof(projectId));
        }
        if (!TransferV1Protocol.IsValidRelativePath(path))
        {
            throw new ArgumentException("path must be a normalized relative path", nameof(path));
        }
        if (ByteOffsets.Utf8ByteCount(text) > TransferV1Protocol.MaxDocumentBytes)
        {
            throw new ArgumentException($"document text exceeds {TransferV1Protocol.MaxDocumentBytes} UTF-8 bytes", nameof(text));
        }
        ProjectId = projectId;
        Path = path;
        Revision = revision;
        Text = text;
        SourceSha256 = sourceSha256;
    }

    [JsonPropertyName("project_id")]
    public string ProjectId { get; }

    [JsonPropertyName("path")]
    public string Path { get; }

    [JsonPropertyName("revision")]
    public ulong Revision { get; }

    [JsonPropertyName("text")]
    public string Text { get; }

    [JsonPropertyName("source_sha256")]
    public string SourceSha256 { get; }

    /// <summary>Builds a document, computing <see cref="SourceSha256"/> as the Rust store does (<c>format!("{:x}", Sha256::digest(text))</c>).</summary>
    public static EditLedgerDocument Create(string projectId, string path, ulong revision, string text) =>
        new(projectId, path, revision, text, ComputeSha256(text));

    /// <summary>Lowercase hex SHA-256 of <paramref name="text"/>'s UTF-8 bytes.</summary>
    public static string ComputeSha256(string text)
    {
        byte[] hash = System.Security.Cryptography.SHA256.HashData(System.Text.Encoding.UTF8.GetBytes(text));
        return Convert.ToHexString(hash).ToLowerInvariant();
    }
}

/// <summary>
/// A durably applied, not-yet-confirmed (or since-confirmed) edit
/// (<c>AppliedTransaction</c>). <see cref="DocumentBefore"/> is retained only
/// until the bridge acknowledges the matching receipt.
/// </summary>
public sealed record AppliedTransaction(
    [property: JsonPropertyName("edit")] CaptureEdit Edit,
    [property: JsonPropertyName("receipt")] CaptureApplied Receipt,
    [property: JsonPropertyName("document_before")] EditLedgerDocument? DocumentBefore,
    [property: JsonPropertyName("document_after_sha256")] string DocumentAfterSha256,
    [property: JsonPropertyName("confirmed")] bool Confirmed);

// MARK: requests (`{id, operation, ...fields}`)

/// <summary><c>initialize</c> request: the source document to seed a new store with.</summary>
public sealed record InitializeRequest
{
    public InitializeRequest(string id, EditLedgerDocument document)
    {
        Id = id;
        Document = document;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "initialize";

    [JsonPropertyName("document")]
    public EditLedgerDocument Document { get; }
}

/// <summary><c>apply</c> request: one prepared, previously persisted edit.</summary>
public sealed record ApplyRequest
{
    public ApplyRequest(string id, CaptureEdit edit)
    {
        Id = id;
        Edit = edit;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "apply";

    [JsonPropertyName("edit")]
    public CaptureEdit Edit { get; }
}

/// <summary><c>replace_document</c> request: ordinary typing/undo, guarded by an expected revision and source hash.</summary>
public sealed record ReplaceDocumentRequest
{
    public ReplaceDocumentRequest(string id, ulong expectedRevision, string expectedSha256, string text)
    {
        Id = id;
        ExpectedRevision = expectedRevision;
        ExpectedSha256 = expectedSha256;
        Text = text;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "replace_document";

    [JsonPropertyName("expected_revision")]
    public ulong ExpectedRevision { get; }

    [JsonPropertyName("expected_sha256")]
    public string ExpectedSha256 { get; }

    [JsonPropertyName("text")]
    public string Text { get; }
}

/// <summary><c>confirm</c> request: acknowledge that the bridge durably received this receipt.</summary>
public sealed record ConfirmRequest
{
    public ConfirmRequest(string id, CaptureApplied receipt)
    {
        Id = id;
        Receipt = receipt;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "confirm";

    [JsonPropertyName("receipt")]
    public CaptureApplied Receipt { get; }
}

/// <summary><c>status</c> request: no fields beyond the envelope.</summary>
public sealed record StatusRequest
{
    public StatusRequest(string id) => Id = id;

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "status";
}

/// <summary><c>recovery_export</c> request: no fields beyond the envelope.</summary>
public sealed record RecoveryExportRequest
{
    public RecoveryExportRequest(string id) => Id = id;

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "recovery_export";
}

/// <summary><c>recovery_import</c> request: a batch of bridge observations for one still-current snapshot.</summary>
public sealed record RecoveryImportRequest
{
    public RecoveryImportRequest(string id, RecoveryImport recovery)
    {
        Id = id;
        Recovery = recovery;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "recovery_import";

    [JsonPropertyName("recovery")]
    public RecoveryImport Recovery { get; }
}

/// <summary><c>compact</c> request: explicit, acknowledged payload compaction.</summary>
public sealed record CompactRequest
{
    public CompactRequest(string id, RetentionPolicy policy)
    {
        Id = id;
        Policy = policy;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "compact";

    [JsonPropertyName("policy")]
    public RetentionPolicy Policy { get; }
}

/// <summary><c>apply_group</c> request: 1-64 nonoverlapping edits applied and undone as one command.</summary>
public sealed record ApplyGroupRequest
{
    public ApplyGroupRequest(string id, GroupedEdit group)
    {
        Id = id;
        Group = group;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "apply_group";

    [JsonPropertyName("group")]
    public GroupedEdit Group { get; }
}

/// <summary><c>undo</c> request: move one whole history entry backward.</summary>
public sealed record UndoRequest
{
    public UndoRequest(string id, HistoryMove command)
    {
        Id = id;
        Command = command;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "undo";

    [JsonPropertyName("command")]
    public HistoryMove Command { get; }
}

/// <summary><c>redo</c> request: move one whole history entry forward.</summary>
public sealed record RedoRequest
{
    public RedoRequest(string id, HistoryMove command)
    {
        Id = id;
        Command = command;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "redo";

    [JsonPropertyName("command")]
    public HistoryMove Command { get; }
}

/// <summary><c>retain_history</c> request: discard excess undo/redo payloads only.</summary>
public sealed record RetainHistoryRequest
{
    public RetainHistoryRequest(string id, HistoryRetentionPolicy policy)
    {
        Id = id;
        Policy = policy;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "retain_history";

    [JsonPropertyName("policy")]
    public HistoryRetentionPolicy Policy { get; }
}

/// <summary><c>history_status</c> request: no fields beyond the envelope.</summary>
public sealed record HistoryStatusRequest
{
    public HistoryStatusRequest(string id) => Id = id;

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "history_status";
}

/// <summary><c>checkpoint_status</c> request: no fields beyond the envelope.</summary>
public sealed record CheckpointStatusRequest
{
    public CheckpointStatusRequest(string id) => Id = id;

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "checkpoint_status";
}

/// <summary><c>checkpoint_rotate</c> request: publish a new internal checkpoint and prune old ones per policy.</summary>
public sealed record CheckpointRotateRequest
{
    public CheckpointRotateRequest(string id, ExportAuthorization authorization, RotationPolicy policy)
    {
        Id = id;
        Authorization = authorization;
        Policy = policy;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "checkpoint_rotate";

    [JsonPropertyName("authorization")]
    public ExportAuthorization Authorization { get; }

    [JsonPropertyName("policy")]
    public RotationPolicy Policy { get; }
}

/// <summary><c>checkpoint_read</c> request: read one committed checkpoint generation.</summary>
public sealed record CheckpointReadRequest
{
    public CheckpointReadRequest(string id, ulong generation)
    {
        Id = id;
        Generation = generation;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "checkpoint_read";

    [JsonPropertyName("generation")]
    public ulong Generation { get; }
}

/// <summary><c>checkpoint_export</c> request: export the full private internal checkpoint envelope.</summary>
public sealed record CheckpointExportRequest
{
    public CheckpointExportRequest(string id, ExportAuthorization authorization)
    {
        Id = id;
        Authorization = authorization;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "checkpoint_export";

    [JsonPropertyName("authorization")]
    public ExportAuthorization Authorization { get; }
}

/// <summary><c>checkpoint_plan</c> request: pure review step before an import may be approved.</summary>
public sealed record CheckpointPlanRequest
{
    public CheckpointPlanRequest(string id, Checkpoint checkpoint, StoreIdentity expectedIdentity)
    {
        Id = id;
        Checkpoint = checkpoint;
        ExpectedIdentity = expectedIdentity;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "checkpoint_plan";

    [JsonPropertyName("checkpoint")]
    public Checkpoint Checkpoint { get; }

    [JsonPropertyName("expected_identity")]
    public StoreIdentity ExpectedIdentity { get; }
}

/// <summary><c>checkpoint_import</c> request: apply an exact, previously reviewed import plan.</summary>
public sealed record CheckpointImportRequest
{
    public CheckpointImportRequest(string id, Checkpoint checkpoint, ImportPlan plan, ImportAuthorization authorization)
    {
        Id = id;
        Checkpoint = checkpoint;
        Plan = plan;
        Authorization = authorization;
    }

    [JsonPropertyName("id")]
    public string Id { get; }

    [JsonPropertyName("operation")]
    public string Operation => "checkpoint_import";

    [JsonPropertyName("checkpoint")]
    public Checkpoint Checkpoint { get; }

    [JsonPropertyName("plan")]
    public ImportPlan Plan { get; }

    [JsonPropertyName("authorization")]
    public ImportAuthorization Authorization { get; }
}

// MARK: reply payloads

public sealed record InitializeResult([property: JsonPropertyName("document")] EditLedgerDocument Document);

public sealed record ApplyResult(
    [property: JsonPropertyName("receipt")] CaptureApplied Receipt,
    [property: JsonPropertyName("document")] EditLedgerDocument Document);

public sealed record ReplaceDocumentResult([property: JsonPropertyName("document")] EditLedgerDocument Document);

public sealed record ConfirmResult([property: JsonPropertyName("confirmed")] CaptureApplied Confirmed);

public sealed record StatusResult(
    [property: JsonPropertyName("document")] EditLedgerDocument? Document,
    [property: JsonPropertyName("pending_receipts")] IReadOnlyList<AppliedTransaction> PendingReceipts);

public sealed record CheckpointImportResult([property: JsonPropertyName("document")] EditLedgerDocument Document);

// MARK: recovery (ledger-local; never adds transfer-v1 fields)

public sealed record RecoveryExport(
    [property: JsonPropertyName("snapshot_token")] string SnapshotToken,
    [property: JsonPropertyName("current_document")] EditLedgerDocument CurrentDocument,
    [property: JsonPropertyName("pending_receipts")] IReadOnlyList<AppliedTransaction> PendingReceipts);

/// <summary>The <c>recovery_import</c> request's <c>recovery</c> field: one still-current snapshot plus bridge observations.</summary>
public sealed record RecoveryImport(
    [property: JsonPropertyName("snapshot_token")] string SnapshotToken,
    [property: JsonPropertyName("observations")] IReadOnlyList<BridgeObservation> Observations);

/// <summary>
/// One bridge <c>capture_status</c> observation fed back into <c>recovery_import</c>
/// (internally tagged on <c>status</c>: <c>applied</c>/<c>prepared</c>/<c>unavailable</c>).
/// </summary>
[JsonConverter(typeof(BridgeObservationJsonConverter))]
public abstract record BridgeObservation
{
    private BridgeObservation()
    {
    }

    public sealed record Applied(CaptureApplied Receipt) : BridgeObservation;

    public sealed record Prepared(CaptureEdit Edit) : BridgeObservation;

    public sealed record Unavailable(string CaptureId, string Reason) : BridgeObservation;
}

/// <summary>Reads/writes <see cref="BridgeObservation"/>'s <c>status</c>-tagged wire shape.</summary>
public sealed class BridgeObservationJsonConverter : JsonConverter<BridgeObservation>
{
    public override BridgeObservation Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        JsonElement root = document.RootElement;
        string status = root.TryGetProperty("status", out JsonElement statusElement)
            ? statusElement.GetString() ?? throw new JsonException("bridge observation 'status' must be a string")
            : throw new JsonException("bridge observation missing 'status'");
        return status switch
        {
            "applied" => new BridgeObservation.Applied(root.GetProperty("receipt").Deserialize<CaptureApplied>(options)!),
            "prepared" => new BridgeObservation.Prepared(root.GetProperty("edit").Deserialize<CaptureEdit>(options)!),
            "unavailable" => new BridgeObservation.Unavailable(
                root.GetProperty("capture_id").GetString() ?? throw new JsonException("'capture_id' must be a string"),
                root.GetProperty("reason").GetString() ?? throw new JsonException("'reason' must be a string")),
            _ => throw new JsonException($"unknown bridge observation status '{status}'"),
        };
    }

    public override void Write(Utf8JsonWriter writer, BridgeObservation value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        switch (value)
        {
            case BridgeObservation.Applied applied:
                writer.WriteString("status", "applied");
                writer.WritePropertyName("receipt");
                JsonSerializer.Serialize(writer, applied.Receipt, options);
                break;
            case BridgeObservation.Prepared prepared:
                writer.WriteString("status", "prepared");
                writer.WritePropertyName("edit");
                JsonSerializer.Serialize(writer, prepared.Edit, options);
                break;
            case BridgeObservation.Unavailable unavailable:
                writer.WriteString("status", "unavailable");
                writer.WriteString("capture_id", unavailable.CaptureId);
                writer.WriteString("reason", unavailable.Reason);
                break;
            default:
                throw new JsonException($"unhandled BridgeObservation case {value.GetType()}");
        }
        writer.WriteEndObject();
    }
}

/// <summary>
/// A durable outcome of one imported observation (internally tagged on
/// <c>action</c>: <c>confirmed</c>/<c>replay_receipt</c>/<c>retry_status</c>).
/// </summary>
[JsonConverter(typeof(RecoveryActionJsonConverter))]
public abstract record RecoveryAction
{
    private RecoveryAction()
    {
    }

    public sealed record Confirmed(CaptureApplied Receipt) : RecoveryAction;

    /// <summary>Reopen this original source on the bridge, replay the receipt, then restore the current durable source.</summary>
    public sealed record ReplayReceipt(EditLedgerDocument DocumentBefore, CaptureApplied Receipt) : RecoveryAction;

    public sealed record RetryStatus(string CaptureId, string Reason) : RecoveryAction;
}

/// <summary>Reads/writes <see cref="RecoveryAction"/>'s <c>action</c>-tagged wire shape.</summary>
public sealed class RecoveryActionJsonConverter : JsonConverter<RecoveryAction>
{
    public override RecoveryAction Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        using var document = JsonDocument.ParseValue(ref reader);
        JsonElement root = document.RootElement;
        string action = root.TryGetProperty("action", out JsonElement actionElement)
            ? actionElement.GetString() ?? throw new JsonException("recovery action 'action' must be a string")
            : throw new JsonException("recovery action missing 'action'");
        return action switch
        {
            "confirmed" => new RecoveryAction.Confirmed(root.GetProperty("receipt").Deserialize<CaptureApplied>(options)!),
            "replay_receipt" => new RecoveryAction.ReplayReceipt(
                root.GetProperty("document_before").Deserialize<EditLedgerDocument>(options)!,
                root.GetProperty("receipt").Deserialize<CaptureApplied>(options)!),
            "retry_status" => new RecoveryAction.RetryStatus(
                root.GetProperty("capture_id").GetString() ?? throw new JsonException("'capture_id' must be a string"),
                root.GetProperty("reason").GetString() ?? throw new JsonException("'reason' must be a string")),
            _ => throw new JsonException($"unknown recovery action '{action}'"),
        };
    }

    public override void Write(Utf8JsonWriter writer, RecoveryAction value, JsonSerializerOptions options)
    {
        writer.WriteStartObject();
        switch (value)
        {
            case RecoveryAction.Confirmed confirmed:
                writer.WriteString("action", "confirmed");
                writer.WritePropertyName("receipt");
                JsonSerializer.Serialize(writer, confirmed.Receipt, options);
                break;
            case RecoveryAction.ReplayReceipt replay:
                writer.WriteString("action", "replay_receipt");
                writer.WritePropertyName("document_before");
                JsonSerializer.Serialize(writer, replay.DocumentBefore, options);
                writer.WritePropertyName("receipt");
                JsonSerializer.Serialize(writer, replay.Receipt, options);
                break;
            case RecoveryAction.RetryStatus retry:
                writer.WriteString("action", "retry_status");
                writer.WriteString("capture_id", retry.CaptureId);
                writer.WriteString("reason", retry.Reason);
                break;
            default:
                throw new JsonException($"unhandled RecoveryAction case {value.GetType()}");
        }
        writer.WriteEndObject();
    }
}

public sealed record RecoveryPlan(
    [property: JsonPropertyName("actions")] IReadOnlyList<RecoveryAction> Actions,
    [property: JsonPropertyName("recovery")] RecoveryExport Recovery);

// MARK: retention/compaction

public sealed record RetentionPolicy(
    [property: JsonPropertyName("snapshot_token")] string SnapshotToken,
    [property: JsonPropertyName("acknowledge_permanent_id_retention")] bool AcknowledgePermanentIdRetention,
    [property: JsonPropertyName("acknowledged_through_revision")] ulong AcknowledgedThroughRevision,
    [property: JsonPropertyName("keep_latest_confirmed")] int KeepLatestConfirmed);

public sealed record CompactionReport(
    [property: JsonPropertyName("compacted_payloads")] int CompactedPayloads,
    [property: JsonPropertyName("permanent_edit_ids")] int PermanentEditIds,
    [property: JsonPropertyName("pending_receipts")] int PendingReceipts,
    [property: JsonPropertyName("bytes_before")] int BytesBefore,
    [property: JsonPropertyName("bytes_after")] int BytesAfter,
    [property: JsonPropertyName("snapshot_token")] string SnapshotToken);

// MARK: grouped editing and undo/redo history

/// <summary>One edit inside an <see cref="GroupedEdit"/>, against the group's shared original source snapshot.</summary>
public sealed record SourceEdit
{
    [JsonConstructor]
    public SourceEdit(int startByte, int endByte, string removedText, string replacement)
    {
        if (startByte < 0 || endByte < startByte)
        {
            throw new ArgumentException("start_byte/end_byte must form a nonnegative, ordered range");
        }
        if (ByteOffsets.Utf8ByteCount(replacement) > EditLedgerProtocol.MaxReplacementBytes)
        {
            throw new ArgumentException($"replacement exceeds {EditLedgerProtocol.MaxReplacementBytes} UTF-8 bytes", nameof(replacement));
        }
        StartByte = startByte;
        EndByte = endByte;
        RemovedText = removedText;
        Replacement = replacement;
    }

    [JsonPropertyName("start_byte")]
    public int StartByte { get; }

    [JsonPropertyName("end_byte")]
    public int EndByte { get; }

    [JsonPropertyName("removed_text")]
    public string RemovedText { get; }

    [JsonPropertyName("replacement")]
    public string Replacement { get; }
}

/// <summary>A durable, atomic multi-edit command and undo unit (<c>apply_group</c>'s <c>group</c> field).</summary>
public sealed record GroupedEdit
{
    public const int MinEdits = 1;
    public const int MaxEdits = 64;
    public const int MaxLabelBytes = 256;

    [JsonConstructor]
    public GroupedEdit(string commandId, ulong expectedRevision, string expectedSha256, string label, IReadOnlyList<SourceEdit> edits)
    {
        if (edits.Count is < MinEdits or > MaxEdits)
        {
            throw new ArgumentException($"a group needs {MinEdits}-{MaxEdits} edits", nameof(edits));
        }
        if (ByteOffsets.Utf8ByteCount(label) > MaxLabelBytes)
        {
            throw new ArgumentException($"label exceeds {MaxLabelBytes} UTF-8 bytes", nameof(label));
        }
        CommandId = commandId;
        ExpectedRevision = expectedRevision;
        ExpectedSha256 = expectedSha256;
        Label = label;
        Edits = edits;
    }

    [JsonPropertyName("command_id")]
    public string CommandId { get; }

    [JsonPropertyName("expected_revision")]
    public ulong ExpectedRevision { get; }

    [JsonPropertyName("expected_sha256")]
    public string ExpectedSha256 { get; }

    [JsonPropertyName("label")]
    public string Label { get; }

    [JsonPropertyName("edits")]
    public IReadOnlyList<SourceEdit> Edits { get; }
}

/// <summary>An <c>undo</c>/<c>redo</c> request's <c>command</c> field: a fresh command ID plus an exact revision/hash guard.</summary>
public sealed record HistoryMove(
    [property: JsonPropertyName("command_id")] string CommandId,
    [property: JsonPropertyName("expected_revision")] ulong ExpectedRevision,
    [property: JsonPropertyName("expected_sha256")] string ExpectedSha256);

/// <summary>Common <c>apply_group</c>/<c>undo</c>/<c>redo</c> reply shape.</summary>
public sealed record HistoryResult(
    [property: JsonPropertyName("document")] EditLedgerDocument Document,
    [property: JsonPropertyName("command_revision")] ulong CommandRevision,
    [property: JsonPropertyName("replayed_command")] bool ReplayedCommand,
    [property: JsonPropertyName("can_undo")] bool CanUndo,
    [property: JsonPropertyName("can_redo")] bool CanRedo);

public sealed record HistoryRetentionPolicy(
    [property: JsonPropertyName("snapshot_token")] string SnapshotToken,
    [property: JsonPropertyName("acknowledge_undo_redo_loss")] bool AcknowledgeUndoRedoLoss,
    [property: JsonPropertyName("keep_latest_undo")] int KeepLatestUndo,
    [property: JsonPropertyName("keep_latest_redo")] int KeepLatestRedo);

public sealed record HistoryRetentionReport(
    [property: JsonPropertyName("dropped_payloads")] int DroppedPayloads,
    [property: JsonPropertyName("retained_undo")] int RetainedUndo,
    [property: JsonPropertyName("retained_redo")] int RetainedRedo,
    [property: JsonPropertyName("permanent_command_ids")] int PermanentCommandIds,
    [property: JsonPropertyName("history_bytes")] int HistoryBytes);

/// <summary><c>history_status</c> reply payload.</summary>
public sealed record HistoryStatusResult(
    [property: JsonPropertyName("undo_labels")] IReadOnlyList<string> UndoLabels,
    [property: JsonPropertyName("redo_labels")] IReadOnlyList<string> RedoLabels,
    [property: JsonPropertyName("permanent_command_ids")] int PermanentCommandIds,
    [property: JsonPropertyName("history_bytes")] int HistoryBytes);

// MARK: checkpoint archive and internal export/import

public sealed record StoreIdentity(
    [property: JsonPropertyName("store_id")] string StoreId,
    [property: JsonPropertyName("project_id")] string ProjectId,
    [property: JsonPropertyName("path")] string Path);

public sealed record CheckpointInfo(
    [property: JsonPropertyName("generation")] ulong Generation,
    [property: JsonPropertyName("file_name")] string FileName,
    [property: JsonPropertyName("checkpoint_digest")] string CheckpointDigest,
    [property: JsonPropertyName("source_revision")] ulong SourceRevision,
    [property: JsonPropertyName("bytes")] ulong Bytes);

public sealed record ExportAuthorization(
    [property: JsonPropertyName("acknowledge_private_source_export")] bool AcknowledgePrivateSourceExport);

/// <summary><c>checkpoint_rotate</c>'s <c>policy</c> field (crates/edit-ledger/src/checkpoint/archive.rs).</summary>
public sealed record RotationPolicy
{
    public const int MinKeepLatest = 1;
    public const int MaxKeepLatest = 32;

    /// <summary><c>MAX_ARCHIVE_BYTES</c>: the retained archive's hard byte ceiling.</summary>
    public const ulong MaxArchiveBytes = 512UL * 1024 * 1024;

    [JsonConstructor]
    public RotationPolicy(bool acknowledgeCheckpointDeletion, int keepLatest, ulong maxTotalBytes)
    {
        if (keepLatest is < MinKeepLatest or > MaxKeepLatest)
        {
            throw new ArgumentException($"keep_latest must be {MinKeepLatest}-{MaxKeepLatest}", nameof(keepLatest));
        }
        if (maxTotalBytes is 0 || maxTotalBytes > MaxArchiveBytes)
        {
            throw new ArgumentException($"max_total_bytes must be 1-{MaxArchiveBytes}", nameof(maxTotalBytes));
        }
        AcknowledgeCheckpointDeletion = acknowledgeCheckpointDeletion;
        KeepLatest = keepLatest;
        MaxTotalBytes = maxTotalBytes;
    }

    [JsonPropertyName("acknowledge_checkpoint_deletion")]
    public bool AcknowledgeCheckpointDeletion { get; }

    [JsonPropertyName("keep_latest")]
    public int KeepLatest { get; }

    [JsonPropertyName("max_total_bytes")]
    public ulong MaxTotalBytes { get; }
}

public sealed record RotationReport(
    [property: JsonPropertyName("created")] CheckpointInfo Created,
    [property: JsonPropertyName("retained")] IReadOnlyList<CheckpointInfo> Retained,
    [property: JsonPropertyName("removed_files")] IReadOnlyList<string> RemovedFiles);

/// <summary><c>checkpoint_status</c> reply payload: metadata only, no source text.</summary>
public sealed record CheckpointStatusResult(
    [property: JsonPropertyName("identity")] StoreIdentity Identity,
    [property: JsonPropertyName("current_revision")] ulong CurrentRevision,
    [property: JsonPropertyName("current_source_sha256")] string CurrentSourceSha256,
    [property: JsonPropertyName("checkpoints")] IReadOnlyList<CheckpointInfo> Checkpoints,
    [property: JsonPropertyName("retained_bytes")] ulong RetainedBytes,
    [property: JsonPropertyName("unindexed_generations")] IReadOnlyList<ulong> UnindexedGenerations,
    [property: JsonPropertyName("interrupted_write")] bool InterruptedWrite);

/// <summary>
/// A version-1 internal backup envelope (<c>checkpoint_export</c>/<c>checkpoint_read</c>'s
/// payload; <c>checkpoint_plan</c>/<c>checkpoint_import</c>'s input). <see cref="State"/>
/// is the Rust store's private internal state — this client treats it as an opaque
/// blob to round-trip untouched, never to construct or interpret itself.
/// </summary>
public sealed record Checkpoint(
    [property: JsonPropertyName("format_version")] byte FormatVersion,
    [property: JsonPropertyName("created_unix_ms")] ulong CreatedUnixMs,
    [property: JsonPropertyName("identity")] StoreIdentity Identity,
    [property: JsonPropertyName("integrity_sha256")] string IntegritySha256,
    [property: JsonPropertyName("state")] JsonElement State);

/// <summary>What restoring a reviewed <see cref="ImportPlan"/> would do to the target store.</summary>
[JsonConverter(typeof(ImportActionJsonConverter))]
public enum ImportAction
{
    InitializeEmpty,
    SameSourceMetadata,
    NoOp,
    Blocked,
}

/// <summary>Reads/writes <see cref="ImportAction"/> as the Rust enum's <c>rename_all = "snake_case"</c> wire strings.</summary>
public sealed class ImportActionJsonConverter : JsonConverter<ImportAction>
{
    public override ImportAction Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) =>
        reader.GetString() switch
        {
            "initialize_empty" => ImportAction.InitializeEmpty,
            "same_source_metadata" => ImportAction.SameSourceMetadata,
            "no_op" => ImportAction.NoOp,
            "blocked" => ImportAction.Blocked,
            var other => throw new JsonException($"unknown import action '{other}'"),
        };

    public override void Write(Utf8JsonWriter writer, ImportAction value, JsonSerializerOptions options) =>
        writer.WriteStringValue(value switch
        {
            ImportAction.InitializeEmpty => "initialize_empty",
            ImportAction.SameSourceMetadata => "same_source_metadata",
            ImportAction.NoOp => "no_op",
            ImportAction.Blocked => "blocked",
            _ => throw new JsonException($"unhandled import action {value}"),
        });
}

/// <summary><c>checkpoint_plan</c> reply payload: a target-directory-bound, reviewed (never yet applied) import plan.</summary>
public sealed record ImportPlan(
    [property: JsonPropertyName("plan_id")] string PlanId,
    [property: JsonPropertyName("expected_identity")] StoreIdentity ExpectedIdentity,
    [property: JsonPropertyName("target_directory")] string TargetDirectory,
    [property: JsonPropertyName("checkpoint_digest")] string CheckpointDigest,
    [property: JsonPropertyName("current_state_digest")] string? CurrentStateDigest,
    [property: JsonPropertyName("current_document")] EditLedgerDocument? CurrentDocument,
    [property: JsonPropertyName("checkpoint_document")] EditLedgerDocument CheckpointDocument,
    [property: JsonPropertyName("action")] ImportAction Action,
    [property: JsonPropertyName("conflicts")] IReadOnlyList<string> Conflicts);

/// <summary><c>checkpoint_import</c>'s <c>authorization</c> field: explicit approval of one exact reviewed plan.</summary>
public sealed record ImportAuthorization(
    [property: JsonPropertyName("approve_plan_id")] string ApprovePlanId,
    [property: JsonPropertyName("allow_initialize_empty")] bool AllowInitializeEmpty,
    [property: JsonPropertyName("allow_same_source_metadata")] bool AllowSameSourceMetadata);

// MARK: service envelope

/// <summary>
/// Every reply's outer shape (<c>ServiceReply</c>, crates/edit-ledger/src/service.rs).
/// <see cref="SessionId"/>/<see cref="Sequence"/> must be compared against the
/// caller's last-known observation before trusting <see cref="DocumentRevision"/>
/// or <see cref="Payload"/> — see <see cref="FlashTeX.Ipc.EditLedgerClient.AssertObservationIsCurrent"/>.
/// <see cref="CommandSucceeded"/> can be <c>true</c> with <see cref="Error"/> also
/// set (code <c>reply_too_large</c>): the command durably completed, only the
/// reply payload was too large to return.
/// </summary>
public sealed record ServiceReply<TPayload>(
    [property: JsonPropertyName("session_id")] string SessionId,
    [property: JsonPropertyName("sequence")] ulong Sequence,
    [property: JsonPropertyName("id")] string? Id,
    [property: JsonPropertyName("document_revision")] ulong? DocumentRevision,
    [property: JsonPropertyName("document_sha256")] string? DocumentSha256,
    [property: JsonPropertyName("command_succeeded")] bool CommandSucceeded,
    [property: JsonPropertyName("payload")] TPayload? Payload,
    [property: JsonPropertyName("error")] TransferV1.ErrorPayload? Error);
