// name: DocumentFilesClientTests.cs
// purpose: End-to-end test of DocumentFilesClient against the REAL
//   flashtex-project-files binary (cargo build --release from
//   crates/project-files), round-tripping save/read/status through the
//   actual Windows helper process — no mock, no fake echo script.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.ProjectFilesV1;
using Xunit;

namespace FlashTeX.ProjectFiles.Tests;

public sealed class DocumentFilesClientTests : IDisposable
{
    private readonly string _root;

    public DocumentFilesClientTests()
    {
        _root = Directory.CreateTempSubdirectory("flashtex-project-files-tests-").FullName;
    }

    public void Dispose()
    {
        try
        {
            Directory.Delete(_root, recursive: true);
        }
        catch (IOException)
        {
            // Best-effort cleanup; a lingering handle on a temp directory isn't a test failure.
        }
    }

    private static void AssertBinaryExists()
    {
        Assert.True(
            File.Exists(TestPaths.ProjectFilesExePath),
            $"expected a release build of flashtex-project-files at '{TestPaths.ProjectFilesExePath}' " +
            "(run `cargo build --release` from crates/project-files first)");
    }

    private DocumentFilesClient StartClient(List<string> stderrLines) =>
        DocumentFilesClient.Start(TestPaths.ProjectFilesExePath, _root, stderrLines.Add);

    [Fact]
    public async Task PingAsync_AgainstRealBinary_ReportsTheBoundRoot()
    {
        AssertBinaryExists();
        var stderrLines = new List<string>();
        await using DocumentFilesClient client = StartClient(stderrLines);

        PingPayload ping = await client.PingAsync().WaitAsync(TimeSpan.FromSeconds(10));

        Assert.Equal(ProjectFilesV1Protocol.ProtocolName, ping.Protocol);
        Assert.True(ping.Pid > 0);
        Assert.Equal(
            new DirectoryInfo(_root).FullName.TrimEnd('\\'),
            new DirectoryInfo(ping.Root).FullName.TrimEnd('\\'));
    }

    [Fact]
    public async Task ReadAsync_OfAMissingFile_ReportsExistsFalseNotAnError()
    {
        AssertBinaryExists();
        var stderrLines = new List<string>();
        await using DocumentFilesClient client = StartClient(stderrLines);

        ReadPayload read = await client.ReadAsync("missing.tex").WaitAsync(TimeSpan.FromSeconds(10));

        Assert.False(read.Exists);
        Assert.Equal("missing.tex", read.Path);
        Assert.Null(read.Text);
    }

    [Fact]
    public async Task SaveAsync_ThenReadAsync_RoundTripsExactlyTheSavedText()
    {
        AssertBinaryExists();
        var stderrLines = new List<string>();
        await using DocumentFilesClient client = StartClient(stderrLines);
        const string text = "\\documentclass{article}\n\\begin{document}\nHello.\n\\end{document}\n";

        SaveOutcome saved = await client.SaveAsync("main.tex", text, Expected.NewFile).WaitAsync(TimeSpan.FromSeconds(10));

        SaveOutcome.Saved savedOutcome = Assert.IsType<SaveOutcome.Saved>(saved);
        Assert.Equal("main.tex", savedOutcome.Receipt.Path);
        Assert.Equal((long)System.Text.Encoding.UTF8.GetByteCount(text), savedOutcome.Receipt.Bytes);

        ReadPayload read = await client.ReadAsync("main.tex").WaitAsync(TimeSpan.FromSeconds(10));
        Assert.True(read.Exists);
        Assert.Equal(text, read.Text);
        Assert.Equal(savedOutcome.Receipt.Sha256, read.Sha256);
    }

    [Fact]
    public async Task StatusAsync_AfterASave_IsUnchangedForItsOwnHashAndModifiedForAStaleOne()
    {
        AssertBinaryExists();
        var stderrLines = new List<string>();
        await using DocumentFilesClient client = StartClient(stderrLines);

        SaveOutcome saved = await client.SaveAsync("notes.tex", "v1\n", Expected.NewFile).WaitAsync(TimeSpan.FromSeconds(10));
        SaveOutcome.Saved savedOutcome = Assert.IsType<SaveOutcome.Saved>(saved);

        StatusPayload unchanged = await client.StatusAsync("notes.tex", savedOutcome.Receipt.Sha256).WaitAsync(TimeSpan.FromSeconds(10));
        Assert.Equal(DiskState.unchanged, unchanged.State);

        StatusPayload modified = await client.StatusAsync("notes.tex", new string('0', 64)).WaitAsync(TimeSpan.FromSeconds(10));
        Assert.Equal(DiskState.modified, modified.State);
    }

    [Fact]
    public async Task SaveAsync_WhenTheFileChangedExternally_ReportsAConflictInsteadOfOverwriting()
    {
        AssertBinaryExists();
        var stderrLines = new List<string>();
        await using DocumentFilesClient client = StartClient(stderrLines);

        SaveOutcome first = await client.SaveAsync("shared.tex", "original\n", Expected.NewFile).WaitAsync(TimeSpan.FromSeconds(10));
        SaveOutcome.Saved firstSaved = Assert.IsType<SaveOutcome.Saved>(first);

        // An out-of-contract writer changes the file directly on disk, bypassing the helper.
        string diskPath = Path.Combine(_root, "shared.tex");
        await File.WriteAllTextAsync(diskPath, "changed on disk\n");

        SaveOutcome conflicted = await client
            .SaveAsync("shared.tex", "editor's version\n", Expected.Hash(firstSaved.Receipt.Sha256))
            .WaitAsync(TimeSpan.FromSeconds(10));

        SaveOutcome.Conflict conflict = Assert.IsType<SaveOutcome.Conflict>(conflicted);
        Assert.Equal(ConflictKind.modified_externally, conflict.Details.Kind);
        Assert.Equal("changed on disk\n", await File.ReadAllTextAsync(diskPath));
    }

    [Fact]
    public async Task PingAsync_RunTwiceOnTheSameClient_CorrelatesEachReplyToItsOwnRequest()
    {
        AssertBinaryExists();
        var stderrLines = new List<string>();
        await using DocumentFilesClient client = StartClient(stderrLines);

        Task<PingPayload> first = client.PingAsync();
        Task<PingPayload> second = client.PingAsync();
        PingPayload[] results = await Task.WhenAll(first, second).WaitAsync(TimeSpan.FromSeconds(10));

        Assert.All(results, r => Assert.Equal(ProjectFilesV1Protocol.ProtocolName, r.Protocol));
    }

    [Fact]
    public async Task ReadAsync_OfAPathOutsideTheRoot_IsRefusedNotFollowed()
    {
        AssertBinaryExists();
        var stderrLines = new List<string>();
        await using DocumentFilesClient client = StartClient(stderrLines);

        ProjectFilesErrorException error = await Assert.ThrowsAsync<ProjectFilesErrorException>(
            () => client.ReadAsync("../outside.tex").WaitAsync(TimeSpan.FromSeconds(10)));

        Assert.Equal("invalid_path", error.Code);
    }
}
