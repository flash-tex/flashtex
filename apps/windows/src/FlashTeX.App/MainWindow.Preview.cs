// name: MainWindow.Preview.cs
// purpose: Chooses between the two preview renderers and keeps both fed. When the shell
// holds a validated rendering-v2 display list for the current revision the Win2D precision
// renderer (PreviewV2Host) is shown; otherwise — no v2 support in the attached worker, the
// capability declined, validation refused the frame, or a font did not resolve — the
// runtime-v1 renderer (PreviewHost) stays, unchanged. Both raise the same SourceRequested
// event, so click-to-source routes through one path.

using FlashTeX.Preview;
using FlashTeX.Protocol.RuntimeV1;
using FlashTeX.Shell;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private PreviewHost? _previewHost;
    private PreviewV2Host? _previewV2Host;
    private FontFileStore? _previewFonts;

    /// <summary>Why the v2 renderer is not currently showing, for the status bar; null when it is.</summary>
    internal string? PreviewV2Status { get; private set; }

    private void WirePreviewPane()
    {
        _previewHost = new PreviewHost();
        _previewHost.SourceRequested += OnPreviewSourceRequested;
        _previewV2Host = new PreviewV2Host();
        _previewV2Host.SourceRequested += OnPreviewSourceRequested;
        PreviewPaneHost.Child = _previewHost;
        _shell.PropertyChanged += (_, e) =>
        {
            // RenderGeneration, not HasCompileResult: a bool that is already true raises no
            // change notification, so the pane would refresh exactly once per session.
            if (e.PropertyName == nameof(ShellModel.RenderGeneration))
            {
                DispatcherQueue.TryEnqueue(RefreshPreview);
            }
        };
        RefreshPreview();
    }

    private void RefreshPreview()
    {
        if (TryShowDisplayListV2())
        {
            return;
        }
        _previewV2Host?.Clear();
        PreviewPaneHost.Child = _previewHost;
        _previewHost?.Display(_shell.LastCompileResult?.Pages);
        if (_shell.LastDisplayList is not null && PreviewV2Status is { } refusal)
        {
            // A frame arrived and validated on the wire but could not be PAINTED exactly
            // (typically an unresolvable font). Say so instead of quietly showing v1.
            _shell.WorkerStatus += " · v1 fallback: " + refusal;
        }
    }

    /// <summary>
    /// Shows the v2 frame if there is one and it prepares cleanly. Returns false (leaving
    /// <see cref="PreviewV2Status"/> explaining why) for every other case, including a
    /// refusal — a display list that cannot be painted exactly is never painted partially.
    /// </summary>
    private bool TryShowDisplayListV2()
    {
        if (_previewV2Host is not { } host || _shell.LastDisplayList is not { } list)
        {
            PreviewV2Status = _shell.DisplayListRefusal ?? "no display list";
            return false;
        }
        _previewFonts ??= new FontFileStore(PreviewFontDirectories.All());
        if (host.TryDisplay(list, _previewFonts) is { } refusal)
        {
            PreviewV2Status = refusal;
            return false;
        }
        PreviewV2Status = null;
        PreviewPaneHost.Child = host;
        return true;
    }

    private void OnPreviewSourceRequested(object? sender, SourceRange source)
    {
        if (_shell.SwitchActiveDocument(source.Path))
        {
            _editorHost?.RevealSourceRange(source);
        }
    }
}
