// name: MainWindow.ProjectFiles.cs
// purpose: Native open/save flows backed by flashtex-project-files. The helper owns
// rooted, no-follow reads and optimistic-concurrency saves; the WinUI window owns
// picker ownership and user-facing conflict feedback.

using FlashTeX.ProjectFiles;
using FlashTeX.Protocol.ProjectFilesV1;
using FlashTeX.Shell;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private sealed record OpenFileBinding(DocumentFilesClient Client, string Root, string RelativePath, string? Sha256);

    private readonly Dictionary<string, DocumentFilesClient> _fileClientsByRoot = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, OpenFileBinding> _fileBindingsByDocument = new(StringComparer.OrdinalIgnoreCase);
    private string? _activeProjectRoot;
    private string? _projectFilesToolMissingReason;

    private void WireProjectTree()
    {
        _shell.Documents.CollectionChanged += (_, _) => RebuildProjectTree();
        _shell.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName == nameof(ShellModel.ActiveDocumentPath))
            {
                RebuildProjectTree();
            }
        };
        WireDocumentWatchers();
        RebuildProjectTree();
    }

    private void RebuildProjectTree()
    {
        var panel = new StackPanel { Spacing = 2, Padding = new Thickness(6) };
        panel.Children.Add(new TextBlock { Text = "PROJECT", FontSize = 12, Opacity = 0.65, Margin = new Thickness(6, 4, 6, 6) });
        if (_shell.Documents.Count == 0)
        {
            panel.Children.Add(new TextBlock { Text = "No files open", Opacity = 0.65, Margin = new Thickness(6) });
        }
        foreach (var document in _shell.Documents)
        {
            string label = Path.GetFileName(document.Path);
            var button = new Button
            {
                Content = document.IsDirty ? label + " •" : label,
                HorizontalAlignment = HorizontalAlignment.Stretch,
                HorizontalContentAlignment = HorizontalAlignment.Left,
                Padding = new Thickness(8, 5, 8, 5),
                FontWeight = document.Path == _shell.ActiveDocumentPath
                    ? Microsoft.UI.Text.FontWeights.SemiBold
                    : Microsoft.UI.Text.FontWeights.Normal,
            };
            string path = document.Path;
            button.Click += (_, _) => _shell.SwitchActiveDocument(path);
            panel.Children.Add(button);
        }

        if (_activeProjectRoot is not null && ActiveDocumentOrNull() is { } active && _fileBindingsByDocument.TryGetValue(active.Path, out var binding))
        {
            foreach (var reference in ProjectIncludes.Scan(active.Text).Where(reference => reference.Literal))
            {
                foreach (string candidate in ProjectIncludes.Candidates(reference.Argument))
                {
                    if (_shell.Documents.Any(document => document.Path == candidate))
                    {
                        continue;
                    }
                    var include = new Button
                    {
                        Content = "↳ " + candidate,
                        HorizontalAlignment = HorizontalAlignment.Stretch,
                        HorizontalContentAlignment = HorizontalAlignment.Left,
                        Padding = new Thickness(16, 4, 8, 4),
                        Opacity = 0.8,
                    };
                    include.Click += (_, _) => _ = OpenProjectRelativeFileAsync(binding, candidate);
                    panel.Children.Add(include);
                    break; // candidate order is .tex then literal; opening probes both.
                }
            }
        }

        AppendOutlineSection(panel, ActiveDocumentOrNull());
        ProjectTreeHost.Child = panel;
    }

    private static bool IsProjectFileCommand(string id) =>
        id is CommandIds.OpenLatexFile or CommandIds.Save or CommandIds.SaveAs or CommandIds.NewFile;

    private bool TryStartProjectFileCommand(string commandId)
    {
        var command = CommandRegistry.All.First(c => c.Id == commandId);
        if (!command.CanExecute(_shell))
        {
            return false;
        }
        _ = RunProjectFileCommandAsync(commandId);
        return true;
    }

    private async Task RunProjectFileCommandAsync(string commandId)
    {
        try
        {
            switch (commandId)
            {
                case CommandIds.OpenLatexFile:
                    await OpenLatexFileAsync().ConfigureAwait(true);
                    break;
                case CommandIds.Save:
                    await SaveActiveFileAsync(saveAs: false).ConfigureAwait(true);
                    break;
                case CommandIds.SaveAs:
                    await SaveActiveFileAsync(saveAs: true).ConfigureAwait(true);
                    break;
                case CommandIds.NewFile:
                    await CreateNewFileAsync().ConfigureAwait(true);
                    break;
            }
        }
        catch (Exception ex)
        {
            await ShowProjectFileDialogAsync("File operation failed", ex.Message).ConfigureAwait(true);
        }
        finally
        {
            RefreshCommandEnabledState();
        }
    }

    private async Task OpenLatexFileAsync()
    {
        var picker = new FileOpenPicker { SuggestedStartLocation = PickerLocationId.DocumentsLibrary };
        picker.FileTypeFilter.Add(".tex");
        picker.FileTypeFilter.Add(".bib");
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        var file = await picker.PickSingleFileAsync();
        if (file is null)
        {
            return;
        }

        string selectedPath = file.Path;
        string root = Path.GetDirectoryName(selectedPath) ?? throw new InvalidOperationException("The selected file has no parent directory.");
        if (!await SetProjectRootForSaveAsync(root).ConfigureAwait(true))
        {
            return;
        }
        string documentPath = Path.GetFileName(selectedPath);
        var client = GetOrStartFileClient(root);
        var read = await client.ReadAsync(documentPath).ConfigureAwait(true);
        if (!read.Exists || read.Text is null)
        {
            throw new IOException($"The selected file no longer exists: {selectedPath}");
        }

        await _shell.OpenDocumentAsync(documentPath, read.Text).ConfigureAwait(true);
        _fileBindingsByDocument[documentPath] = new OpenFileBinding(client, root, read.Path, read.Sha256);
    }

    private async Task SaveActiveFileAsync(bool saveAs)
    {
        var document = ActiveDocumentOrThrow();
        if (!saveAs && _fileBindingsByDocument.TryGetValue(document.Path, out var binding))
        {
            await SaveBoundDocumentAsync(document, binding).ConfigureAwait(true);
            return;
        }

        var picker = new FileSavePicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            SuggestedFileName = Path.GetFileNameWithoutExtension(document.Path),
        };
        picker.FileTypeChoices.Add("LaTeX source", new List<string> { ".tex" });
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        var destination = await picker.PickSaveFileAsync();
        if (destination is null)
        {
            return;
        }

        string selectedPath = destination.Path;
        string root = Path.GetDirectoryName(selectedPath) ?? throw new InvalidOperationException("The selected location has no parent directory.");
        if (!await SetProjectRootForSaveAsync(root).ConfigureAwait(true))
        {
            return;
        }
        string destinationPath = Path.GetFileName(selectedPath);
        var client = GetOrStartFileClient(root);
        var onDisk = await client.ReadAsync(destinationPath).ConfigureAwait(true);
        var newBinding = new OpenFileBinding(client, root, destinationPath, onDisk.Sha256);
        var savedBinding = await SaveBoundDocumentAsync(document, newBinding).ConfigureAwait(true);
        if (savedBinding is null)
        {
            return;
        }

        if (!string.Equals(document.Path, destinationPath, StringComparison.OrdinalIgnoreCase))
        {
            await _shell.OpenDocumentAsync(destinationPath, document.Text).ConfigureAwait(true);
            _fileBindingsByDocument[destinationPath] = savedBinding;
            _shell.MarkDocumentSaved(destinationPath);
            _fileBindingsByDocument.Remove(document.Path);
            await _shell.CloseDocumentAsync(document.Path).ConfigureAwait(true);
        }
    }

    private async Task CreateNewFileAsync()
    {
        var picker = new FileSavePicker
        {
            SuggestedStartLocation = PickerLocationId.DocumentsLibrary,
            SuggestedFileName = "untitled",
        };
        picker.FileTypeChoices.Add("LaTeX source", new List<string> { ".tex" });
        InitializeWithWindow.Initialize(picker, WindowNative.GetWindowHandle(this));
        var destination = await picker.PickSaveFileAsync();
        if (destination is null)
        {
            return;
        }

        string selectedPath = destination.Path;
        string root = Path.GetDirectoryName(selectedPath) ?? throw new InvalidOperationException("The selected location has no parent directory.");
        if (!await EstablishProjectRootAsync(root).ConfigureAwait(true))
        {
            return;
        }
        string path = Path.GetFileName(selectedPath);
        var client = GetOrStartFileClient(root);
        var existing = await client.ReadAsync(path).ConfigureAwait(true);
        if (existing.Exists)
        {
            await ShowProjectFileDialogAsync("File already exists", "Choose a new filename rather than replacing an existing project source file.").ConfigureAwait(true);
            return;
        }

        const string template = "\\documentclass{article}\n\\begin{document}\n\n\\end{document}\n";
        var binding = new OpenFileBinding(client, root, path, null);
        var transient = new ShellDocument(path, template, 1, false);
        var savedBinding = await SaveBoundDocumentAsync(transient, binding).ConfigureAwait(true);
        if (savedBinding is null)
        {
            return;
        }
        await _shell.OpenDocumentAsync(path, template).ConfigureAwait(true);
        _fileBindingsByDocument[path] = savedBinding;
        _shell.MarkDocumentSaved(path);
    }

    private async Task<OpenFileBinding?> SaveBoundDocumentAsync(ShellDocument document, OpenFileBinding binding)
    {
        Expected expected = binding.Sha256 is { } hash ? Expected.Hash(hash) : Expected.NewFile;
        SaveOutcome outcome = await binding.Client.SaveAsync(binding.RelativePath, document.Text, expected).ConfigureAwait(true);
        if (outcome is SaveOutcome.Conflict conflict)
        {
            await ShowProjectFileDialogAsync("Save conflict", "The file changed on disk and was not overwritten (" + conflict.Details.Kind + ").").ConfigureAwait(true);
            return null;
        }

        var saved = (SaveOutcome.Saved)outcome;
        var updated = binding with { Sha256 = saved.Receipt.Sha256 };
        _fileBindingsByDocument[document.Path] = updated;
        _shell.MarkDocumentSaved(document.Path);
        return updated;
    }

    private DocumentFilesClient GetOrStartFileClient(string root)
    {
        if (_fileClientsByRoot.TryGetValue(root, out var existing))
        {
            return existing;
        }

        string? executable = ProjectFilesToolLocator.FindExecutable();
        if (executable is null || !File.Exists(executable))
        {
            _projectFilesToolMissingReason = "flashtex-project-files.exe was not found. Build it with `cargo build --release` in crates/project-files.";
            throw new FileNotFoundException(_projectFilesToolMissingReason, executable);
        }
        var client = DocumentFilesClient.Start(executable, root, line => _shell.WorkerStatus = "file helper: " + line);
        _fileClientsByRoot.Add(root, client);
        return client;
    }

    private async Task OpenProjectRelativeFileAsync(OpenFileBinding sourceBinding, string candidate)
    {
        try
        {
            var read = await sourceBinding.Client.ReadAsync(candidate).ConfigureAwait(true);
            if (!read.Exists || read.Text is null)
            {
                return;
            }
            await _shell.OpenDocumentAsync(read.Path, read.Text).ConfigureAwait(true);
            _fileBindingsByDocument[read.Path] = new OpenFileBinding(sourceBinding.Client, sourceBinding.Root, read.Path, read.Sha256);
        }
        catch (Exception ex)
        {
            await ShowProjectFileDialogAsync("Could not open included file", ex.Message).ConfigureAwait(true);
        }
    }

    private async Task<bool> EstablishProjectRootAsync(string root)
    {
        if (string.Equals(_activeProjectRoot, root, StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }
        if (_shell.Documents.Any(document => document.IsDirty))
        {
            await ShowProjectFileDialogAsync("Save your changes first", "Opening a different project would replace the current unsaved workspace.").ConfigureAwait(true);
            return false;
        }
        foreach (string path in _shell.Documents.Select(document => document.Path).ToList())
        {
            await _shell.CloseDocumentAsync(path).ConfigureAwait(true);
        }
        _fileBindingsByDocument.Clear();
        _activeProjectRoot = root;
        _shell.ProjectRoot = root;
        return true;
    }

    /// <summary>
    /// Save As/New File may be the operation that turns the seeded in-memory document
    /// into a real project. In that case retain the live buffer while establishing its
    /// first root; switching an already-rooted workspace through Save As is deliberately
    /// refused so a failed optimistic save can never discard an open document.
    /// </summary>
    private async Task<bool> SetProjectRootForSaveAsync(string root)
    {
        if (_activeProjectRoot is null)
        {
            _activeProjectRoot = root;
            _shell.ProjectRoot = root;
            return true;
        }
        if (string.Equals(_activeProjectRoot, root, StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }
        await ShowProjectFileDialogAsync("Save As stays in this project", "Choose a location under the currently opened project, or open the other project first.").ConfigureAwait(true);
        return false;
    }

    private ShellDocument? ActiveDocumentOrNull() => _shell.Documents.FirstOrDefault(d => d.Path == _shell.ActiveDocumentPath);

    private ShellDocument ActiveDocumentOrThrow() => ActiveDocumentOrNull()
        ?? throw new InvalidOperationException("There is no active document to save.");

    private async Task ShowProjectFileDialogAsync(string title, string message)
    {
        var dialog = new ContentDialog { Title = title, Content = message, CloseButtonText = "OK", XamlRoot = Content.XamlRoot };
        await dialog.ShowAsync();
    }

    private async Task DisposeProjectFilesClientsAsync()
    {
        foreach (var client in _fileClientsByRoot.Values)
        {
            await client.DisposeAsync().ConfigureAwait(true);
        }
        _fileClientsByRoot.Clear();
        _fileBindingsByDocument.Clear();
    }
}
