// name: ICommandContext.cs
// purpose: The minimal state a CommandRegistry command can gate on
//   (CanExecute) and the single forwarding hook its Execute delegate calls
//   into. Kept small on purpose: this is the seam a future real ShellModel
//   implements once WorkerClient/PreviewControllerClient/EditLedgerClient are
//   wired up; extend it then, field by field, rather than speculatively now.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Shell;

/// <summary>
/// What a <see cref="Command"/>'s <c>CanExecute</c> predicate reads, and where
/// its <c>Execute</c> delegate forwards to. <see cref="PerformAction"/> is the
/// one seam between "the registry decided this command may run" and "the app
/// actually runs it" — the registry never contains per-command UI or IPC
/// logic itself (see <see cref="CommandRegistry.Execute"/>), so a real
/// implementation of this interface (the eventual <c>ShellModel</c>) is free
/// to route each command id to compile/export/navigation logic as those
/// pieces land, without CommandRegistry.cs changing.
/// </summary>
public interface ICommandContext
{
    /// <summary>Whether a document is open and active in the editor.</summary>
    bool HasActiveDocument { get; }

    /// <summary>Whether a compile result is loaded (gates preview/export/diagnostic-navigation commands).</summary>
    bool HasCompileResult { get; }

    /// <summary>Whether a compiler worker process is attached (gates Compile).</summary>
    bool HasWorkerAttached { get; }

    /// <summary>
    /// Whether a validated rendering-v2 display list is loaded for the current revision
    /// (gates "Export PDF (Exact, v2)", which needs glyph-level v2 geometry a v1 compile
    /// result cannot supply). A DEFAULT implementation of false so existing implementors —
    /// including test doubles — keep compiling and simply report "no v2 frame".
    /// </summary>
    bool HasDisplayList => false;

    /// <summary>Runs the command identified by <paramref name="commandId"/> (a <see cref="Command.Id"/>).</summary>
    void PerformAction(string commandId);
}
