// name: ResourceCacheTests.cs
// purpose: Unit tests for FlashTeX.Preview.ResourceCache<T>: insert/retrieve by SHA-256,
//   generation-based eviction, and the compile-time paintable/metrics-only font split.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Preview;
using FlashTeX.Protocol.RenderingV2;

namespace FlashTeX.Preview.Tests;

public class ResourceCacheTests
{
    private const string HashA = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    private const string HashB = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    private static FontResource MakeFontMetadata(string fontId, string sha256, string format) =>
        new(fontId, sha256, format == "core14-afm" ? 0 : 4096, format, FaceIndex: 0, UnitsPerEm: 1000, GlyphCount: 200, PostscriptName: "Test-Font");

    [Fact]
    public void GetOrAdd_InsertsThenRetrievesByHash()
    {
        var cache = new FontResourceCache();
        var metadata = MakeFontMetadata("f1", HashA, "static-truetype");
        var built = cache.GetOrAdd(HashA, generation: 1, factory: () => CachedFontResource.Create(metadata, new byte[] { 1, 2, 3 }));

        Assert.IsType<CachedFontResource.Paintable>(built);
        Assert.True(cache.TryGet(HashA, out var retrieved));
        Assert.Same(built, retrieved);
        Assert.Equal(1, cache.Count);
    }

    [Fact]
    public void GetOrAdd_SecondCallForSameHashDoesNotRebuild()
    {
        var cache = new FontResourceCache();
        var metadata = MakeFontMetadata("f1", HashA, "static-truetype");
        int factoryCalls = 0;
        CachedFontResource Factory()
        {
            factoryCalls++;
            return CachedFontResource.Create(metadata, new byte[] { 9 });
        }

        cache.GetOrAdd(HashA, generation: 1, Factory);
        cache.GetOrAdd(HashA, generation: 2, Factory);

        Assert.Equal(1, factoryCalls);
    }

    [Fact]
    public void EvictUnreferenced_RemovesEntriesNotTouchedInCurrentGeneration_KeepsTouchedOnes()
    {
        var cache = new FontResourceCache();
        var metaA = MakeFontMetadata("fa", HashA, "static-truetype");
        var metaB = MakeFontMetadata("fb", HashB, "static-truetype");

        cache.GetOrAdd(HashA, generation: 1, () => CachedFontResource.Create(metaA, new byte[] { 1 }));
        cache.GetOrAdd(HashB, generation: 1, () => CachedFontResource.Create(metaB, new byte[] { 2 }));
        Assert.Equal(2, cache.Count);

        // Generation 2's document only references font A.
        cache.GetOrAdd(HashA, generation: 2, () => throw new InvalidOperationException("should not rebuild a still-cached entry"));
        int evicted = cache.EvictUnreferenced(currentGeneration: 2);

        Assert.Equal(1, evicted);
        Assert.Equal(1, cache.Count);
        Assert.True(cache.TryGet(HashA, out _));
        Assert.False(cache.TryGet(HashB, out _));
    }

    [Fact]
    public void EvictUnreferenced_MarkReferencedWithoutRebuild_AlsoSurvives()
    {
        var cache = new FontResourceCache();
        var metaA = MakeFontMetadata("fa", HashA, "static-truetype");
        cache.GetOrAdd(HashA, generation: 1, () => CachedFontResource.Create(metaA, new byte[] { 1 }));

        Assert.True(cache.MarkReferenced(HashA, generation: 2));
        cache.EvictUnreferenced(currentGeneration: 2);

        Assert.True(cache.TryGet(HashA, out _));
    }

    [Fact]
    public void Create_PaintableFormat_ProducesPaintableCaseWithBytes()
    {
        var metadata = MakeFontMetadata("f1", HashA, "opentype-cff");
        var resource = CachedFontResource.Create(metadata, new byte[] { 1, 2, 3, 4 });

        var paintable = Assert.IsType<CachedFontResource.Paintable>(resource);
        Assert.Equal(4, paintable.Bytes.Length);
    }

    [Fact]
    public void Create_MetricsOnlyFormat_ProducesMetricsOnlyCase_WithNoBytesMember()
    {
        var metadata = MakeFontMetadata("f1", HashA, "core14-afm");
        var resource = CachedFontResource.Create(metadata, ReadOnlyMemory<byte>.Empty);

        // The type system, not a runtime `bytes.Length == 0` check, is what forbids
        // painting a metrics-only resource: CachedFontResource.MetricsOnly has no Bytes
        // property at all, so a method that requires CachedFontResource.Paintable simply
        // does not compile against this value.
        var metricsOnly = Assert.IsType<CachedFontResource.MetricsOnly>(resource);
        Assert.Equal("core14-afm", metricsOnly.Metadata.Format);
        Assert.False(RequiresPaintable(resource, out _));
    }

    /// <summary>Demonstrates the compile-time boundary: only a Paintable can be handed to a painter.</summary>
    private static bool RequiresPaintable(CachedFontResource resource, out ReadOnlyMemory<byte> bytes)
    {
        if (resource is CachedFontResource.Paintable paintable)
        {
            bytes = paintable.Bytes;
            return true;
        }
        bytes = default;
        return false;
    }

    [Fact]
    public void GetOrAdd_RejectsKeyThatIsNotA64CharHexDigest()
    {
        var cache = new FontResourceCache();
        Assert.Throws<ArgumentException>(() =>
            cache.GetOrAdd("not-a-hash", generation: 1, () => CachedFontResource.Create(MakeFontMetadata("f", HashA, "static-truetype"), new byte[] { 1 })));
    }
}
