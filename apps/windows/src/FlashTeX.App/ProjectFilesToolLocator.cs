// name: ProjectFilesToolLocator.cs
// purpose: Finds the hardened Rust file helper used for all on-disk source I/O.

namespace FlashTeX.App;

internal static class ProjectFilesToolLocator
{
    private const string RelativePath = "crates/project-files/target/release/flashtex-project-files.exe";

    public static string? FindExecutable()
    {
        var root = CompilerLocator.FindRepoRoot(new DirectoryInfo(AppContext.BaseDirectory));
        return root is null ? null : Path.Combine(root.FullName, RelativePath.Replace('/', Path.DirectorySeparatorChar));
    }
}
