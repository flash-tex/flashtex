// name: MainWindow.xaml.cs
// purpose: The app shell's main window: composes the menu bar (MainWindow.Menu.cs),
//   toolbar (MainWindow.Toolbar.cs), tab strip (MainWindow.Tabs.cs) and status
//   bar (MainWindow.StatusBar.cs) around a real FlashTeX.Shell.ShellModel, and
//   wires the two pane splitters (GridColumnResizer.cs). Ported from the
//   DESIGN of apps/mac/Sources/FlashTeXMac/ContentView.swift + ShellChrome.swift
//   + DocumentTabBar.swift + FlashTeXMacApp.swift's `.commands{}` block,
//   adapted to WinUI3 idioms rather than copied verbatim: no WebView2 editor
//   or Win2D preview are wired yet (FlashTeX.Editor/FlashTeX.Preview are
//   out of scope for this milestone; their panes are placeholders), and there
//   is no project/file-system concept yet (FlashTeX.ProjectFiles is not
//   referenced), so the seeded document is in-memory only.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Shell;
using Microsoft.UI.Xaml;
using Windows.Graphics;

namespace FlashTeX.App;

public sealed partial class MainWindow : Window
{
    /// <summary>Mirrors the Mac original's minimum window size (ContentView.swift: "sidebar + editor + preview + Problems panel").</summary>
    private const int InitialWindowWidth = 1400;
    private const int InitialWindowHeight = 820;
    private const int InitialWindowX = 20;
    private const int InitialWindowY = 10;

    private const double ProjectTreeMinWidth = 140;
    private const double ProjectTreeMaxWidth = 480;
    private const double ProjectTreeDefaultWidth = 220;
    private const double EditorPaneMinWidth = 200;
    private const double EditorPaneMaxWidth = 1200;
    private const double EditorPaneDefaultWidth = 420;

    private const string SeedDocumentPath = "main.tex";
    private const string SeedDocumentText = "\\documentclass{article}\n\\begin{document}\nGood {unclosed and \\nosuchcommand{arg} here.\n\\end{document}\n";

    private readonly ShellModel _shell = new(editLedgerExecutablePath: EditLedgerToolLocator.FindExecutable());

    public MainWindow()
    {
        InitializeComponent();
        Title = "FlashTeX";
        AppWindow.Resize(new SizeInt32(InitialWindowWidth, InitialWindowHeight));
        AppWindow.Move(new PointInt32(InitialWindowX, InitialWindowY));

        BuildMenuBar();
        SetupTitleBar();
        BuildKeyboardAccelerators();
        BuildToolbar();
        WireTabStrip();
        WireStatusBar();
        WireExport();
        WireEditorPane();
        WirePreviewPane();
        WireProjectTree();
        WireProblemsPanel();
        WirePaneSplitters();

        // Command availability (Compile needs a worker, Save/Undo/Redo need an active
        // document, Export needs a compile result, ...) can change from several different
        // ShellModel operations, so re-evaluate it on any model or document-list change
        // rather than trying to enumerate every triggering path individually.
        _shell.PropertyChanged += (_, _) => RefreshCommandEnabledState();
        _shell.Documents.CollectionChanged += (_, _) => RefreshCommandEnabledState();

        Closed += OnWindowClosed;
        _ = StartupAsync();
    }

    /// <summary>Seeds one in-memory document and attaches the real compiler worker, then compiles once, so the demo loop has something to show without a file-open dialog (no FlashTeX.ProjectFiles yet).</summary>
    private async Task StartupAsync()
    {
        try
        {
            await _shell.OpenDocumentAsync(SeedDocumentPath, SeedText()).ConfigureAwait(true);

            if (CompilerLocator.FindWorker() is { } worker && File.Exists(worker.ExecutablePath))
            {
                _shell.AttachWorker(worker.ExecutablePath, onStderrLine: line => _shell.WorkerStatus = "stderr: " + line, arguments: worker.Arguments);
                await _shell.CompileNowAsync().ConfigureAwait(true);
            }
            else
            {
                _shell.WorkerStatus = "compiler not found at expected path";
            }
        }
        catch (Exception ex)
        {
            // Startup runs fire-and-forget from the constructor (no XAML element yet exists to
            // await it against); an unobserved exception here would otherwise silently leave the
            // status bar reading its "no worker attached" field initializer forever with no clue
            // why. Surface it directly in the status bar instead of only a debugger/log sink.
            _shell.WorkerStatus = "startup failed: " + ex;
        }

        RefreshCommandEnabledState();
    }

    /// <summary>
    /// The seeded document's text: <c>FLASHTEX_SEED_TEX</c>'s file when it is set and
    /// readable, else the built-in stub. This exists so a real, glyph-heavy fixture can be
    /// loaded for rendering verification without driving the file-open picker; it changes
    /// nothing when the variable is unset, which is every ordinary launch.
    /// </summary>
    private static string SeedText()
    {
        if (Environment.GetEnvironmentVariable("FLASHTEX_SEED_TEX") is { Length: > 0 } path && File.Exists(path))
        {
            try
            {
                return File.ReadAllText(path);
            }
            catch (IOException)
            {
                // Fall through to the built-in stub rather than failing startup.
            }
        }
        return SeedDocumentText;
    }

    private void WirePaneSplitters()
    {
        var savedLayout = PaneSettings.TryLoad();
        GridColumnResizer.Attach(LeftSplitter, ProjectTreeColumn, ProjectTreeMinWidth, ProjectTreeMaxWidth,
            savedLayout?.ProjectTreeWidth ?? ProjectTreeDefaultWidth,
            onDragCompleted: width => PaneSettings.Save((PaneSettings.TryLoad() ?? DefaultLayout) with { ProjectTreeWidth = width }));
        GridColumnResizer.Attach(CenterSplitter, EditorColumn, EditorPaneMinWidth, EditorPaneMaxWidth,
            savedLayout?.EditorWidth ?? EditorPaneDefaultWidth,
            onDragCompleted: width => PaneSettings.Save((PaneSettings.TryLoad() ?? DefaultLayout) with { EditorWidth = width }));
    }

    private static PaneLayout DefaultLayout => new(ProjectTreeDefaultWidth, EditorPaneDefaultWidth);

    private void OnWindowClosed(object sender, WindowEventArgs args)
    {
        foreach (var watcher in _watchersByDocument.Values)
        {
            watcher.Dispose();
        }
        _watchersByDocument.Clear();

        try
        {
            DisposeProjectFilesClientsAsync().GetAwaiter().GetResult();
            _projectSearchSession.DisposeAsync().GetAwaiter().GetResult();
            _shell.DetachWorkerAsync().GetAwaiter().GetResult();
        }
        catch (InvalidOperationException)
        {
            // Worker process already gone (e.g. it exited on its own); nothing left to detach.
        }
    }
}
