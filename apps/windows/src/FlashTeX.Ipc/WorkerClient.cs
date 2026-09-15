// name: WorkerClient.cs
// purpose: Client for the flashtex-compiler JSON Lines worker (runtime-v1,
//   docs/contracts/runtime-v1.md). Sends `compile` envelopes and decodes the
//   `compile_result` (or `error`) reply. Ported from the responsibility of
//   apps/mac/Sources/FlashTeXMac/WorkerClient.swift, built on the shared
//   FlashTeX.Ipc.HelperProcess transport.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;
using RenderingV2 = FlashTeX.Protocol.RenderingV2;

namespace FlashTeX.Ipc;

/// <summary>Thrown when the worker replies with an <c>error</c> envelope instead of <c>compile_result</c>.</summary>
public sealed class WorkerErrorException(string requestId, string message)
    : Exception($"flashtex-compiler returned an error for request '{requestId}': {message}")
{
    public string RequestId { get; } = requestId;
}

/// <summary>
/// One compile round trip: the runtime-v1 result, and the negotiated rendering-v2 display
/// list when — and only when — a correctly correlated, structurally valid sibling arrived.
/// </summary>
/// <param name="Result">The runtime-v1 <c>compile_result</c>; always present.</param>
/// <param name="DisplayList">
/// The validated display list, or null. Null is the NORMAL case for a worker that does not
/// speak <c>display-list-v2</c> (<c>flashtex-compiler</c> does not; <c>flashtex-render</c> does).
/// </param>
/// <param name="DisplayListRefusal">
/// Why there is no display list this round, in human-readable form for the status bar and
/// the handoff log; null exactly when <paramref name="DisplayList"/> is non-null.
/// </param>
public sealed record CompileExchange(
    CompileResult Result,
    RenderingV2.DisplayList? DisplayList,
    string? DisplayListRefusal);

/// <summary>
/// One running <c>flashtex-compiler</c> child process. Each <see cref="CompileAsync"/>
/// call is a single request/reply round trip correlated by the caller-supplied
/// <paramref name="id"/> (the wire's <c>id</c> field, required unique per the
/// runtime-v1 contract).
/// </summary>
public sealed class WorkerClient : IAsyncDisposable
{
    /// <summary>The layout capability whose acceptance promises one correlated <c>display_list</c> sibling line.</summary>
    public const string DisplayListV2Capability = "display-list-v2";

    private readonly HelperProcess _process;

    private WorkerClient(HelperProcess process)
    {
        _process = process;
    }

    /// <summary>
    /// Launches <paramref name="executablePath"/> (a runtime-v1 worker binary:
    /// <c>flashtex-compiler</c>, or <c>flashtex-render</c> for display-list-v2) as a worker.
    /// </summary>
    /// <param name="arguments">
    /// Command-line arguments for the worker, e.g. <c>flashtex-render</c>'s repeatable
    /// <c>--font-dir</c>. Empty for a worker that takes none.
    /// </param>
    public static WorkerClient Start(
        string executablePath,
        Action<string> onStderrLine,
        string? workingDirectory = null,
        IReadOnlyList<string>? arguments = null)
    {
        var process = HelperProcess.Start(
            new HelperProcessOptions
            {
                ExecutablePath = executablePath,
                Arguments = arguments ?? Array.Empty<string>(),
                WorkingDirectory = workingDirectory,
                MaxLineBytes = JsonLinesProtocol.MaxLineBytes,
            },
            onStderrLine);
        return new WorkerClient(process);
    }

    /// <summary>
    /// Sends a <c>compile</c> request and awaits its reply. Throws
    /// <see cref="WorkerErrorException"/> if the worker replies with an <c>error</c>
    /// envelope, or <see cref="RuntimeV1DecodeException"/> if the reply's
    /// <c>protocol_version</c>/<c>type</c> otherwise fails to match expectations.
    /// </summary>
    public async Task<CompileResult> CompileAsync(string id, CompileRequest request, CancellationToken cancellationToken = default)
    {
        var envelope = new Envelope<CompileRequest>(FlashTeX.Protocol.RuntimeV1.Protocol.Version, id, "compile", request);
        JsonDocument reply = await _process.SendAsync(id, envelope, FlashTeXJson.Options, cancellationToken)
            .ConfigureAwait(false);
        return DecodeCompileResult(id, reply);
    }

    /// <summary>
    /// Sends a <c>compile</c> request that also asks for the negotiated rendering-v2
    /// <c>display_list</c> sibling, and returns the v1 result together with the sibling —
    /// but only when that sibling passes every correlation and structural check the
    /// contract requires. The v1 <see cref="CompileResult"/> is returned either way: a
    /// missing, declined, late, duplicate or wrongly correlated sibling degrades the caller
    /// to the v1 renderer, it never fails the compile.
    /// </summary>
    public async Task<CompileExchange> CompileWithDisplayListAsync(
        string id,
        CompileRequest request,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        var envelope = new Envelope<CompileRequest>(FlashTeX.Protocol.RuntimeV1.Protocol.Version, id, "compile", request);
        HelperExchange exchange = await _process
            .SendAsync(id, envelope, FlashTeXJson.Options, ExpectedSiblingCount, HelperProcess.DefaultSiblingTimeout, cancellationToken)
            .ConfigureAwait(false);

        CompileResult result = DecodeCompileResult(id, exchange.Reply.Document);
        if (exchange.Siblings.Count == 0)
        {
            return new CompileExchange(result, null, DescribeMissingSibling(result));
        }
        if (exchange.Siblings.Count > 1)
        {
            // "Duplicate, interleaved, unsolicited, malformed or wrongly correlated
            // siblings must not become render candidates."
            return new CompileExchange(result, null, $"refused {exchange.Siblings.Count} display_list siblings for one accepted request (exactly one is promised)");
        }

        try
        {
            RenderingV2.Envelope displayList = RenderingV2.RenderingV2Protocol.Decode(exchange.Siblings[0].Bytes.Span, FlashTeXJson.Options);
            CheckSiblingCorrelation(displayList, id, request, result);
            return new CompileExchange(result, displayList.Payload, null);
        }
        catch (RenderingV2.RenderingV2ValidationException ex)
        {
            return new CompileExchange(result, null, ex.ToString());
        }
    }

    /// <summary>
    /// How many sibling lines this reply committed to. The producer writes exactly one
    /// <c>display_list</c> line for an ACCEPTED <c>display-list-v2</c> capability and a
    /// nonfailed result; a declined capability is signalled by the reply echoing an
    /// accepted list that no longer contains it, so this reads the reply rather than the
    /// request.
    /// </summary>
    private static int ExpectedSiblingCount(JsonDocument reply)
    {
        if (!reply.RootElement.TryGetProperty("payload", out JsonElement payload)
            || !payload.TryGetProperty("layout_capabilities", out JsonElement accepted)
            || accepted.ValueKind != JsonValueKind.Array)
        {
            return 0;
        }
        bool acceptedV2 = accepted.EnumerateArray()
            .Any(c => c.ValueKind == JsonValueKind.String && c.GetString() == DisplayListV2Capability);
        if (!acceptedV2)
        {
            return 0;
        }
        // "A failed result cannot promise a display sibling."
        string? status = payload.TryGetProperty("status", out JsonElement statusElement) ? statusElement.GetString() : null;
        return status == "failed" ? 0 : 1;
    }

    private static string DescribeMissingSibling(CompileResult result) =>
        result.LayoutCapabilities?.Contains(DisplayListV2Capability) == true
            ? result.Status == FlashTeX.Protocol.RuntimeV1.Status.failed
                ? "display-list-v2 was accepted but the result failed, so no sibling is promised"
                : "display-list-v2 was accepted but no display_list sibling arrived within the transport bound"
            : "the worker declined display-list-v2 (no sibling promised)";

    /// <summary>
    /// "The sibling request ID, project ID and compile revision must equal the admitted
    /// request and result. Declared document paths, byte lengths and raw UTF-8 SHA-256
    /// bindings must match their admitted source snapshot."
    /// </summary>
    private static void CheckSiblingCorrelation(RenderingV2.Envelope sibling, string requestId, CompileRequest request, CompileResult result)
    {
        RenderingV2.DisplayList list = sibling.Payload;
        if (sibling.Id != requestId)
        {
            throw Refuse($"display_list id '{sibling.Id}' does not equal the admitted request id '{requestId}'");
        }
        if (list.ProjectId != request.ProjectId || list.ProjectId != result.ProjectId)
        {
            throw Refuse($"display_list project '{list.ProjectId}' does not equal the admitted request/result project '{request.ProjectId}'/'{result.ProjectId}'");
        }
        if (list.Revision != request.Revision || list.Revision != result.Revision)
        {
            throw Refuse($"display_list revision {list.Revision} does not equal the admitted request/result revision {request.Revision}/{result.Revision}");
        }

        var admitted = request.Documents.ToDictionary(d => d.Path, StringComparer.Ordinal);
        foreach (RenderingV2.DocumentResource declared in list.Documents)
        {
            if (!admitted.TryGetValue(declared.Path, out Document? source))
            {
                throw Refuse($"display_list declares document '{declared.Path}', which is not part of the admitted source snapshot");
            }
            byte[] bytes = Encoding.UTF8.GetBytes(source.Text);
            if (declared.ByteLength != bytes.Length)
            {
                throw Refuse($"display_list document '{declared.Path}' declares {declared.ByteLength} bytes; the admitted source is {bytes.Length}");
            }
            string sha256 = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
            if (!string.Equals(declared.Sha256, sha256, StringComparison.Ordinal))
            {
                throw Refuse($"display_list document '{declared.Path}' sha256 {declared.Sha256} does not bind the admitted source bytes ({sha256})");
            }
        }
    }

    private static RenderingV2.RenderingV2ValidationException Refuse(string message) =>
        new("wrongly_correlated_sibling", message);

    private static CompileResult DecodeCompileResult(string id, JsonDocument reply)
    {
        string type = reply.RootElement.GetProperty("type").GetString()
            ?? throw new RuntimeV1DecodeException("reply envelope has a null 'type'");
        if (type == "error")
        {
            string message = reply.RootElement.TryGetProperty("payload", out JsonElement payload)
                && payload.TryGetProperty("message", out JsonElement messageElement)
                    ? messageElement.GetString() ?? "(no message)"
                    : "(no message)";
            throw new WorkerErrorException(id, message);
        }

        Envelope<CompileResult> decoded = reply.RootElement.Deserialize<Envelope<CompileResult>>(FlashTeXJson.Options)
            ?? throw new RuntimeV1DecodeException("compile_result envelope decoded to null");
        if (decoded.ProtocolVersion != FlashTeX.Protocol.RuntimeV1.Protocol.Version)
        {
            throw RuntimeV1DecodeException.UnsupportedVersion(decoded.ProtocolVersion);
        }
        if (decoded.Type != "compile_result")
        {
            throw RuntimeV1DecodeException.UnexpectedType("compile_result", decoded.Type);
        }
        return decoded.Payload;
    }

    public ValueTask DisposeAsync() => _process.DisposeAsync();
}
