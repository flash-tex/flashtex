// name: BridgeClient.cs
// purpose: Client for the flashtex-bridge JSON Lines helper (transfer-v1,
//   docs/contracts/transfer-v1.md) — the companion-device ("Nearby") capture
//   transfer/proposal/insertion protocol. Ported from the responsibility of
//   apps/mac/Sources/FlashTeXMac/BridgeClient.swift/BridgeSession.swift, built
//   on the shared FlashTeX.Ipc.HelperProcess transport.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text.Json;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;
using FlashTeX.Protocol.TransferV1;

namespace FlashTeX.Ipc;

/// <summary>Thrown when the bridge replies with an <c>error {code, message}</c> envelope.</summary>
public sealed class BridgeErrorException(string requestId, string code, string message)
    : Exception($"flashtex-bridge returned error '{code}' for request '{requestId}': {message}")
{
    public string RequestId { get; } = requestId;
    public string Code { get; } = code;
}

/// <summary>
/// One running <c>flashtex-bridge --store &lt;dir&gt;</c> child process. Every request
/// is a single round trip correlated by the caller-supplied <paramref name="id"/>
/// against the reply type <see cref="Request.ReplyType"/> declares for it, per
/// <c>crates/bridge/src/main.rs</c>'s own dispatch table.
/// </summary>
public sealed class BridgeClient : IAsyncDisposable
{
    private readonly HelperProcess _process;

    private BridgeClient(HelperProcess process)
    {
        _process = process;
    }

    /// <summary>Launches <paramref name="executablePath"/> (the <c>flashtex-bridge</c> binary) against <paramref name="storeDirectory"/>.</summary>
    public static BridgeClient Start(
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
                MaxLineBytes = TransferV1Protocol.MaxLineBytes,
            },
            onStderrLine);
        return new BridgeClient(process);
    }

    /// <summary>The bridge's <c>document_opened</c> reply carries no payload (per <c>crates/bridge/src/main.rs</c>).</summary>
    public Task<Empty> DocumentOpenAsync(string id, DocumentOpen request, CancellationToken cancellationToken = default) =>
        SendAsync<DocumentOpen, Empty>(Request.DocumentOpen, id, request, cancellationToken);

    public Task<DocumentUpdated> DocumentEditAsync(string id, DocumentEdit request, CancellationToken cancellationToken = default) =>
        SendAsync<DocumentEdit, DocumentUpdated>(Request.DocumentEdit, id, request, cancellationToken);

    public Task<Anchor> DestinationPinAsync(string id, DestinationPin request, CancellationToken cancellationToken = default) =>
        SendAsync<DestinationPin, Anchor>(Request.DestinationPin, id, request, cancellationToken);

    public Task<FlashTeX.Protocol.TransferV1.CaptureReceived> CaptureSubmitAsync(string id, CaptureSubmit request, CancellationToken cancellationToken = default) =>
        SendAsync<CaptureSubmit, FlashTeX.Protocol.TransferV1.CaptureReceived>(Request.CaptureSubmit, id, request, cancellationToken);

    public Task<CaptureProposal> CaptureConvertAsync(string id, CaptureConvert request, CancellationToken cancellationToken = default) =>
        SendAsync<CaptureConvert, CaptureProposal>(Request.CaptureConvert, id, request, cancellationToken);

    public Task<CaptureEdit> CapturePrepareInsertAsync(string id, CapturePrepareInsert request, CancellationToken cancellationToken = default) =>
        SendAsync<CapturePrepareInsert, CaptureEdit>(Request.CapturePrepareInsert, id, request, cancellationToken);

    public Task<CaptureApplied> CaptureAppliedAsync(string id, CaptureId request, CancellationToken cancellationToken = default) =>
        SendAsync<CaptureId, CaptureApplied>(Request.CaptureApplied, id, request, cancellationToken);

    public Task<CaptureStatus> CaptureStatusAsync(string id, CaptureId request, CancellationToken cancellationToken = default) =>
        SendAsync<CaptureId, CaptureStatus>(Request.CaptureStatus, id, request, cancellationToken);

    /// <summary>The bridge's <c>capture_rejected</c> reply echoes only the <c>capture_id</c>.</summary>
    public Task<CaptureId> CaptureRejectAsync(string id, CaptureId request, CancellationToken cancellationToken = default) =>
        SendAsync<CaptureId, CaptureId>(Request.CaptureReject, id, request, cancellationToken);

    private async Task<TReply> SendAsync<TPayload, TReply>(
        Request kind, string id, TPayload payload, CancellationToken cancellationToken)
    {
        var envelope = new Envelope<TPayload>(FlashTeX.Protocol.RuntimeV1.Protocol.Version, id, kind.WireName(), payload);
        JsonDocument reply = await _process.SendAsync(id, envelope, FlashTeXJson.Options, cancellationToken)
            .ConfigureAwait(false);

        string type = reply.RootElement.GetProperty("type").GetString()
            ?? throw new RuntimeV1DecodeException("reply envelope has a null 'type'");
        if (type == "error")
        {
            FlashTeX.Protocol.TransferV1.ErrorPayload error = reply.RootElement.GetProperty("payload")
                .Deserialize<FlashTeX.Protocol.TransferV1.ErrorPayload>(FlashTeXJson.Options)
                ?? throw new RuntimeV1DecodeException("error payload decoded to null");
            throw new BridgeErrorException(id, error.Code, error.Message);
        }

        string expectedType = kind.ReplyType();
        if (type != expectedType)
        {
            throw RuntimeV1DecodeException.UnexpectedType(expectedType, type);
        }

        return reply.RootElement.GetProperty("payload").Deserialize<TReply>(FlashTeXJson.Options)
            ?? throw new RuntimeV1DecodeException($"{expectedType} payload decoded to null");
    }

    public ValueTask DisposeAsync() => _process.DisposeAsync();
}
