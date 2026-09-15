// name: PreviewControllerToolLocator.cs
// purpose: Locates the release build of the flashtex-preview-controller binary
//   (crates/preview-controller, one [[bin]] crate, `cargo build --release`) —
//   the durable project-navigation helper both project-wide search
//   (search_literal) and citation rename (plan_citation_rename[_at]) go
//   through. Mirrors CompilerLocator/PdfToolLocator/ProjectFilesToolLocator's
//   walk-up-to-`.git` strategy rather than a fourth, different one.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.App;

/// <summary>Finds the built <c>flashtex-preview-controller.exe</c> tool binary on disk, if any.</summary>
internal static class PreviewControllerToolLocator
{
    private const string RelativePath = "crates/preview-controller/target/release/flashtex-preview-controller.exe";

    /// <summary>
    /// The expected path to the release <c>flashtex-preview-controller.exe</c> build, or null
    /// if no repo root (a `.git` directory above <see cref="AppContext.BaseDirectory"/>) could
    /// be found. Callers must still check <see cref="File.Exists"/> before starting it.
    /// </summary>
    public static string? FindExecutable()
    {
        var root = CompilerLocator.FindRepoRoot(new DirectoryInfo(AppContext.BaseDirectory));
        return root is null ? null : Path.Combine(root.FullName, RelativePath.Replace('/', Path.DirectorySeparatorChar));
    }
}
