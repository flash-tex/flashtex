// name: MainWindow.CitationRename.cs
// purpose: Citation-key rename flow (CommandIds.CitationRename): a ContentDialog
//   asking for the existing and new citation key, planned via the shared
//   ProjectSessionClient's PreviewControllerClient.PlanCitationRenameAsync and
//   validated with FlashTeX.Editor.CitationRename.ParsePlan (schema/kind checks,
//   snapshot/version agreement, nonoverlapping-edit validation — the same pure
//   logic the Mac original ported), then applied across whichever documents the
//   plan touches. Applying only edits ShellModel's in-memory buffers (opening a
//   not-yet-open touched file first): nothing is written to disk automatically,
//   matching this app's normal "dirty until Save" model and DocumentFilesClient's
//   own hash-checked save conflict handling.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;
using FlashTeX.Editor;
using FlashTeX.Ipc;
using FlashTeX.Protocol;
using FlashTeX.Protocol.PreviewControllerV1;
using FlashTeX.Protocol.ProjectFilesV1;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private bool _citationRenameOpen;

    private bool TryStartCitationRename()
    {
        if (_citationRenameOpen)
        {
            return true;
        }
        if (_activeProjectRoot is null || BoundEntryPathForSession() is null)
        {
            _ = ShowProjectFileDialogAsync("No project for citation rename", "Open or save a .tex file first so there is a project to rename across.");
            return true;
        }
        _ = ShowCitationRenameAsync();
        return true;
    }

    private async Task ShowCitationRenameAsync()
    {
        _citationRenameOpen = true;
        try
        {
            var oldKeyBox = new TextBox { PlaceholderText = "e.g. smith2020" };
            var newKeyBox = new TextBox { PlaceholderText = "New key", Margin = new Thickness(0, 8, 0, 0) };
            var status = new TextBlock { Opacity = 0.8, TextWrapping = TextWrapping.Wrap, Margin = new Thickness(0, 10, 0, 0) };
            var content = new StackPanel { MinWidth = 380 };
            content.Children.Add(new TextBlock { Text = "Existing citation key" });
            content.Children.Add(oldKeyBox);
            content.Children.Add(new TextBlock { Text = "New citation key" });
            content.Children.Add(newKeyBox);
            content.Children.Add(status);

            var dialog = new ContentDialog
            {
                Title = "Rename Citation",
                Content = content,
                PrimaryButtonText = "Plan Rename",
                CloseButtonText = "Cancel",
                XamlRoot = Content.XamlRoot,
                DefaultButton = ContentDialogButton.Primary,
            };

            CitationRename.Plan? pendingPlan = null;

            dialog.PrimaryButtonClick += (sender, args) =>
            {
                args.Cancel = true; // two clicks: first plans (reviewed), second applies
                if (pendingPlan is null)
                {
                    _ = PlanCitationRenameAsync(oldKeyBox.Text.Trim(), newKeyBox.Text.Trim(), status, dialog, plan => pendingPlan = plan);
                }
                else
                {
                    CitationRename.Plan planToApply = pendingPlan;
                    pendingPlan = null;
                    dialog.Hide();
                    _ = ApplyCitationRenameAsync(planToApply);
                }
            };

            await dialog.ShowAsync();
        }
        finally
        {
            _citationRenameOpen = false;
        }
    }

    private async Task PlanCitationRenameAsync(string oldKey, string newKey, TextBlock status, ContentDialog dialog, Action<CitationRename.Plan> onAccepted)
    {
        if (oldKey.Length == 0)
        {
            status.Text = "Enter the existing citation key.";
            return;
        }
        if (CitationRename.KeyProblem(newKey) is { } problem)
        {
            status.Text = "Cannot rename: " + problem;
            return;
        }

        status.Text = "Planning…";
        try
        {
            string root = _activeProjectRoot ?? throw new InvalidOperationException("no active project root");
            string entryPath = BoundEntryPathForSession() ?? throw new InvalidOperationException("no bound document to seed the project graph from");
            PreviewControllerClient controller = await _projectSearchSession
                .GetOrStartAsync(root, entryPath, line => _shell.WorkerStatus = "preview-controller: " + line)
                .ConfigureAwait(true);
            ProjectStatusResult projectStatus = await controller
                .ProjectStatusAsync($"status-{Guid.NewGuid():N}")
                .ConfigureAwait(true);

            var payload = new PlanCitationRenamePayload(
                projectStatus.SourceVersions, projectStatus.MembershipGeneration, PreviewControllerProtocol.MaxPlanBytes, oldKey, newKey);
            PlanResult reply = await controller.PlanCitationRenameAsync($"plan-{Guid.NewGuid():N}", payload).ConfigureAwait(true);

            CitationRename.PlanResult parsed = CitationRename.ParsePlan(BuildParsePlanInput(reply), oldKey, newKey);
            switch (parsed)
            {
                case CitationRename.PlanResult.Accepted accepted:
                    status.Text = accepted.Plan.Summary + ". Review, then Apply Rename (nothing is written to disk until you Save each file).";
                    dialog.PrimaryButtonText = "Apply Rename";
                    onAccepted(accepted.Plan);
                    break;
                case CitationRename.PlanResult.Refused refused:
                    status.Text = refused.Reason;
                    break;
            }
        }
        catch (Exception ex)
        {
            status.Text = "Plan failed: " + (ex is PreviewControllerErrorException helperError ? CitationRename.Explain(helperError.Message) : ExplainSessionFailure(ex));
        }
    }

    private async Task ApplyCitationRenameAsync(CitationRename.Plan plan)
    {
        try
        {
            foreach (string path in plan.Paths)
            {
                await ApplyCitationRenameToFileAsync(path, plan).ConfigureAwait(true);
            }
            ReconcileDocumentWatchers();
            await ShowProjectFileDialogAsync(
                "Citation rename applied",
                $"{plan.Summary}. Review each changed tab and Save when ready — nothing was written to disk automatically.").ConfigureAwait(true);
        }
        catch (Exception ex)
        {
            await ShowProjectFileDialogAsync("Citation rename failed", ex.Message).ConfigureAwait(true);
        }
    }

    private async Task ApplyCitationRenameToFileAsync(string path, CitationRename.Plan plan)
    {
        var openDocument = _shell.Documents.FirstOrDefault(d => d.Path == path);
        string currentText;
        if (openDocument is not null)
        {
            currentText = openDocument.Text;
        }
        else
        {
            OpenFileBinding? sourceBinding = _fileBindingsByDocument.Values.FirstOrDefault();
            if (sourceBinding is null)
            {
                await ShowProjectFileDialogAsync("Citation rename incomplete", $"No file client is available to read “{path}”.").ConfigureAwait(true);
                return;
            }
            ReadPayload read = await sourceBinding.Client.ReadAsync(path).ConfigureAwait(true);
            if (!read.Exists || read.Text is null)
            {
                await ShowProjectFileDialogAsync("Citation rename incomplete", $"“{path}” no longer exists.").ConfigureAwait(true);
                return;
            }
            currentText = read.Text;
            await _shell.OpenDocumentAsync(path, currentText).ConfigureAwait(true);
            _fileBindingsByDocument[path] = new OpenFileBinding(sourceBinding.Client, sourceBinding.Root, read.Path, read.Sha256);
        }

        string? updatedText = ApplyReplacementEdits(currentText, plan.EditsIn(path));
        if (updatedText is null)
        {
            await ShowProjectFileDialogAsync("Citation rename skipped a file", $"“{path}” changed since the rename was planned; re-run Rename Citation.").ConfigureAwait(true);
            return;
        }
        _shell.UpdateDocumentText(path, updatedText);
    }

    /// <summary>
    /// Applies a plan's edits for one file in reverse byte order (the plan already
    /// validated they are forward-sorted and nonoverlapping — reversing keeps every
    /// earlier offset valid as later ones are spliced first), refusing (returns
    /// null) if any span's current text no longer matches what the plan expected.
    /// </summary>
    private static string? ApplyReplacementEdits(string text, IReadOnlyList<CitationRename.ReplacementEdit> edits)
    {
        foreach (CitationRename.ReplacementEdit edit in edits.Reverse())
        {
            (int Utf16Start, int Utf16End)? range = ByteOffsets.Utf16RangeForUtf8Bytes(text, edit.StartByte, edit.EndByte);
            if (range is not (int utf16Start, int utf16End) || text[utf16Start..utf16End] != edit.ExpectedText)
            {
                return null;
            }
            text = text[..utf16Start] + edit.Replacement + text[utf16End..];
        }
        return text;
    }

    /// <summary>
    /// Re-shapes a <see cref="PlanResult"/> reply's strongly-typed
    /// (<c>ulong</c>-keyed) fields into the raw JSON object shape
    /// <see cref="CitationRename.ParsePlan"/> expects (its ported logic reads
    /// <c>source_versions</c>/<c>membership_generation</c> as <c>int</c>, which
    /// only matters if a project ever reaches billions of revisions).
    /// </summary>
    private static JsonElement BuildParsePlanInput(PlanResult reply)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartObject();
            writer.WriteStartObject("source_versions");
            foreach ((string path, ulong revision) in reply.SourceVersions)
            {
                writer.WriteNumber(path, revision);
            }
            writer.WriteEndObject();
            writer.WriteNumber("membership_generation", reply.MembershipGeneration);
            writer.WritePropertyName("plan");
            reply.Plan.WriteTo(writer);
            writer.WriteEndObject();
        }
        using var document = JsonDocument.Parse(stream.ToArray());
        return document.RootElement.Clone();
    }
}
