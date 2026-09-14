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
    static let notRunnable: Set<AccessibilityCommand> = [.completion, .completionList, .toggleComment, .signatureHelp, .selectPreviewItemSource, .nextSearchMatch]

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
        case .completion, .completionList, .toggleComment, .signatureHelp, .selectPreviewItemSource, .nextSearchMatch: return false
        case .goToMatching: model.goToMatching()
        case .goToDefinition: model.goToDefinition() // ShellModel+EditorNavigation.swift
        case .goToSymbol: model.editorNavigation.symbolPickerShown = true
        case .selectEnvironment: model.selectEnvironment()
        case .wrapInEnvironment: model.editorNavigation.wrapShown = true
        case .renameSymbol: model.presentRenameSymbol()
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
        case .increaseEditorFontSize: model.increaseEditorFontSize()
        case .decreaseEditorFontSize: model.decreaseEditorFontSize()
        case .resetEditorFontSize: model.resetEditorFontSize()
        }
        return true
    }
}

struct CommandPalette: View {
    @Environment(ShellModel.self) var model
    @Environment(\.openWindow) private var openWindow
    @Environment(\.dismiss) private var dismiss
    @State private var query = ""
    @State private var selected: AccessibilityCommand?
    @FocusState private var fieldFocused: Bool

    static let identifier = "command.palette"

    private var rows: [CommandPaletteModel.Row] { CommandPaletteModel.rows(matching: query) }

    var body: some View {
        let rows = rows
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Image(systemName: "command").foregroundStyle(.secondary)
                TextField("Type a command, menu or shortcut…", text: $query)
                    .textFieldStyle(.plain).font(.title3)
                    .focused($fieldFocused)
                    .accessibilityLabel("Command palette search")
                    .onKeyPress(.downArrow) { move(1, in: rows); return .handled }
                    .onKeyPress(.upArrow) { move(-1, in: rows); return .handled }
                    .onKeyPress(.return) { run(selectedRow(in: rows)); return .handled }
                    .onKeyPress(.escape) { dismiss(); return .handled }
                Text("\(rows.count)").font(.caption).foregroundStyle(.tertiary).monospacedDigit()
            }
            .padding(.horizontal, 14).padding(.vertical, 10)
            Divider()
            if rows.isEmpty {
                ContentUnavailableView.search(text: query)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List(rows, selection: $selected) { row in
                    PaletteRow(row: row, selected: row.id == selectedRow(in: rows)?.id)
                        .tag(row.id)
                        .contentShape(Rectangle())
                        .onTapGesture { run(row) }
                }
                .listStyle(.plain)
                .accessibilityIdentifier(Self.identifier)
            }
            Divider()
            HStack(spacing: 14) {
                hint("↑↓", "choose"); hint("⏎", "run"); hint("esc", "close")
                Spacer()
                Text("Every command in Help > FlashTeX Accessibility Help is listed here with its shortcut.")
                    .font(.caption2).foregroundStyle(.tertiary).lineLimit(1)
            }
            .padding(.horizontal, 14).padding(.vertical, 6)
        }
        .frame(width: 620, height: 440)
        .onAppear { fieldFocused = true; selected = rows.first?.id }
        .onChange(of: query) { _, _ in selected = self.rows.first?.id }
    }

    private func hint(_ key: String, _ what: String) -> some View {
        HStack(spacing: 3) { KeyCap(key); Text(what).font(.caption2).foregroundStyle(.secondary) }
    }

    private func selectedRow(in rows: [CommandPaletteModel.Row]) -> CommandPaletteModel.Row? {
        rows.first { $0.id == selected } ?? rows.first
    }

    private func move(_ delta: Int, in rows: [CommandPaletteModel.Row]) {
        guard !rows.isEmpty else { return }
        let i = rows.firstIndex { $0.id == selected } ?? 0
        selected = rows[(i + delta + rows.count) % rows.count].id
    }

    private func run(_ row: CommandPaletteModel.Row?) {
        guard let row else { return }
        guard row.runnable else {
            model.navigationNote = "\(row.entry.title) is a key inside the \(row.entry.menu.lowercased()); see Help > FlashTeX Accessibility Help."
            dismiss(); return
        }
        dismiss()
        // Run after the sheet is gone so panels and windows open over the main window.
        let command = row.id
        DispatchQueue.main.async { _ = CommandPaletteModel.perform(command, model: model, openWindow: openWindow) }
    }
}

private struct PaletteRow: View {
    let row: CommandPaletteModel.Row
    let selected: Bool

    var body: some View {
        HStack(spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(row.entry.title).font(.body)
                        .foregroundStyle(row.runnable ? Color.primary : Color.secondary)
                    Text(row.entry.menu).font(.caption2)
                        .padding(.horizontal, 5).padding(.vertical, 1)
                        .background(Color.secondary.opacity(0.12), in: Capsule())
                        .foregroundStyle(.secondary)
                }
                Text(row.entry.description).font(.caption).foregroundStyle(.secondary).lineLimit(1)
            }
            Spacer(minLength: 8)
            HStack(spacing: 4) {
                ForEach(row.entry.shortcuts, id: \.self) { s in KeyCap(s) }
            }
            if !row.runnable {
                Image(systemName: "keyboard").foregroundStyle(.tertiary).help("A key inside the editor or a window; not runnable from the palette")
            }
        }
        .padding(.vertical, 3)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(row.entry.helpLine + (row.runnable ? "" : " Not runnable from the palette."))
    }
}

/// A shortcut spelling drawn as a key cap ("⌘⇧P", "Esc", "File > Export PDF…").
struct KeyCap: View {
    let text: String
    init(_ text: String) { self.text = text }

    var body: some View {
        Text(text)
            .font(.caption.monospaced())
            .padding(.horizontal, 5).padding(.vertical, 2)
            .background(Color.primary.opacity(0.06), in: RoundedRectangle(cornerRadius: 4))
            .overlay(RoundedRectangle(cornerRadius: 4).strokeBorder(Color.primary.opacity(0.15)))
            .lineLimit(1)
    }
}
