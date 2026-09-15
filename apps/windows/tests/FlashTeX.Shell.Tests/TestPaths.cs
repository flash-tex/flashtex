// name: TestPaths.cs
// purpose: Locates repo-relative paths (the compiled flashtex-compiler and
//   flashtex-edit-ledger binaries) from the test output directory, independent
//   of the test runner's working directory. Mirrors
//   FlashTeX.Ipc.Tests/TestPaths.cs (test projects do not share code, so this
//   is a deliberate small duplicate rather than a new shared project).
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Shell.Tests;

internal static class TestPaths
{
    /// <summary>Walks up from the test binary's directory until a `.git` marker locates the repo root.</summary>
    public static string RepoRoot { get; } = FindRepoRoot();

    /// <summary>The release build of `crates/compiler` (`cargo build --release` from that directory produces this).</summary>
    public static string CompilerWorkerExePath { get; } =
        Path.Combine(RepoRoot, "crates", "compiler", "target", "release", "flashtex-compiler.exe");

    /// <summary>The release build of `crates/edit-ledger` (`cargo build --release` from that directory produces this).</summary>
    public static string EditLedgerExePath { get; } =
        Path.Combine(RepoRoot, "crates", "edit-ledger", "target", "release", "flashtex-edit-ledger.exe");

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
