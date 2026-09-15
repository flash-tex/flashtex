// name: PreviewControllerClientTests.cs
// purpose: End-to-end tests of PreviewControllerClient against the REAL
//   flashtex-preview-controller binary (cargo build --release from
//   crates/preview-controller), using file-backed startup (project_root +
//   private_ledger_root) so fixtures are plain .tex files on disk rather than
//   hand-constructed edit-ledger store internals. Covers the mandatory startup
//   `ready` handshake, a compile-and-preview round trip against the real
//   flashtex-compiler, an ordinary edit's unsolicited (`id`:null) preview
//   update arriving through the async-enumerable side channel (distinct from
//   the synchronous request/reply the edit itself returns), and
//   `configure_display_candidates` actually flipping the negotiated capability.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Ipc;
using FlashTeX.Protocol.PreviewControllerV1;
using FlashTeX.Protocol.RuntimeV1;
using Xunit;

namespace FlashTeX.Ipc.Tests;

public class PreviewControllerClientTests
{
    private const string ProjectId = "preview-controller-tests";
    private const string EntryPath = "main.tex";
    private static readonly TimeSpan UpdateTimeout = TimeSpan.FromSeconds(30);

    [Fact]
    public async Task StartAsync_ThenCompile_ProducesARealCompiledPreview()
    {
        RequireBinaries();
        await using TestProject project = TestProject.Create(entryText: "Hello, preview controller!\n");
        await using PreviewControllerClient client = await StartAsync(project, TestPaths.CompilerWorkerExePath);

        Assert.Null(client.Ready.CompilerError);

        SubmittedResult submitted = await client.CompileAsync("compile-1").WaitAsync(TimeSpan.FromSeconds(30));
        Assert.True(submitted.Submitted);

        PreviewUpdate.Preview preview = await WaitForPreviewAsync(client, _ => true);
        Assert.True(preview.Result.Payload.Status is Status.ok or Status.recovered);
        Assert.Contains(
            preview.Result.Payload.Pages[0].Items,
            item => item is PageItem.OfText { Item.Text: var text } && text.Contains("Hello", StringComparison.Ordinal));
    }

    [Fact]
    public async Task EditAsync_DeliversItsCompiledResultAsAnUnsolicitedUpdateFrame()
    {
        RequireBinaries();
        await using TestProject project = TestProject.Create(entryText: "original\n");
        await using PreviewControllerClient client = await StartAsync(project, TestPaths.CompilerWorkerExePath);

        DocumentResult before = await client.DocumentAsync("doc", EntryPath).WaitAsync(TimeSpan.FromSeconds(30));

        // The edit's own synchronous reply never carries compiler output — only an
        // admission (compile_request_id/compile_revision). The actual compiled
        // preview arrives later as an `update` frame whose top-level `id` is null,
        // correlated back to this edit only via `source_versions`/`compile_revision`.
        EditResult edited = await client
            .EditAsync("edit", new EditPayload(EntryPath, before.Document.Revision, before.Document.SourceSha256, "edited unsolicited content\n"))
            .WaitAsync(TimeSpan.FromSeconds(30));
        Assert.Null(edited.PreviewError);
        Assert.Equal(2UL, edited.Document.Revision);

        PreviewUpdate.Preview preview = await WaitForPreviewAsync(
            client, p => p.SourceVersions.TryGetValue(EntryPath, out ulong revision) && revision == 2);
        Assert.Contains(
            preview.Result.Payload.Pages[0].Items,
            item => item is PageItem.OfText { Item.Text: var text } && text.Contains("edited", StringComparison.Ordinal));
    }

    [Fact]
    public async Task ConfigureDisplayCandidates_TogglesTheNegotiatedCapabilityAndRejectsUnconfirmedEnable()
    {
        RequireBinaries();
        await using TestProject project = TestProject.Create(entryText: "candidates\n");
        await using PreviewControllerClient client = await StartAsync(project, TestPaths.CompilerWorkerExePath);

        // Client-side guard: enabling without an explicit renderer-support
        // confirmation must fail synchronously, before ever reaching the wire.
        // The discard-in-a-block form (not `() => expr`) keeps this the non-async
        // `Assert.Throws<T>(Action)` overload rather than the obsolete Task one.
        Assert.Throws<ArgumentException>(() =>
        {
            _ = client.ConfigureDisplayCandidatesAsync("bad", PreviewControllerProtocol.DisplayCandidatesCapability, enabled: true);
        });

        DisplayCandidatesResult enabled = await client
            .ConfigureDisplayCandidatesAsync(
                "enable", PreviewControllerProtocol.DisplayCandidatesCapability, enabled: true, rendererSupportConfirmed: true)
            .WaitAsync(TimeSpan.FromSeconds(30));
        Assert.True(enabled.Enabled);
        Assert.Equal(PreviewControllerProtocol.DisplayCandidatesCapability, enabled.Capability);

        DisplayCandidatesResult disabled = await client
            .ConfigureDisplayCandidatesAsync("disable", PreviewControllerProtocol.DisplayCandidatesCapability, enabled: false)
            .WaitAsync(TimeSpan.FromSeconds(30));
        Assert.False(disabled.Enabled);

        // Disabled is the documented default: a subsequent compile must never
        // surface a display_candidate frame, independent of whether this build of
        // the real compiler implements the negotiated display-list-v2 payload.
        await client.CompileAsync("compile-after-disable").WaitAsync(TimeSpan.FromSeconds(30));
        List<PreviewUpdate> quiet = await CollectUpdatesAsync(client, TimeSpan.FromSeconds(2));
        Assert.DoesNotContain(quiet, update => update is PreviewUpdate.DisplayCandidate);
    }

    private static void RequireBinaries()
    {
        Assert.True(
            File.Exists(TestPaths.PreviewControllerExePath),
            $"expected a release build of flashtex-preview-controller at '{TestPaths.PreviewControllerExePath}' " +
            "(run `cargo build --release` from crates/preview-controller first)");
        Assert.True(
            File.Exists(TestPaths.CompilerWorkerExePath),
            $"expected a release build of flashtex-compiler at '{TestPaths.CompilerWorkerExePath}' " +
            "(run `cargo build --release` from crates/compiler first)");
    }

    private static async Task<PreviewControllerClient> StartAsync(TestProject project, string? compilerPath)
    {
        var config = new LaunchConfig(
            sessionId: "session-1",
            projectId: ProjectId,
            entryPath: EntryPath,
            projectRoot: project.Root.FullName,
            privateLedgerRoot: project.PrivateLedgerRoot.FullName,
            compilerPath: compilerPath);
        return await PreviewControllerClient.StartAsync(
            TestPaths.PreviewControllerExePath, config, project.ConfigDirectory.FullName, _ => { });
    }

    /// <summary>Reads updates until one matching <paramref name="predicate"/> arrives, or fails after <see cref="UpdateTimeout"/>.</summary>
    private static async Task<PreviewUpdate.Preview> WaitForPreviewAsync(
        PreviewControllerClient client, Func<PreviewUpdate.Preview, bool> predicate)
    {
        using var deadline = new CancellationTokenSource(UpdateTimeout);
        try
        {
            await foreach (PreviewUpdate update in client.Updates(deadline.Token))
            {
                if (update is PreviewUpdate.Preview preview && predicate(preview))
                {
                    return preview;
                }
            }
        }
        catch (OperationCanceledException) when (deadline.IsCancellationRequested)
        {
        }
        throw new TimeoutException($"no matching preview update arrived within {UpdateTimeout}");
    }

    /// <summary>Collects every update frame that arrives within <paramref name="window"/>, then returns (never throws on timeout).</summary>
    private static async Task<List<PreviewUpdate>> CollectUpdatesAsync(PreviewControllerClient client, TimeSpan window)
    {
        var collected = new List<PreviewUpdate>();
        using var deadline = new CancellationTokenSource(window);
        try
        {
            await foreach (PreviewUpdate update in client.Updates(deadline.Token))
            {
                collected.Add(update);
            }
        }
        catch (OperationCanceledException) when (deadline.IsCancellationRequested)
        {
        }
        return collected;
    }
}

/// <summary>A disposable trio of temp directories for one file-backed preview-controller session.</summary>
public sealed class TestProject : IAsyncDisposable
{
    public required DirectoryInfo Root { get; init; }
    public required DirectoryInfo PrivateLedgerRoot { get; init; }
    public required DirectoryInfo ConfigDirectory { get; init; }

    public static TestProject Create(string entryText)
    {
        DirectoryInfo root = Directory.CreateTempSubdirectory("flashtex-preview-controller-root-");
        File.WriteAllText(Path.Combine(root.FullName, "main.tex"), entryText);
        return new TestProject
        {
            Root = root,
            PrivateLedgerRoot = Directory.CreateTempSubdirectory("flashtex-preview-controller-ledger-"),
            ConfigDirectory = Directory.CreateTempSubdirectory("flashtex-preview-controller-config-"),
        };
    }

    public ValueTask DisposeAsync()
    {
        foreach (DirectoryInfo dir in new[] { Root, PrivateLedgerRoot, ConfigDirectory })
        {
            try
            {
                dir.Delete(recursive: true);
            }
            catch (IOException)
            {
                // Best-effort cleanup; a still-open handle from a just-killed helper
                // process must never fail the test itself.
            }
        }
        return ValueTask.CompletedTask;
    }
}
