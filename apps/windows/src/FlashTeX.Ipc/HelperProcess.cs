// name: HelperProcess.cs
// purpose: Generic newline-delimited-JSON (JSON Lines) child-process transport
//   shared by every FlashTeX.Ipc client (WorkerClient, BridgeClient,
//   PreviewControllerClient, EditLedgerClient). Owns process lifetime, stdio
//   framing, request/reply correlation by the wire's top-level `id` field, and
//   a side channel for unsolicited frames (an absent/null `id`). Hard-fails
//   (kills the child) on an oversized line or a reader-side JSON error rather
//   than silently degrading, per every protocol doc under docs/contracts/.
// author: Claude Sonnet 5
// date: 2026-09-13
// modified: 2026-09-14 (Claude Opus 5) — correlated SIBLING replies. The
//   negotiated `display-list-v2` frame arrives as a SECOND stdout line carrying
//   the SAME top-level `id` as the `compile_result` that precedes it (verified
//   against the real crates/render-pipeline producer, and see
//   docs/contracts/runtime-v1-display-list-v2.md "Negotiation and complete reply
//   transaction"). Before this change such a line raced: it either landed on an
//   already-completed TaskCompletionSource and was dropped, or slipped into
//   `UnsolicitedLines` after `SendAsync`'s `finally` unregistered the id, with
//   the winner depending on thread timing. `SendAsync`'s sibling overload now
//   keeps the id registered until the caller-declared number of siblings has
//   been drained, so a promised sibling is never lost and a late/duplicate one
//   can never be mistaken for the next request's.

using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using System.Threading.Channels;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Ipc;

/// <summary>Configuration for launching one helper child process.</summary>
public sealed class HelperProcessOptions
{
    public required string ExecutablePath { get; init; }

    public IReadOnlyList<string> Arguments { get; init; } = Array.Empty<string>();

    public string? WorkingDirectory { get; init; }

    /// <summary>Lines longer than this (in UTF-8 bytes, excluding the newline) are a fatal framing error.</summary>
    public int MaxLineBytes { get; init; } = JsonLinesProtocol.MaxLineBytes;
}

/// <summary>Thrown when the helper's stdio stream violates the JSON Lines framing contract.</summary>
public sealed class HelperProtocolException(string message, Exception? inner = null)
    : Exception(message, inner);

/// <summary>Thrown when the helper process exits (or was never started) while a caller awaits a reply.</summary>
public sealed class HelperProcessExitedException(int? exitCode)
    : Exception($"helper process exited{(exitCode is { } code ? $" with code {code}" : string.Empty)} before replying")
{
    public int? ExitCode { get; } = exitCode;
}

/// <summary>One decoded stdout line, kept alongside the exact bytes it was framed from.</summary>
/// <remarks>
/// The raw bytes matter for the rendering-v2 sibling: its consumer
/// (<c>RenderingV2Protocol.Decode</c>) validates the envelope from UTF-8 bytes, and
/// re-serializing a <see cref="System.Text.Json.JsonDocument"/> to get them back would
/// both cost a second copy of a multi-megabyte frame and risk numeric reserialization
/// changing what was validated.
/// </remarks>
public sealed record HelperLine(JsonDocument Document, ReadOnlyMemory<byte> Bytes);

/// <summary>
/// A completed request: the reply line, plus the sibling lines that followed it under the
/// same <c>id</c> (see <see cref="HelperProcess.SendAsync{TRequest}(string, TRequest, JsonSerializerOptions, Func{JsonDocument, int}?, TimeSpan, CancellationToken)"/>).
/// <see cref="Siblings"/> may be SHORTER than the caller asked for when the helper never
/// wrote one: a missing promised sibling is reported to the caller, never fabricated, and
/// never fails the reply that already arrived.
/// </summary>
public sealed record HelperExchange(HelperLine Reply, IReadOnlyList<HelperLine> Siblings);

/// <summary>
/// One running helper child process speaking JSON Lines over stdio. A line
/// carrying a non-null top-level <c>id</c> completes the matching pending
/// <see cref="SendAsync"/> call (or becomes one of its siblings, if the caller declared it
/// expects any); a line with a null or absent <c>id</c> is published to
/// <see cref="UnsolicitedLines"/> instead (e.g. preview-controller's <c>update</c> frames).
/// </summary>
public sealed class HelperProcess : IAsyncDisposable
{
    /// <summary>
    /// How long <see cref="SendAsync{TRequest}(string, TRequest, JsonSerializerOptions, Func{JsonDocument, int}?, TimeSpan, CancellationToken)"/>
    /// waits for a promised sibling line after its reply arrived. The producer writes the
    /// sibling "immediately after that result and before a later request's reply", so this
    /// only bounds a helper that promised one and then failed to write it.
    /// </summary>
    public static readonly TimeSpan DefaultSiblingTimeout = TimeSpan.FromSeconds(15);

    private readonly Process _process;
    private readonly HelperProcessOptions _options;
    private readonly Action<string> _onStderrLine;
    private readonly LineSplitter _splitter = new();
    private readonly ConcurrentDictionary<string, PendingRequest> _pending = new();
    private readonly Channel<JsonDocument> _unsolicited = Channel.CreateUnbounded<JsonDocument>();

    /// <summary>One outstanding request id: its reply, then any sibling lines sharing that id.</summary>
    private sealed class PendingRequest
    {
        public TaskCompletionSource<HelperLine> Reply { get; } = new(TaskCreationOptions.RunContinuationsAsynchronously);

        public Channel<HelperLine> Siblings { get; } = Channel.CreateUnbounded<HelperLine>();

        /// <summary>Routes one same-id line: the first completes the reply, any later one is a sibling.</summary>
        public void Accept(HelperLine line)
        {
            if (!Reply.TrySetResult(line))
            {
                Siblings.Writer.TryWrite(line);
            }
        }

        public void Fault(Exception exception)
        {
            Reply.TrySetException(exception);
            Siblings.Writer.TryComplete(exception);
        }
    }
    private readonly CancellationTokenSource _lifetime = new();
    private readonly SemaphoreSlim _writeGate = new(1, 1);
    private readonly Task _readLoop;
    private readonly Task _stderrLoop;
    private int _faulted;

    private HelperProcess(Process process, HelperProcessOptions options, Action<string> onStderrLine)
    {
        _process = process;
        _options = options;
        _onStderrLine = onStderrLine;
        _readLoop = Task.Run(() => ReadLoopAsync(_lifetime.Token));
        _stderrLoop = Task.Run(() => StderrLoopAsync(_lifetime.Token));
    }

    /// <summary>Launches <paramref name="options"/>' executable with redirected UTF-8 stdio.</summary>
    /// <param name="onStderrLine">
    /// Called (off the caller's thread) for every line the helper writes to stderr;
    /// the project logger should be wired in here at WARN, never silently discarded.
    /// </param>
    public static HelperProcess Start(HelperProcessOptions options, Action<string> onStderrLine)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = options.ExecutablePath,
            WorkingDirectory = options.WorkingDirectory ?? string.Empty,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
            StandardOutputEncoding = new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: false),
            StandardErrorEncoding = new System.Text.UTF8Encoding(encoderShouldEmitUTF8Identifier: false),
        };
        foreach (string arg in options.Arguments)
        {
            startInfo.ArgumentList.Add(arg);
        }

        var process = new Process { StartInfo = startInfo, EnableRaisingEvents = true };
        if (!process.Start())
        {
            throw new HelperProtocolException($"failed to start helper process '{options.ExecutablePath}'");
        }

        return new HelperProcess(process, options, onStderrLine);
    }

    /// <summary>Frames sent by the helper with a null/absent <c>id</c> (e.g. preview-controller <c>update</c> frames).</summary>
    public ChannelReader<JsonDocument> UnsolicitedLines => _unsolicited.Reader;

    public bool HasExited => _process.HasExited;

    /// <summary>
    /// Encodes <paramref name="request"/> as one JSON line, writes it to the helper's
    /// stdin, and awaits the reply line whose top-level <c>id</c> equals <paramref name="id"/>.
    /// </summary>
    public async Task<JsonDocument> SendAsync<TRequest>(
        string id,
        TRequest request,
        JsonSerializerOptions jsonOptions,
        CancellationToken cancellationToken = default)
    {
        HelperExchange exchange = await SendAsync(id, request, jsonOptions, siblingCount: null, DefaultSiblingTimeout, cancellationToken)
            .ConfigureAwait(false);
        return exchange.Reply.Document;
    }

    /// <summary>
    /// Encodes <paramref name="request"/> as one JSON line, writes it to the helper's
    /// stdin, awaits the reply line whose top-level <c>id</c> equals <paramref name="id"/>,
    /// and then drains the further same-id lines that reply promises.
    /// </summary>
    /// <param name="siblingCount">
    /// Inspects the reply and returns how many sibling lines the helper committed to write
    /// for it (0 for none). Only the caller can answer this: for runtime-v1 it means
    /// reading back the <c>layout_capabilities</c> the worker actually ACCEPTED, since a
    /// declined or unrequested capability produces no sibling. Null means "never expect one".
    /// </param>
    /// <param name="siblingTimeout">
    /// Bound on waiting for a promised-but-unwritten sibling. On expiry the reply is still
    /// returned, with fewer <see cref="HelperExchange.Siblings"/> than requested, so a v1
    /// result is never discarded because its optional v2 frame went missing.
    /// </param>
    public async Task<HelperExchange> SendAsync<TRequest>(
        string id,
        TRequest request,
        JsonSerializerOptions jsonOptions,
        Func<JsonDocument, int>? siblingCount,
        TimeSpan siblingTimeout,
        CancellationToken cancellationToken = default)
    {
        ThrowIfFaulted();
        var pending = new PendingRequest();
        if (!_pending.TryAdd(id, pending))
        {
            throw new InvalidOperationException($"a request with id '{id}' is already pending");
        }

        try
        {
            byte[] line = JsonLines.EncodeLine(request, jsonOptions);
            if (line.Length - 1 > _options.MaxLineBytes)
            {
                throw new HelperProtocolException(
                    $"outgoing line for id '{id}' is {line.Length - 1} bytes, exceeding the {_options.MaxLineBytes}-byte limit");
            }

            await _writeGate.WaitAsync(cancellationToken).ConfigureAwait(false);
            try
            {
                await _process.StandardInput.BaseStream.WriteAsync(line, cancellationToken).ConfigureAwait(false);
                await _process.StandardInput.BaseStream.FlushAsync(cancellationToken).ConfigureAwait(false);
            }
            finally
            {
                _writeGate.Release();
            }

            using CancellationTokenRegistration exitRegistration = _lifetime.Token.Register(
                static state => ((PendingRequest)state!).Fault(new HelperProcessExitedException(null)),
                pending);
            HelperLine reply = await pending.Reply.Task.WaitAsync(cancellationToken).ConfigureAwait(false);
            int expected = siblingCount?.Invoke(reply.Document) ?? 0;
            IReadOnlyList<HelperLine> siblings = expected <= 0
                ? Array.Empty<HelperLine>()
                : await DrainSiblingsAsync(pending, expected, siblingTimeout, cancellationToken).ConfigureAwait(false);
            return new HelperExchange(reply, siblings);
        }
        finally
        {
            _pending.TryRemove(id, out _);
        }
    }

    /// <summary>
    /// Reads up to <paramref name="expected"/> sibling lines, giving up after
    /// <paramref name="siblingTimeout"/>. Returns however many actually arrived: the caller
    /// decides what a short result means (for display-list-v2 it means "no v2 frame this
    /// revision", not "the compile failed").
    /// </summary>
    private static async Task<IReadOnlyList<HelperLine>> DrainSiblingsAsync(
        PendingRequest pending,
        int expected,
        TimeSpan siblingTimeout,
        CancellationToken cancellationToken)
    {
        var siblings = new List<HelperLine>(expected);
        using var timeout = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeout.CancelAfter(siblingTimeout);
        try
        {
            while (siblings.Count < expected)
            {
                siblings.Add(await pending.Siblings.Reader.ReadAsync(timeout.Token).ConfigureAwait(false));
            }
        }
        catch (OperationCanceledException) when (!cancellationToken.IsCancellationRequested)
        {
            // Promised sibling never written within the bound; report what arrived.
        }
        catch (ChannelClosedException)
        {
            // The helper exited mid-exchange; the reply we already hold is still valid.
        }
        return siblings;
    }

    private async Task ReadLoopAsync(CancellationToken cancellationToken)
    {
        try
        {
            Stream stdout = _process.StandardOutput.BaseStream;
            byte[] buffer = new byte[64 * 1024];
            while (!cancellationToken.IsCancellationRequested)
            {
                int read = await stdout.ReadAsync(buffer, cancellationToken).ConfigureAwait(false);
                if (read == 0)
                {
                    break; // helper closed stdout (exited or crashed)
                }

                foreach (byte[] line in _splitter.Append(buffer.AsSpan(0, read)))
                {
                    if (line.Length > _options.MaxLineBytes)
                    {
                        throw new HelperProtocolException(
                            $"incoming line is {line.Length} bytes, exceeding the {_options.MaxLineBytes}-byte limit");
                    }

                    DispatchLine(line);
                }
            }

            FaultAllPending(new HelperProcessExitedException(_process.HasExited ? _process.ExitCode : null));
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            Volatile.Write(ref _faulted, 1);
            FaultAllPending(ex);
        }
    }

    private void DispatchLine(byte[] line)
    {
        JsonDocument document;
        string? id;
        try
        {
            document = JsonDocument.Parse(line);
            id = document.RootElement.TryGetProperty("id", out JsonElement idElement)
                && idElement.ValueKind == JsonValueKind.String
                    ? idElement.GetString()
                    : null;
        }
        catch (JsonException ex)
        {
            throw new HelperProtocolException("helper wrote a line that is not valid JSON", ex);
        }

        if (id is not null && _pending.TryGetValue(id, out PendingRequest? pending))
        {
            pending.Accept(new HelperLine(document, line));
        }
        else
        {
            // Unbounded channel: TryWrite never fails except after Complete(), which
            // only happens from DisposeAsync, by which point nobody is reading anyway.
            _unsolicited.Writer.TryWrite(document);
        }
    }

    private async Task StderrLoopAsync(CancellationToken cancellationToken)
    {
        try
        {
            using var reader = new StreamReader(_process.StandardError.BaseStream, System.Text.Encoding.UTF8);
            while (await reader.ReadLineAsync(cancellationToken).ConfigureAwait(false) is { } line)
            {
                _onStderrLine(line);
            }
        }
        catch (OperationCanceledException)
        {
            // Disposal in progress; nothing left to drain.
        }
    }

    private void FaultAllPending(Exception exception)
    {
        foreach (string id in _pending.Keys.ToArray())
        {
            if (_pending.TryRemove(id, out PendingRequest? pending))
            {
                pending.Fault(exception);
            }
        }
    }

    private void ThrowIfFaulted()
    {
        if (Volatile.Read(ref _faulted) != 0)
        {
            throw new HelperProtocolException("this helper process has already faulted and cannot accept new requests");
        }
    }

    /// <summary>Kills the child process (if still running) and releases all readers.</summary>
    public async ValueTask DisposeAsync()
    {
        await _lifetime.CancelAsync().ConfigureAwait(false);
        _unsolicited.Writer.TryComplete();
        FaultAllPending(new ObjectDisposedException(nameof(HelperProcess)));

        try
        {
            if (!_process.HasExited)
            {
                _process.Kill(entireProcessTree: true);
            }
        }
        catch (InvalidOperationException)
        {
            // Already exited between the check and the kill; nothing to do.
        }

        try
        {
            await Task.WhenAll(_readLoop, _stderrLoop).ConfigureAwait(false);
        }
        catch (OperationCanceledException)
        {
            // Expected: cancelling `_lifetime` above unblocks the loops' pending
            // reads by design. Any *other* failure was already surfaced to
            // callers via FaultAllPending/the unsolicited channel completion.
        }

        _process.Dispose();
        _writeGate.Dispose();
        _lifetime.Dispose();
    }
}
