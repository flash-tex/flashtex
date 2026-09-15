// name: ShellModel.cs
// purpose: The editor/preview shell's application state. Composes the
//   document/tab lifecycle (ShellModel.Documents.cs), compile-pipeline
//   orchestration over a WorkerClient (ShellModel.Compile.cs), and durable
//   undo/redo over a per-document EditLedgerClient (ShellModel.EditLedger.cs)
//   with a throttled ShellChrome mirror for the window chrome. Ported from the
//   DESIGN of apps/mac/Sources/FlashTeXMac/ShellModel.swift and
//   ShellModel+Controller.swift, adapted rather than transliterated: this port
//   has no project/file-system concept yet (FlashTeX.ProjectFiles is not wired
//   in), so every open document is an independent in-memory tab rather than
//   members of one project with an entry document; and compiles are
//   serialized one at a time (see ShellModel.Compile.cs) rather than
//   correlated by revision/capability-switch, since there is no concurrent-
//   request race to resolve yet. Bridge/Nearby orchestration, export-session/
//   PDF wiring and display-candidate admission remain out of scope here.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Collections.ObjectModel;
using System.ComponentModel;
using CommunityToolkit.Mvvm.ComponentModel;

namespace FlashTeX.Shell;

/// <summary>
/// One open document's local editing state. The wire protocol only carries
/// path and text (<see cref="FlashTeX.Protocol.RuntimeV1.Document"/>);
/// revision and dirty are shell-side bookkeeping the protocol type has no
/// need for, so this wraps rather than duplicates it. <see cref="Revision"/>
/// is a shell-local edit counter (bumped by <see cref="ShellModel.UpdateDocumentText"/>
/// and by an applied undo/redo), unrelated to the edit-ledger's own
/// <c>ulong</c> revision (<see cref="FlashTeX.Protocol.EditLedgerV1.EditLedgerDocument.Revision"/>),
/// which <see cref="ShellModel"/> tracks separately per path.
/// </summary>
public sealed record ShellDocument(string Path, string Text, int Revision, bool IsDirty)
{
    /// <summary>The wire-protocol shape (path + text only) for a compile/save request.</summary>
    public FlashTeX.Protocol.RuntimeV1.Document ToProtocolDocument() => new(Path, Text);
}

/// <summary>Keys <see cref="ShellModel"/> registers on its <see cref="ShellModel.Chrome"/>.</summary>
public static class ShellChromeKeys
{
    public const string WordCount = "wordCount";
    public const string ActiveDocumentDirty = "activeDocumentDirty";
    public const string HasCompileResult = "hasCompileResult";
}

public partial class ShellModel : ObservableObject
{
    private const double DefaultEditorFontSizePt = 13.0;

    private readonly IChromeScheduler _compileScheduler;
    private readonly TimeSpan _compileDebounceInterval;
    private readonly string? _editLedgerExecutablePath;

    [ObservableProperty]
    private string? _activeDocumentPath;

    [ObservableProperty]
    private int _wordCount;

    [ObservableProperty]
    private double _editorFontSize = DefaultEditorFontSizePt;

    /// <summary>
    /// Absolute root of the active project, when the host has one. It is supplied to the
    /// compiler only for project-root-dependent resources such as \includegraphics; source
    /// document paths themselves remain project-relative protocol identifiers.
    /// </summary>
    [ObservableProperty]
    private string? _projectRoot;

    /// <summary>Every open document, in tab order.</summary>
    public ObservableCollection<ShellDocument> Documents { get; } = new();

    /// <summary>The throttled chrome mirror this model keeps a couple of fields on, as a composition demonstration.</summary>
    public ShellChrome Chrome { get; }

    /// <param name="chrome">Defaults to a real throttled mirror; tests inject one built on a fake scheduler.</param>
    /// <param name="compileScheduler">
    /// Schedules the debounced auto-compile, independent of <paramref name="chrome"/>'s own
    /// scheduler (the two windows serve different purposes). Defaults to a real one-shot
    /// timer; tests inject a <c>ManualChromeScheduler</c> to drive debouncing deterministically.
    /// </param>
    /// <param name="compileDebounceInterval">Defaults to <see cref="DefaultCompileDebounceInterval"/>.</param>
    /// <param name="editLedgerExecutablePath">
    /// Path to the <c>flashtex-edit-ledger</c> binary. When null, documents open without a
    /// durable undo/redo store: <see cref="UndoActiveDocumentAsync"/>/<see cref="RedoActiveDocumentAsync"/>
    /// simply have nothing to call for them (a no-op), matching this port's "in-memory only for
    /// now" scope for anything that would otherwise need real project/store-path management.
    /// </param>
    public ShellModel(
        ShellChrome? chrome = null,
        IChromeScheduler? compileScheduler = null,
        TimeSpan? compileDebounceInterval = null,
        string? editLedgerExecutablePath = null)
    {
        Chrome = chrome ?? new ShellChrome();
        _compileScheduler = compileScheduler ?? new RealTimeChromeScheduler();
        _compileDebounceInterval = compileDebounceInterval ?? DefaultCompileDebounceInterval;
        _editLedgerExecutablePath = editLedgerExecutablePath;

        Chrome.Register(ShellChromeKeys.WordCount, () => WordCount);
        Chrome.Register(ShellChromeKeys.ActiveDocumentDirty, () => ActiveDocument?.IsDirty ?? false);
        Chrome.Register(ShellChromeKeys.HasCompileResult, () => HasCompileResult);

        PropertyChanged += OnModelPropertyChanged;
        Documents.CollectionChanged += (_, _) => Chrome.NotifyPossibleChange();
    }

    private ShellDocument? ActiveDocument => Documents.FirstOrDefault(d => d.Path == ActiveDocumentPath);

    private void OnModelPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName is nameof(WordCount) or nameof(ActiveDocumentPath) or nameof(HasCompileResult))
        {
            Chrome.NotifyPossibleChange();
        }
    }
}
