// name: MainWindow.Editor.cs
// purpose: Wires the shared EditorHost (WebView2 + CodeMirror 6) into the
//   editor pane's Border. EditorHost is created in code-behind rather than
//   declared in MainWindow.xaml because it takes the real ShellModel in its
//   constructor (no parameterless constructor for XAML to invoke). One
//   EditorHost instance lives for the lifetime of the window; it resyncs
//   itself to whichever document ShellModel.ActiveDocumentPath currently
//   names (see EditorHost.xaml.cs), so no per-tab wiring is needed here
//   beyond ShellModel already driving ActiveDocumentPath from the tab strip
//   (MainWindow.Tabs.cs).
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private EditorHost? _editorHost;

    private void WireEditorPane()
    {
        _editorHost = new EditorHost(_shell);
        EditorPaneHost.Child = _editorHost;
    }
}
