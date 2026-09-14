import Foundation

/// The keyboard workflow as data: every app action a screen-reader user can
/// reach, its shortcut(s) exactly as the README table spells them, the menu it
/// lives in, and a discoverable description. The UI can render this as an
/// "Accessibility help" list; the test target checks it against the README.
public enum AccessibilityCommand: String, CaseIterable, Equatable {
    case editorPreferences
    case openLaTeXFile, newProject, newFile, save, saveAs, openFixture, reloadFixture
    case attachBuiltCompiler, attachRenderPipeline, attachWorker, compile
    case exportPDF, exportPDFViaRust, exportPDFExact
    case pinInsertionPoint, openCaptureProposal, submitSampleCapture, convertCapture, nearbyCompanion
    case restoreDiscardedBuffer
    case undo
    case commandPalette, toggleProblems, toggleCaptures
    case zoomIn, zoomOut, actualSize, fitWidth, increaseEditorFontSize, decreaseEditorFontSize, resetEditorFontSize
    case completion, completionList, toggleComment, signatureHelp, toggleVimKeybindings
    case goToMatching, nextDiagnostic, previousDiagnostic, nextOccurrence, previousOccurrence, copyDiagnosticsAsText, revealCaretInPreview
    case goToDefinition, goToSymbol, goToLine, selectEnvironment, wrapInEnvironment, renameSymbol
    case selectPreviewItemSource
    case accessibilityHelp
    case durableHistory, findInProject, nextSearchMatch, renameCitation
    case find, findAndReplace, findNext, findPrevious, useSelectionForFind, jumpToSelection

    public struct Entry: Equatable {
        public var command: AccessibilityCommand
        public var title: String
        /// Shortcut spellings as in the README ("⌘⇧]", "Esc", "⌃Space", "Click preview text").
        public var shortcuts: [String]
        public var menu: String
        public var description: String
        /// What must be true for the command to be enabled, or nil if always.
        public var requires: String?
        /// The menu item's title exactly as `FlashTeXMacApp`/`NavigationCommands`
        /// wire it (`Button("…")`), or nil for commands that are not menu items
        /// (editor keys, a preview click). The test target reads the shell
        /// source and checks the title and its `keyboardShortcut` agree.
        public var menuItem: String?

        public init(command: AccessibilityCommand, title: String, shortcuts: [String], menu: String,
                    description: String, requires: String? = nil, menuItem: String? = nil) {
            self.command = command; self.title = title; self.shortcuts = shortcuts; self.menu = menu
            self.description = description; self.requires = requires; self.menuItem = menuItem
        }

        /// One line for a help list: "Next diagnostic — ⌘⇧] (Navigate): …".
        public var helpLine: String {
            "\(title) — \(shortcuts.joined(separator: " or ")) (\(menu)): \(description)"
                + (requires.map { " Requires \($0)." } ?? "")
        }
    }

    public var entry: Entry {
        switch self {
        case .openLaTeXFile:
            return Entry(command: self, title: "Open LaTeX file", shortcuts: ["⌘O"], menu: "File",
                         description: "Opens a .tex file as the main.tex entry document and compiles it when a worker is attached.",
                         menuItem: "Open LaTeX File…")
        case .newProject:
            return Entry(command: self, title: "New Project", shortcuts: ["⌘⌥N"], menu: "File",
                         description: "Opens the New Project sheet: choose a folder, a project name and a template (Blank article, Article with sections, Report with chapters, Homework sheet); the files are written under <folder>/<name>, main.tex opens as the entry document and its include tree shows in the sidebar. Existing files are never overwritten without confirmation.",
                         menuItem: "New Project…")
        case .newFile:
            return Entry(command: self, title: "New File", shortcuts: ["⌘N"], menu: "File",
                         description: "Opens the New File sheet (also the sidebar's + button and the project row's context menu): a rooted .tex name, subfolders allowed, never above the project root; “Insert \\input at the caret” (on by default while the entry document is active) posts one undoable edit, then the new file opens in a tab.",
                         requires: "a saved entry document (a project root)",
                         menuItem: "New File…")
        case .save:
            return Entry(command: self, title: "Save", shortcuts: ["⌘S"], menu: "File",
                         description: "Saves the entry document as UTF-8; the editor header says “edited” while unsaved.",
                         menuItem: "Save")
        case .saveAs:
            return Entry(command: self, title: "Save As", shortcuts: ["⌘⇧S"], menu: "File",
                         description: "Saves the entry document under a new name.",
                         menuItem: "Save As…")
        case .openFixture:
            return Entry(command: self, title: "Open compile result fixture", shortcuts: ["⌘⇧O"], menu: "File",
                         description: "Loads a runtime v1 compile_result JSON into the preview; a sibling -request.json seeds the editor.",
                         menuItem: "Open Compile Result Fixture…")
        case .reloadFixture:
            return Entry(command: self, title: "Reload fixture", shortcuts: ["File > Reload Fixture"], menu: "File",
                         description: "Reloads the current fixture from disk (developer-only; confirms via Save/Discard/Cancel before replacing a real or unsaved document).",
                         menuItem: "Reload Fixture")
        case .attachBuiltCompiler:
            return Entry(command: self, title: "Attach built compiler", shortcuts: ["⌘⇧K"], menu: "File",
                         description: "Attaches the FlashTeX compiler found via $FLASHTEX_COMPILER or crates/compiler/target.",
                         menuItem: "Attach Built Compiler")
        case .attachRenderPipeline:
            return Entry(command: self, title: "Attach render pipeline", shortcuts: ["⌘⇧R"], menu: "File",
                         description: "Attaches flashtex-render (crates/render-pipeline) found via $FLASHTEX_RENDER, the app bundle, or crates/render-pipeline/target: the producer measured with Latin Modern metrics, so the preview shows Computer Modern-style text.",
                         menuItem: "Attach Render Pipeline (Latin Modern)")
        case .attachWorker:
            return Entry(command: self, title: "Attach worker executable", shortcuts: ["⌘K"], menu: "File",
                         description: "Chooses any executable speaking runtime v1 JSON Lines and attaches it as the compiler.",
                         menuItem: "Attach Worker Executable…")
        case .compile:
            return Entry(command: self, title: "Compile now", shortcuts: ["⌘B"], menu: "File / toolbar",
                         description: "Sends the current buffers to the attached worker; auto-compile also runs 250 ms after edits.",
                         requires: "an attached worker",
                         menuItem: "Compile")
        case .exportPDF:
            return Entry(command: self, title: "Export PDF", shortcuts: ["⌘⇧E"], menu: "File",
                         description: "Writes the current preview as a PDF with CoreGraphics (always white).",
                         requires: "a compile result",
                         menuItem: "Export PDF…")
        case .exportPDFViaRust:
            return Entry(command: self, title: "Export PDF via Rust writer", shortcuts: ["⌘⌥E"], menu: "File",
                         description: "Pipes the compile result to flashtex-pdf --verify (always white).",
                         requires: "a compile result",
                         menuItem: "Export PDF via Rust Writer…")
        case .exportPDFExact:
            return Entry(command: self, title: "Export PDF (exact, v2)", shortcuts: ["File > Export PDF (exact, v2)…"], menu: "File",
                         description: "Hands the loaded v2 display list to flashtex-pdf-exact from-v2: glyphs by original GID, embedded font programs, typed rules; refuses what it cannot express exactly.",
                         requires: "a loaded v2 display list and a built flashtex-pdf-exact",
                         menuItem: "Export PDF (exact, v2)…")
        case .pinInsertionPoint:
            return Entry(command: self, title: "Pin insertion point", shortcuts: ["⌘⌥P"], menu: "Edit",
                         description: "Records the caret as the destination anchor for capture proposals; the capture bar reads it back.",
                         menuItem: "Pin Insertion Point")
        case .openCaptureProposal:
            return Entry(command: self, title: "Open capture proposal", shortcuts: ["Edit > Open Capture Proposal…"], menu: "Edit",
                         description: "Queues a capture_proposal file for review; Return in the sheet approves and inserts one undoable edit.",
                         menuItem: "Open Capture Proposal…")
        case .toggleCaptures:
            return Entry(command: self, title: "Toggle Captures inspector", shortcuts: ["⌘⇧I"], menu: "View",
                         description: "Shows or hides the Captures inspector: every capture the paired iPad sent with its image, instruction and state (received, converting, proposal ready, inserted), the proposed LaTeX/TikZ, and Insert at caret / Edit / Review… / Reject. Opening it starts advertising and attaches the capture bridge; Pairing code… opens the Nearby window with a code.",
                         menuItem: "Toggle Captures")
        case .submitSampleCapture:
            return Entry(command: self, title: "Submit sample capture", shortcuts: ["⌘⇧U"], menu: "Edit",
                         description: "Sends a chosen PNG/JPEG as capture_submit through the attached bridge.",
                         requires: "an attached capture bridge",
                         menuItem: "Submit Sample Capture…")
        case .convertCapture:
            return Entry(command: self, title: "Convert capture", shortcuts: ["⌘⇧G"], menu: "Edit",
                         description: "Sends capture_convert for the latest received capture; the proposal opens for review.",
                         requires: "a received capture",
                         menuItem: "Convert Capture")
        case .nearbyCompanion:
            return Entry(command: self, title: "Nearby Companion", shortcuts: ["⌘⇧N"], menu: "Edit",
                         description: "Opens the window that advertises this Mac to a paired iPad/iPhone companion: pairing code (also as a QR image; Copy code or ⌘C on the code copies the digits), paired devices with a per-companion permission pop-up (Captures allowed / View only), received captures (nearby-v1 proposal). Return shows or resumes a pairing code, Esc cancels it or dismisses a banner; the status row, step indicator and every announcement are VoiceOver text.",
                         menuItem: "Nearby Companion…")
        case .undo:
            return Entry(command: self, title: "Undo", shortcuts: ["⌘Z"], menu: "Edit",
                         description: "Undoes the last edit, including an approved capture insertion.")
        case .commandPalette:
            return Entry(command: self, title: "Command palette", shortcuts: ["⌘⇧P"], menu: "View",
                         description: "Opens a searchable list of every command in this table with its menu and shortcut; type to filter, ↑/↓ choose, Return runs it, Esc closes. Editor keys and the preview click are listed as hints only.",
                         menuItem: "Command Palette…")
        case .toggleProblems:
            return Entry(command: self, title: "Toggle Problems panel", shortcuts: ["⌘⇧M"], menu: "View",
                         description: "Shows or hides the Problems panel under the editor and preview: the grouped diagnostics list with a severity filter, jump, explanation lines and Fix…; the sidebar's Problems rows and the status bar counts open it too.",
                         menuItem: "Toggle Problems")
        case .zoomIn:
            return Entry(command: self, title: "Zoom in preview", shortcuts: ["⌘="], menu: "View",
                         description: "Multiplies preview zoom by 1.25 (up to 4x fit width); pages wider than the pane scroll horizontally. Pinching on the preview and the header's + button do the same.",
                         menuItem: "Zoom In")
        case .zoomOut:
            return Entry(command: self, title: "Zoom out preview", shortcuts: ["⌘-"], menu: "View",
                         description: "Divides preview zoom by 1.25 (down to 0.25x fit width); the header's − button does the same.",
                         menuItem: "Zoom Out")
        case .actualSize:
            return Entry(command: self, title: "Actual size preview", shortcuts: ["⌘0"], menu: "View",
                         description: "Sets one PDF point to one screen point (100 %) when the 0.25x…4x zoom bounds permit it.",
                         menuItem: "Actual Size")
        case .fitWidth:
            return Entry(command: self, title: "Fit width preview", shortcuts: ["⌘9"], menu: "View",
                         description: "Resets the preview zoom to 1x so the widest page fits the pane width (the default); double-clicking the header percentage does the same.",
                         menuItem: "Fit Width")
        case .increaseEditorFontSize:
            return Entry(command: self, title: "Increase editor font size", shortcuts: ["⌘⌥="], menu: "View",
                         description: "Grows the editor font by 1 pt (up to 36 pt); the gutter and highlighting follow. The size is the Settings font-size preference, so it persists. Pinching over the editor does the same.",
                         menuItem: "Increase Editor Font Size")
        case .decreaseEditorFontSize:
            return Entry(command: self, title: "Decrease editor font size", shortcuts: ["⌘⌥-"], menu: "View",
                         description: "Shrinks the editor font by 1 pt (down to 8 pt); the gutter and highlighting follow and the preference persists.",
                         menuItem: "Decrease Editor Font Size")
        case .resetEditorFontSize:
            return Entry(command: self, title: "Reset editor font size", shortcuts: ["⌘⌥0"], menu: "View",
                         description: "Restores the editor font to the default 13 pt (the Settings font-size preference).",
                         menuItem: "Reset Editor Font Size")
        case .completion:
            return Entry(command: self, title: "Completion popup", shortcuts: ["Esc", "⌃Space"], menu: "Editor",
                         description: "Lists supported commands, \\end{…} for open environments, labels, citation keys and document words for the token at the caret; the list never takes the keyboard from the editor.")
        case .completionList:
            return Entry(command: self, title: "Completion list keys", shortcuts: ["↑", "↓", "Tab", "⇧Tab", "Return"], menu: "Editor",
                         description: "While the completion list is open: ↑/↓ or Tab/⇧Tab choose the candidate (wrapping; VoiceOver announces “n of m: candidate, kind, origin”), Return or Enter inserts it over the typed token, Esc closes without inserting; typing narrows the list and any other caret move closes it.",
                         requires: "an open completion list")
        case .signatureHelp:
            return Entry(command: self, title: "Signature help", shortcuts: ["⌘⇧Space"], menu: "Editor",
                         description: "Shows the signature of the command whose argument the caret is in; also opens on `{`/`[` typed after a command name. `}`, Esc, or leaving the argument closes it.")
        case .toggleVimKeybindings:
            return Entry(command: self, title: "Toggle Vim keybindings", shortcuts: ["⌃⌘V"], menu: "View",
                         description: "Switches the source editor's modal Vim emulation (normal/insert/visual modes, motions, operators, text objects, registers, marks, `/` search and `:` commands) on or off; the same as the Settings switch. The status bar shows -- NORMAL -- / -- INSERT -- / -- VISUAL --.",
                         menuItem: "Toggle Vim Keybindings")
        case .toggleComment:
            return Entry(command: self, title: "Toggle comment", shortcuts: ["⌘/"], menu: "Editor",
                         description: "Toggles a `% ` line comment on every line the selection touches: all commented lines are uncommented, otherwise the non-blank lines are commented; one undo step.")
        case .goToMatching:
            return Entry(command: self, title: "Go to matching", shortcuts: ["⌘⇧D"], menu: "Navigate",
                         description: "Selects the matching \\begin/\\end or \\label/\\ref for the command under the caret; misses are explained in the footer.",
                         menuItem: "Go to Matching \\begin/\\end or \\label/\\ref")
        case .goToDefinition:
            return Entry(command: self, title: "Go to definition", shortcuts: ["⌃⌘J"], menu: "Navigate",
                         description: "Selects the \\newcommand/\\def/\\DeclareMathOperator/\\newenvironment definition of the command or environment under the caret (in any open document; ⌘-click does the same); labels, citations and files keep their Go to Matching routes.",
                         menuItem: "Go to Definition")
        case .goToSymbol:
            return Entry(command: self, title: "Go to symbol", shortcuts: ["⌘⇧T"], menu: "Navigate",
                         description: "Opens the symbol picker: fuzzy search over every heading, environment and label of the open documents; ↑/↓ choose, Return goes there, Esc closes.",
                         menuItem: "Go to Symbol…")
        case .goToLine:
            return Entry(command: self, title: "Go to line", shortcuts: ["⌘L"], menu: "Navigate",
                         description: "Opens a field for a 1-based line, line:column, or +N/−N relative to the caret; out-of-range numbers clamp, invalid text shows an inline hint. Return selects the caret and centres it, Esc cancels. Typing :42 in the Commands list jumps directly.",
                         menuItem: "Go to Line…")
        case .selectEnvironment:
            return Entry(command: self, title: "Select environment", shortcuts: ["⌘⇧A"], menu: "Navigate",
                         description: "Selects the innermost \\begin{X}…\\end{X} around the caret (nesting and unbalanced text tolerated); again selects the enclosing one. The caret on a \\begin or \\end also highlights its partner like a bracket.",
                         menuItem: "Select Environment")
        case .wrapInEnvironment:
            return Entry(command: self, title: "Wrap selection in environment", shortcuts: ["⌘⇧W"], menu: "Navigate",
                         description: "Asks for an environment name (suggestions: common ones, then those the document uses) and wraps the selection in \\begin{X}…\\end{X} — whole lines as an indented block, otherwise inline — as one undoable edit with the caret at the body.",
                         menuItem: "Wrap Selection in Environment…")
        case .renameSymbol:
            return Entry(command: self, title: "Rename symbol", shortcuts: ["⌥⇧R"], menu: "Navigate",
                         description: "Renames the \\label key (every \\ref/\\eqref/\\pageref/\\autoref/\\cref use) or the user command (every \\foo, word-boundary aware, comments and verbatim skipped) under the caret across the open documents: Plan shows the per-file counts, Apply is one undoable edit per document (one guarded apply_group per file when the durable helper is attached).",
                         menuItem: "Rename Symbol…")
        case .nextDiagnostic:
            return Entry(command: self, title: "Next diagnostic", shortcuts: ["⌘⇧]"], menu: "Navigate",
                         description: "Selects the next diagnostic with a source in the active document (wrapping); refused if its span was edited since the compile.",
                         requires: "a compile result",
                         menuItem: "Next Diagnostic")
        case .previousDiagnostic:
            return Entry(command: self, title: "Previous diagnostic", shortcuts: ["⌘⇧["], menu: "Navigate",
                         description: "Selects the previous diagnostic with a source in the active document (wrapping).",
                         requires: "a compile result",
                         menuItem: "Previous Diagnostic")
        case .nextOccurrence:
            return Entry(command: self, title: "Next occurrence", shortcuts: ["⌘⌥]"], menu: "Navigate",
                         description: "Steps to the next place of the diagnostics panel's selected group (else the group under the current diagnostic), wrapping within the group; the row then reads “k of n”.",
                         requires: "the diagnostics panel (a result with diagnostics)",
                         menuItem: "Next Occurrence")
        case .previousOccurrence:
            return Entry(command: self, title: "Previous occurrence", shortcuts: ["⌘⌥["], menu: "Navigate",
                         description: "Steps to the previous place of the selected group, wrapping within the group.",
                         requires: "the diagnostics panel (a result with diagnostics)",
                         menuItem: "Previous Occurrence")
        case .copyDiagnosticsAsText:
            return Entry(command: self, title: "Copy diagnostics as text", shortcuts: ["⌘⌥C"], menu: "Edit",
                         description: "Copies the selected diagnostics row as “path:line: error/warning: message” lines (every place of a grouped row; all diagnostics when nothing is selected); ⌘C does the same while the list has the keyboard.",
                         requires: "the diagnostics panel (a result with diagnostics)",
                         menuItem: "Copy Diagnostics as Text")
        case .revealCaretInPreview:
            return Entry(command: self, title: "Reveal caret in preview", shortcuts: ["⌘⇧J"], menu: "Navigate",
                         description: "Selects the source span of the preview item under the caret and names its page and item.",
                         requires: "a compile result",
                         menuItem: "Reveal Caret in Preview")
        case .restoreDiscardedBuffer:
            return Entry(command: self, title: "Restore Discarded Buffer", shortcuts: ["Edit > Restore Discarded Buffer"], menu: "Edit",
                         description: "Brings back the unsaved text replaced by a Discard decision when another file was opened; the restored buffer stays unsaved.",
                         requires: "a discarded buffer from this session",
                         menuItem: "Restore Discarded Buffer")
        case .accessibilityHelp:
            return Entry(command: self, title: "Help window", shortcuts: ["Help > FlashTeX Accessibility Help"], menu: "Help",
                         description: "Opens the Accessibility Help window: focus order, what VoiceOver reads in each pane, and every command in this table.",
                         menuItem: "FlashTeX Accessibility Help")
        case .selectPreviewItemSource:
            return Entry(command: self, title: "Select source of a preview item", shortcuts: ["Click preview text"], menu: "Preview",
                         description: "Selects the item's source in the editor; with VoiceOver, use the “Go to source” action on the item.",
                         requires: "a compile result")
        case .editorPreferences:
            return Entry(command: self, title: "Settings window", shortcuts: ["⌘,"], menu: "FlashTeX",
                         description: "Opens the editor preferences (the system Settings item): font family and size, wrapping, tab width and indent style, appearance, auto-close braces, completion list, Restore Defaults; Tab walks the controls top to bottom, ⌘W closes and the editor keeps the keyboard.")
        case .durableHistory:
            return Entry(command: self, title: "Durable History window", shortcuts: ["Edit > Durable History…"], menu: "Edit",
                         description: "Opens the durable undo/redo history on the helper's edit ledger: Refresh, Undo, Redo (Retry/Discard after an uncertain reply), retention gauge, then the undo and redo stacks as a list; ⌘W closes and the editor keeps the keyboard.",
                         menuItem: "Durable History…")
        case .findInProject:
            return Entry(command: self, title: "Find in Project window", shortcuts: ["⌘⇧F"], menu: "Edit",
                         description: "Opens the project search: the literal field takes the keyboard, Return searches or goes to the selected match, ↑/↓ move the selection, then Search, Go to Match, Next Match, scope, match limit, results, the replacement field, Plan Replacement and Apply; Esc closes and the editor keeps the keyboard.",
                         menuItem: "Find in Project…")
        case .nextSearchMatch:
            return Entry(command: self, title: "Next match", shortcuts: ["⌘G"], menu: "Find in Project window",
                         description: "Selects the next search match (wrapping) and goes to it in the editor; only while the Find in Project window is key.",
                         requires: "a search with matches")
        case .renameCitation:
            return Entry(command: self, title: "Rename citation window", shortcuts: ["Edit > Rename Citation…"], menu: "Edit",
                         description: "Opens the reviewed citation rename: the helper plans every \\cite occurrence across the project (plan_citation_rename), the plan is shown for review, and Apply sends one apply_group; also in the toolbar.",
                         menuItem: "Rename Citation…")
        case .find:
            return Entry(command: self, title: "Find", shortcuts: ["⌘F"], menu: "Edit",
                         description: "Opens the source editor's find bar (AppKit's built-in incremental search) over the focused document.",
                         menuItem: "Find…")
        case .findAndReplace:
            return Entry(command: self, title: "Find and Replace", shortcuts: ["⌘⌥F"], menu: "Edit",
                         description: "Opens the find bar already showing its Replace row; a replacement goes through the editor's normal undoable edit path, so ⌘Z undoes it and the preview recompiles.",
                         menuItem: "Find and Replace…")
        case .findNext:
            return Entry(command: self, title: "Find Next", shortcuts: ["Edit > Find Next"], menu: "Edit",
                         description: "Selects the next find-bar match in the focused editor. No key equivalent: ⌘G is Find in Project's Next match and ⇧⌘G is Convert Capture, so Return in the find bar's search field is the keyboard way to find next.",
                         menuItem: "Find Next")
        case .findPrevious:
            return Entry(command: self, title: "Find Previous", shortcuts: ["Edit > Find Previous"], menu: "Edit",
                         description: "Selects the previous find-bar match in the focused editor. No key equivalent, for the same reason as Find Next: Shift-Return in the find bar's search field is the keyboard way to find previous.",
                         menuItem: "Find Previous")
        case .useSelectionForFind:
            return Entry(command: self, title: "Use Selection for Find", shortcuts: ["⌘E"], menu: "Edit",
                         description: "Sets the focused editor's current selection as the find bar's search string.",
                         menuItem: "Use Selection for Find")
        case .jumpToSelection:
            return Entry(command: self, title: "Jump to Selection", shortcuts: ["⌘J"], menu: "Edit",
                         description: "Scrolls the focused editor's current selection into view and centers it.",
                         menuItem: "Jump to Selection")
        }
    }

    public static var entries: [Entry] { allCases.map(\.entry) }

    /// Every shortcut spelling in the table (README parity).
    public static var allShortcuts: Set<String> { Set(entries.flatMap(\.shortcuts)) }

    /// Lines for an "Accessibility help" list, in menu order.
    public static var helpLines: [String] { entries.map(\.helpLine) }
}

/// Keyboard focus order of the main window's panes and why it is that way.
///
/// The order is the view order of the main window (mac-ui-redesign): a
/// `NavigationSplitView` whose sidebar is `WorkspaceSidebar`
/// (WorkspaceSidebar.swift: Project, Outline, Problems sections) and whose
/// detail is an `HSplitView` of `EditorPane` (document tabs, source text
/// view, capture bar, bridge bar) and `PreviewPane` (pages), then the
/// `ProblemsPanel` (ProblemsPanel.swift: grouped diagnostics) under both.
/// SwiftUI's Tab cycle and VoiceOver's VO-Right follow that tree, so each
/// pane names the container and the source marker that places it; the test
/// target reads those files and fails when the panes are reordered without
/// updating this table.
public enum FocusOrder {
    public struct Pane: Equatable {
        public var name: String
        public var contents: String
        public var rationale: String
        /// The view type the pane lives in (`WorkspaceSidebar`, `EditorPane`, `PreviewPane`, `ProblemsPanel`).
        public var container: String
        /// Text that places the pane inside `container`'s body, in order.
        public var sourceMarker: String
        /// File under `apps/mac/Sources/FlashTeXMac` that declares `container`.
        public var sourceFile: String

        public init(name: String, contents: String, rationale: String, container: String, sourceMarker: String, sourceFile: String = "ContentView.swift") {
            self.name = name; self.contents = contents; self.rationale = rationale
            self.container = container; self.sourceMarker = sourceMarker; self.sourceFile = sourceFile
        }
    }

    public static let panes: [Pane] = [
        Pane(name: "Sidebar",
             contents: "Project: every open member as a row (“main.tex, entry, edited, active”; bibliography members say so) plus not-yet-open \\input/\\include targets (“chapter1.tex, not open, included from main.tex; activate to open”); Outline: Sections, Environments and Labels of the active buffer as disclosure groups, each row “section Title, line n” and activation selects it in the editor; Problems: error/warning counts whose activation shows the Problems panel filtered to that severity.",
             rationale: "Leads the window because it answers “where am I in the project” before editing; every row is a button that drives an existing operation (switch, open include, select, show problems) so nothing is reachable only by mouse.",
             container: "WorkspaceSidebar", sourceMarker: "ProjectSection()", sourceFile: "WorkspaceSidebar.swift"),
        Pane(name: "Tabs",
             contents: "One tab per open document (“main.tex, entry, edited”), the active one selected; a Detach button on non-entry members; then the Project menu (open \\input/\\include targets, save or detach a member, bibliography kinds), the kind indicator and the byte/UTF-16 counts.",
             rationale: "Directly above the editor because a tab changes what the editor shows; switching goes through ProjectDocuments so each document keeps its caret.",
             container: "EditorPane", sourceMarker: "DocumentTabBar()"),
        Pane(name: "Editor",
             contents: "Source text view, labelled “LaTeX source”. Caret moves announce line and column.",
             rationale: "Editing is the primary task; the caret drives caret sync, diagnostics at caret, and every Navigate command.",
             container: "EditorPane", sourceMarker: "SourceEditorView("),
        Pane(name: "Capture bar",
             contents: "Pin insertion point, the pinned anchor, and the review button for queued proposals; one group whose value reads the anchor and proposal count.",
             rationale: "Directly under the editor because pinning starts from the caret; the bar's value is what a capture proposal will insert against.",
             container: "EditorPane", sourceMarker: "CaptureBar()"),
        Pane(name: "Bridge bar",
             contents: "Capture bridge status, the pinned bridge destination, the latest capture's state, and a Convert button when a capture is received.",
             rationale: "Status text with at most one button; it follows the capture bar because a received capture is converted, then reviewed, from the same place.",
             container: "EditorPane", sourceMarker: "BridgeBar()"),
        Pane(name: "Preview",
             contents: "A header line (source badge, compile status, freshness, layout capabilities), then pages in reading order; each page is a landmark (“Page n of m, k lines”), each line a group, each item static text with a “Go to source” action.",
             rationale: "Follows the editor column so a user can check what the last edit produced, page by page, without leaving the keyboard.",
             container: "PreviewPane", sourceMarker: "PreviewView("),
        Pane(name: "Problems",
             contents: "Header with counts, a severity filter (All / Errors / Warnings) and Hide; then the list of the compile result's diagnostics, identical ones folded into one row: “Diagnostic n of m: Error/Warning: message, 12 places, 3 of 12, main.tex line 41”, the recovery note as the value, “Go to source” when it has a source. ↑/↓ select a row, Return jumps to its current occurrence, Esc returns the keyboard to the editor, ⌘C copies the selection as path:line: message lines.",
             rationale: "Comes after the preview because the preview is still shown when errors exist; diagnostics refine, not replace, it. View > Toggle Problems (⌘⇧M) hides and shows it.",
             container: "ProblemsPanel", sourceMarker: "problemsList(", sourceFile: "ProblemsPanel.swift"),
    ]

    /// Non-focusable status text around the panes, in view order, so the help
    /// can say what VoiceOver reads when it walks the whole window.
    public static let statusLines: [String] = [
        "Toolbar: Compile (⌘B), the Producer menu (attach the built compiler ⌘⇧K, the Latin Modern render pipeline ⌘⇧R, any executable ⌘K, auto-compile, detach), v2 pane and Dark preview switches, Find in Project (⌘⇧F), Rename Citation, Durable History, the Export menu (⌘⇧E, ⌘⌥E, exact v2), Nearby (⌘⇧N), the Problems toggle with its count (⌘⇧M) and Commands (the palette, ⌘⇧P); every tooltip names the menu shortcut.",
        "Preview header: preview source badge, compile status, whether the editor is ahead of the preview, and the accepted layout capabilities.",
        "Status bar (bottom): editor revision, the active document's durable revision, compile latency, the route (fixture / worker / controller), error and warning counts (a button that shows the Problems panel), then the last navigation note, the stale-diagnostics note, or the preview-click hint; capture notes and the exact-export progress on the right.",
    ]

    /// "Sidebar → Tabs → Editor → Capture bar → Bridge bar → Preview → Problems"
    public static var description: String { panes.map(\.name).joined(separator: " → ") }

    /// Full help text: order plus rationale for each pane.
    public static var helpLines: [String] {
        panes.enumerated().map { i, p in "\(i + 1). \(p.name): \(p.contents) \(p.rationale)" }
    }
}

/// Keyboard focus order inside each secondary panel window. SwiftUI's Tab
/// cycle follows the view tree, so each control names the source text that
/// declares it (in `sourceFile`, in this order); the test target reads the
/// panel source and fails when a control is added, removed or reordered
/// without updating this table, and when a control has no spoken name.
public enum PanelFocusOrder {
    public struct Control: Equatable {
        /// What VoiceOver says when Tab lands on the control.
        public var name: String
        /// Text that declares the control in `Panel.sourceFile`, in order.
        public var sourceMarker: String
        /// When the control is present/enabled, or nil if always.
        public var when: String?

        public init(name: String, sourceMarker: String, when: String? = nil) {
            self.name = name; self.sourceMarker = sourceMarker; self.when = when
        }
    }

    public struct Panel: Equatable {
        public var name: String
        public var windowTitle: String
        public var command: AccessibilityCommand
        /// Where keyboard focus lands when the window opens.
        public var initialFocus: String
        /// How the window closes from the keyboard and where focus returns.
        public var closing: String
        public var controls: [Control]
        /// File under `apps/mac/Sources/FlashTeXMac` that declares the controls.
        public var sourceFile: String

        public var helpLine: String {
            "\(name) (\(command.entry.shortcuts.joined(separator: " or "))): opens with focus on \(initialFocus). Tab order: "
                + controls.map { $0.name + ($0.when.map { " (\($0))" } ?? "") }.joined(separator: " → ") + ". \(closing)"
        }
    }

    public static let panels: [Panel] = [
        Panel(name: "Settings", windowTitle: "Editor Preferences", command: .editorPreferences,
              initialFocus: "the font family pop-up",
              closing: "⌘W closes the window; the editor text view is first responder again.",
              controls: [
                Control(name: "Editor font family", sourceMarker: "Picker(\"Family\""),
                Control(name: "Editor font size (slider)", sourceMarker: "Slider(value: $prefs.fontSize"),
                Control(name: "Editor font size stepper", sourceMarker: "Stepper(value: $prefs.fontSize"),
                Control(name: "Wrap long lines", sourceMarker: "Toggle(\"Wrap long lines\""),
                Control(name: "Tab width", sourceMarker: "Stepper(value: $prefs.tabWidth"),
                Control(name: "Indent style (radio group)", sourceMarker: "Picker(\"Indent with\""),
                Control(name: "Editor appearance (segments)", sourceMarker: "Picker(\"Editor appearance\""),
                Control(name: "Auto-close brackets & math", sourceMarker: "Toggle(\"Auto-close brackets & math\""),
                Control(name: "Show completion list", sourceMarker: "Toggle(\"Show completion list\""),
                Control(name: "Check spelling", sourceMarker: "Toggle(\"Check spelling\""),
                Control(name: "Relative line numbers", sourceMarker: "Toggle(\"Relative line numbers\""),
                Control(name: "Vim keybindings", sourceMarker: "Toggle(\"Vim keybindings\""),
                Control(name: "Preview follows the caret", sourceMarker: "Toggle(\"Preview follows the caret\""),
                Control(name: "Restore Defaults", sourceMarker: "Button(\"Restore Defaults\""),
              ],
              sourceFile: "EditorPreferences.swift"),
        Panel(name: "Durable History", windowTitle: "Durable History", command: .durableHistory,
              initialFocus: "the undo/redo list (every button is disabled until a preview controller is attached)",
              closing: "⌘W closes the window; the editor text view is first responder again.",
              controls: [
                Control(name: "Refresh history", sourceMarker: "accessibilityIdentifier(\"history.refresh\")", when: "controller attached"),
                Control(name: "Undo, n steps available", sourceMarker: "accessibilityIdentifier(\"history.undo\")", when: "controller attached"),
                Control(name: "Redo, n steps available", sourceMarker: "accessibilityIdentifier(\"history.redo\")", when: "controller attached"),
                Control(name: "Retry the uncertain undo/redo", sourceMarker: "accessibilityIdentifier(\"history.retry\")", when: "after an uncertain reply"),
                Control(name: "Discard the uncertain undo/redo", sourceMarker: "accessibilityIdentifier(\"history.discard\")", when: "after an uncertain reply"),
                Control(name: "Undo and redo stacks (list)", sourceMarker: "accessibilityIdentifier(\"history.stacks\")"),
              ],
              sourceFile: "EditHistoryPanel.swift"),
        Panel(name: "Find in Project", windowTitle: "Find in Project", command: .findInProject,
              initialFocus: "the literal field",
              closing: "Esc or ⌘W closes the window; the editor text view is first responder again.",
              controls: [
                Control(name: "Literal to find in the project, case-sensitive", sourceMarker: "TextField(\"Find in project"),
                Control(name: "Search", sourceMarker: "Button(\"Search\")", when: "non-empty literal"),
                Control(name: "Go to Match", sourceMarker: "Button(\"Go to Match\")", when: "a match is selected"),
                Control(name: "Next Match", sourceMarker: "Button(\"Next Match\")", when: "matches"),
                Control(name: "Search scope", sourceMarker: "Picker(\"Scope\""),
                Control(name: "Max matches", sourceMarker: "Stepper(\"Max matches"),
                Control(name: "Search results, n matches (list)", sourceMarker: "List(selection: $client.selectedID)", when: "after a search"),
                Control(name: "Replacement text", sourceMarker: "TextField(\"Replace with"),
                Control(name: "Plan Replacement", sourceMarker: "Button(\"Plan Replacement\")", when: "complete search with matches"),
                Control(name: "Apply n replacements", sourceMarker: "Button(\"Apply \\(plan.summary)\")", when: "a planned proposal"),
                Control(name: "Retry path", sourceMarker: "Button(\"Retry \\(outcome.path)\")", when: "an uncertain apply"),
              ],
              sourceFile: "ProjectSearchPanel.swift"),
        Panel(name: "Nearby Companion", windowTitle: "Nearby Companion", command: .nearbyCompanion,
              initialFocus: "Show Pairing Code (Return); focus follows the pairing state: the code while it is shown or verified, Resume when interrupted, Dismiss on paired/error, Cancel while receiving",
              closing: "Esc cancels the current pairing step (or dismisses a banner); ⌘W closes the window; the editor text view is first responder again. The whole pairing is keyboard-only: Return shows or resumes a code, Esc cancels it.",
              controls: [
                Control(name: "Advertise on the local network (switch)", sourceMarker: "Toggle(\"Advertise\""),
                Control(name: "Dismiss", sourceMarker: "accessibilityIdentifier(\"nearby.pairing.dismiss\")", when: "after an error"),
                Control(name: "Show New Code", sourceMarker: "showCodeButton(title: \"Show New Code\")", when: "after an error"),
                Control(name: "Dismiss", sourceMarker: "accessibilityIdentifier(\"nearby.pairing.dismiss\")", when: "paired banner"),
                Control(name: "Cancel receiving", sourceMarker: "accessibilityLabel(\"Cancel receiving\")", when: "receiving a capture"),
                Control(name: "Show Pairing Code / Show New Code", sourceMarker: "Button(title) { controller.showCode() }", when: "idle, paired, error or expired"),
                Control(name: "Pairing code (spoken as digits in pairs; ⌘C copies it; a QR image of the same payload sits beside it)", sourceMarker: "accessibilityIdentifier(\"nearby.pairing.code\")", when: "code shown or verifying"),
                Control(name: "Copy code", sourceMarker: "Button(\"Copy code\")", when: "code shown or verifying"),
                Control(name: "Cancel pairing", sourceMarker: "accessibilityLabel(\"Cancel pairing\")", when: "code shown or verifying"),
                Control(name: "Resume", sourceMarker: "accessibilityIdentifier(\"nearby.pairing.resume\")", when: "interrupted, code still valid"),
                Control(name: "Cancel interrupted pairing", sourceMarker: "accessibilityLabel(\"Cancel interrupted pairing\")", when: "interrupted, code still valid"),
                Control(name: "Dismiss", sourceMarker: "accessibilityIdentifier(\"nearby.pairing.dismiss\")", when: "interrupted, code expired"),
                Control(name: "Show New Code", sourceMarker: "showCodeButton(title: \"Show New Code\")", when: "interrupted, code expired"),
                Control(name: "Permission for <companion> (pop-up: Captures allowed / View only)", sourceMarker: "Picker(\"Permission\"", when: "one per paired companion"),
                Control(name: "Forget <companion>", sourceMarker: "Button(\"Forget\")", when: "one per paired companion"),
                Control(name: "Clear refused captures", sourceMarker: "Button(\"Clear\")", when: "after a refused capture"),
              ],
              sourceFile: "NearbyView.swift"),
    ]

    public static var helpLines: [String] { panels.map(\.helpLine) }
}
