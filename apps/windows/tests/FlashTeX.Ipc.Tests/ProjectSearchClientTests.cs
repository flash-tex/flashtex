// name: ProjectSearchClientTests.cs
// purpose: End-to-end test of ProjectSearchClient (search_literal, served by the
//   REAL flashtex-preview-controller binary delegating to crates/project-index)
//   against a small multi-document fixture project, covering: matches across two
//   attached documents with an exact `complete` termination, a tiny `max_work`
//   budget producing an explicit non-exhaustive `work_limit` termination instead
//   of a silently partial result, and version-checked navigation refusing a
//   stale `source_versions` snapshot rather than searching newer text unnoticed.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Ipc;
using FlashTeX.Protocol.PreviewControllerV1;
using Xunit;

namespace FlashTeX.Ipc.Tests;

public class ProjectSearchClientTests
{
    private const string ProjectId = "project-search-tests";
    private const string EntryPath = "main.tex";
    private const string ChapterPath = "chapter.tex";

    [Fact]
    public async Task SearchAsync_AcrossTwoAttachedDocuments_FindsBothMatchesAndRefusesAStaleSnapshot()
    {
        Assert.True(
            File.Exists(TestPaths.PreviewControllerExePath),
            $"expected a release build of flashtex-preview-controller at '{TestPaths.PreviewControllerExePath}' " +
            "(run `cargo build --release` from crates/preview-controller first)");

        DirectoryInfo root = Directory.CreateTempSubdirectory("flashtex-project-search-root-");
        DirectoryInfo privateLedgerRoot = Directory.CreateTempSubdirectory("flashtex-project-search-ledger-");
        DirectoryInfo configDirectory = Directory.CreateTempSubdirectory("flashtex-project-search-config-");
        try
        {
            File.WriteAllText(Path.Combine(root.FullName, EntryPath), "alpha in main\n");
            File.WriteAllText(Path.Combine(root.FullName, ChapterPath), "alpha in chapter\n");

            var config = new LaunchConfig(
                sessionId: "session-1", projectId: ProjectId, entryPath: EntryPath,
                projectRoot: root.FullName, privateLedgerRoot: privateLedgerRoot.FullName);
            await using PreviewControllerClient client = await PreviewControllerClient.StartAsync(
                TestPaths.PreviewControllerExePath, config, configDirectory.FullName, _ => { });
            var search = new ProjectSearchClient(client);

            // Only the entry document is attached at startup; chapter.tex exists on
            // disk but is not yet part of the project until explicitly opened.
            SnapshotResult initialSnapshot = await client.SnapshotAsync("snap-1").WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal(new[] { EntryPath }, initialSnapshot.SourceVersions.Keys);

            IReadOnlyDictionary<string, ulong> staleVersions = initialSnapshot.SourceVersions;
            MembershipResult opened = await client
                .OpenDocumentAsync("open", new MembershipPayload(ChapterPath, initialSnapshot.SourceVersions, initialSnapshot.MembershipGeneration))
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal("alpha in chapter\n", opened.Document?.Text);
            Assert.Equal(2, opened.SourceVersions.Count);

            SearchResult found = await search.SearchAsync(opened.SourceVersions, "alpha", maxMatches: 10, maxWork: 10_000)
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal(SearchTermination.complete, found.Termination);
            Assert.Equal(2, found.Matches.Count);
            Assert.Contains(found.Matches, m => m.Path == EntryPath);
            Assert.Contains(found.Matches, m => m.Path == ChapterPath);

            SearchResult bounded = await search.SearchAsync(opened.SourceVersions, "alpha", maxMatches: 10, maxWork: 1)
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal(SearchTermination.work_limit, bounded.Termination);
            Assert.True(bounded.Matches.Count <= found.Matches.Count);

            // The pre-open snapshot no longer matches current membership (it is
            // missing chapter.tex entirely) — this must be refused, not silently
            // searched against a project state the caller never actually observed.
            await Assert.ThrowsAsync<PreviewControllerErrorException>(async () =>
                await search.SearchAsync(staleVersions, "alpha", maxMatches: 10, maxWork: 10_000)
                    .WaitAsync(TimeSpan.FromSeconds(30)));
        }
        finally
        {
            TryDelete(root);
            TryDelete(privateLedgerRoot);
            TryDelete(configDirectory);
        }
    }

    private static void TryDelete(DirectoryInfo dir)
    {
        try
        {
            dir.Delete(recursive: true);
        }
        catch (IOException)
        {
            // Best-effort cleanup only; never fail the test on a lingering handle.
        }
    }
}
