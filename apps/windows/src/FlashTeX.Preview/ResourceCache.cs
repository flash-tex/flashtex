// name: ResourceCache.cs
// purpose: SHA-256-keyed cache for rendering-v2 font/image resources, generation-scoped
//   eviction, and a compile-time-safe distinction between paintable font resources
//   (embedded program bytes) and metrics-only `core14-afm` resources (never
//   paintable, per docs/contracts/runtime-v1-display-list-v2.md).
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RenderingV2;

namespace FlashTeX.Preview;

/// <summary>
/// A font resource that has been resolved against its declared <see cref="FontResource"/>
/// metadata. Closed hierarchy (mirrors <c>Item</c>/<c>PathCommand</c> in RenderingV2.cs):
/// only <see cref="Paintable"/> exposes program bytes, so a painter that needs bytes can
/// only be handed a <see cref="Paintable"/> — a `core14-afm` metrics-only resource simply
/// does not have a member to read bytes from, at compile time, never a runtime check.
/// </summary>
public abstract record CachedFontResource
{
    private CachedFontResource(FontResource Metadata)
    {
        this.Metadata = Metadata;
    }

    public FontResource Metadata { get; }

    /// <summary>A resource with an embedded font program: `static-truetype` or `opentype-cff`.</summary>
    public sealed record Paintable(FontResource Metadata, ReadOnlyMemory<byte> Bytes) : CachedFontResource(Metadata);

    /// <summary>A `core14-afm` metrics-only resource: no program bytes exist to paint with.</summary>
    public sealed record MetricsOnly(FontResource Metadata) : CachedFontResource(Metadata);

    /// <summary>
    /// Builds the correct case for <paramref name="metadata"/>.Format, matching the same
    /// paintable/metrics-only split <see cref="RenderingV2Protocol"/> validation enforces.
    /// </summary>
    public static CachedFontResource Create(FontResource metadata, ReadOnlyMemory<byte> bytes)
    {
        if (RenderingV2Protocol.PaintableFontFormats.Contains(metadata.Format))
        {
            return new Paintable(metadata, bytes);
        }
        if (RenderingV2Protocol.MetricsOnlyFontFormats.Contains(metadata.Format))
        {
            return new MetricsOnly(metadata);
        }
        throw new ArgumentException($"font resource {metadata.FontId} format '{metadata.Format}' is neither paintable nor metrics-only", nameof(metadata));
    }
}

/// <summary>A resolved image resource: the wire format never embeds image bytes (see <see cref="ImageResource"/>), so every cached image carries the bytes read for it.</summary>
public sealed record CachedImageResource(ImageResource Metadata, ReadOnlyMemory<byte> Bytes);

/// <summary>
/// Generation-scoped cache entry: <see cref="LastReferencedGeneration"/> is the highest
/// document generation that has touched this entry since it was inserted.
/// </summary>
internal sealed record CacheEntry<TValue>(TValue Value, int LastReferencedGeneration);

/// <summary>
/// A SHA-256-keyed cache with generation-scoped eviction. A "generation" is a caller-owned
/// counter (e.g. a compile/document revision): callers touch every resource still in use
/// while building generation N, then call <see cref="EvictUnreferenced"/> with N so
/// resources no longer referenced by the current document are freed, without needing to
/// re-decode/re-load resources that recur unchanged across compiles (the same motivation
/// as V2PageCache.swift's page reuse, applied to font/image resources instead of pages).
/// </summary>
/// <typeparam name="TValue">The cached resource value, e.g. <see cref="CachedFontResource"/>.</typeparam>
public class ResourceCache<TValue> where TValue : notnull
{
    private readonly Dictionary<string, CacheEntry<TValue>> entries = new(StringComparer.Ordinal);
    private readonly object gate = new();

    /// <summary>Number of resources currently cached.</summary>
    public int Count
    {
        get { lock (this.gate) { return this.entries.Count; } }
    }

    /// <summary>
    /// Returns the cached value for <paramref name="sha256Hex"/>, or builds it with
    /// <paramref name="factory"/> on a miss. Either way, the entry is marked referenced by
    /// <paramref name="generation"/> so a later <see cref="EvictUnreferenced"/> keeps it.
    /// </summary>
    public TValue GetOrAdd(string sha256Hex, int generation, Func<TValue> factory)
    {
        ArgumentNullException.ThrowIfNull(factory);
        ValidateKey(sha256Hex);
        lock (this.gate)
        {
            if (this.entries.TryGetValue(sha256Hex, out var existing))
            {
                this.entries[sha256Hex] = existing with { LastReferencedGeneration = generation };
                return existing.Value;
            }
            var value = factory();
            this.entries[sha256Hex] = new CacheEntry<TValue>(value, generation);
            return value;
        }
    }

    /// <summary>Looks up a cached resource without inserting or touching its generation.</summary>
    public bool TryGet(string sha256Hex, out TValue value)
    {
        ValidateKey(sha256Hex);
        lock (this.gate)
        {
            if (this.entries.TryGetValue(sha256Hex, out var existing))
            {
                value = existing.Value;
                return true;
            }
            value = default!;
            return false;
        }
    }

    /// <summary>Marks an already-cached resource as referenced by <paramref name="generation"/> without rebuilding it. Returns false if it is not cached.</summary>
    public bool MarkReferenced(string sha256Hex, int generation)
    {
        ValidateKey(sha256Hex);
        lock (this.gate)
        {
            if (!this.entries.TryGetValue(sha256Hex, out var existing))
            {
                return false;
            }
            this.entries[sha256Hex] = existing with { LastReferencedGeneration = generation };
            return true;
        }
    }

    /// <summary>
    /// Removes every entry whose last-referenced generation is older than
    /// <paramref name="currentGeneration"/> (i.e. not touched while building the current
    /// document). Returns the number of entries evicted.
    /// </summary>
    public int EvictUnreferenced(int currentGeneration)
    {
        lock (this.gate)
        {
            var stale = this.entries
                .Where(pair => pair.Value.LastReferencedGeneration < currentGeneration)
                .Select(pair => pair.Key)
                .ToList();
            foreach (var key in stale)
            {
                this.entries.Remove(key);
            }
            return stale.Count;
        }
    }

    /// <summary>Drops every cached resource.</summary>
    public void Clear()
    {
        lock (this.gate)
        {
            this.entries.Clear();
        }
    }

    private static void ValidateKey(string sha256Hex)
    {
        if (!IsHex64(sha256Hex))
        {
            throw new ArgumentException("resource cache keys must be 64 lowercase hex digits (a sha256 digest)", nameof(sha256Hex));
        }
    }

    private static bool IsHex64(string s)
    {
        if (s.Length != 64)
        {
            return false;
        }
        foreach (char c in s)
        {
            if (!((c >= '0' && c <= '9') || (c >= 'a' && c <= 'f')))
            {
                return false;
            }
        }
        return true;
    }
}

/// <summary>Concrete font resource cache: SHA-256 digest to <see cref="CachedFontResource"/>.</summary>
public sealed class FontResourceCache : ResourceCache<CachedFontResource>
{
}

/// <summary>Concrete image resource cache: SHA-256 digest to <see cref="CachedImageResource"/>.</summary>
public sealed class ImageResourceCache : ResourceCache<CachedImageResource>
{
}
