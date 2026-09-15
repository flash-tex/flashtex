// name: FontFileStore.cs
// purpose: Content-addressed resolution of a rendering-v2 `fonts[]` resource to a real
//   font FILE on disk, porting the design of apps/mac/Sources/FlashTeXMac/
//   GlyphRunRenderer.swift's `V2FontStore`. The display-list-v2 wire format carries NO
//   font bytes (see FontResource in RenderingV2.cs: sha256 + byte_length + face_index +
//   units_per_em + glyph_count + postscript_name, and nothing else), so the consumer must
//   find bytes whose SHA-256 is exactly the declared digest among fonts it already has.
//   Platform font NAMES are never an identity here and nothing is ever substituted: an
//   unresolvable font is a refusal for the whole frame, which drops the caller back to the
//   runtime-v1 renderer rather than painting something that merely looks similar.
// author: Claude Opus 5
// date: 2026-09-14

using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text;
using FlashTeX.Protocol.RenderingV2;

namespace FlashTeX.Preview;

/// <summary>One discovered font file: its path plus the identity of the bytes seen at discovery.</summary>
public sealed record DiscoveredFontFile(string Path, string Sha256, long ByteLength);

/// <summary>
/// A font resource that resolved to authenticated bytes on disk. <see cref="Path"/> is
/// what a platform font loader should be pointed at; <see cref="Bytes"/> are the exact
/// bytes whose SHA-256 was verified against <see cref="Resource"/>.
/// </summary>
public sealed record ResolvedFontFile(FontResource Resource, string Path, ReadOnlyMemory<byte> Bytes);

/// <summary>
/// Bounded, content-addressed font discovery and resolution. Every <c>.otf</c>/<c>.ttf</c>
/// in the supplied directories is hashed once at construction; a display list's font
/// resource resolves only if its <c>sha256</c> names one of those files exactly, its
/// declared <c>byte_length</c> agrees, and the SFNT metadata parsed out of those same bytes
/// (units per em, glyph count, PostScript name) matches what the producer declared.
/// </summary>
/// <remarks>
/// Re-reading the file at resolve time and re-hashing it (rather than trusting the
/// discovery-time digest) is the contract's "Resource verification must bind the bytes
/// actually used by the renderer, not an earlier path read", and matches the Mac source's
/// GH31 rule. The residual gap on Windows is that DirectWrite is handed a PATH, not a
/// buffer (<c>CanvasFontSet</c> has only a URI constructor), so a file replaced between
/// this verification and DirectWrite's own read would not be caught here. The metadata
/// cross-check below is parsed from the verified bytes, not from the loaded face, so it
/// cannot be fooled by that window either way.
/// </remarks>
public sealed class FontFileStore
{
    private static readonly string[] FontExtensions = [".otf", ".ttf"];

    private readonly Dictionary<string, DiscoveredFontFile> byHash = new(StringComparer.Ordinal);
    private readonly List<DiscoveredFontFile> discovered = [];

    /// <summary>Hashes every font file in <paramref name="directories"/> (missing directories are skipped).</summary>
    public FontFileStore(IEnumerable<string> directories)
    {
        ArgumentNullException.ThrowIfNull(directories);
        foreach (string directory in directories)
        {
            Discover(directory);
        }
    }

    /// <summary>Every font file discovered, in discovery order.</summary>
    public IReadOnlyList<DiscoveredFontFile> Files => this.discovered;

    /// <summary>
    /// Resolves <paramref name="resource"/> to authenticated bytes on disk, or throws a
    /// diagnostic-bearing <see cref="RenderingV2ValidationException"/>. Metrics-only
    /// (<c>core14-afm</c>) resources carry no program bytes at all and are refused here
    /// rather than substituted — the contract calls them "not authenticated paintable font
    /// files", so a list that paints with one has no exact rendering available.
    /// </summary>
    public ResolvedFontFile Resolve(FontResource resource)
    {
        ArgumentNullException.ThrowIfNull(resource);
        string shortDigest = resource.Sha256.Length >= 12 ? resource.Sha256[..12] : resource.Sha256;
        if (!resource.IsPaintable)
        {
            throw new RenderingV2ValidationException(
                "font_resource_unavailable",
                $"font resource {shortDigest}… ({resource.PostscriptName}) is format '{resource.Format}', which carries no program bytes and is never paintable");
        }
        if (!this.byHash.TryGetValue(resource.Sha256, out DiscoveredFontFile? file))
        {
            throw new RenderingV2ValidationException(
                "font_resource_unavailable",
                $"font resource {shortDigest}… ({resource.PostscriptName}): none of the {this.discovered.Count} discovered font files has this content hash");
        }
        if (file.ByteLength != resource.ByteLength)
        {
            throw new RenderingV2ValidationException(
                "font_resource_mismatch",
                $"font resource {shortDigest}… declares byte_length {resource.ByteLength}; '{System.IO.Path.GetFileName(file.Path)}' is {file.ByteLength} bytes");
        }

        byte[] bytes = ReadVerified(file, shortDigest);
        CheckDeclaredMetadata(resource, bytes, shortDigest, System.IO.Path.GetFileName(file.Path));
        return new ResolvedFontFile(resource, file.Path, bytes);
    }

    /// <summary>Re-reads and re-authenticates the file discovery recorded, so changed bytes never inherit the discovered hash.</summary>
    private static byte[] ReadVerified(DiscoveredFontFile file, string shortDigest)
    {
        byte[] bytes;
        try
        {
            bytes = File.ReadAllBytes(file.Path);
        }
        catch (IOException ex)
        {
            throw new RenderingV2ValidationException("font_resource_unavailable", $"font resource {shortDigest}…: '{file.Path}' could not be read ({ex.Message})");
        }
        catch (UnauthorizedAccessException ex)
        {
            throw new RenderingV2ValidationException("font_resource_unavailable", $"font resource {shortDigest}…: '{file.Path}' could not be read ({ex.Message})");
        }
        string actual = Sha256Hex(bytes);
        if (bytes.LongLength != file.ByteLength || !string.Equals(actual, file.Sha256, StringComparison.Ordinal))
        {
            throw new RenderingV2ValidationException(
                "font_resource_mismatch",
                $"font resource {shortDigest}…: '{System.IO.Path.GetFileName(file.Path)}' on disk ({bytes.LongLength} bytes, sha256 {Shorten(actual)}…) no longer matches the bytes discovered at startup; refusing to load changed bytes under the discovered hash");
        }
        return bytes;
    }

    private static void CheckDeclaredMetadata(FontResource resource, ReadOnlySpan<byte> bytes, string shortDigest, string fileName)
    {
        SfntMetadata metadata = SfntMetadata.Parse(bytes, resource.FaceIndex, shortDigest);
        if (metadata.UnitsPerEm != resource.UnitsPerEm)
        {
            throw new RenderingV2ValidationException(
                "font_resource_mismatch",
                $"font resource {shortDigest}… declares units_per_em {resource.UnitsPerEm}; '{fileName}' has {metadata.UnitsPerEm}");
        }
        if (metadata.GlyphCount != resource.GlyphCount)
        {
            throw new RenderingV2ValidationException(
                "font_resource_mismatch",
                $"font resource {shortDigest}… declares glyph_count {resource.GlyphCount}; '{fileName}' has {metadata.GlyphCount}");
        }
        if (metadata.PostscriptName is { } name && !string.Equals(name, resource.PostscriptName, StringComparison.Ordinal))
        {
            throw new RenderingV2ValidationException(
                "font_resource_mismatch",
                $"font resource {shortDigest}… declares postscript_name '{resource.PostscriptName}'; '{fileName}' reports '{name}'");
        }
    }

    private void Discover(string directory)
    {
        string[] files;
        try
        {
            if (!Directory.Exists(directory))
            {
                return;
            }
            files = Directory.GetFiles(directory);
        }
        catch (IOException)
        {
            return;
        }
        catch (UnauthorizedAccessException)
        {
            return;
        }

        Array.Sort(files, StringComparer.OrdinalIgnoreCase);
        foreach (string path in files)
        {
            if (!FontExtensions.Contains(System.IO.Path.GetExtension(path), StringComparer.OrdinalIgnoreCase))
            {
                continue;
            }
            try
            {
                byte[] bytes = File.ReadAllBytes(path);
                var entry = new DiscoveredFontFile(path, Sha256Hex(bytes), bytes.LongLength);
                // First directory wins for identical content (the bytes are the identity).
                if (this.byHash.TryAdd(entry.Sha256, entry))
                {
                    this.discovered.Add(entry);
                }
            }
            catch (IOException)
            {
                // An unreadable candidate simply is not discovered; it is never substituted for.
            }
            catch (UnauthorizedAccessException)
            {
            }
        }
    }

    private static string Shorten(string digest) => digest.Length >= 12 ? digest[..12] : digest;

    internal static string Sha256Hex(ReadOnlySpan<byte> bytes) => Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
}

/// <summary>
/// The three identity fields a display list declares for a font, read straight out of the
/// SFNT tables of the verified bytes: <c>head.unitsPerEm</c>, <c>maxp.numGlyphs</c> and
/// name ID 6 of the <c>name</c> table. Deliberately a minimal reader — it never rewrites,
/// subsets or reshapes the font, it only cross-checks what the producer declared.
/// </summary>
internal readonly record struct SfntMetadata(int UnitsPerEm, int GlyphCount, string? PostscriptName)
{
    private const uint TtcTag = 0x74746366; // 'ttcf'

    public static SfntMetadata Parse(ReadOnlySpan<byte> bytes, int faceIndex, string shortDigest)
    {
        int tableDirectory = FindTableDirectory(bytes, faceIndex, shortDigest);
        ushort tableCount = ReadUInt16(bytes, tableDirectory + 4, shortDigest);
        int unitsPerEm = 0;
        int glyphCount = 0;
        string? postscriptName = null;
        for (int i = 0; i < tableCount; i++)
        {
            int record = tableDirectory + 12 + (i * 16);
            uint tag = ReadUInt32(bytes, record, shortDigest);
            int offset = (int)ReadUInt32(bytes, record + 8, shortDigest);
            switch (tag)
            {
                case 0x68656164: // 'head'
                    unitsPerEm = ReadUInt16(bytes, offset + 18, shortDigest);
                    break;
                case 0x6D617870: // 'maxp'
                    glyphCount = ReadUInt16(bytes, offset + 4, shortDigest);
                    break;
                case 0x6E616D65: // 'name'
                    postscriptName = ReadPostscriptName(bytes, offset, shortDigest);
                    break;
            }
        }
        if (unitsPerEm == 0 || glyphCount == 0)
        {
            throw Malformed(shortDigest, "the font has no usable 'head'/'maxp' table");
        }
        return new SfntMetadata(unitsPerEm, glyphCount, postscriptName);
    }

    private static int FindTableDirectory(ReadOnlySpan<byte> bytes, int faceIndex, string shortDigest)
    {
        if (ReadUInt32(bytes, 0, shortDigest) != TtcTag)
        {
            return faceIndex == 0 ? 0 : throw Malformed(shortDigest, $"face_index {faceIndex} was requested but the file is not a font collection");
        }
        uint faces = ReadUInt32(bytes, 8, shortDigest);
        if (faceIndex < 0 || faceIndex >= faces)
        {
            throw Malformed(shortDigest, $"face_index {faceIndex} is outside the collection's {faces} faces");
        }
        return (int)ReadUInt32(bytes, 12 + (faceIndex * 4), shortDigest);
    }

    /// <summary>Name ID 6 (PostScript name); prefers the Windows/Unicode BE record, then Macintosh Roman.</summary>
    private static string? ReadPostscriptName(ReadOnlySpan<byte> bytes, int table, string shortDigest)
    {
        ushort count = ReadUInt16(bytes, table + 2, shortDigest);
        int stringOffset = table + ReadUInt16(bytes, table + 4, shortDigest);
        string? macintosh = null;
        for (int i = 0; i < count; i++)
        {
            int record = table + 6 + (i * 12);
            ushort platform = ReadUInt16(bytes, record, shortDigest);
            ushort nameId = ReadUInt16(bytes, record + 6, shortDigest);
            if (nameId != 6)
            {
                continue;
            }
            int length = ReadUInt16(bytes, record + 8, shortDigest);
            int offset = stringOffset + ReadUInt16(bytes, record + 10, shortDigest);
            if (offset < 0 || length < 0 || offset + length > bytes.Length)
            {
                throw Malformed(shortDigest, "a 'name' record points outside the file");
            }
            ReadOnlySpan<byte> value = bytes.Slice(offset, length);
            if (platform == 3)
            {
                return Encoding.BigEndianUnicode.GetString(value);
            }
            macintosh ??= Encoding.ASCII.GetString(value);
        }
        return macintosh;
    }

    private static ushort ReadUInt16(ReadOnlySpan<byte> bytes, int offset, string shortDigest)
    {
        if (offset < 0 || offset + 2 > bytes.Length)
        {
            throw Malformed(shortDigest, $"a table offset ({offset}) lies outside the {bytes.Length}-byte file");
        }
        return BinaryPrimitives.ReadUInt16BigEndian(bytes[offset..]);
    }

    private static uint ReadUInt32(ReadOnlySpan<byte> bytes, int offset, string shortDigest)
    {
        if (offset < 0 || offset + 4 > bytes.Length)
        {
            throw Malformed(shortDigest, $"a table offset ({offset}) lies outside the {bytes.Length}-byte file");
        }
        return BinaryPrimitives.ReadUInt32BigEndian(bytes[offset..]);
    }

    private static RenderingV2ValidationException Malformed(string shortDigest, string what) =>
        new("font_resource_mismatch", $"font resource {shortDigest}…: {what}");
}
