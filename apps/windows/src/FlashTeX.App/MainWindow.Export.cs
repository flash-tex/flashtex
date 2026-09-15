// name: MainWindow.Export.cs
// purpose: Wires the three registered-but-unimplemented export commands
//   (FlashTeX.Shell.CommandIds.ExportPdf/ExportPdfViaRustWriter/ExportPdfExact)
//   to real behavior. ShellModel.PerformAction (ShellModel.Compile.cs) leaves
//   these as no-ops deliberately: a file-save picker and a result dialog are
//   UI-thread/window concerns ShellModel has no dependency on today, and per
//   its existing design (no window/UI-thread dependency anywhere else in that
//   class) should not gain one just for export. MainWindow.Menu.cs's
//   ExecuteCommand special-cases these three ids to call into this file
//   instead of forwarding to ShellModel.PerformAction, mirroring how Compile
//   already gets model-level handling without needing UI here: export is the
//   inverse case, needing UI-level handling without touching the model beyond
//   reading ShellModel.LastCompileResult (ShellModel.Compile.cs).
// author: Claude Sonnet 5
// date: 2026-09-14
// modified: 2026-09-14 (Claude Opus 5) — Export PDF (Exact, v2) is now REAL. It
//   writes from ShellModel.LastDisplayList, the negotiated display-list-v2 frame
//   the compile pipeline now captures and validates, through
//   PdfExportClient.ExportExactFromDisplayListAsync. It is gated by
//   CommandRegistry's NeedsDisplayList, so it is simply disabled (not faked)
//   when the attached worker does not speak display-list-v2.

using FlashTeX.Ipc;
using FlashTeX.Shell;
using Microsoft.UI.Xaml.Controls;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private const string DefaultExportBaseName = "document";

    private PdfExportClient? _pdfExportClient;
    private string? _pdfToolMissingReason;

    /// <summary>
    /// Locates the crates/pdf release tool binaries and constructs a <see cref="PdfExportClient"/>
    /// if both are present. Called once from the constructor, alongside <c>StartupAsync</c>'s
    /// analogous <c>CompilerLocator</c> lookup; missing tools are not fatal — export commands stay
    /// gated by <c>NeedsCompileResult</c> as usual, and a clear reason is shown only if the user
    /// actually tries to export before the tools are built.
    /// </summary>
    private void WireExport()
    {
        var (pdfWriterPath, pdfExactWriterPath) = PdfToolLocator.FindPdfToolExecutables();
        if (pdfWriterPath is null || pdfExactWriterPath is null)
        {
            _pdfToolMissingReason = "Could not locate the repository root (no .git directory found above the app's base directory), so the flashtex-pdf/flashtex-pdf-exact tools could not be found.";
            return;
        }

        if (!File.Exists(pdfWriterPath) || !File.Exists(pdfExactWriterPath))
        {
            _pdfToolMissingReason = $"flashtex-pdf/flashtex-pdf-exact were not found at '{pdfWriterPath}' / '{pdfExactWriterPath}'. Build them first: cd crates/pdf && cargo build --release";
            return;
        }

        _pdfExportClient = new PdfExportClient(pdfWriterPath, pdfExactWriterPath);
    }

    /// <summary>Whether <paramref name="commandId"/> is one of the three export commands this file handles.</summary>
    private static bool IsExportCommand(string commandId) =>
        commandId is CommandIds.ExportPdf or CommandIds.ExportPdfViaRustWriter or CommandIds.ExportPdfExact;

    /// <summary>
    /// Entry point called from <c>MainWindow.Menu.cs</c>'s <c>ExecuteCommand</c> for the three export
    /// command ids, in place of forwarding to <c>CommandRegistry.Execute</c>/<c>ShellModel.PerformAction</c>.
    /// Applies the same <c>NeedsCompileResult</c> gate <see cref="CommandRegistry"/> would have applied,
    /// then runs the (necessarily async, UI-driving) export flow fire-and-forget, matching the synchronous
    /// <c>bool ExecuteCommand(string)</c> signature every other command dispatch path already uses.
    /// </summary>
    private bool TryStartExport(string commandId)
    {
        var command = CommandRegistry.All.First(c => c.Id == commandId);
        if (!command.CanExecute(_shell))
        {
            return false;
        }

        _ = RunExportCommandAsync(commandId);
        return true;
    }

    /// <summary>
    /// Wraps <see cref="RunExportCoreAsync"/> in a top-level try/catch: this runs fire-and-forget from
    /// <see cref="TryStartExport"/> (the synchronous <c>ExecuteCommand</c> contract has no caller to
    /// await it against), so an unhandled exception here would otherwise be an unobserved task
    /// exception the user never sees any sign of, rather than the clear failure message this feature
    /// exists to guarantee.
    /// </summary>
    private async Task RunExportCommandAsync(string commandId)
    {
        try
        {
            await RunExportCoreAsync(commandId).ConfigureAwait(true);
        }
        catch (Exception ex)
        {
            await ShowExportDialogAsync("Export failed", $"An unexpected error occurred while exporting: {ex.Message}")
                .ConfigureAwait(true);
        }
    }

    private async Task RunExportCoreAsync(string commandId)
    {
        if (commandId == CommandIds.ExportPdfExact)
        {
            await RunExactExportAsync().ConfigureAwait(true);
            return;
        }

        if (_shell.LastCompileResult is not { } result)
        {
            // NeedsCompileResult already gates this in the normal case; reachable only if a compile
            // somehow cleared HasCompileResult without ever setting LastCompileResult.
            await ShowExportDialogAsync("Nothing to export", "Compile the document at least once before exporting a PDF.")
                .ConfigureAwait(true);
            return;
        }

        if (_pdfExportClient is not { } exportClient)
        {
            await ShowExportDialogAsync(
                    "PDF export tool not found",
                    _pdfToolMissingReason ?? "flashtex-pdf/flashtex-pdf-exact were not found.")
                .ConfigureAwait(true);
            return;
        }

        string? outputPath = await PickPdfSaveLocationAsync().ConfigureAwait(true);
        if (outputPath is null)
        {
            return; // User cancelled the save picker.
        }

        bool verify = commandId == CommandIds.ExportPdfViaRustWriter;
        PdfExportResult exportResult = await exportClient.ExportViaCompileResultAsync(result, outputPath, verify)
            .ConfigureAwait(true);

        if (exportResult.Succeeded)
        {
            await ShowExportDialogAsync("Export complete", $"Exported to {outputPath}").ConfigureAwait(true);
        }
        else
        {
            await ShowExportDialogAsync(
                    "Export failed",
                    exportResult.ErrorMessage ?? "The PDF export tool reported an unspecified failure.")
                .ConfigureAwait(true);
        }
    }

    /// <summary>
    /// "Export PDF (Exact, v2)": writes the PDF from the validated rendering-v2 display list
    /// the shell captured for the current revision, so glyphs are written by their original
    /// font glyph id with the real font program embedded — the same frame the Win2D preview
    /// is painting, not a re-measured approximation of it. Gated by
    /// <c>CommandRegistry</c>'s <c>NeedsDisplayList</c>; the guard below is for the case
    /// where the display list was dropped between the gate and this call.
    /// </summary>
    private async Task RunExactExportAsync()
    {
        if (_shell.LastDisplayList is not { } displayList)
        {
            await ShowExportDialogAsync(
                    "No rendering-v2 display list",
                    "This export needs a negotiated display-list-v2 frame for the current revision, and there "
                    + $"is none: {_shell.DisplayListRefusal ?? "no display list"}. "
                    + "The attached worker must accept the display-list-v2 layout capability — "
                    + "flashtex-render does, flashtex-compiler does not.")
                .ConfigureAwait(true);
            return;
        }
        if (_pdfExportClient is not { } exportClient)
        {
            await ShowExportDialogAsync("PDF export tool not found", _pdfToolMissingReason ?? "flashtex-pdf-exact was not found.")
                .ConfigureAwait(true);
            return;
        }

        string? outputPath = await PickPdfSaveLocationAsync().ConfigureAwait(true);
        if (outputPath is null)
        {
            return; // User cancelled the save picker.
        }

        // The exact writer embeds the font PROGRAMS, which the wire does not carry, so it is
        // pointed at the same content-addressed font directory the preview resolves from.
        PdfExportResult exportResult = await exportClient
            .ExportExactFromDisplayListAsync(displayList, outputPath, PreviewFontDirectories.Primary())
            .ConfigureAwait(true);

        await ShowExportDialogAsync(
                exportResult.Succeeded ? "Export complete" : "Export failed",
                exportResult.Succeeded
                    ? $"Exported to {outputPath}"
                    : exportResult.ErrorMessage ?? "The exact PDF export tool reported an unspecified failure.")
            .ConfigureAwait(true);
    }

    /// <summary>
    /// Shows the standard Win32 save-file dialog for an unpackaged app (needs
    /// <see cref="WindowNative.GetWindowHandle"/> + <see cref="InitializeWithWindow.Initialize"/>,
    /// since a <see cref="FileSavePicker"/> has no implicit owner window outside a packaged app).
    /// Returns null if the user cancels.
    /// </summary>
    private async Task<string?> PickPdfSaveLocationAsync()
    {
        var picker = new FileSavePicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            SuggestedFileName = SuggestedExportFileName(),
        };
        picker.FileTypeChoices.Add("PDF Document", new List<string> { ".pdf" });

        var windowHandle = WindowNative.GetWindowHandle(this);
        InitializeWithWindow.Initialize(picker, windowHandle);

        var file = await picker.PickSaveFileAsync();
        return file?.Path;
    }

    private string SuggestedExportFileName()
    {
        var activePath = _shell.ActiveDocumentPath;
        if (string.IsNullOrEmpty(activePath))
        {
            return DefaultExportBaseName;
        }

        var baseName = Path.GetFileNameWithoutExtension(activePath);
        return string.IsNullOrEmpty(baseName) ? DefaultExportBaseName : baseName;
    }

    /// <summary>A brief modal confirmation/failure dialog; export outcomes are infrequent enough that a
    /// transient status-bar message (as StatusBar.cs uses for compile status) would be too easy to miss,
    /// especially a failure the user specifically needs to see the reason for.</summary>
    private async Task ShowExportDialogAsync(string title, string message)
    {
        var dialog = new ContentDialog
        {
            Title = title,
            Content = message,
            CloseButtonText = "OK",
            XamlRoot = Content.XamlRoot,
        };
        await dialog.ShowAsync();
    }
}
