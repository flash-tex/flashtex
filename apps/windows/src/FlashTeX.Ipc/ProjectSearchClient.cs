// name: ProjectSearchClient.cs
// purpose: Thin project-wide literal search wrapper over PreviewControllerClient's
//   `search_literal` operation (crates/preview-controller/STDIO.md). `project-index`
//   ships no standalone binary; this same flashtex-preview-controller helper serves
//   search_literal by delegating to that crate's search logic as a library. This
//   wrapper only generates a fresh correlation id per query and forwards defaults,
//   so callers driving search from a UI do not have to manage request IDs themselves.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.PreviewControllerV1;

namespace FlashTeX.Ipc;

/// <summary>
/// Case-sensitive raw literal search (including comments and verbatim text) over
/// one project's exact current source, via a shared <see cref="PreviewControllerClient"/>
/// connection. Every result carries an explicit <see cref="SearchResult.Termination"/>
/// reason (<c>complete</c>/<c>match_limit</c>/<c>work_limit</c>/<c>cancelled</c>) —
/// a non-<c>complete</c> termination must never be presented to the user as exhaustive.
/// This route does not implement mid-query cancellation or regular expressions.
/// </summary>
public sealed class ProjectSearchClient(PreviewControllerClient controller)
{
    /// <summary>
    /// Searches for <paramref name="literal"/> across <paramref name="sourceVersions"/>
    /// (the exact current version map from a prior <see cref="PreviewControllerClient.SnapshotAsync"/>
    /// or <see cref="PreviewControllerClient.ProjectStatusAsync"/> call). A stale map
    /// — one that no longer matches the helper's current source — is refused rather
    /// than silently searched against newer text.
    /// </summary>
    public Task<SearchResult> SearchAsync(
        IReadOnlyDictionary<string, ulong> sourceVersions,
        string literal,
        int maxMatches = PreviewControllerProtocol.MaxSearchMatches,
        int maxWork = PreviewControllerProtocol.MaxSearchWork,
        IReadOnlyList<string>? documents = null,
        CancellationToken cancellationToken = default)
    {
        var payload = new SearchLiteralPayload(sourceVersions, literal, maxMatches, maxWork, documents);
        return controller.SearchLiteralAsync($"search-{Guid.NewGuid():N}", payload, cancellationToken);
    }
}
