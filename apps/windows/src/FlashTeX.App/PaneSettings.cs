// name: PaneSettings.cs
// purpose: Persists split-pane widths (project tree, editor) across launches.
//   Deliberately NOT Windows.Storage.ApplicationData: this app runs unpackaged
//   (WindowsPackageType=None, FlashTeX.App.csproj), and ApplicationData.Current
//   throws "the process has no package identity" without a real package/MSIX
//   identity. A small JSON file under %LOCALAPPDATA%\FlashTeX works
//   regardless of packaging and needs no extra dependency.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;

namespace FlashTeX.App;

/// <summary>The subset of window layout this app remembers between launches.</summary>
internal sealed record PaneLayout(double ProjectTreeWidth, double EditorWidth);

/// <summary>Reads and writes <see cref="PaneLayout"/> to a small JSON file, tolerating any I/O failure as "use defaults".</summary>
internal static class PaneSettings
{
    private static readonly string FilePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "FlashTeX", "window-panes.json");

    /// <summary>The saved layout, or null if none was saved yet or the file could not be read.</summary>
    public static PaneLayout? TryLoad()
    {
        try
        {
            return File.Exists(FilePath)
                ? JsonSerializer.Deserialize<PaneLayout>(File.ReadAllText(FilePath))
                : null;
        }
        catch (Exception ex) when (ex is IOException or JsonException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    /// <summary>Best-effort save; a failure here must never crash a pane-resize gesture.</summary>
    public static void Save(PaneLayout layout)
    {
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(FilePath)!);
            File.WriteAllText(FilePath, JsonSerializer.Serialize(layout));
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Best-effort only; the in-memory column widths are still correct for this session.
        }
    }
}
