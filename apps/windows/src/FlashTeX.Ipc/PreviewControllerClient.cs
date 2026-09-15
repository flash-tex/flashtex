// name: PreviewControllerClient.cs
// purpose: Client for the flashtex-preview-controller JSON Lines helper
//   (crates/preview-controller/STDIO.md) — the durable live-preview compilation
//   route: source persistence/undo history (edit-ledger), project navigation
//   (project-index, including search_literal), and compiler submission/polling.
//   The wire envelope is `{protocol_version,session_id,id,type,payload}`; every
//   synchronous reply collapses `type` to `result`/`error` regardless of
//   operation (unlike runtime-v1/transfer-v1's per-operation reply type), and
//   unsolicited frames (`id` null) are `ready` once at startup then `update`s
//   discriminated on `payload.kind`. Built on the shared FlashTeX.Ipc.HelperProcess
//   transport, following the pattern of WorkerClient/BridgeClient/EditLedgerClient.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using FlashTeX.Protocol;
using FlashTeX.Protocol.PreviewControllerV1;
using FlashTeX.Protocol.TransferV1;

namespace FlashTeX.Ipc;

/// <summary>Thrown when the helper replies with an <c>error {message}</c> envelope.</summary>
public sealed class PreviewControllerErrorException(string requestId, string message)
    : Exception($"flashtex-preview-controller returned an error for request '{requestId}': {message}")
{
    public string RequestId { get; } = requestId;
}

/// <summary>Thrown when a reply violates the protocol's own envelope shape (not a durable operation failure).</summary>
public sealed class PreviewControllerProtocolException(string message) : Exception(message);

/// <summary>
/// One running <c>flashtex-preview-controller CONFIG.json</c> child process: one
/// project's durable live-preview compilation controller. <see cref="StartAsync"/>
/// writes <paramref name="config"/>'s JSON to disk (the helper takes a config
/// *file path* as its only CLI argument, never over stdio) and awaits the
/// mandatory startup <c>ready</c> frame before returning. Every request must be
/// awaited serially per the documented bounded admission queue (16 waiting
/// operations); callers should not fan out concurrent requests unread.
/// </summary>
public sealed class PreviewControllerClient : IAsyncDisposable
{
    private readonly HelperProcess _process;
    private readonly string _sessionId;

    /// <summary>The mandatory first frame this helper ever sends, captured once at startup.</summary>
    public ReadyPayload Ready { get; }

    private PreviewControllerClient(HelperProcess process, string sessionId, ReadyPayload ready)
    {
        _process = process;
        _sessionId = sessionId;
        Ready = ready;
    }

    /// <summary>
    /// Writes <paramref name="config"/> to a fresh file under <paramref name="configDirectory"/>,
    /// launches <paramref name="executablePath"/> against it, and awaits the
    /// mandatory <c>ready</c> frame (STDIO.md: always the helper's first output line).
    /// </summary>
    public static async Task<PreviewControllerClient> StartAsync(
        string executablePath,
        LaunchConfig config,
        string configDirectory,
        Action<string> onStderrLine,
        string? workingDirectory = null,
        CancellationToken cancellationToken = default)
    {
        string configPath = Path.Combine(configDirectory, $"preview-controller-config-{Guid.NewGuid():N}.json");
        await File.WriteAllBytesAsync(
            configPath, JsonSerializer.SerializeToUtf8Bytes(config, FlashTeXJson.Options), cancellationToken)
            .ConfigureAwait(false);

        HelperProcess process = HelperProcess.Start(
            new HelperProcessOptions
            {
                ExecutablePath = executablePath,
                Arguments = [configPath],
                WorkingDirectory = workingDirectory,
                MaxLineBytes = PreviewControllerProtocol.MaxLineBytes,
            },
            onStderrLine);

        ReadyPayload ready = await ReadReadyFrameAsync(process, cancellationToken).ConfigureAwait(false);
        return new PreviewControllerClient(process, config.SessionId, ready);
    }

    /// <summary>
    /// Awaits the helper's first unsolicited frame, but never hangs forever if the
    /// process exits before writing anything at all (e.g. a malformed config file):
    /// <see cref="HelperProcess.UnsolicitedLines"/> is only completed on disposal,
    /// not on the child's own exit, so exit is polled alongside the channel wait.
    /// </summary>
    private static async Task<ReadyPayload> ReadReadyFrameAsync(HelperProcess process, CancellationToken cancellationToken)
    {
        Task<bool> waitForFrame = process.UnsolicitedLines.WaitToReadAsync(cancellationToken).AsTask();
        while (!waitForFrame.IsCompleted && !process.HasExited)
        {
            await Task.WhenAny(waitForFrame, Task.Delay(20, cancellationToken)).ConfigureAwait(false);
        }
        if (!waitForFrame.IsCompleted || !await waitForFrame.ConfigureAwait(false))
        {
            throw new HelperProcessExitedException(null);
        }
        JsonDocument frame = await process.UnsolicitedLines.ReadAsync(cancellationToken).ConfigureAwait(false);
        string type = frame.RootElement.TryGetProperty("type", out JsonElement typeElement)
            ? typeElement.GetString() ?? throw new PreviewControllerProtocolException("first frame has a null 'type'")
            : throw new PreviewControllerProtocolException("first frame is missing 'type'");
        if (type != "ready")
        {
            throw new PreviewControllerProtocolException($"expected 'ready' as the helper's first frame, got '{type}'");
        }
        return frame.RootElement.GetProperty("payload").Deserialize<ReadyPayload>(FlashTeXJson.Options)
            ?? throw new PreviewControllerProtocolException("ready payload decoded to null");
    }

    /// <summary>
    /// Every <c>update</c> frame emitted after startup, decoded to a typed
    /// <see cref="PreviewUpdate"/>. Enumerate this continuously off the UI thread;
    /// it completes only when the helper process exits.
    /// </summary>
    public async IAsyncEnumerable<PreviewUpdate> Updates([EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await foreach (JsonDocument frame in _process.UnsolicitedLines.ReadAllAsync(cancellationToken).ConfigureAwait(false))
        {
            string type = frame.RootElement.TryGetProperty("type", out JsonElement typeElement)
                ? typeElement.GetString() ?? throw new PreviewControllerProtocolException("update frame has a null 'type'")
                : throw new PreviewControllerProtocolException("update frame is missing 'type'");
            if (type != "update")
            {
                throw new PreviewControllerProtocolException($"expected an 'update' frame, got '{type}'");
            }
            yield return frame.RootElement.GetProperty("payload").Deserialize<PreviewUpdate>(FlashTeXJson.Options)
                ?? throw new PreviewControllerProtocolException("update payload decoded to null");
        }
    }

    // MARK: document/source operations

    public Task<DocumentResult> DocumentAsync(string id, string path, CancellationToken cancellationToken = default) =>
        SendAsync<PathPayload, DocumentResult>(id, "document", new PathPayload(path), cancellationToken);

    public Task<EditResult> EditAsync(string id, EditPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<EditPayload, EditResult>(id, "edit", payload, cancellationToken);

    public Task<SubmittedResult> CompileAsync(string id, string? sourceBindingToken = null, CancellationToken cancellationToken = default) =>
        SendAsync<CompilePayload, SubmittedResult>(id, "compile", new CompilePayload(sourceBindingToken), cancellationToken);

    /// <summary>Restarts the configured compiler. This also silently disables negotiated completed-snapshot bindings (STDIO.md).</summary>
    public Task<SubmittedResult> RestartAsync(string id, CancellationToken cancellationToken = default) =>
        SendAsync<Empty, SubmittedResult>(id, "restart", new Empty(), cancellationToken);

    public Task<ClosedResult> CloseAsync(string id, CancellationToken cancellationToken = default) =>
        SendAsync<Empty, ClosedResult>(id, "close", new Empty(), cancellationToken);

    // MARK: reviewed capture insertion

    public Task<ReviewResult> ReviewAsync(string id, CaptureEdit edit, CancellationToken cancellationToken = default) =>
        SendAsync<ReviewPayload, ReviewResult>(id, "review", new ReviewPayload(edit), cancellationToken);

    /// <summary>Call only from the actual user-approval handler, after displaying the exact reviewed edit (STDIO.md).</summary>
    public Task<ApplyReviewedResult> ApplyReviewedAsync(
        string id, string approvalToken, bool userApproved, CancellationToken cancellationToken = default) =>
        SendAsync<ApplyReviewedPayload, ApplyReviewedResult>(
            id, "apply_reviewed", new ApplyReviewedPayload(approvalToken, userApproved), cancellationToken);

    public Task<RetiredResult> RetireReviewAsync(string id, string approvalToken, CancellationToken cancellationToken = default) =>
        SendAsync<ApprovalTokenPayload, RetiredResult>(id, "retire_review", new ApprovalTokenPayload(approvalToken), cancellationToken);

    public Task<RecoveryResult> RecoveryAsync(string id, string path, CancellationToken cancellationToken = default) =>
        SendAsync<PathPayload, RecoveryResult>(id, "recovery", new PathPayload(path), cancellationToken);

    public Task<ConfirmedResult> ConfirmReceiptAsync(
        string id, string path, CaptureApplied receipt, CancellationToken cancellationToken = default) =>
        SendAsync<ConfirmReceiptPayload, ConfirmedResult>(
            id, "confirm_receipt", new ConfirmReceiptPayload(path, receipt), cancellationToken);

    // MARK: lexical project navigation (project-index)

    public Task<SnapshotResult> SnapshotAsync(string id, CancellationToken cancellationToken = default) =>
        SendAsync<Empty, SnapshotResult>(id, "snapshot", new Empty(), cancellationToken);

    public Task<CompleteResult> CompleteAsync(string id, CompletePayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<CompletePayload, CompleteResult>(id, "complete", payload, cancellationToken);

    public Task<NavigateResult> NavigateAsync(string id, NavigatePayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<NavigatePayload, NavigateResult>(id, "navigate", payload, cancellationToken);

    /// <summary>
    /// Case-sensitive raw literal search over exact current source (project-index's
    /// search logic; this same binary is the only route to it — <c>project-index</c>
    /// itself ships no standalone binary). See also <see cref="ProjectSearchClient"/>
    /// for a request-ID-managing convenience wrapper.
    /// </summary>
    public Task<SearchResult> SearchLiteralAsync(string id, SearchLiteralPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<SearchLiteralPayload, SearchResult>(id, "search_literal", payload, cancellationToken);

    public Task<PlanResult> PlanLiteralReplacementAsync(
        string id, PlanLiteralReplacementPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<PlanLiteralReplacementPayload, PlanResult>(id, "plan_literal_replacement", payload, cancellationToken);

    public Task<PlanResult> PlanCitationRenameAsync(
        string id, PlanCitationRenamePayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<PlanCitationRenamePayload, PlanResult>(id, "plan_citation_rename", payload, cancellationToken);

    public Task<PlanResult> PlanCitationRenameAtAsync(
        string id, PlanCitationRenameAtPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<PlanCitationRenameAtPayload, PlanResult>(id, "plan_citation_rename_at", payload, cancellationToken);

    // MARK: durable undo/redo history

    public Task<HistoryStatusReply> HistoryStatusAsync(string id, string path, CancellationToken cancellationToken = default) =>
        SendAsync<PathPayload, HistoryStatusReply>(id, "history_status", new PathPayload(path), cancellationToken);

    public Task<ApplyHistoryResult> ApplyGroupAsync(string id, ApplyGroupPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<ApplyGroupPayload, ApplyHistoryResult>(id, "apply_group", payload, cancellationToken);

    public Task<ApplyHistoryResult> UndoAsync(string id, HistoryMovePayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<HistoryMovePayload, ApplyHistoryResult>(id, "undo", payload, cancellationToken);

    public Task<ApplyHistoryResult> RedoAsync(string id, HistoryMovePayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<HistoryMovePayload, ApplyHistoryResult>(id, "redo", payload, cancellationToken);

    // MARK: negotiation

    public Task<SubmittedResult> ConfigureLayoutAsync(
        string id, ConfigureLayoutPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<ConfigureLayoutPayload, SubmittedResult>(id, "configure_layout", payload, cancellationToken);

    /// <summary>
    /// Opts into (or back out of) <c>display_candidate</c> update frames. Default
    /// startup is OFF; enabling requires an explicit renderer-support confirmation
    /// (validated client-side too, so a missing confirmation fails fast rather than
    /// round-tripping to the helper).
    /// </summary>
    public Task<DisplayCandidatesResult> ConfigureDisplayCandidatesAsync(
        string id, string capability, bool enabled, bool rendererSupportConfirmed = false, CancellationToken cancellationToken = default)
    {
        if (enabled && !rendererSupportConfirmed)
        {
            throw new ArgumentException(
                "enabling display candidates requires an explicit renderer support confirmation", nameof(rendererSupportConfirmed));
        }
        var payload = new ConfigureDisplayCandidatesPayload(capability, enabled, enabled ? true : null);
        return SendAsync<ConfigureDisplayCandidatesPayload, DisplayCandidatesResult>(id, "configure_display_candidates", payload, cancellationToken);
    }

    public Task<CompletedSnapshotsResult> ConfigureCompletedSnapshotsAsync(
        string id, bool enabled, CancellationToken cancellationToken = default) =>
        SendAsync<ConfigureCompletedSnapshotsPayload, CompletedSnapshotsResult>(
            id, "configure_completed_snapshots",
            new ConfigureCompletedSnapshotsPayload(PreviewControllerProtocol.CompletedSnapshotsCapability, enabled),
            cancellationToken);

    // MARK: file-backed project operations (project_root/private_ledger_root startup only)

    public Task<FileStatusResult> FileStatusAsync(string id, string path, CancellationToken cancellationToken = default) =>
        SendAsync<PathPayload, FileStatusResult>(id, "file_status", new PathPayload(path), cancellationToken);

    public Task<ExportResult> ExportAsync(string id, ExportPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<ExportPayload, ExportResult>(id, "export", payload, cancellationToken);

    public Task<ReloadResult> ReloadAsync(string id, ReloadPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<ReloadPayload, ReloadResult>(id, "reload", payload, cancellationToken);

    public Task<MembershipResult> OpenDocumentAsync(string id, MembershipPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<MembershipPayload, MembershipResult>(id, "open_document", payload, cancellationToken);

    public Task<MembershipResult> DetachDocumentAsync(string id, MembershipPayload payload, CancellationToken cancellationToken = default) =>
        SendAsync<MembershipPayload, MembershipResult>(id, "detach_document", payload, cancellationToken);

    public Task<ProjectStatusResult> ProjectStatusAsync(string id, int? maxDocuments = null, CancellationToken cancellationToken = default) =>
        SendAsync<ProjectStatusPayload, ProjectStatusResult>(id, "project_status", new ProjectStatusPayload(maxDocuments), cancellationToken);

    /// <summary>
    /// Sends one request and decodes its generic <c>{type:"result"|"error",payload}</c>
    /// reply — every operation shares this reply shape, unlike runtime-v1/transfer-v1's
    /// per-operation reply <c>type</c>.
    /// </summary>
    private async Task<TResult> SendAsync<TPayload, TResult>(
        string id, string type, TPayload payload, CancellationToken cancellationToken)
    {
        ValidateId(id);
        var envelope = new Envelope<TPayload>(1, _sessionId, id, type, payload);
        JsonDocument reply = await _process.SendAsync(id, envelope, FlashTeXJson.Options, cancellationToken).ConfigureAwait(false);

        string replyType = reply.RootElement.TryGetProperty("type", out JsonElement typeElement)
            ? typeElement.GetString() ?? throw new PreviewControllerProtocolException("reply has a null 'type'")
            : throw new PreviewControllerProtocolException("reply is missing 'type'");
        if (replyType == "error")
        {
            FlashTeX.Protocol.PreviewControllerV1.ErrorPayload error = reply.RootElement.GetProperty("payload")
                .Deserialize<FlashTeX.Protocol.PreviewControllerV1.ErrorPayload>(FlashTeXJson.Options)
                ?? throw new PreviewControllerProtocolException("error payload decoded to null");
            throw new PreviewControllerErrorException(id, error.Message);
        }
        if (replyType != "result")
        {
            throw new PreviewControllerProtocolException($"expected 'result' or 'error' for '{type}', got '{replyType}'");
        }
        return reply.RootElement.GetProperty("payload").Deserialize<TResult>(FlashTeXJson.Options)
            ?? throw new PreviewControllerProtocolException($"'{type}' payload decoded to null");
    }

    private static void ValidateId(string id)
    {
        if (id.Length == 0 || Encoding.UTF8.GetByteCount(id) > PreviewControllerProtocol.MaxRequestIdBytes)
        {
            throw new ArgumentException(
                $"request id must be 1-{PreviewControllerProtocol.MaxRequestIdBytes} UTF-8 bytes", nameof(id));
        }
    }

    public ValueTask DisposeAsync() => _process.DisposeAsync();
}
