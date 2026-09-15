// name: EditorHost.xaml.cs
// purpose: Lifecycle and ShellModel wiring for the shared editor-pane WebView2
//   control: initializes CoreWebView2 against the built web/dist host page,
//   mirrors ShellModel's active document/diagnostics/theme/font size into it,
//   and applies edits the JS side reports back into ShellModel. See
//   EditorHost.Bridge.cs for the wire envelope shapes and incoming-message
//   dispatch, and EditorHost.Completion.cs for the native completion/
//   signature-help Flyouts. One instance is shared by every open tab (per
//   FlashTeX.App.csproj's architecture note): switching tabs re-sends a full
//   `set_document` for the newly active path rather than this control (or the
//   JS page it hosts) keeping a separate CodeMirror state per tab -- the wire
//   protocol (FlashTeX.Editor/web/src/bridge.ts) has no document/path
//   identifier on any message, so multiple concurrent CM6 states could not be
//   addressed individually without extending that already-tested contract;
//   deferred, see HANDOFF.md's "Known gotchas" for this milestone.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Collections.Specialized;
using System.ComponentModel;
using FlashTeX.Protocol;
using FlashTeX.Shell;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.Web.WebView2.Core;

namespace FlashTeX.App;

public sealed partial class EditorHost : UserControl
{
    private const string VirtualHostName = "flashtex-editor.invalid";
    private static readonly Uri HomePage = new($"https://{VirtualHostName}/index.html");

    private readonly ShellModel _shell;

    /// <summary>
    /// True whenever this host is itself the source of a <see cref="ShellModel.Documents"/>
    /// change (an <c>edit_made</c> from JS just applied via <see cref="ShellModel.UpdateDocumentText"/>)
    /// -- guards <see cref="OnDocumentsChanged"/> against re-sending that same change back to JS
    /// as a resync.
    /// </summary>
    private bool _applyingRemoteEdit;

    /// <summary>
    /// This host's best-known mirror of the active document's text: what the JS/CM6 side
    /// actually has, as opposed to <c>_shell.Documents</c> (the source of truth, which can race
    /// a moment ahead while this host is itself mid-update). Byte-offset math for both incoming
    /// <c>edit_made</c> messages and outgoing native-originated edits is always against this
    /// field, never against <c>_shell.Documents</c> directly.
    /// </summary>
    private string _mirroredText = "";

    private string? _mirroredPath;
    private bool _webViewReady;

    public EditorHost(ShellModel shell)
    {
        _shell = shell;
        InitializeComponent();
        Loaded += OnLoaded;
        Unloaded += OnUnloaded;
        ActualThemeChanged += (_, _) => SendTheme();
    }

    private async void OnLoaded(object sender, RoutedEventArgs e)
    {
        _shell.PropertyChanged += OnShellPropertyChanged;
        _shell.Documents.CollectionChanged += OnDocumentsChanged;
        await InitializeWebViewAsync().ConfigureAwait(true);
    }

    private void OnUnloaded(object sender, RoutedEventArgs e)
    {
        _shell.PropertyChanged -= OnShellPropertyChanged;
        _shell.Documents.CollectionChanged -= OnDocumentsChanged;
    }

    private async Task InitializeWebViewAsync()
    {
        try
        {
            await WebView.EnsureCoreWebView2Async().AsTask().ConfigureAwait(true);
        }
        catch (Exception ex)
        {
            // Most commonly: the WebView2 Runtime is not installed on this machine. Surface it
            // in the pane itself rather than leaving a silent blank control.
            StatusText.Text = "Editor unavailable: " + ex.Message;
            return;
        }

        string? webRoot = FindEditorWebDistDirectory();
        if (webRoot is null)
        {
            StatusText.Text = "Editor unavailable: web/dist build output not found "
                + "(run `npm run build` under src/FlashTeX.Editor/web).";
            return;
        }

        WebView.CoreWebView2.SetVirtualHostNameToFolderMapping(VirtualHostName, webRoot, CoreWebView2HostResourceAccessKind.Deny);
        WebView.CoreWebView2.WebMessageReceived += OnWebMessageReceived;
        WebView.CoreWebView2.NavigationCompleted += OnNavigationCompleted;
        WebView.Source = HomePage;
    }

    private void OnNavigationCompleted(CoreWebView2 sender, CoreWebView2NavigationCompletedEventArgs args)
    {
        if (!args.IsSuccess)
        {
            StatusText.Text = $"Editor failed to load (WebErrorStatus.{args.WebErrorStatus}).";
            return;
        }
        StatusText.Text = "";
        _webViewReady = true;
        SyncActiveDocumentToWebView(forceFullResync: true);
        SendTheme();
        SendFontSize();
    }

    /// <summary>
    /// Finds the built web/dist directory: first next to the running executable (EditorWeb/,
    /// see FlashTeX.App.csproj's copy-to-output item), then by walking up to the repo root and
    /// looking under src/FlashTeX.Editor/web/dist -- the normal `dotnet run` development loop,
    /// where the csproj's copy-to-output-directory item only fires if dist already existed at
    /// build time (it is not restored/rebuilt automatically by `dotnet build`).
    /// </summary>
    private static string? FindEditorWebDistDirectory()
    {
        string besideExecutable = Path.Combine(AppContext.BaseDirectory, "EditorWeb");
        if (File.Exists(Path.Combine(besideExecutable, "index.html")))
        {
            return besideExecutable;
        }

        if (CompilerLocator.FindRepoRoot(new DirectoryInfo(AppContext.BaseDirectory)) is { } repoRoot)
        {
            string underRepo = Path.Combine(repoRoot.FullName, "apps", "windows", "src", "FlashTeX.Editor", "web", "dist");
            if (File.Exists(Path.Combine(underRepo, "index.html")))
            {
                return underRepo;
            }
        }
        return null;
    }

    private void OnShellPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        // ShellModel raises PropertyChanged synchronously on whatever thread called the
        // property's setter -- normally already the UI thread for this app (HANDOFF.md), but
        // this dispatches defensively anyway: touching CoreWebView2 off the UI thread is exactly
        // the class of bug the status bar's "no worker attached" regression turned out to be.
        DispatcherQueue.TryEnqueue(() =>
        {
            switch (e.PropertyName)
            {
                case nameof(ShellModel.ActiveDocumentPath):
                    SyncActiveDocumentToWebView(forceFullResync: true);
                    break;
                case nameof(ShellModel.Diagnostics):
                    SendDiagnostics();
                    break;
                case nameof(ShellModel.EditorFontSize):
                    SendFontSize();
                    break;
            }
        });
    }

    private void OnDocumentsChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (_applyingRemoteEdit)
        {
            return; // this host's own edit_made -> UpdateDocumentText round trip; JS already has it.
        }
        DispatcherQueue.TryEnqueue(() => SyncActiveDocumentToWebView(forceFullResync: false));
    }

    /// <summary>
    /// Brings the WebView2/CM6 side back in line with ShellModel's active document: a full
    /// <c>set_document</c> when the active path itself changed (tab switch) or <paramref
    /// name="forceFullResync"/> is set, or when the active document's text differs from this
    /// host's mirror for a reason other than this host's own edit_made (undo/redo via the edit
    /// ledger, a future capture/quick-fix insertion applied directly to ShellModel.Documents,
    /// ...). CM6 undo history and cursor position are lost on a full resync -- acceptable for
    /// tab switches (a fresh CM6 state per newly-shown tab is this milestone's documented
    /// limitation, see this file's header) but coarser than ideal for undo/redo; a byte-precise
    /// `apply_edit` there would need this host to compute the same diff the edit ledger already
    /// produced, left as future work.
    /// </summary>
    private void SyncActiveDocumentToWebView(bool forceFullResync)
    {
        if (!_webViewReady)
        {
            return;
        }
        ShellDocument? active = ActiveDocument();
        if (active is null)
        {
            return;
        }
        bool pathChanged = active.Path != _mirroredPath;
        if (!forceFullResync && !pathChanged && active.Text == _mirroredText)
        {
            return;
        }
        _mirroredPath = active.Path;
        _mirroredText = active.Text;
        SendSetDocument(active.Text, active.Revision);
        SendDiagnostics();
    }

    private ShellDocument? ActiveDocument()
    {
        foreach (ShellDocument document in _shell.Documents)
        {
            if (document.Path == _shell.ActiveDocumentPath)
            {
                return document;
            }
        }
        return null;
    }

    /// <summary>
    /// Applies a native-originated change (a completion insertion today; a future quick fix or
    /// similar) to both ShellModel (the source of truth) and the live CM6 view (via
    /// `apply_edit`), so the two never disagree about what the document contains.
    /// </summary>
    private void ApplyNativeEdit(int startByte, int endByte, string replacement)
    {
        if (_mirroredPath is not { } path)
        {
            return;
        }
        (int Utf16Start, int Utf16End)? range = ByteOffsets.Utf16RangeForUtf8Bytes(_mirroredText, startByte, endByte);
        if (range is not (int utf16Start, int utf16End))
        {
            return;
        }
        string newText = _mirroredText[..utf16Start] + replacement + _mirroredText[utf16End..];

        _applyingRemoteEdit = true;
        try
        {
            _shell.UpdateDocumentText(path, newText);
        }
        finally
        {
            _applyingRemoteEdit = false;
        }
        _mirroredText = newText;

        int newRevision = ActiveDocument()?.Revision ?? 0;
        SendApplyEdit(startByte, endByte, replacement, newRevision);
    }
}
