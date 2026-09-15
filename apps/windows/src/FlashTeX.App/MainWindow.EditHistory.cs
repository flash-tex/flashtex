// name: MainWindow.EditHistory.cs
// purpose: Wires CommandIds.ToggleEditHistory to a singleton EditHistoryWindow —
//   reactivated (never recreated) on repeated invocation, per this port's
//   requirement that the edit-history panel be one auxiliary Window instance,
//   not a modal dialog rebuilt each time.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.App;

public sealed partial class MainWindow
{
    private EditHistoryWindow? _editHistoryWindow;

    private bool TryToggleEditHistory()
    {
        if (_editHistoryWindow is null)
        {
            _editHistoryWindow = new EditHistoryWindow(_shell);
            _editHistoryWindow.Closed += (_, _) => _editHistoryWindow = null;
        }
        _editHistoryWindow.Activate();
        return true;
    }
}
