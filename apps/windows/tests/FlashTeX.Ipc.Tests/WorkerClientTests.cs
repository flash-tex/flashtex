// name: WorkerClientTests.cs
// purpose: End-to-end test of WorkerClient against the REAL flashtex-compiler
//   binary (cargo build --release from crates/compiler), round-tripping the
//   shared protocol/fixtures/compile-request.json through the actual Windows
//   engine process — the strongest available proof that the runtime-v1 IPC
//   client works against the genuine Rust engine on this platform, not just a
//   stand-in echo script.
// author: Claude Sonnet 5
// date: 2026-09-13

using FlashTeX.Ipc;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;
using Xunit;

namespace FlashTeX.Ipc.Tests;

public class WorkerClientTests
{
    [Fact]
    public async Task CompileAsync_AgainstRealCompilerBinary_ProducesAPageForTheFixtureRequest()
    {
        Assert.True(
            File.Exists(TestPaths.CompilerWorkerExePath),
            $"expected a release build of flashtex-compiler at '{TestPaths.CompilerWorkerExePath}' " +
            "(run `cargo build --release` from crates/compiler first)");

        string fixtureJson = await File.ReadAllTextAsync(
            Path.Combine(TestPaths.RepoRoot, "protocol", "fixtures", "compile-request.json"));
        Envelope<CompileRequest> fixture = System.Text.Json.JsonSerializer
            .Deserialize<Envelope<CompileRequest>>(fixtureJson, FlashTeXJson.Options)
            ?? throw new InvalidOperationException("fixture decoded to null");

        var stderrLines = new List<string>();
        await using WorkerClient worker = WorkerClient.Start(
            TestPaths.CompilerWorkerExePath, line => stderrLines.Add(line));

        CompileResult result = await worker.CompileAsync(fixture.Id, fixture.Payload)
            .WaitAsync(TimeSpan.FromSeconds(30));

        Assert.True(
            result.Status is Status.ok or Status.recovered,
            $"expected ok/recovered, got {result.Status}; stderr: {string.Join('\n', stderrLines)}");
        Assert.NotEmpty(result.Pages);
        Assert.Contains(
            result.Pages[0].Items,
            item => item is PageItem.OfText { Item.Text: var text } && text.Contains("Hello", StringComparison.Ordinal));
    }

    [Fact]
    public async Task CompileAsync_RunTwiceOnTheSameWorker_CorrelatesEachReplyToItsOwnRequest()
    {
        Assert.True(File.Exists(TestPaths.CompilerWorkerExePath));

        await using WorkerClient worker = WorkerClient.Start(TestPaths.CompilerWorkerExePath, _ => { });

        var first = new CompileRequest("demo", 1, "main.tex", [new Document("main.tex", "Alpha.\n")]);
        var second = new CompileRequest("demo", 2, "main.tex", [new Document("main.tex", "Beta.\n")]);

        Task<CompileResult> firstTask = worker.CompileAsync("id-alpha", first);
        Task<CompileResult> secondTask = worker.CompileAsync("id-beta", second);
        CompileResult[] results = await Task.WhenAll(firstTask, secondTask).WaitAsync(TimeSpan.FromSeconds(30));

        Assert.Contains(results[0].Pages[0].Items, item => item is PageItem.OfText { Item.Text: var t } && t.Contains("Alpha", StringComparison.Ordinal));
        Assert.Contains(results[1].Pages[0].Items, item => item is PageItem.OfText { Item.Text: var t } && t.Contains("Beta", StringComparison.Ordinal));
    }
}
