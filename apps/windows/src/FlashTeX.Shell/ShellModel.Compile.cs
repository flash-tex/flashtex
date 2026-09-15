// name: ShellModel.Compile.cs
// purpose: Compile-pipeline orchestration for ShellModel: attaching a
//   WorkerClient, scheduling a debounced (or explicit "compile now") compile
//   of every open document, and applying the returned CompileResult into
//   diagnostics/word-count/has-compile-result state. Also implements
//   ICommandContext so CommandRegistry can gate and dispatch Compile/Undo/Redo
//   against real state. Adapted from the responsibility of
//   apps/mac/Sources/FlashTeXMac/ShellModel.swift's `compile()`/
//   `scheduleAutoCompile()`/`handle(_:)`, simplified: compiles are serialized
//   one at a time (the newest buffers go out the instant the in-flight one
//   returns, mirroring the Swift source's `compileQueued`) rather than
//   correlated by revision/id/capability-switch, since this port has no
//   concurrent-request race to resolve yet (no layout-capability negotiation,
//   no display-list-v2 delta). Diagnostic rebasing across edits and a full
//   Problems-panel model belong to a future editor-facing layer; this only
//   exposes the latest result's raw diagnostics plus severity counts, per the
//   task's explicit scope for this port.
// author: Claude Sonnet 5
// date: 2026-09-14

using CommunityToolkit.Mvvm.ComponentModel;
using FlashTeX.Ipc;
using FlashTeX.Protocol.RuntimeV1;
using RenderingV2 = FlashTeX.Protocol.RenderingV2;

namespace FlashTeX.Shell;

public partial class ShellModel : ICommandContext
{
    private const string CompileProjectId = "flashtex-shell";

    /// <summary>
    /// Layout capabilities every compile requests. Only <c>display-list-v2</c> is asked for:
    /// this consumer is prepared to validate that sibling (WorkerClient does the correlation
    /// checks, PreviewV2Host the rendering ones), and per the contract a capability "is
    /// requested only by a consumer prepared to validate it". A worker that does not support
    /// it declines by omission and the ordinary full v1 reply is unchanged — which is exactly
    /// what <c>flashtex-compiler</c> does, so nothing about the v1 path regresses.
    /// <c>display-list-v2-images</c> is deliberately NOT requested: this port has no verified
    /// image-decoding path yet, and requesting a capability it could not paint would be a
    /// contract violation rather than a nice-to-have.
    /// </summary>
    private static readonly IReadOnlyList<string> RequestedLayoutCapabilities = new[] { WorkerClient.DisplayListV2Capability };

    /// <summary>Default delay between an edit and the compile it triggers, when the injected scheduler is a real one.</summary>
    public static readonly TimeSpan DefaultCompileDebounceInterval = TimeSpan.FromMilliseconds(250);

    private WorkerClient? _worker;
    private bool _compileRefreshPending;
    private bool _compileInFlight;
    private bool _compileQueuedAfterInFlight;
    private int _compileRevision;

    [ObservableProperty]
    private bool _hasCompileResult;

    [ObservableProperty]
    private IReadOnlyList<Diagnostic> _diagnostics = Array.Empty<Diagnostic>();

    [ObservableProperty]
    private string _workerStatus = "no worker attached";

    /// <summary>The most recently started compile (debounced or explicit), for tests to await; null before the first one.</summary>
    public Task? LastCompileTask { get; private set; }

    /// <summary>The most recently applied compile result (successful or with diagnostics), for a later
    /// export command to reuse without re-running the compiler; null until the first result is applied.</summary>
    public CompileResult? LastCompileResult { get; private set; }

    /// <summary>
    /// The rendering-v2 display list correlated with <see cref="LastCompileResult"/>, or null
    /// when this compile produced none (the worker does not speak <c>display-list-v2</c>, it
    /// declined the capability, the result failed, or the promised sibling failed validation).
    /// Never a leftover from an earlier revision: <see cref="ApplyCompileExchange"/> assigns it —
    /// including to null — on every applied result.
    /// </summary>
    public RenderingV2.DisplayList? LastDisplayList { get; private set; }

    /// <summary>Why <see cref="LastDisplayList"/> is null, for the status bar; null when a display list is present.</summary>
    public string? DisplayListRefusal { get; private set; }

    /// <summary>
    /// Bumped once per applied compile result. Views observe THIS rather than
    /// <see cref="HasCompileResult"/> to know a new frame is available: a bool that is already
    /// true raises no change notification, so a view bound to it would only ever refresh once.
    /// </summary>
    [ObservableProperty]
    private int _renderGeneration;

    /// <summary>Diagnostics of the applied result's severity <see cref="Severity.error"/>.</summary>
    public int ErrorCount => Diagnostics.Count(d => d.Severity == Severity.error);

    /// <summary>Diagnostics of the applied result's severity <see cref="Severity.warning"/>.</summary>
    public int WarningCount => Diagnostics.Count(d => d.Severity == Severity.warning);

    /// <summary>Whether a document is open and active (<see cref="ICommandContext.HasActiveDocument"/>).</summary>
    public bool HasActiveDocument => ActiveDocument is not null;

    /// <summary>Whether a compiler worker process is attached (<see cref="ICommandContext.HasWorkerAttached"/>).</summary>
    public bool HasWorkerAttached => _worker is not null;

    /// <summary>Whether a validated v2 display list is loaded (<see cref="ICommandContext.HasDisplayList"/>).</summary>
    public bool HasDisplayList => LastDisplayList is not null;

    /// <summary>
    /// Launches <paramref name="executablePath"/> (a runtime-v1 worker binary) as this
    /// shell's compiler worker, with optional <paramref name="arguments"/> — the
    /// display-list-v2-capable <c>flashtex-render</c> needs <c>--font-dir</c>.
    /// </summary>
    public void AttachWorker(string executablePath, Action<string>? onStderrLine = null, IReadOnlyList<string>? arguments = null)
    {
        _worker = WorkerClient.Start(executablePath, onStderrLine ?? (_ => { }), workingDirectory: null, arguments);
        WorkerStatus = "attached: " + System.IO.Path.GetFileName(executablePath);
        OnPropertyChanged(nameof(HasWorkerAttached));
    }

    /// <summary>Detaches the current worker (if any), releasing its child process. Safe to call with none attached.</summary>
    public async Task DetachWorkerAsync()
    {
        if (_worker is not { } worker)
        {
            return;
        }
        _worker = null;
        await worker.DisposeAsync().ConfigureAwait(true);
        WorkerStatus = "no worker attached";
        OnPropertyChanged(nameof(HasWorkerAttached));
    }

    /// <summary>
    /// Runs the command identified by <paramref name="commandId"/> (<see cref="ICommandContext.PerformAction"/>).
    /// Commands this port has not wired up yet (export, navigation, zoom, ...) are no-ops
    /// here; <see cref="CommandRegistry"/> still gates which commands may run at all via
    /// each <c>Command.CanExecute</c>, so this is never reached for a genuinely disallowed one.
    /// </summary>
    public void PerformAction(string commandId)
    {
        switch (commandId)
        {
            case CommandIds.Compile:
                LastCompileTask = CompileNowAsync();
                break;
            case CommandIds.Undo:
                _ = UndoActiveDocumentAsync();
                break;
            case CommandIds.Redo:
                _ = RedoActiveDocumentAsync();
                break;
        }
    }

    /// <summary>
    /// Schedules a debounced compile of every open document. Rapid edits within one
    /// <see cref="_compileDebounceInterval"/> window coalesce into a single compile,
    /// mirroring <see cref="ShellChrome"/>'s own coalescing pattern (and, for tests,
    /// its "inject a fake scheduler" convention). A no-op with no worker attached.
    /// </summary>
    private void ScheduleAutoCompile()
    {
        if (_worker is null || _compileRefreshPending)
        {
            return;
        }
        _compileRefreshPending = true;
        _compileScheduler.Schedule(FireDebouncedCompile, _compileDebounceInterval);
    }

    private void FireDebouncedCompile()
    {
        _compileRefreshPending = false;
        LastCompileTask = CompileNowAsync();
    }

    /// <summary>
    /// Sends every open document to the attached worker and applies its reply. A call
    /// while one is already in flight only marks that the newest buffers should go out
    /// next (mirroring the Mac source's <c>compileQueued</c>) instead of sending
    /// concurrently — this shell serializes compiles rather than correlating concurrent
    /// replies by revision. A no-op with no worker attached or no documents open.
    /// </summary>
    public async Task CompileNowAsync(CancellationToken cancellationToken = default)
    {
        if (_worker is not { } worker || Documents.Count == 0)
        {
            return;
        }
        if (_compileInFlight)
        {
            _compileQueuedAfterInFlight = true;
            return;
        }

        _compileInFlight = true;
        try
        {
            do
            {
                _compileQueuedAfterInFlight = false;
                await RunOneCompileAsync(worker, cancellationToken).ConfigureAwait(true);
            }
            while (_compileQueuedAfterInFlight);
        }
        finally
        {
            _compileInFlight = false;
        }
    }

    private async Task RunOneCompileAsync(WorkerClient worker, CancellationToken cancellationToken)
    {
        string entryPath = ActiveDocumentPath ?? Documents[0].Path;
        var request = new CompileRequest(
            CompileProjectId,
            ++_compileRevision,
            entryPath,
            Documents.Select(d => d.ToProtocolDocument()).ToList(),
            layoutCapabilities: RequestedLayoutCapabilities,
            projectRoot: ProjectRoot);
        try
        {
            CompileExchange exchange = await worker
                .CompileWithDisplayListAsync($"compile-{Guid.NewGuid():N}", request, cancellationToken)
                .ConfigureAwait(true);
            ApplyCompileExchange(exchange);
        }
        catch (WorkerErrorException ex)
        {
            WorkerStatus = "compile failed: " + ex.Message;
        }
    }

    private void ApplyCompileExchange(CompileExchange exchange)
    {
        CompileResult result = exchange.Result;
        Diagnostics = result.Diagnostics;
        WordCount = CountWords(ActiveDocument?.Text ?? string.Empty);
        HasCompileResult = true;
        LastCompileResult = result;
        LastDisplayList = exchange.DisplayList;
        DisplayListRefusal = exchange.DisplayListRefusal;
        WorkerStatus = $"revision {result.Revision}: {result.Status}, {result.Diagnostics.Count} diagnostics · {DescribeDisplayList(exchange)}";
        RenderGeneration++;
    }

    /// <summary>The status-bar phrase that says, truthfully, whether this revision is painted from v2 or v1.</summary>
    private static string DescribeDisplayList(CompileExchange exchange) =>
        exchange.DisplayList is { } list
            ? $"display-list-v2: {list.Pages.Count} page(s), {list.Fonts.Count} font(s)"
            : $"runtime-v1 preview ({exchange.DisplayListRefusal ?? "no display list"})";

    private static int CountWords(string text) =>
        text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).Length;
}
