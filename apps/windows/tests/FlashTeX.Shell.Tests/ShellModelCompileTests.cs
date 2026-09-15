// name: ShellModelCompileTests.cs
// purpose: Tests for ShellModel.Compile.cs against the REAL flashtex-compiler
//   binary (crates/compiler, `cargo build --release`): an edit round-trips
//   through the attached WorkerClient into applied diagnostics/word-count/
//   has-compile-result state, and rapid edits within one debounce window
//   coalesce into exactly one compile (driven deterministically with
//   ManualChromeScheduler, mirroring ShellChromeTests' convention — no real
//   sleeping). Also covers ICommandContext gating/dispatch (HasWorkerAttached,
//   HasCompileResult, PerformAction(Compile)) now that ShellModel implements it.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RuntimeV1;
using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

public class ShellModelCompileTests
{
    private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(30);

    private static void RequireCompilerBuilt() =>
        Assert.True(
            File.Exists(TestPaths.CompilerWorkerExePath),
            $"expected a release build of flashtex-compiler at '{TestPaths.CompilerWorkerExePath}' " +
            "(run `cargo build --release` from crates/compiler first)");

    [Fact]
    public async Task UpdateDocumentText_WithWorkerAttached_CompilesAndUpdatesDiagnosticsAndWordCount()
    {
        RequireCompilerBuilt();
        var model = new ShellModel(compileDebounceInterval: TimeSpan.Zero);
        model.AttachWorker(TestPaths.CompilerWorkerExePath);
        await model.OpenDocumentAsync("main.tex", "Alpha Beta Gamma.\n");

        model.UpdateDocumentText("main.tex", "Alpha Beta Gamma Delta.\n");

        Assert.NotNull(model.LastCompileTask);
        await model.LastCompileTask!.WaitAsync(Timeout);

        Assert.True(model.HasCompileResult);
        Assert.Equal(4, model.WordCount);
        Assert.Empty(model.Diagnostics.Where(d => d.Severity == Severity.error));
        Assert.Contains("revision", model.WorkerStatus, StringComparison.Ordinal);

        await model.DetachWorkerAsync();
    }

    [Fact]
    public async Task UpdateDocumentText_RapidEdits_CoalesceIntoExactlyOneDebouncedCompile()
    {
        RequireCompilerBuilt();
        var scheduler = new ManualChromeScheduler();
        var model = new ShellModel(compileScheduler: scheduler);
        model.AttachWorker(TestPaths.CompilerWorkerExePath);
        await model.OpenDocumentAsync("main.tex", "one\n");

        model.UpdateDocumentText("main.tex", "one two\n");
        model.UpdateDocumentText("main.tex", "one two three\n");
        model.UpdateDocumentText("main.tex", "one two three four\n");

        Assert.Equal(1, scheduler.PendingCount); // one scheduled compile, not three

        scheduler.Fire();
        Assert.NotNull(model.LastCompileTask);
        await model.LastCompileTask!.WaitAsync(Timeout);

        Assert.Equal(4, model.WordCount); // the newest buffer, not an intermediate one

        await model.DetachWorkerAsync();
    }

    [Fact]
    public void UpdateDocumentText_WithNoWorkerAttached_SchedulesNoCompile()
    {
        var scheduler = new ManualChromeScheduler();
        var model = new ShellModel(compileScheduler: scheduler);

        model.UpdateDocumentText("missing.tex", "text");

        Assert.Equal(0, scheduler.PendingCount);
        Assert.Null(model.LastCompileTask);
    }

    [Fact]
    public async Task CompileNowAsync_CalledWhileACompileIsInFlight_SendsTheNewestBufferOnceItReturns()
    {
        RequireCompilerBuilt();
        // A manual scheduler makes the race deterministic: everything up to the
        // compiler's real reply runs synchronously (C# async methods run
        // synchronously up to their first suspension point), so by the time
        // `scheduler.Fire()` returns, `first`'s in-flight compile has already
        // synchronously observed and recorded the queued follow-up edit.
        var scheduler = new ManualChromeScheduler();
        var model = new ShellModel(compileScheduler: scheduler);
        model.AttachWorker(TestPaths.CompilerWorkerExePath);
        await model.OpenDocumentAsync("main.tex", "first\n");

        Task first = model.CompileNowAsync();
        model.UpdateDocumentText("main.tex", "first second\n"); // schedules a debounced compile behind `first`
        Assert.Equal(1, scheduler.PendingCount);
        scheduler.Fire(); // observes `first` still in flight; only marks the newest buffer to send next
        Task? queuedFollowUp = model.LastCompileTask;

        await first.WaitAsync(Timeout);
        if (queuedFollowUp is not null)
        {
            await queuedFollowUp.WaitAsync(Timeout);
        }

        Assert.Equal(2, model.WordCount); // the newest buffer was sent, not dropped

        await model.DetachWorkerAsync();
    }

    [Fact]
    public void ICommandContext_HasWorkerAttached_ReflectsAttachState()
    {
        ICommandContext model = new ShellModel();
        Assert.False(model.HasWorkerAttached);
    }

    [Fact]
    public async Task ICommandContext_PerformActionCompile_TriggersACompile()
    {
        RequireCompilerBuilt();
        var model = new ShellModel();
        model.AttachWorker(TestPaths.CompilerWorkerExePath);
        await model.OpenDocumentAsync("main.tex", "hello\n");

        ((ICommandContext)model).PerformAction(CommandIds.Compile);

        Assert.NotNull(model.LastCompileTask);
        await model.LastCompileTask!.WaitAsync(Timeout);
        Assert.True(model.HasCompileResult);

        await model.DetachWorkerAsync();
    }
}
