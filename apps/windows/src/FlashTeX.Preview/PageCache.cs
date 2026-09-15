// name: PageCache.cs
// purpose: Per-page derived-geometry cache, porting the design (not the CoreGraphics
//   specifics) of apps/mac/Sources/FlashTeXMac/V2PageCache.swift: reuse of whatever a
//   future Win2D layer builds from a `Page` (dirty-page tracking + LRU eviction), keyed
//   by page content identity rather than by document revision alone so unrelated pages
//   survive a compile that only touched one page.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Security.Cryptography;
using System.Text.Json;
using FlashTeX.Protocol.RenderingV2;

namespace FlashTeX.Preview;

/// <summary>
/// Computes a stable content-identity digest for a <see cref="Page"/>: the SHA-256 of its
/// paint-ordered items, serialized through the same `Item`/`Path`/`PathCommand`
/// converters the wire format uses. Two pages with byte-identical items hash identically
/// regardless of document revision, mirroring V2PageCache.swift's "identity is the raw
/// bytes" rule (there, the raw JSON bytes of the page; here, a hash of the decoded items,
/// since this consumer caches post-decode `Page` values rather than raw JSON spans).
/// </summary>
public static class PageContentDigest
{
    private static readonly JsonSerializerOptions Options = new();

    public static string Compute(Page page)
    {
        ArgumentNullException.ThrowIfNull(page);
        byte[] itemBytes = JsonSerializer.SerializeToUtf8Bytes(page.Items, Options);
        byte[] hash = SHA256.HashData(itemBytes);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }
}

/// <summary>One cached derived-drawable entry: the built value plus the content digest it was built from.</summary>
internal sealed record PageCacheEntry<TDrawable>(TDrawable Drawable, string ContentDigest)
    where TDrawable : notnull;

/// <summary>
/// Caches one derived value per page number, invalidating automatically whenever the
/// page's content digest changes and evicting least-recently-used pages once
/// <see cref="MaxEntries"/> is exceeded. <typeparamref name="TDrawable"/> is supplied
/// entirely by the caller (e.g. a future Win2D <c>CanvasCommandList</c> wrapper); this
/// type owns only the cache-invalidation and eviction policy, never the drawing.
/// </summary>
/// <typeparam name="TDrawable">The expensive-to-build per-page value a miss constructs.</typeparam>
public sealed class PageCache<TDrawable> where TDrawable : notnull
{
    public const int DefaultMaxEntries = 64;

    private readonly Dictionary<int, PageCacheEntry<TDrawable>> entriesByPage = new();
    private readonly LinkedList<int> lruOrder = new();
    private readonly Dictionary<int, LinkedListNode<int>> lruNodes = new();
    private readonly object gate = new();

    public PageCache(int maxEntries = DefaultMaxEntries)
    {
        if (maxEntries < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(maxEntries), maxEntries, "a page cache needs room for at least one entry");
        }
        this.MaxEntries = maxEntries;
    }

    public int MaxEntries { get; }

    public int Count
    {
        get { lock (this.gate) { return this.entriesByPage.Count; } }
    }

    public int HitCount { get; private set; }

    public int MissCount { get; private set; }

    /// <summary>True if <paramref name="page"/> has no cached entry, or its cached entry was built from different content — i.e. <see cref="GetOrBuild"/> would call <paramref name="build"/> rather than reuse.</summary>
    public bool IsDirty(Page page)
    {
        ArgumentNullException.ThrowIfNull(page);
        lock (this.gate)
        {
            return !this.entriesByPage.TryGetValue(page.Number, out var entry) || entry.ContentDigest != PageContentDigest.Compute(page);
        }
    }

    /// <summary>
    /// Returns the cached drawable for <paramref name="page"/>'s current content, calling
    /// <paramref name="build"/> only when the page is new or its content changed since the
    /// last call (a cache miss). A page whose content is byte-identical to what is cached
    /// — the common case when other pages change but this one does not — is reused
    /// without ever invoking <paramref name="build"/>.
    /// </summary>
    public TDrawable GetOrBuild(Page page, Func<Page, TDrawable> build)
    {
        ArgumentNullException.ThrowIfNull(page);
        ArgumentNullException.ThrowIfNull(build);
        string digest = PageContentDigest.Compute(page);
        lock (this.gate)
        {
            if (this.entriesByPage.TryGetValue(page.Number, out var existing) && existing.ContentDigest == digest)
            {
                this.HitCount++;
                this.Touch(page.Number);
                return existing.Drawable;
            }
            this.MissCount++;
            var drawable = build(page);
            this.entriesByPage[page.Number] = new PageCacheEntry<TDrawable>(drawable, digest);
            this.Touch(page.Number);
            this.EvictLeastRecentlyUsed();
            return drawable;
        }
    }

    /// <summary>Drops the cached entry for one page (e.g. it scrolled out of the retained window).</summary>
    public void Invalidate(int pageNumber)
    {
        lock (this.gate)
        {
            this.entriesByPage.Remove(pageNumber);
            if (this.lruNodes.Remove(pageNumber, out var node))
            {
                this.lruOrder.Remove(node);
            }
        }
    }

    /// <summary>Drops every cached entry.</summary>
    public void Clear()
    {
        lock (this.gate)
        {
            this.entriesByPage.Clear();
            this.lruNodes.Clear();
            this.lruOrder.Clear();
        }
    }

    private void Touch(int pageNumber)
    {
        if (this.lruNodes.Remove(pageNumber, out var existingNode))
        {
            this.lruOrder.Remove(existingNode);
        }
        this.lruNodes[pageNumber] = this.lruOrder.AddLast(pageNumber);
    }

    private void EvictLeastRecentlyUsed()
    {
        while (this.entriesByPage.Count > this.MaxEntries && this.lruOrder.First is { } oldest)
        {
            this.lruOrder.RemoveFirst();
            this.lruNodes.Remove(oldest.Value);
            this.entriesByPage.Remove(oldest.Value);
        }
    }
}
