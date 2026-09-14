import AppKit
import SwiftUI
import FlashTeXAccessibility

/// View > Command Palette… (⌘⇧P, mac-ui-redesign): every command of the
/// accessibility command table (`AccessibilityCommand`, README-parity
/// tested) as a searchable list with its menu and shortcut, run from the
/// keyboard. The table is the single source: a command that is not in it is
/// not in the palette, and the palette can only run what a menu item or
/// window already does (`CommandPaletteModel.perform`). Editor keys
/// (completion, toggle comment, signature help, the preview click, ⌘G in the
/// search window) are listed as hints and cannot be run from here.
enum CommandPaletteModel {
    struct Row: Identifiable, Equatable {
        let entry: AccessibilityCommand.Entry
        var id: AccessibilityCommand { entry.command }
        var runnable: Bool { CommandPaletteModel.isRunnable(entry.command) }
    }

    /// Commands the palette cannot run: they are keys inside the editor, a
    /// mouse action on the preview, or a key that only the search window has.
    static let notRunnable: Set<AccessibilityCommand> = [.completion, .completionList, .toggleComment, .duplicateLine, .signatureHelp, .selectPreviewItemSource, .nextSearchMatch]

    static func isRunnable(_ command: AccessibilityCommand) -> Bool { !notRunnable.contains(command) }

    /// All rows in table order.
    static var all: [Row] { AccessibilityCommand.entries.map(Row.init) }

    /// Rows matching `query`: every whitespace-separated term must occur
    /// (case-insensitively) in the title, menu, a shortcut, or the menu item.
    /// Title matches rank first, then menu-item matches, then the rest, each
    /// group keeping table order; the empty query lists everything.
    static func rows(matching query: String) -> [Row] {
        let terms = query.lowercased().split(whereSeparator: \.isWhitespace).map(String.init)
        guard !terms.isEmpty else { return all }
        func rank(_ r: Row) -> Int? {
            let title = r.entry.title.lowercased()
            let item = r.entry.menuItem?.lowercased() ?? ""
            let menu = r.entry.menu.lowercased()
            let keys = r.entry.shortcuts.joined(separator: " ").lowercased()
            let description = r.entry.description.lowercased()
            var best = 3
            for t in terms {
                if title.contains(t) { best = min(best, 0) }
                else if item.contains(t) { best = min(best, 1) }
                else if menu.contains(t) || keys.contains(t) { best = min(best, 2) }
                else if description.contains(t) { best = min(best, 3) }
                else { return nil }
            }
            return best
        }
        let ranked = all.enumerated().compactMap { i, r in rank(r).map { (rank: $0, index: i, row: r) } }
        return ranked.sorted { ($0.rank, $0.index) < ($1.rank, $1.index) }.map(\.row)
    }

    /// A leading-colon line jump (`:42`, `:42:7`, `:+5` / `:-5`). Nil when the
    /// query is not that form, so ordinary command filtering still runs.
    static func lineJumpInput(from query: String) -> String? {
        let t = query.trimmingCharacters(in: .whitespaces)
        guard t.hasPrefix(":"), t.count > 1 else { return nil }
        if case .success = EditorNavigation.parseLineTarget(t) { return t }
        return nil
    }

    /// Palette `:N` route: resolve against the active buffer and select. False
    /// when `query` is not a colon jump (the caller should run a command row).
    @MainActor
    static func performLineJump(_ query: String, model: ShellModel) -> Bool {
        guard let input = lineJumpInput(from: query) else { return false }
        return model.applyGoToLine(input)
    }

    /// Runs `command` through the same model operations its menu item uses.
    /// Returns false for commands that cannot be run from the palette.
    @MainActor
    static func perform(_ command: AccessibilityCommand, model: ShellModel, openWindow: OpenWindowAction) -> Bool {
        switch command {
        case .editorPreferences:
            NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
        case .openLaTeXFile: model.openTexPanel()
        case .newProject: model.scaffold.presentNewProject() // ProjectScaffoldViews.swift
        case .newFile: model.scaffold.presentNewFile()
        case .save: model.saveTexInteractive()
        case .saveAs: _ = model.saveTexAs()
        case .openFixture: model.openFixturePanel()
        case .reloadFixture: model.reloadFixture()
        case .attachBuiltCompiler: _ = model.attachDiscoveredWorker()
        case .attachRenderPipeline: _ = model.attachDiscoveredRenderPipeline()
        case .attachWorker: model.attachWorkerPanel()
        case .compile: if !model.outputBoundExplicitRetry() { model.compile() }
        case .exportPDF: model.exportPDF()
        case .exportPDFViaRust: model.exportPDFViaRust()
        case .exportPDFExact: model.exportPDFExact()
        case .printDocument: model.printDocument()
        case .printSource: model.printSource()
        case .pinInsertionPoint: model.pinAnchorAtCaret()
        case .openCaptureProposal: model.openProposalPanel()
        case .submitSampleCapture: model.submitSampleCapturePanel()
        case .convertCapture: model.convertLatestCapture()
        case .nearbyCompanion: openWindow(id: "nearby")
        case .restoreDiscardedBuffer: _ = model.restoreDiscardedBuffer()
        case .undo: NSApp.sendAction(Selector(("undo:")), to: nil, from: nil)
        case .find: EditorFindAction.send(.showFindInterface)
        case .findAndReplace: EditorFindAction.send(.showReplaceInterface)
        case .findNext: EditorFindAction.send(.nextMatch)
        case .findPrevious: EditorFindAction.send(.previousMatch)
        case .useSelectionForFind: EditorFindAction.send(.setSearchString)
        case .jumpToSelection: EditorFindAction.centerSelection()
        case .reindentLines: EditorIndentationAction.reindentLines()
        case .reindentDocument: EditorIndentationAction.reindentDocument()
        case .completion, .completionList, .toggleComment, .duplicateLine, .signatureHelp, .selectPreviewItemSource, .nextSearchMatch: return false
        case .goToMatching: model.goToMatching()
        case .goToDefinition: model.goToDefinition() // ShellModel+EditorNavigation.swift
        case .goToSymbol: model.editorNavigation.symbolPickerShown = true
        case .goToLine: model.presentGoToLine()
        case .selectEnvironment: model.selectEnvironment()
        case .wrapInEnvironment: model.editorNavigation.wrapShown = true
        case .renameSymbol: model.presentRenameSymbol()
        case .fold: EditorFoldAction.fold()
        case .unfold: EditorFoldAction.unfold()
        case .foldAll: EditorFoldAction.foldAll()
        case .unfoldAll: EditorFoldAction.unfoldAll()
        case .nextDiagnostic: model.goToDiagnostic(forward: true)
        case .previousDiagnostic: model.goToDiagnostic(forward: false)
        case .nextOccurrence: model.stepOccurrence(forward: true, panel: model.problemsPanel)
        case .previousOccurrence: model.stepOccurrence(forward: false, panel: model.problemsPanel)
        case .copyDiagnosticsAsText: _ = model.copyDiagnosticsAsText(panel: model.problemsPanel)
        case .revealCaretInPreview: model.revealCaretInPreview()
        case .accessibilityHelp: openWindow(id: AccessibilityHelpView.windowID)
        case .durableHistory: openWindow(id: EditHistoryPanel.windowID)
        case .findInProject: openWindow(id: ProjectSearch.windowID)
        case .renameCitation: openWindow(id: CitationRename.windowID)
        case .commandPalette: model.commandPaletteShown.toggle()
        case .toggleProblems: model.problemsVisible.toggle()
        case .toggleCaptures: model.captureInboxVisible.toggle() // CaptureInbox.swift
        case .toggleVimKeybindings: EditorPreferences.shared.vimKeybindings.toggle() // VimMode.swift
        case .zoomIn: model.previewZoomIn() // PreviewZoom.swift
        case .zoomOut: model.previewZoomOut()
        case .actualSize: model.previewActualSize()
        case .fitWidth: model.previewFitWidth()
        case .fitPage: model.previewFitPage()
        case .increaseEditorFontSize: model.increaseEditorFontSize()
        case .decreaseEditorFontSize: model.decreaseEditorFontSize()
        case .resetEditorFontSize: model.resetEditorFontSize()
        }
        return true
    }
}

/// Search Everywhere for a LaTeX project (design-principles §10): scope
/// tabs (All / Files / Sections / Labels / Citations / Commands), dense rows
/// with a typed icon and dimmed context, a full-width selection band and a
/// quiet footer. Tab cycles the scopes. Files switch or open project
/// members; Sections and Labels jump the editor there; Commands run exactly
/// what their menu items run. Everything is built from state the shell
/// already holds — the project listing, the outline scan, the helper's
/// project-index metadata and the accessibility command table.
enum PaletteScope: String, CaseIterable {
    case all = "All", files = "Files", sections = "Sections", labels = "Labels", citations = "Citations", commands = "Commands"
}

struct PaletteEntry: Identifiable, Equatable {
    enum Payload: Equatable {
        case command(AccessibilityCommand)
        case file(path: String, open: Bool, from: String?)
        case outline(DocumentOutline.Item)
        case citation(name: String, detail: String, definedIn: String?)
        /// `:N` / `:N:C` / `:+N` query: not a scope, a jump. #476 has no
        /// typed `:` prefix (colons are only in internal ids like `file:`).
        case lineJump
    }
    let id: String
    let title: String
    /// Dimmed context after the title (menu, path, line…).
    let context: String
    let payload: Payload
}

struct CommandPalette: View {
    @Environment(ShellModel.self) var model
    @Environment(\.openWindow) private var openWindow
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @State private var scope: PaletteScope = .all
    @State private var selected: String?
    /// Outline of the active buffer, scanned once when the palette opens.
    @State private var outline: [DocumentOutline.Item] = []
    @FocusState private var fieldFocused: Bool

    static let identifier = "command.palette"

    var body: some View {
        let entries = filteredEntries()
        VStack(spacing: 0) {
            HStack(spacing: DS.Space.m) {
                Image(systemName: "magnifyingglass").foregroundStyle(DS.Colors.textSecondary)
                TextField("Search files, sections, labels and commands…", text: $query)
                    .textFieldStyle(.plain).font(DS.Fonts.field)
                    .focused($fieldFocused)
                    .accessibilityLabel("Palette search")
                    .onKeyPress(.downArrow) { move(1, in: entries); return .handled }
                    .onKeyPress(.upArrow) { move(-1, in: entries); return .handled }
                    .onKeyPress(.return) { run(selectedEntry(in: entries)); return .handled }
                    .onKeyPress(.escape) { dismiss(); return .handled }
                    .onKeyPress(.tab) { cycleScope(1); return .handled }
                Text("\(entries.count)").font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textTertiary)
            }
            .padding(.horizontal, DS.Space.l).padding(.vertical, DS.Space.m)
            // The scope strip (Search Everywhere): click or Tab through.
            HStack(spacing: DS.Space.xxs) {
                ForEach(PaletteScope.allCases, id: \.self) { s in
                    Button {
                        scope = s; selected = nil
                    } label: {
                        Text(s.rawValue)
                            .font(scope == s ? DS.Fonts.header : DS.Fonts.secondary)
                            .foregroundStyle(scope == s ? DS.Colors.textPrimary : DS.Colors.textSecondary)
                            .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xxs)
                            .background(scope == s ? DS.Colors.textPrimary.opacity(DS.State.raisedFillOpacity) : .clear,
                                        in: RoundedRectangle(cornerRadius: DS.Radius.tab))
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(scope == s ? .isSelected : [])
                }
                Spacer()
            }
            .padding(.horizontal, DS.Space.l).padding(.bottom, DS.Space.s)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Search scope")
            Divider()
            if entries.isEmpty {
                ContentUnavailableView.search(text: query)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(entries, selection: $selected) { entry in
                    PaletteRow(entry: entry)
                        .tag(entry.id)
                        .contentShape(Rectangle())
                        .onTapGesture { run(entry) }
                }
                .listStyle(.plain)
                .environment(\.defaultMinListRowHeight, DS.Row.paletteResult)
                .accessibilityIdentifier(Self.identifier)
            }
            Divider()
            HStack(spacing: DS.Space.l) {
                hint("↑↓", "choose"); hint("⇥", "scope"); hint("⏎", "open"); hint("esc", "close")
                Spacer()
                Text("Files, sections, labels, citations, and every command of Help > FlashTeX Accessibility Help.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textTertiary).lineLimit(1)
            }
            .padding(.horizontal, DS.Space.l).padding(.vertical, DS.Space.s)
        }
        .frame(width: DS.Layout.paletteSize.width, height: DS.Layout.paletteSize.height)
        .onAppear {
            fieldFocused = true
            outline = model.outline
            selected = filteredEntries().first?.id
        }
        .onChange(of: query) { _, _ in selected = filteredEntries().first?.id }
        .onChange(of: scope) { _, _ in selected = filteredEntries().first?.id }
    }

    // MARK: entries

    private func allEntries() -> [PaletteEntry] {
        var out: [PaletteEntry] = []
        if scope == .all || scope == .files {
            for doc in model.chrome.listing {
                out.append(PaletteEntry(id: "file:\(doc.path)", title: doc.path,
                                        context: doc.role == .entry ? "entry document" : "open",
                                        payload: .file(path: doc.path, open: true, from: nil)))
            }
            for n in model.chrome.closure.nodes where n.state == .available {
                let name = n.resolvedPath ?? n.reference.argument
                out.append(PaletteEntry(id: "file+:\(name)", title: name,
                                        context: "included from \(n.from) — not open",
                                        payload: .file(path: name, open: false, from: n.from)))
            }
        }
        if scope == .all || scope == .sections {
            for item in outline where item.kind == .section {
                out.append(PaletteEntry(id: "outline:\(item.id)", title: item.displayTitle,
                                        context: "\\\(item.command) · line \(item.line)",
                                        payload: .outline(item)))
            }
        }
        if scope == .all || scope == .labels {
            for item in outline where item.kind == .label {
                out.append(PaletteEntry(id: "outline:\(item.id)", title: item.title,
                                        context: "\\label · line \(item.line)",
                                        payload: .outline(item)))
            }
        }
        if scope == .all || scope == .citations {
            for item in model.completionMetadata?.citations ?? [] {
                out.append(PaletteEntry(id: "cite:\(item.name)", title: item.name,
                                        context: item.detail(noun: "defined", revision: model.completionMetadata?.revision ?? 0),
                                        payload: .citation(name: item.name, detail: "", definedIn: item.definedIn)))
            }
        }
        if scope == .all || scope == .commands {
            for row in CommandPaletteModel.all {
                out.append(PaletteEntry(id: "cmd:\(row.entry.title)", title: row.entry.title,
                                        context: row.entry.menu + (row.entry.shortcuts.first.map { " · \($0)" } ?? ""),
                                        payload: .command(row.id)))
            }
        }
        return out
    }

    private func filteredEntries() -> [PaletteEntry] {
        // `:42` is a query prefix, not a scope: it short-circuits Files /
        // Sections / … the same way the old palette replaced its list.
        if let jump = CommandPaletteModel.lineJumpInput(from: query) {
            let preview: String = {
                switch EditorNavigation.resolveLineTarget(model.activeText, input: jump, caret: model.caretUTF16) {
                case .success(let t): return "Go to line \(t.line), column \(t.column)"
                case .failure(let h): return h.message
                }
            }()
            return [PaletteEntry(id: "goto:\(jump)", title: preview,
                                 context: "⏎ jumps · esc closes", payload: .lineJump)]
        }
        let entries = allEntries()
        let terms = query.lowercased().split(whereSeparator: \.isWhitespace).map(String.init)
        guard !terms.isEmpty else { return entries }
        // Commands keep their own richer ranking (menus, shortcuts,
        // descriptions); everything else matches on title + context, title
        // matches first.
        func rank(_ e: PaletteEntry) -> Int? {
            if case .command(let command) = e.payload {
                return CommandPaletteModel.rows(matching: query).contains { $0.id == command } ? 2 : nil
            }
            let title = e.title.lowercased(), context = e.context.lowercased()
            var best = 3
            for t in terms {
                if title.contains(t) { best = min(best, title.hasPrefix(t) ? 0 : 1) }
                else if context.contains(t) { best = min(best, 3) }
                else { return nil }
            }
            return best
        }
        return entries.enumerated().compactMap { i, e in rank(e).map { (rank: $0, index: i, entry: e) } }
            .sorted { ($0.rank, $0.index) < ($1.rank, $1.index) }.map(\.entry)
    }

    private func hint(_ key: String, _ what: String) -> some View {
        HStack(spacing: DS.Space.xxs) { KeyCap(key); Text(what).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary) }
    }

    private func selectedEntry(in entries: [PaletteEntry]) -> PaletteEntry? {
        entries.first { $0.id == selected } ?? entries.first
    }

    private func move(_ delta: Int, in entries: [PaletteEntry]) {
        guard !entries.isEmpty else { return }
        let i = entries.firstIndex { $0.id == selected } ?? 0
        selected = entries[(i + delta + entries.count) % entries.count].id
    }

    private func cycleScope(_ delta: Int) {
        let all = PaletteScope.allCases
        let i = all.firstIndex(of: scope) ?? 0
        scope = all[(i + delta + all.count) % all.count]
        selected = nil
    }

    private func run(_ entry: PaletteEntry?) {
        guard let entry else { return }
        switch entry.payload {
        case .command(let command):
            guard CommandPaletteModel.isRunnable(command) else {
                let e = command.entry
                model.navigationNote = "\(e.title) is a key inside the \(e.menu.lowercased()); see Help > FlashTeX Accessibility Help."
                dismiss(); return
            }
            dismiss()
            // Run after the sheet is gone so panels and windows open over the main window.
            DispatchQueue.main.async { _ = CommandPaletteModel.perform(command, model: model, openWindow: openWindow) }
        case .file(let path, let open, let from):
            dismiss()
            if open { model.switchOrNote(path) }
            else { Task { await model.project.openDocument(path, role: from.map { .included(from: $0) } ?? .opened) } }
        case .outline(let item):
            dismiss()
            model.reveal(outlineItem: item)
        case .citation(_, _, let definedIn):
            dismiss()
            if let definedIn { model.switchOrNote(definedIn) }
        case .lineJump:
            let q = query
            dismiss()
            DispatchQueue.main.async { _ = CommandPaletteModel.performLineJump(q, model: model) }
        }
    }
}

private struct PaletteRow: View {
    let entry: PaletteEntry

    var body: some View {
        HStack(spacing: DS.Space.m) {
            icon
                .font(DS.Fonts.secondary)
                .frame(width: DS.Size.fileIcon)
            Text(entry.title).font(DS.Fonts.base).lineLimit(1)
            Text(entry.context).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1)
            Spacer(minLength: DS.Space.m)
            trailing
        }
        .padding(.vertical, DS.Space.xxs)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(spoken)
    }

    @ViewBuilder private var icon: some View {
        switch entry.payload {
        case .command:
            Image(systemName: "command").foregroundStyle(DS.Colors.textSecondary)
        case .file(let path, _, _):
            let style = FileTypeStyle.of(path: path)
            Image(systemName: style.systemImage).foregroundStyle(style.color)
        case .outline(let item):
            Image(systemName: OutlineItemStyle.icon(item)).foregroundStyle(OutlineItemStyle.color(item))
        case .citation:
            Image(systemName: "quote.opening").foregroundStyle(DS.Colors.typeLabel)
        case .lineJump:
            Image(systemName: "number").foregroundStyle(DS.Colors.textSecondary)
        }
    }

    @ViewBuilder private var trailing: some View {
        if case .command(let command) = entry.payload {
            HStack(spacing: DS.Space.xs) {
                ForEach(command.entry.shortcuts, id: \.self) { s in KeyCap(s) }
            }
            if !CommandPaletteModel.isRunnable(command) {
                Image(systemName: "keyboard").foregroundStyle(DS.Colors.textTertiary)
                    .help("A key inside the editor or a window; not runnable from the palette")
            }
        }
    }

    private var spoken: String {
        switch entry.payload {
        case .command(let command): return command.entry.helpLine + (CommandPaletteModel.isRunnable(command) ? "" : " Not runnable from the palette.")
        case .file(let path, let open, _): return "\(path), \(open ? "open" : "not open"); activate to show it"
        case .outline(let item): return "\(item.command) \(item.title), line \(item.line); activate to select it"
        case .citation(let name, _, _): return "citation \(name); activate to open its bibliography source"
        case .lineJump: return "\(entry.title); activate to jump"
        }
    }
}

/// A shortcut spelling drawn as a key cap ("⌘⇧P", "Esc", "File > Export PDF…").
struct KeyCap: View {
    let text: String
    init(_ text: String) { self.text = text }

    var body: some View {
        Text(text)
            .font(DS.Fonts.monoSecondary)
            .padding(.horizontal, DS.Space.xs).padding(.vertical, DS.Space.xxs)
            .background(DS.Colors.textPrimary.opacity(DS.State.raisedFillOpacity), in: RoundedRectangle(cornerRadius: DS.Radius.control))
            .overlay(RoundedRectangle(cornerRadius: DS.Radius.control).strokeBorder(DS.Colors.textPrimary.opacity(DS.State.hairlineOpacity)))
            .lineLimit(1)
    }
}
