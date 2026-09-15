// name: EditLedgerClient.cs
// purpose: Client for the flashtex-edit-ledger durable-undo-history helper
//   (crates/edit-ledger/README.md's "Private JSON Lines helper" section) — the
//   durable per-document edit ledger, undo/redo history and checkpoint archive.
//   Every request is `{id, operation, ...fields}` (not runtime-v1/transfer-v1's
//   `{protocol_version,id,type,payload}` envelope) and every reply is one
//   `ServiceReply<TPayload>` carrying session_id/sequence/document_revision
//   alongside its payload or error, per crates/edit-ledger/src/service.rs. Built
//   on the shared FlashTeX.Ipc.HelperProcess transport, following the pattern of
//   WorkerClient/BridgeClient.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text;
using System.Text.Json;
using FlashTeX.Protocol;
using FlashTeX.Protocol.EditLedgerV1;
using FlashTeX.Protocol.TransferV1;

namespace FlashTeX.Ipc;

/// <summary>Thrown when a command durably failed and the ledger replied with <c>error {code, message}</c>.</summary>
public sealed class EditLedgerErrorException(string requestId, string code, string message)
    : Exception($"flashtex-edit-ledger returned error '{code}' for request '{requestId}': {message}")
{
    public string RequestId { get; } = requestId;
    public string Code { get; } = code;
}

/// <summary>
/// Thrown when a command actually succeeded and durably advanced the document
/// (per <c>command_succeeded</c>/<c>document_revision</c>) but its reply payload
/// was omitted for exceeding the helper's reply-size cap (<c>reply_too_large</c>,
/// crates/edit-ledger/README.md's "Background service adapter" section). Callers
/// must not treat this as a rollback: re-read durable state (e.g. <see cref="EditLedgerClient.StatusAsync"/>)
/// instead of retrying the original command.
/// </summary>
public sealed class EditLedgerReplyTooLargeException(string requestId, ulong? documentRevision, string? documentSha256)
    : Exception($"flashtex-edit-ledger request '{requestId}' succeeded durably but its reply payload exceeded the size cap")
{
    public string RequestId { get; } = requestId;
    public ulong? DocumentRevision { get; } = documentRevision;
    public string? DocumentSha256 { get; } = documentSha256;
}

/// <summary>Thrown when a reply violates the service protocol itself (not a durable command failure).</summary>
public sealed class EditLedgerProtocolException(string message) : Exception(message);

/// <summary>
/// Thrown by <see cref="EditLedgerClient.AssertObservationIsCurrent"/> when a
/// caller-held <see cref="EditLedgerObservation"/> cannot be trusted against this
/// client's current session — either it names a different (superseded)
/// <c>session_id</c>, or it claims a <c>sequence</c> this client never actually
/// observed. This is the explicit guard crates/edit-ledger/README.md requires:
/// "native consumers must compare the current session identity before updating
/// UI and reject older sequence/revision observations."
/// </summary>
public sealed class EditLedgerStaleSessionException(string message) : Exception(message);

/// <summary>
/// Thrown when a reply's <c>sequence</c> does not advance past the last one this
/// client observed for the same <c>session_id</c> — a transport-consistency
/// guard against reordering or duplicate delivery within one helper session.
/// </summary>
public sealed class EditLedgerStaleReplyException(string message) : Exception(message);

/// <summary>
/// The most recently observed <c>(session_id, sequence, document_revision,
/// document_sha256)</c> from one <see cref="EditLedgerClient"/>. A restart of the
/// underlying helper process (a fresh client against the same store directory)
/// starts a new <see cref="SessionId"/>; an observation captured before that
/// restart must be re-validated with <see cref="EditLedgerClient.AssertObservationIsCurrent"/>
/// before it is trusted, never applied blindly.
/// </summary>
public readonly record struct EditLedgerObservation(
    string SessionId, ulong Sequence, ulong? DocumentRevision, string? DocumentSha256);

/// <summary>
/// One running <c>flashtex-edit-ledger --store &lt;dir&gt;</c> child process: the
/// durable per-document edit ledger, undo/redo history and checkpoint archive
/// (crates/edit-ledger/README.md). One store directory belongs to one document;
/// its parent directory must already exist. Requests on this client are meant to
/// be issued serially from one background queue (mirroring the README's "serial
/// background I/O queue" adapter contract) — the helper's own bounded admission
/// queue (default capacity 4, including unread replies) rejects and then exits
/// on an overflowing burst of concurrent requests, so callers should await each
/// request rather than fan out many at once.
/// </summary>
public sealed class EditLedgerClient : IAsyncDisposable
{
    private readonly HelperProcess _process;
    private EditLedgerObservation? _lastObservation;

    private EditLedgerClient(HelperProcess process)
    {
        _process = process;
    }

    /// <summary>Launches <paramref name="executablePath"/> (the <c>flashtex-edit-ledger</c> binary) against <paramref name="storeDirectory"/> (its parent must already exist).</summary>
    public static EditLedgerClient Start(
        string executablePath,
        string storeDirectory,
        Action<string> onStderrLine,
        string? workingDirectory = null)
    {
        var process = HelperProcess.Start(
            new HelperProcessOptions
            {
                ExecutablePath = executablePath,
                Arguments = ["--store", storeDirectory],
                WorkingDirectory = workingDirectory,
                MaxLineBytes = EditLedgerProtocol.MaxLineBytes,
            },
            onStderrLine);
        return new EditLedgerClient(process);
    }

    /// <summary>The most recently observed session/sequence/document state, or null before any reply has been received.</summary>
    public EditLedgerObservation? LastObservation => _lastObservation;

    /// <summary>
    /// Guards against trusting a stale, possibly cross-restart observation.
    /// Throws <see cref="EditLedgerStaleSessionException"/> if this client has
    /// not yet observed any reply, if <paramref name="observation"/> names a
    /// different <c>session_id</c> than this client's current session (the
    /// helper process was restarted since that observation was captured), or if
    /// it claims a <c>sequence</c> ahead of anything this client has actually
    /// observed. Callers should call this before acting on a cached observation
    /// (e.g. one saved to resume after a crash) rather than assuming continuity.
    /// </summary>
    public void AssertObservationIsCurrent(EditLedgerObservation observation)
    {
        if (_lastObservation is not { } current)
        {
            throw new EditLedgerStaleSessionException(
                "this client has not observed any reply yet; there is nothing to compare against");
        }
        if (observation.SessionId != current.SessionId)
        {
            throw new EditLedgerStaleSessionException(
                $"observation belongs to session '{observation.SessionId}', but this client's current " +
                $"session is '{current.SessionId}' (the helper process was restarted)");
        }
        if (observation.Sequence > current.Sequence)
        {
            throw new EditLedgerStaleSessionException(
                $"observation claims sequence {observation.Sequence}, ahead of the {current.Sequence} " +
                "this client has actually observed");
        }
    }

    /// <summary>Seeds a new store with its authoritative source document. Never re-initializes an existing store with different source.</summary>
    public Task<InitializeResult> InitializeAsync(string id, EditLedgerDocument document, CancellationToken cancellationToken = default) =>
        SendAsync<InitializeRequest, InitializeResult>(id, new InitializeRequest(id, document), cancellationToken);

    /// <summary>Durably applies one prepared edit. An identical retry of a known edit ID returns its original receipt without changing source.</summary>
    public Task<ApplyResult> ApplyAsync(string id, CaptureEdit edit, CancellationToken cancellationToken = default) =>
        SendAsync<ApplyRequest, ApplyResult>(id, new ApplyRequest(id, edit), cancellationToken);

    /// <summary>Ordinary typing/undo: replaces the whole document, guarded by an expected revision and source hash.</summary>
    public Task<ReplaceDocumentResult> ReplaceDocumentAsync(
        string id, ulong expectedRevision, string expectedSha256, string text, CancellationToken cancellationToken = default) =>
        SendAsync<ReplaceDocumentRequest, ReplaceDocumentResult>(
            id, new ReplaceDocumentRequest(id, expectedRevision, expectedSha256, text), cancellationToken);

    /// <summary>Acknowledges that the bridge durably received this receipt; the retained recovery snapshot for it is released.</summary>
    public Task<ConfirmResult> ConfirmAsync(string id, CaptureApplied receipt, CancellationToken cancellationToken = default) =>
        SendAsync<ConfirmRequest, ConfirmResult>(id, new ConfirmRequest(id, receipt), cancellationToken);

    /// <summary>Reads the current durable document plus every not-yet-confirmed applied transaction.</summary>
    public Task<StatusResult> StatusAsync(string id, CancellationToken cancellationToken = default) =>
        SendAsync<StatusRequest, StatusResult>(id, new StatusRequest(id), cancellationToken);

    /// <summary>Exports a fresh recovery snapshot token, the current document, and every pending (unconfirmed) receipt.</summary>
    public Task<RecoveryExport> RecoveryExportAsync(string id, CancellationToken cancellationToken = default) =>
        SendAsync<RecoveryExportRequest, RecoveryExport>(id, new RecoveryExportRequest(id), cancellationToken);

    /// <summary>Imports a batch of bridge observations for one still-current snapshot token as one atomic acknowledgement.</summary>
    public Task<RecoveryPlan> RecoveryImportAsync(string id, RecoveryImport recovery, CancellationToken cancellationToken = default) =>
        SendAsync<RecoveryImportRequest, RecoveryPlan>(id, new RecoveryImportRequest(id, recovery), cancellationToken);

    /// <summary>Explicit, acknowledged compaction of already-confirmed payloads at or below a revision cutoff. Never evicts edit/capture IDs.</summary>
    public Task<CompactionReport> CompactAsync(string id, RetentionPolicy policy, CancellationToken cancellationToken = default) =>
        SendAsync<CompactRequest, CompactionReport>(id, new CompactRequest(id, policy), cancellationToken);

    /// <summary>Applies 1-64 nonoverlapping edits against one original snapshot as one atomic transaction and undo unit.</summary>
    public Task<HistoryResult> ApplyGroupAsync(string id, GroupedEdit group, CancellationToken cancellationToken = default) =>
        SendAsync<ApplyGroupRequest, HistoryResult>(id, new ApplyGroupRequest(id, group), cancellationToken);

    /// <summary>Moves one whole history entry backward. An identical retry of a known command ID replays without moving history again.</summary>
    public Task<HistoryResult> UndoAsync(string id, HistoryMove command, CancellationToken cancellationToken = default) =>
        SendAsync<UndoRequest, HistoryResult>(id, new UndoRequest(id, command), cancellationToken);

    /// <summary>Moves one whole history entry forward. An identical retry of a known command ID replays without moving history again.</summary>
    public Task<HistoryResult> RedoAsync(string id, HistoryMove command, CancellationToken cancellationToken = default) =>
        SendAsync<RedoRequest, HistoryResult>(id, new RedoRequest(id, command), cancellationToken);

    /// <summary>Reports retained undo/redo labels, permanent command-ID count, and total history payload bytes.</summary>
    public Task<HistoryStatusResult> HistoryStatusAsync(string id, CancellationToken cancellationToken = default) =>
        SendAsync<HistoryStatusRequest, HistoryStatusResult>(id, new HistoryStatusRequest(id), cancellationToken);

    /// <summary>Discards excess undo/redo payloads only (never command IDs, capture IDs, or pending receipt snapshots).</summary>
    public Task<HistoryRetentionReport> RetainHistoryAsync(string id, HistoryRetentionPolicy policy, CancellationToken cancellationToken = default) =>
        SendAsync<RetainHistoryRequest, HistoryRetentionReport>(id, new RetainHistoryRequest(id, policy), cancellationToken);

    /// <summary>Reports checkpoint archive metadata (no source text): identity, retained generations, bytes, interrupted-write flags.</summary>
    public Task<CheckpointStatusResult> CheckpointStatusAsync(string id, CancellationToken cancellationToken = default) =>
        SendAsync<CheckpointStatusRequest, CheckpointStatusResult>(id, new CheckpointStatusRequest(id), cancellationToken);

    /// <summary>Publishes a new internal checkpoint and prunes old ones per <paramref name="policy"/>'s retention budget.</summary>
    public Task<RotationReport> CheckpointRotateAsync(
        string id, ExportAuthorization authorization, RotationPolicy policy, CancellationToken cancellationToken = default) =>
        SendAsync<CheckpointRotateRequest, RotationReport>(id, new CheckpointRotateRequest(id, authorization, policy), cancellationToken);

    /// <summary>Reads one committed checkpoint generation from the archive index.</summary>
    public Task<Checkpoint> CheckpointReadAsync(string id, ulong generation, CancellationToken cancellationToken = default) =>
        SendAsync<CheckpointReadRequest, Checkpoint>(id, new CheckpointReadRequest(id, generation), cancellationToken);

    /// <summary>Exports the full private internal checkpoint envelope (source, history, and permanent receipt/command IDs).</summary>
    public Task<Checkpoint> CheckpointExportAsync(string id, ExportAuthorization authorization, CancellationToken cancellationToken = default) =>
        SendAsync<CheckpointExportRequest, Checkpoint>(id, new CheckpointExportRequest(id, authorization), cancellationToken);

    /// <summary>Pure review step: validates a checkpoint and returns a target-directory-bound plan with any conflicts, never mutates the store.</summary>
    public Task<ImportPlan> CheckpointPlanAsync(
        string id, Checkpoint checkpoint, StoreIdentity expectedIdentity, CancellationToken cancellationToken = default) =>
        SendAsync<CheckpointPlanRequest, ImportPlan>(id, new CheckpointPlanRequest(id, checkpoint, expectedIdentity), cancellationToken);

    /// <summary>Applies an exact, previously reviewed import plan. Refused if the target/checkpoint changed since that plan was produced.</summary>
    public Task<CheckpointImportResult> CheckpointImportAsync(
        string id, Checkpoint checkpoint, ImportPlan plan, ImportAuthorization authorization, CancellationToken cancellationToken = default) =>
        SendAsync<CheckpointImportRequest, CheckpointImportResult>(
            id, new CheckpointImportRequest(id, checkpoint, plan, authorization), cancellationToken);

    /// <summary>
    /// Sends one request and decodes its <c>ServiceReply</c>, recording its
    /// session/sequence observation before interpreting <c>command_succeeded</c>/
    /// <c>error</c>/<c>payload</c>.
    /// </summary>
    private async Task<TReply> SendAsync<TRequest, TReply>(string id, TRequest request, CancellationToken cancellationToken)
    {
        ValidateId(id);
        JsonDocument document = await _process.SendAsync(id, request, FlashTeXJson.Options, cancellationToken)
            .ConfigureAwait(false);
        ServiceReply<TReply> reply = document.RootElement.Deserialize<ServiceReply<TReply>>(FlashTeXJson.Options)
            ?? throw new EditLedgerProtocolException("service reply decoded to null");

        RecordObservation(reply.SessionId, reply.Sequence, reply.DocumentRevision, reply.DocumentSha256);

        if (reply.Error is { } error)
        {
            if (reply.CommandSucceeded && error.Code == "reply_too_large")
            {
                throw new EditLedgerReplyTooLargeException(id, reply.DocumentRevision, reply.DocumentSha256);
            }
            throw new EditLedgerErrorException(id, error.Code, error.Message);
        }
        if (!reply.CommandSucceeded)
        {
            throw new EditLedgerProtocolException($"request '{id}' failed without an error payload");
        }
        return reply.Payload is { } payload
            ? payload
            : throw new EditLedgerProtocolException($"request '{id}' succeeded but its reply carried no payload");
    }

    /// <summary>
    /// The staleness guard itself: within one session, a reply's sequence must
    /// strictly advance past the last one seen for that same session. This never
    /// fires on a legitimate process restart (a fresh <see cref="EditLedgerClient"/>
    /// simply starts a new session and this baseline resets) — that cross-restart
    /// case is instead guarded explicitly via <see cref="AssertObservationIsCurrent"/>.
    /// </summary>
    private void RecordObservation(string sessionId, ulong sequence, ulong? documentRevision, string? documentSha256)
    {
        if (_lastObservation is { } previous && previous.SessionId == sessionId && sequence <= previous.Sequence)
        {
            throw new EditLedgerStaleReplyException(
                $"reply sequence {sequence} for session '{sessionId}' does not advance past the previously " +
                $"observed sequence {previous.Sequence}");
        }
        _lastObservation = new EditLedgerObservation(sessionId, sequence, documentRevision, documentSha256);
    }

    private static void ValidateId(string id)
    {
        if (id.Length == 0 || Encoding.UTF8.GetByteCount(id) > EditLedgerProtocol.MaxRequestIdBytes)
        {
            throw new ArgumentException(
                $"request id must be 1-{EditLedgerProtocol.MaxRequestIdBytes} UTF-8 bytes", nameof(id));
        }
    }

    public ValueTask DisposeAsync() => _process.DisposeAsync();
}
