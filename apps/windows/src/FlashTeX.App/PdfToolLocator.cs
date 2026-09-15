// name: PdfToolLocator.cs
// purpose: Locates the release builds of the flashtex-pdf/flashtex-pdf-exact
//   CLI tools (crates/pdf, `cargo build --release`, one crate with two [[bin]]
//   targets) relative to the repo root, so MainWindow can construct a real
//   FlashTeX.Ipc.PdfExportClient at startup without a hardcoded absolute path.
//   Walks up from the running app's base directory until a `.git` marker
//   locates the repo root, reusing CompilerLocator.FindRepoRoot rather than
//   duplicating the same walk-up logic a second time.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.App;

/// <summary>Finds the built <c>flashtex-pdf.exe</c>/<c>flashtex-pdf-exact.exe</c> tool binaries on disk, if any.</summary>
internal static class PdfToolLocator
{
    private const string PdfWriterRelativePath = "crates/pdf/target/release/flashtex-pdf.exe";
    private const string PdfExactWriterRelativePath = "crates/pdf/target/release/flashtex-pdf-exact.exe";

    /// <summary>
    /// The expected paths to the release <c>flashtex-pdf.exe</c>/<c>flashtex-pdf-exact.exe</c>
    /// builds, or (null, null) if no repo root (a `.git` directory above
    /// <see cref="AppContext.BaseDirectory"/>) could be found. Callers must still check
    /// <see cref="File.Exists"/> before constructing a <see cref="FlashTeX.Ipc.PdfExportClient"/>.
    /// </summary>
    public static (string? PdfWriterPath, string? PdfExactWriterPath) FindPdfToolExecutables()
    {
        var repoRoot = CompilerLocator.FindRepoRoot(new DirectoryInfo(AppContext.BaseDirectory));
        if (repoRoot is null)
        {
            return (null, null);
        }

        return (
            Path.Combine(repoRoot.FullName, PdfWriterRelativePath.Replace('/', Path.DirectorySeparatorChar)),
            Path.Combine(repoRoot.FullName, PdfExactWriterRelativePath.Replace('/', Path.DirectorySeparatorChar)));
    }
}
