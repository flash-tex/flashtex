// name: CommandRegistry.cs
// purpose: Single shared registry of every app command (menu bar, toolbar and
//   command palette all dispatch through it) — a C# port of the DESIGN of
//   apps/mac/Sources/FlashTeXMac/CommandPalette.swift's CommandPaletteModel:
//   the searchable command table, its ranked substring search, and one
//   Execute dispatch entry point. Not a verbatim port (the Swift source reads
//   the AppKit accessibility-command table and switches into ~40 ShellModel
//   methods that don't exist yet in this port); the ranking algorithm's exact
//   behavior is preserved, ported from CommandPaletteModel.rows(matching:).
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Shell;

/// <summary>
/// One command the app can run from a menu item, a toolbar button, or the
/// command palette. <see cref="Id"/> is the stable, serializable identity
/// (e.g. for a future user-remapped-shortcuts file); everything else is
/// display metadata plus the two dispatch hooks.
/// </summary>
public sealed record Command(
    string Id,
    string Title,
    KeyboardShortcut DefaultShortcut,
    string Category,
    string? MenuItem,
    string Description,
    Func<ICommandContext, bool> CanExecute,
    Action<ICommandContext> Execute);

/// <summary>Stable command identifiers, referenced by id rather than by table position.</summary>
public static class CommandIds
{
    public const string OpenLatexFile = "openLatexFile";
    public const string NewProject = "newProject";
    public const string NewFile = "newFile";
    public const string Save = "save";
    public const string SaveAs = "saveAs";
    public const string Compile = "compile";
    public const string ExportPdf = "exportPdf";
    public const string ExportPdfViaRustWriter = "exportPdfViaRustWriter";
    public const string ExportPdfExact = "exportPdfExact";
    public const string Undo = "undo";
    public const string Redo = "redo";
    public const string Find = "find";
    public const string CommandPalette = "commandPalette";
    public const string ToggleProblems = "toggleProblems";
    public const string ToggleCaptures = "toggleCaptures";
    public const string ZoomIn = "zoomIn";
    public const string ZoomOut = "zoomOut";
    public const string ResetZoom = "resetZoom";
    public const string ActualSize = "actualSize";
    public const string IncreaseEditorFontSize = "increaseEditorFontSize";
    public const string DecreaseEditorFontSize = "decreaseEditorFontSize";
    public const string ResetEditorFontSize = "resetEditorFontSize";
    public const string GoToMatching = "goToMatching";
    public const string GoToDefinition = "goToDefinition";
    public const string GoToSymbol = "goToSymbol";
    public const string NextDiagnostic = "nextDiagnostic";
    public const string PreviousDiagnostic = "previousDiagnostic";
    public const string NextOccurrence = "nextOccurrence";
    public const string PreviousOccurrence = "previousOccurrence";
    public const string RevealCaretInPreview = "revealCaretInPreview";

    // Appended (not part of the original ~30-command core): project-wide search,
    // citation rename and the edit-history panel. See MainWindow.Menu.cs's
    // ExecuteCommand for how each is special-cased, matching the existing
    // export/project-file/command-palette precedent.
    public const string ProjectSearch = "projectSearch";
    public const string CitationRename = "citationRename";
    public const string ToggleEditHistory = "toggleEditHistory";
}

public static class CommandRegistry
{
    /// <summary>The description-tier rank: a term matched only in the description, the weakest accepted match.</summary>
    private const int DescriptionRank = 3;

    private static bool Always(ICommandContext context) => true;
    private static bool NeedsActiveDocument(ICommandContext context) => context.HasActiveDocument;
    private static bool NeedsWorkerAttached(ICommandContext context) => context.HasWorkerAttached;
    private static bool NeedsCompileResult(ICommandContext context) => context.HasCompileResult;

    /// <summary>Gates the exact v2 export: only a validated display list carries original GIDs and font programs.</summary>
    private static bool NeedsDisplayList(ICommandContext context) => context.HasDisplayList;

    /// <summary>Every command Execute forwards to, closing over its own id (see <see cref="ICommandContext.PerformAction"/>).</summary>
    private static Action<ICommandContext> Forward(string commandId) => context => context.PerformAction(commandId);

    private static Command Make(string id, string title, ShortcutModifiers modifiers, string key, string category,
        string? menuItem, string description, Func<ICommandContext, bool> canExecute) =>
        new(id, title, new KeyboardShortcut(modifiers, key), category, menuItem, description, canExecute, Forward(id));

    /// <summary>Every command in table order, matching the ~30-command core of the Mac accessibility command table.</summary>
    public static IReadOnlyList<Command> All { get; } = new[]
    {
        Make(CommandIds.OpenLatexFile, "Open LaTeX File", ShortcutModifiers.Ctrl, "O", "File",
            "Open LaTeX File…", "Opens a .tex file as the main.tex entry document and compiles it when a worker is attached.", Always),
        Make(CommandIds.NewProject, "New Project", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "N", "File",
            "New Project…", "Opens the New Project flow: choose a folder, name and template; main.tex opens as the entry document.", Always),
        Make(CommandIds.NewFile, "New File", ShortcutModifiers.Ctrl, "N", "File",
            "New File…", "Opens the New File flow for a rooted .tex name under the project root; the new file opens in a tab.", NeedsActiveDocument),
        Make(CommandIds.Save, "Save", ShortcutModifiers.Ctrl, "S", "File",
            "Save", "Saves the entry document as UTF-8.", NeedsActiveDocument),
        Make(CommandIds.SaveAs, "Save As", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "S", "File",
            "Save As…", "Saves the entry document under a new name.", NeedsActiveDocument),
        Make(CommandIds.Compile, "Compile Now", ShortcutModifiers.Ctrl, "B", "File",
            "Compile", "Sends the current buffers to the attached worker.", NeedsWorkerAttached),
        Make(CommandIds.ExportPdf, "Export PDF", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "E", "File",
            "Export PDF…", "Writes the current preview as a PDF.", NeedsCompileResult),
        Make(CommandIds.ExportPdfViaRustWriter, "Export PDF via Rust Writer", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "E", "File",
            "Export PDF via Rust Writer…", "Pipes the compile result through the Rust PDF writer with verification.", NeedsCompileResult),
        Make(CommandIds.ExportPdfExact, "Export PDF (Exact, v2)", ShortcutModifiers.None, string.Empty, "File",
            "Export PDF (Exact, v2)…", "Writes glyphs by original GID with embedded font programs from a loaded v2 display list.", NeedsDisplayList),
        Make(CommandIds.Undo, "Undo", ShortcutModifiers.Ctrl, "Z", "Edit",
            "Undo", "Undoes the last edit.", NeedsActiveDocument),
        Make(CommandIds.Redo, "Redo", ShortcutModifiers.Ctrl, "Y", "Edit",
            "Redo", "Redoes the last undone edit.", NeedsActiveDocument),
        Make(CommandIds.Find, "Find", ShortcutModifiers.Ctrl, "F", "Edit",
            "Find…", "Opens the source editor's find bar over the focused document.", NeedsActiveDocument),
        Make(CommandIds.CommandPalette, "Command Palette", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "P", "View",
            "Command Palette…", "Opens a searchable list of every command with its menu and shortcut.", Always),
        Make(CommandIds.ToggleProblems, "Toggle Problems Panel", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "M", "View",
            "Toggle Problems", "Shows or hides the Problems panel of grouped diagnostics.", Always),
        Make(CommandIds.ToggleCaptures, "Toggle Captures Inspector", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "I", "View",
            "Toggle Captures", "Shows or hides the Captures inspector.", Always),
        Make(CommandIds.ZoomIn, "Zoom In Preview", ShortcutModifiers.Ctrl, "+", "View",
            "Zoom In", "Multiplies preview zoom by 1.25, up to 4x fit width.", NeedsCompileResult),
        Make(CommandIds.ZoomOut, "Zoom Out Preview", ShortcutModifiers.Ctrl, "-", "View",
            "Zoom Out", "Divides preview zoom by 1.25, down to 0.25x fit width.", NeedsCompileResult),
        Make(CommandIds.ResetZoom, "Fit Width Preview", ShortcutModifiers.Ctrl, "9", "View",
            "Fit Width", "Resets the preview zoom so the widest page fits the pane width.", NeedsCompileResult),
        Make(CommandIds.ActualSize, "Actual Size Preview", ShortcutModifiers.Ctrl, "0", "View",
            "Actual Size", "Sets one PDF point to one screen point.", NeedsCompileResult),
        Make(CommandIds.IncreaseEditorFontSize, "Increase Editor Font Size", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "+", "View",
            "Increase Editor Font Size", "Grows the editor font by 1pt, up to 36pt.", Always),
        Make(CommandIds.DecreaseEditorFontSize, "Decrease Editor Font Size", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "-", "View",
            "Decrease Editor Font Size", "Shrinks the editor font by 1pt, down to 8pt.", Always),
        Make(CommandIds.ResetEditorFontSize, "Reset Editor Font Size", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "0", "View",
            "Reset Editor Font Size", "Restores the editor font to the default 13pt.", Always),
        Make(CommandIds.GoToMatching, "Go to Matching", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "D", "Navigate",
            "Go to Matching \\begin/\\end or \\label/\\ref", "Selects the matching begin/end or label/ref for the command under the caret.", NeedsActiveDocument),
        Make(CommandIds.GoToDefinition, "Go to Definition", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "J", "Navigate",
            "Go to Definition", "Selects the definition of the command or environment under the caret.", NeedsActiveDocument),
        Make(CommandIds.GoToSymbol, "Go to Symbol", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "T", "Navigate",
            "Go to Symbol…", "Opens the symbol picker: fuzzy search over headings, environments and labels.", NeedsActiveDocument),
        Make(CommandIds.NextDiagnostic, "Next Diagnostic", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "]", "Navigate",
            "Next Diagnostic", "Selects the next diagnostic with a source in the active document, wrapping.", NeedsCompileResult),
        Make(CommandIds.PreviousDiagnostic, "Previous Diagnostic", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "[", "Navigate",
            "Previous Diagnostic", "Selects the previous diagnostic with a source in the active document, wrapping.", NeedsCompileResult),
        Make(CommandIds.NextOccurrence, "Next Occurrence", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "]", "Navigate",
            "Next Occurrence", "Steps to the next place of the selected diagnostic group, wrapping.", NeedsCompileResult),
        Make(CommandIds.PreviousOccurrence, "Previous Occurrence", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "[", "Navigate",
            "Previous Occurrence", "Steps to the previous place of the selected diagnostic group, wrapping.", NeedsCompileResult),
        Make(CommandIds.RevealCaretInPreview, "Reveal Caret in Preview", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "J", "Navigate",
            "Reveal Caret in Preview", "Selects the source span of the preview item under the caret.", NeedsCompileResult),

        // Appended: see CommandIds's own trailing comment for why these are not part
        // of the original core table order.
        Make(CommandIds.ProjectSearch, "Find in Project", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "F", "Edit",
            "Find in Project…", "Searches a literal string across every file the active project's entry document reaches.", NeedsActiveDocument),
        Make(CommandIds.CitationRename, "Rename Citation", ShortcutModifiers.Ctrl | ShortcutModifiers.Alt, "R", "Edit",
            "Rename Citation…", "Renames a citation key across the project via a reviewed, durable-helper-planned edit.", NeedsActiveDocument),
        Make(CommandIds.ToggleEditHistory, "Toggle Edit History", ShortcutModifiers.Ctrl | ShortcutModifiers.Shift, "H", "View",
            "Toggle Edit History", "Shows or reactivates the active document's durable undo/redo history panel.", NeedsActiveDocument),
    };

    /// <summary>
    /// Rows matching <paramref name="query"/>: every whitespace-separated term
    /// must occur (case-insensitively) in the title, the menu item, the
    /// category or shortcut, or the description. Title matches rank first,
    /// then menu-item matches, then category/shortcut, then description, each
    /// tier keeping table order; the empty query lists everything. Ported
    /// behavior-for-behavior from CommandPaletteModel.rows(matching:) in
    /// CommandPalette.swift: a row's rank is the *best* (lowest) tier any one
    /// of its terms reached, and a row is excluded entirely if any term
    /// matches none of the four fields.
    /// </summary>
    public static IReadOnlyList<Command> Search(string query)
    {
        var terms = query.ToLowerInvariant().Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
        if (terms.Length == 0)
        {
            return All;
        }

        var ranked = new List<(int Rank, int Index, Command Command)>();
        for (var index = 0; index < All.Count; index++)
        {
            var rank = Rank(All[index], terms);
            if (rank is int r)
            {
                ranked.Add((r, index, All[index]));
            }
        }

        ranked.Sort((a, b) => a.Rank != b.Rank ? a.Rank.CompareTo(b.Rank) : a.Index.CompareTo(b.Index));
        return ranked.ConvertAll(x => x.Command);
    }

    private static int? Rank(Command command, string[] terms)
    {
        var title = command.Title.ToLowerInvariant();
        var menuItem = (command.MenuItem ?? string.Empty).ToLowerInvariant();
        var category = command.Category.ToLowerInvariant();
        var shortcut = command.DefaultShortcut.ToString().ToLowerInvariant();
        var description = command.Description.ToLowerInvariant();

        var best = DescriptionRank;
        foreach (var term in terms)
        {
            if (title.Contains(term, StringComparison.Ordinal))
            {
                best = Math.Min(best, 0);
            }
            else if (menuItem.Contains(term, StringComparison.Ordinal))
            {
                best = Math.Min(best, 1);
            }
            else if (category.Contains(term, StringComparison.Ordinal) || shortcut.Contains(term, StringComparison.Ordinal))
            {
                best = Math.Min(best, 2);
            }
            else if (description.Contains(term, StringComparison.Ordinal))
            {
                best = Math.Min(best, DescriptionRank);
            }
            else
            {
                return null;
            }
        }

        return best;
    }

    /// <summary>
    /// The one dispatch entry point the menu bar, toolbar and command palette
    /// all call through. Looks up <paramref name="commandId"/>, checks
    /// <see cref="Command.CanExecute"/>, and if allowed invokes
    /// <see cref="Command.Execute"/> — no UI-specific logic lives here.
    /// Returns false when the id is unknown or the command is not currently
    /// executable, true when it ran.
    /// </summary>
    public static bool Execute(string commandId, ICommandContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        var command = All.FirstOrDefault(c => c.Id == commandId);
        if (command is null || !command.CanExecute(context))
        {
            return false;
        }

        command.Execute(context);
        return true;
    }
}
