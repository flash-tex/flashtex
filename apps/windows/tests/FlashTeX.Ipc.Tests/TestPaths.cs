// name: TestPaths.cs
// purpose: Locates repo-relative paths (the compiled flashtex-compiler binary,
//   shared JSON fixtures) from the test output directory, independent of the
//   test runner's working directory.
// author: Claude Sonnet 5
// date: 2026-09-13

namespace FlashTeX.Ipc.Tests;

internal static class TestPaths
{
    /// <summary>Walks up from the test binary's directory until a `.git` marker locates the repo root.</summary>
    public static string RepoRoot { get; } = FindRepoRoot();

    /// <summary>
    /// The release build of `crates/compiler` (`cargo build --release` from that
    /// directory produces this). Tests that depend on it fail loudly, with the
    /// exact expected path, if it hasn't been built yet — that is a real
    /// precondition, not something to silently skip past.
    /// </summary>
    public static string CompilerWorkerExePath { get; } =
        Path.Combine(RepoRoot, "crates", "compiler", "target", "release", "flashtex-compiler.exe");

    /// <summary>The release build of `crates/bridge` (`cargo build --release` from that directory produces this).</summary>
    public static string BridgeExePath { get; } =
        Path.Combine(RepoRoot, "crates", "bridge", "target", "release", "flashtex-bridge.exe");

    /// <summary>The release build of `crates/edit-ledger` (`cargo build --release` from that directory produces this).</summary>
    public static string EditLedgerExePath { get; } =
        Path.Combine(RepoRoot, "crates", "edit-ledger", "target", "release", "flashtex-edit-ledger.exe");

    /// <summary>The release build of `crates/preview-controller` (`cargo build --release` from that directory produces this).</summary>
    public static string PreviewControllerExePath { get; } =
        Path.Combine(RepoRoot, "crates", "preview-controller", "target", "release", "flashtex-preview-controller.exe");

    /// <summary>The release build of `crates/pdf`'s `flashtex-pdf` binary (`cargo build --release` from that directory produces this).</summary>
    public static string PdfExePath { get; } =
        Path.Combine(RepoRoot, "crates", "pdf", "target", "release", "flashtex-pdf.exe");

    /// <summary>The release build of `crates/pdf`'s `flashtex-pdf-exact` binary (`cargo build --release` from that directory produces this).</summary>
    public static string PdfExactExePath { get; } =
        Path.Combine(RepoRoot, "crates", "pdf", "target", "release", "flashtex-pdf-exact.exe");

    private static string FindRepoRoot()
    {
        var dir = new DirectoryInfo(AppContext.BaseDirectory);
        while (dir is not null)
        {
            if (Directory.Exists(Path.Combine(dir.FullName, ".git")))
            {
                return dir.FullName;
            }
            dir = dir.Parent;
        }
        throw new DirectoryNotFoundException(
            $"could not locate the repo root (a '.git' directory) above '{AppContext.BaseDirectory}'");
    }
}
