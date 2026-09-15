// name: MainWindow.Search.cs
// purpose: Project-wide literal search (CommandIds.ProjectSearch): a simple
//   ContentDialog (matching the existing MainWindow.CommandPalette.cs pattern
//   rather than inventing a second UI shape) that searches via
//   FlashTeX.Ipc.ProjectSearchClient/PreviewControllerClient.search_literal
//   over the active project's ProjectSessionClient. A result switches to (or
//   opens) its file and reveals the match through the same
//   EditorHost.RevealSourceRange navigation MainWindow.Preview.cs already uses.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Ipc;
using FlashTeX.Protocol.PreviewControllerV1;
using FlashTeX.Protocol.ProjectFilesV1;
using FlashTeX.Protocol.RuntimeV1;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.System;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private const int SearchMaxMatches = 200;

    private readonly ProjectSessionClient _projectSearchSession = new();
    private bool _projectSearchOpen;

    private bool TryStartProjectSearch()
    {
        if (_projectSearchOpen)
        {
            return true;
        }
        if (_activeProjectRoot is null || BoundEntryPathForSession() is null)
        {
            _ = ShowProjectFileDialogAsync("No project to search", "Open or save a .tex file first so there is a project root to search across.");
            return true;
        }
        _ = ShowProjectSearchAsync();
        return true;
    }

    /// <summary>The document to seed the shared preview-controller session's project graph from: the active bound document, else any other bound one.</summary>
    private string? BoundEntryPathForSession()
    {
        if (ActiveDocumentOrNull() is { } active && _fileBindingsByDocument.ContainsKey(active.Path))
        {
            return active.Path;
        }
        return _fileBindingsByDocument.Keys.FirstOrDefault();
    }

    private async Task ShowProjectSearchAsync()
    {
        _projectSearchOpen = true;
        try
        {
            var query = new TextBox { PlaceholderText = "Literal text to find…", Margin = new Thickness(0, 0, 0, 8) };
            var status = new TextBlock { Opacity = 0.7, Margin = new Thickness(0, 0, 0, 8) };
            var list = new ListView { SelectionMode = ListViewSelectionMode.Single, MaxHeight = 420, MinWidth = 560 };
            var content = new StackPanel();
            content.Children.Add(query);
            content.Children.Add(status);
            content.Children.Add(list);

            var dialog = new ContentDialog
            {
                Title = "Find in Project",
                Content = content,
                PrimaryButtonText = "Search",
                CloseButtonText = "Close",
                XamlRoot = Content.XamlRoot,
                DefaultButton = ContentDialogButton.Primary,
            };

            async Task RunSearchAsync()
            {
                string literal = query.Text;
                if (literal.Length == 0)
                {
                    return;
                }
                list.Items.Clear();
                status.Text = "Searching…";
                try
                {
                    SearchResult result = await RunProjectSearchAsync(literal).ConfigureAwait(true);
                    foreach (SourceSpan match in result.Matches)
                    {
                        list.Items.Add(new ListViewItem { Content = $"{match.Path}  @ bytes {match.StartByte}-{match.EndByte}", Tag = match });
                    }
                    status.Text = result.Termination == SearchTermination.complete
                        ? $"{result.Matches.Count} match{(result.Matches.Count == 1 ? "" : "es")}"
                        : $"{result.Matches.Count} match{(result.Matches.Count == 1 ? "" : "es")} (stopped early: {result.Termination})";
                }
                catch (Exception ex)
                {
                    status.Text = "Search failed: " + ExplainSessionFailure(ex);
                }
            }

            void OpenSelected()
            {
                if (list.SelectedItem is not ListViewItem { Tag: SourceSpan span })
                {
                    return;
                }
                dialog.Hide();
                _ = OpenSearchResultAsync(span);
            }

            dialog.PrimaryButtonClick += (sender, args) =>
            {
                args.Cancel = true; // keep the dialog open across a search; only Close dismisses it
                _ = RunSearchAsync();
            };
            list.DoubleTapped += (_, _) => OpenSelected();
            query.KeyDown += (_, args) =>
            {
                if (args.Key == VirtualKey.Enter)
                {
                    _ = RunSearchAsync();
                    args.Handled = true;
                }
            };

            _ = DispatcherQueue.TryEnqueue(() => query.Focus(FocusState.Programmatic));
            await dialog.ShowAsync();
        }
        finally
        {
            _projectSearchOpen = false;
        }
    }

    private async Task<SearchResult> RunProjectSearchAsync(string literal)
    {
        string root = _activeProjectRoot ?? throw new InvalidOperationException("no active project root");
        string entryPath = BoundEntryPathForSession() ?? throw new InvalidOperationException("no bound document to seed the project graph from");

        PreviewControllerClient controller = await _projectSearchSession
            .GetOrStartAsync(root, entryPath, line => _shell.WorkerStatus = "preview-controller: " + line)
            .ConfigureAwait(true);
        ProjectStatusResult status = await controller
            .ProjectStatusAsync($"status-{Guid.NewGuid():N}")
            .ConfigureAwait(true);
        var search = new ProjectSearchClient(controller);
        return await search.SearchAsync(status.SourceVersions, literal, SearchMaxMatches).ConfigureAwait(true);
    }

    private async Task OpenSearchResultAsync(SourceSpan span)
    {
        try
        {
            if (!_shell.Documents.Any(d => d.Path == span.Path))
            {
                if (_fileBindingsByDocument.Values.FirstOrDefault() is not { } anyBinding)
                {
                    return;
                }
                ReadPayload read = await anyBinding.Client.ReadAsync(span.Path).ConfigureAwait(true);
                if (!read.Exists || read.Text is null)
                {
                    await ShowProjectFileDialogAsync("Could not open result", $"“{span.Path}” no longer exists.").ConfigureAwait(true);
                    return;
                }
                await _shell.OpenDocumentAsync(span.Path, read.Text).ConfigureAwait(true);
                _fileBindingsByDocument[span.Path] = new OpenFileBinding(anyBinding.Client, anyBinding.Root, read.Path, read.Sha256);
                ReconcileDocumentWatchers();
            }
            _shell.SwitchActiveDocument(span.Path);
            _editorHost?.RevealSourceRange(new SourceRange(span.Path, span.StartByte, span.EndByte));
        }
        catch (Exception ex)
        {
            await ShowProjectFileDialogAsync("Could not open result", ex.Message).ConfigureAwait(true);
        }
    }

    /// <summary>A short, user-facing reason for a project-session failure, without leaking a raw stack trace.</summary>
    private static string ExplainSessionFailure(Exception ex) => ex switch
    {
        PreviewControllerToolMissingException missing => missing.Message,
        PreviewControllerErrorException helperError => helperError.Message,
        _ => ex.Message,
    };
}
