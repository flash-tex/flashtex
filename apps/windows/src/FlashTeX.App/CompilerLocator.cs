// name: CompilerLocator.cs
// purpose: Locates the release build of the flashtex-compiler worker binary
//   (crates/compiler, `cargo build --release`) relative to the repo root, so
//   MainWindow can attach a real ShellModel worker at startup without a
//   hardcoded absolute path. Walks up from the running app's base directory
//   until a `.git` marker locates the repo root, mirroring the equivalent
//   helper the FlashTeX.Shell.Tests suite already uses
//   (tests/FlashTeX.Shell.Tests/TestPaths.cs) rather than duplicating a
//   different lookup strategy.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.App;

/// <summary>Finds the built <c>flashtex-compiler.exe</c> worker binary on disk, if any.</summary>
internal static class CompilerLocator
{
    /// <summary>Overrides the worker binary outright, matching the engine's own convention.</summary>
    private const string OverrideVariable = "FLASHTEX_COMPILER";

    private const string CompilerRelativePath = "crates/compiler/target/release/flashtex-compiler.exe";

    /// <summary>
    /// <c>flashtex-render</c> is a drop-in runtime-v1 worker (its own module docs say so) that
    /// additionally implements the <c>display-list-v2</c> layout capability, which
    /// <c>flashtex-compiler</c> declines. Preferring it is what makes the rendering-v2
    /// precision preview reachable at all; falling back to <c>flashtex-compiler</c> keeps the
    /// runtime-v1 path working exactly as before on a machine where only it is built.
    /// </summary>
    private const string RenderRelativePath = "crates/render-pipeline/target/release/flashtex-render.exe";

    /// <summary>A worker binary and the command line it needs.</summary>
    internal sealed record WorkerLaunch(string ExecutablePath, IReadOnlyList<string> Arguments);

    /// <summary>
    /// The worker binary to attach, or null if no repo root was found. Search order: the
    /// <c>FLASHTEX_COMPILER</c> override, the display-list-v2-capable
    /// <c>flashtex-render.exe</c>, then <c>flashtex-compiler.exe</c>. Callers should still
    /// check <see cref="File.Exists"/>.
    /// </summary>
    public static WorkerLaunch? FindWorker()
    {
        if (Environment.GetEnvironmentVariable(OverrideVariable) is { Length: > 0 } configured && File.Exists(configured))
        {
            return new WorkerLaunch(configured, FontArguments(configured));
        }
        var repoRoot = FindRepoRoot(new DirectoryInfo(AppContext.BaseDirectory));
        if (repoRoot is null)
        {
            return null;
        }
        string render = Path.Combine(repoRoot.FullName, RenderRelativePath.Replace('/', Path.DirectorySeparatorChar));
        string compiler = Path.Combine(repoRoot.FullName, CompilerRelativePath.Replace('/', Path.DirectorySeparatorChar));
        string chosen = File.Exists(render) ? render : compiler;
        return new WorkerLaunch(chosen, FontArguments(chosen));
    }

    /// <summary>
    /// Points <c>flashtex-render</c> at the same font directories the rendering-v2 preview
    /// resolves from, so the content hashes it writes into the display list name bytes the
    /// preview actually has. Passed as repeatable <c>--font-dir</c> ARGUMENTS rather than
    /// through the documented <c>FLASHTEX_FONT_DIRS</c> environment variable, because the
    /// producer splits that variable on ':' — which cuts a Windows path in half at its
    /// drive letter. <c>flashtex-compiler</c> takes no such flag and gets an empty list.
    /// </summary>
    private static IReadOnlyList<string> FontArguments(string executablePath)
    {
        if (!Path.GetFileNameWithoutExtension(executablePath).Equals("flashtex-render", StringComparison.OrdinalIgnoreCase))
        {
            return Array.Empty<string>();
        }
        var arguments = new List<string>();
        foreach (string directory in PreviewFontDirectories.All().Where(Directory.Exists))
        {
            arguments.Add("--font-dir");
            arguments.Add(directory);
        }
        return arguments;
    }

    /// <summary>Shared with <see cref="PdfToolLocator"/> so both locators walk the repo root the same way.</summary>
    internal static DirectoryInfo? FindRepoRoot(DirectoryInfo? start)
    {
        var dir = start;
        while (dir is not null)
        {
            if (Directory.Exists(Path.Combine(dir.FullName, ".git")))
            {
                return dir;
            }
            dir = dir.Parent;
        }
        return null;
    }
}
