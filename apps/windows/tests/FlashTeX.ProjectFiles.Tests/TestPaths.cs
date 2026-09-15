// name: TestPaths.cs
// purpose: Locates repo-relative paths (the compiled flashtex-project-files
//   binary) from the test output directory, independent of the test runner's
//   working directory. Mirrors tests/FlashTeX.Ipc.Tests/TestPaths.cs.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.ProjectFiles.Tests;

internal static class TestPaths
{
    /// <summary>Walks up from the test binary's directory until a `.git` marker locates the repo root.</summary>
    public static string RepoRoot { get; } = FindRepoRoot();

    /// <summary>
    /// The release build of `crates/project-files` (`cargo build --release`
    /// from that directory produces this). Tests that depend on it fail
    /// loudly, with the exact expected path, if it hasn't been built yet.
    /// </summary>
    public static string ProjectFilesExePath { get; } =
        Path.Combine(RepoRoot, "crates", "project-files", "target", "release", "flashtex-project-files.exe");

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
