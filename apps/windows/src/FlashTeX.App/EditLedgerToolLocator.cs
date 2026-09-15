// name: EditLedgerToolLocator.cs
// purpose: Locates the release build of the flashtex-edit-ledger binary
//   (crates/edit-ledger, `cargo build --release`) so MainWindow can give
//   ShellModel a real durable-undo/redo executable path at construction time
//   (ShellModel.EditLedger.cs's editLedgerExecutablePath constructor
//   parameter — previously left null, so no document ever got a store and
//   the new edit-history panel had nothing to show). Mirrors
//   CompilerLocator/PdfToolLocator/ProjectFilesToolLocator's walk-up-to-`.git`
//   strategy rather than a fifth, different one.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.App;

/// <summary>Finds the built <c>flashtex-edit-ledger.exe</c> tool binary on disk, if any.</summary>
internal static class EditLedgerToolLocator
{
    private const string RelativePath = "crates/edit-ledger/target/release/flashtex-edit-ledger.exe";

    /// <summary>
    /// The expected path to the release <c>flashtex-edit-ledger.exe</c> build, or null if it
    /// cannot be found (no repo root, or the binary has not been built on this machine).
    /// </summary>
    public static string? FindExecutable()
    {
        var root = CompilerLocator.FindRepoRoot(new DirectoryInfo(AppContext.BaseDirectory));
        if (root is null)
        {
            return null;
        }
        string path = Path.Combine(root.FullName, RelativePath.Replace('/', Path.DirectorySeparatorChar));
        return File.Exists(path) ? path : null;
    }
}
