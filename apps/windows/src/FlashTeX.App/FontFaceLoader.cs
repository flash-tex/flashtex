// name: FontFaceLoader.cs
// purpose: Turns a content-verified font file (FlashTeX.Preview.FontFileStore) into a
//   DirectWrite CanvasFontFace for the rendering-v2 painter, cached by SHA-256 so a font
//   that recurs across compiles is loaded once. Also locates the directories the store
//   discovers fonts in.
//
//   Win2D's CanvasFontSet has only a Uri constructor (no IBuffer/stream overload), so
//   DirectWrite is pointed at the file path whose bytes FontFileStore just authenticated
//   rather than at the buffer itself — the one place this port cannot bind the bytes the
//   renderer reads as tightly as the CoreGraphics original does. The declared identity
//   (units per em, glyph count, PostScript name) is still cross-checked against the
//   authenticated BYTES in FontFileStore, and the loaded face's glyph count is checked
//   again here, so a mismatched file cannot paint silently.
// author: Claude Opus 5
// date: 2026-09-14

using System.Collections.Concurrent;
using FlashTeX.Preview;
using FlashTeX.Protocol.RenderingV2;
using Microsoft.Graphics.Canvas.Text;

namespace FlashTeX.App;

/// <summary>Loads and caches DirectWrite faces for verified rendering-v2 font resources.</summary>
internal static class FontFaceLoader
{
    private static readonly ConcurrentDictionary<string, CanvasFontFace> FacesBySha256 = new(StringComparer.Ordinal);

    /// <summary>
    /// The face for <paramref name="resolved"/>, loaded once per content hash. Throws
    /// <see cref="RenderingV2ValidationException"/> if DirectWrite cannot load the file or
    /// the loaded face disagrees with the declared glyph count.
    /// </summary>
    public static CanvasFontFace Load(ResolvedFontFile resolved)
    {
        ArgumentNullException.ThrowIfNull(resolved);
        return FacesBySha256.GetOrAdd(resolved.Resource.Sha256, _ => LoadFace(resolved));
    }

    /// <summary>Drops every cached face (e.g. after a DirectWrite device loss).</summary>
    public static void Clear() => FacesBySha256.Clear();

    private static CanvasFontFace LoadFace(ResolvedFontFile resolved)
    {
        string shortDigest = resolved.Resource.Sha256[..12];
        CanvasFontSet set;
        try
        {
            set = new CanvasFontSet(new Uri(resolved.Path));
        }
        catch (Exception ex) when (ex is ArgumentException or UriFormatException or System.Runtime.InteropServices.COMException)
        {
            throw new RenderingV2ValidationException(
                "font_resource_unavailable",
                $"font resource {shortDigest}… ({resolved.Resource.PostscriptName}): DirectWrite could not load '{resolved.Path}' ({ex.Message})");
        }

        if (set.Fonts.Count <= resolved.Resource.FaceIndex)
        {
            throw new RenderingV2ValidationException(
                "font_resource_mismatch",
                $"font resource {shortDigest}…: face_index {resolved.Resource.FaceIndex} is outside the {set.Fonts.Count} face(s) DirectWrite found in '{resolved.Path}'");
        }
        CanvasFontFace face = set.Fonts[resolved.Resource.FaceIndex];
        if (face.GlyphCount != resolved.Resource.GlyphCount)
        {
            throw new RenderingV2ValidationException(
                "font_resource_mismatch",
                $"font resource {shortDigest}… declares glyph_count {resolved.Resource.GlyphCount}; the face DirectWrite loaded has {face.GlyphCount}");
        }
        return face;
    }
}

/// <summary>
/// Where <see cref="FontFileStore"/> looks for the font files a display list can name. The
/// producer (<c>flashtex-render</c>) embeds Latin Modern by content hash, and the repository
/// copy under <c>apps/mac/Fonts</c> is the same set it renders from, so the two agree by
/// construction on a developer machine.
/// </summary>
internal static class PreviewFontDirectories
{
    /// <summary>Environment override, matching the Mac source's <c>FLASHTEX_LM_DIR</c>.</summary>
    public const string OverrideVariable = "FLASHTEX_LM_DIR";

    private const string RepositoryFonts = "apps/mac/Fonts";

    /// <summary>Search order: explicit override, a <c>Fonts</c> folder beside the executable, then the repository copy.</summary>
    public static IReadOnlyList<string> All()
    {
        var directories = new List<string>();
        if (Environment.GetEnvironmentVariable(OverrideVariable) is { Length: > 0 } configured)
        {
            directories.Add(configured);
        }
        directories.Add(System.IO.Path.Combine(AppContext.BaseDirectory, "Fonts"));
        if (CompilerLocator.FindRepoRoot(new DirectoryInfo(AppContext.BaseDirectory)) is { } repoRoot)
        {
            directories.Add(System.IO.Path.Combine(repoRoot.FullName, RepositoryFonts.Replace('/', System.IO.Path.DirectorySeparatorChar)));
        }
        return directories;
    }

    /// <summary>The first existing directory in <see cref="All"/>, for tools that take a single <c>--font-dir</c>.</summary>
    public static string? Primary() => All().FirstOrDefault(Directory.Exists);
}
