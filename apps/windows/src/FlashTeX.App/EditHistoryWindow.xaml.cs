// name: EditHistoryWindow.xaml.cs
// purpose: Singleton auxiliary window showing the active document's durable
//   undo/redo history via ShellModel.GetActiveDocumentHistoryStatusAsync
//   (FlashTeX.Ipc.EditLedgerClient.HistoryStatusAsync). MainWindow.EditHistory.cs
//   reactivates one existing instance rather than creating a new one each time
//   CommandIds.ToggleEditHistory runs, per this port's design for the panel.
//   Polls on a short timer while visible, plus refreshes on demand and on
//   active-document change: flashtex-edit-ledger's history_status is a plain
//   request/reply with no push notification for history changes.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.EditLedgerV1;
using FlashTeX.Shell;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace FlashTeX.App;

public sealed partial class EditHistoryWindow : Window
{
    private static readonly TimeSpan PollInterval = TimeSpan.FromSeconds(1.5);

    private readonly ShellModel _shell;
    private readonly DispatcherQueueTimer _pollTimer;

    public EditHistoryWindow(ShellModel shell)
    {
        InitializeComponent();
        _shell = shell;
        Title = "Edit History";

        RefreshButton.Click += (_, _) => _ = RefreshAsync();
        _shell.PropertyChanged += OnShellPropertyChanged;

        _pollTimer = DispatcherQueue.CreateTimer();
        _pollTimer.Interval = PollInterval;
        _pollTimer.Tick += (_, _) => _ = RefreshAsync();
        _pollTimer.Start();

        Closed += OnClosed;

        _ = RefreshAsync();
    }

    private void OnClosed(object sender, WindowEventArgs args)
    {
        _pollTimer.Stop();
        _shell.PropertyChanged -= OnShellPropertyChanged;
    }

    private void OnShellPropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(ShellModel.ActiveDocumentPath))
        {
            DispatcherQueue.TryEnqueue(() => _ = RefreshAsync());
        }
    }

    private async Task RefreshAsync()
    {
        DocumentText.Text = _shell.ActiveDocumentPath ?? "(no active document)";

        HistoryStatusResult? status = await _shell.GetActiveDocumentHistoryStatusAsync().ConfigureAwait(true);
        EntriesPanel.Children.Clear();
        if (status is null)
        {
            SummaryText.Text = _shell.ActiveDocumentPath is null
                ? "No active document."
                : "This document has no durable edit-ledger store (no flashtex-edit-ledger executable was configured).";
            return;
        }

        SummaryText.Text = $"{status.UndoLabels.Count} undo, {status.RedoLabels.Count} redo entries · " +
            $"{status.PermanentCommandIds} permanent command ids · {status.HistoryBytes} history bytes";

        if (status.UndoLabels.Count == 0 && status.RedoLabels.Count == 0)
        {
            EntriesPanel.Children.Add(new TextBlock { Text = "No edits recorded yet.", Opacity = 0.65 });
            return;
        }

        // The ledger's history_status reports retained labels only, not per-entry
        // timestamps or a documented undo/redo order guarantee — showing an
        // invented "most recent first" claim here would be dishonest, so this
        // just numbers each stack in the order the helper returned it.
        AddSection("Undo stack", status.UndoLabels);
        AddSection("Redo stack", status.RedoLabels);
    }

    private void AddSection(string header, IReadOnlyList<string> labels)
    {
        if (labels.Count == 0)
        {
            return;
        }
        EntriesPanel.Children.Add(new TextBlock { Text = header, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold, Margin = new Thickness(0, 8, 0, 2) });
        for (int i = 0; i < labels.Count; i++)
        {
            EntriesPanel.Children.Add(new TextBlock { Text = $"{i + 1}. {labels[i]}" });
        }
    }
}
