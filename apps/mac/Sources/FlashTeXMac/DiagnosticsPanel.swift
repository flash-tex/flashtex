import AppKit
import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility

// Diagnostics panel follow-up (Commander item 9, mac-diagnostics-3):
//
// - Grouped rows (`EditorDiagnostics.groups`) speak their count and the
//   occurrence they stand on: "12 places, 3 of 12, main.tex line 41"
//   (`DiagnosticRowAccessibility.GroupInfo`, FlashTeXAccessibility).
// - Next / Previous Occurrence (⌘⌥] / ⌘⌥[, Navigate menu) step within one
//   group; ⌘⇧] / ⌘⇧[ keep stepping across every underlined diagnostic.
// - The panel's list has a selection: Return jumps to the selected group's
//   current occurrence, Esc hands the keyboard back to the editor, ⌘C on
//   the focused list (and Edit > Copy Diagnostics as Text, ⌘⌥C, anywhere)
//   copies "path:line: message" lines for the selection — every occurrence
//   of a grouped row, or all diagnostics when nothing is selected.
//
// `ContentView.diagnosticsList` hosts `DiagnosticsListView`; the panel state
// reaches the menu commands through `focusedSceneValue`.

/// Selection and per-group occurrence cursor of the diagnostics panel; the
/// view owns one and publishes it to the scene's commands.
@Observable
final class DiagnosticsPanelState {
    /// Selected group (`EditorDiagnostics.Group.id`), or nil.
    var selection: String?
    /// The occurrence (zero-based, document order) each group stands on;
    /// missing means the first.
    var occurrence: [String: Int] = [:]

    init() {}

    func currentOccurrence(of group: EditorDiagnostics.Group) -> Int {
        min(max(occurrence[group.id] ?? 0, 0), max(group.count - 1, 0))
    }
}

struct DiagnosticsPanelFocusedKey: FocusedValueKey {
    typealias Value = DiagnosticsPanelState
}

extension FocusedValues {
    var diagnosticsPanel: DiagnosticsPanelState? {
        get { self[DiagnosticsPanelFocusedKey.self] }
        set { self[DiagnosticsPanelFocusedKey.self] = newValue }
    }
}

// MARK: - Pure text rules (tested without a window)

extension EditorDiagnostics {
    /// "main.tex line 41" / "main.tex bytes 10..<20" / "no source" for
    /// occurrence `k` of `group` — the location part of `occurrenceLabel`.
    static func occurrenceLocation(_ k: Int, of group: Group, in diagnostics: [RuntimeV1.Diagnostic],
                                   texts: [String: String] = [:]) -> String {
        let label = occurrenceLabel(k, of: group, in: diagnostics, texts: texts)
        let prefix = "\(k + 1) of \(group.count): "
        return label.hasPrefix(prefix) ? String(label.dropFirst(prefix.count)) : label
    }

    /// The spoken group part of a panel row: nil for a single occurrence,
    /// else "12 places, 3 of 12, main.tex line 41".
    static func groupInfo(_ group: Group, occurrence k: Int, in diagnostics: [RuntimeV1.Diagnostic],
                          texts: [String: String] = [:]) -> DiagnosticRowAccessibility.GroupInfo? {
        guard group.count > 1 else { return nil }
        let k = min(max(k, 0), group.count - 1)
        return .init(count: group.count, occurrence: k, location: occurrenceLocation(k, of: group, in: diagnostics, texts: texts))
    }

    /// One "path:line: message" line per diagnostic (`path:line:` from the
    /// compiled text when known, else `path:byte<start>:`; unsourced
    /// diagnostics read `-:0: message`), each prefixed with the severity as
    /// compilers print it, so a copied block pastes into an issue as is:
    ///
    ///     main.tex:41: error: \in is not supported in math mode
    ///     -:0: warning: Overfull line
    static func copyLine(_ d: RuntimeV1.Diagnostic, texts: [String: String] = [:]) -> String {
        let severity = d.severity == .error ? "error" : "warning"
        let header: String
        if let s = d.source {
            if let text = texts[s.path], let line = lineNumber(ofByte: s.startByte, in: text) {
                header = "\(s.path):\(line): \(severity): \(d.message)"
            } else {
                header = "\(s.path):byte\(s.startByte): \(severity): \(d.message)"
            }
        } else {
            header = "-:0: \(severity): \(d.message)"
        }
        var lines = [header]
        if let notes = d.notes {
            for note in notes where !note.isEmpty { lines.append("= note: \(note)") }
        }
        if let help = d.help?.message, !help.isEmpty { lines.append("= help: \(help)") }
        return lines.joined(separator: "\n")
    }

    /// The clipboard text for the panel: the selected group's occurrences in
    /// document order, or every diagnostic (group by group) when `selection`
    /// is nil or names no group. Lines are joined with "\n", no trailing newline.
    static func copyText(groups: [Group], selection: String?, in diagnostics: [RuntimeV1.Diagnostic],
                         texts: [String: String] = [:]) -> String {
        let chosen = groups.first { $0.id == selection }.map { [$0] } ?? groups
        return chosen.flatMap { g in g.occurrences.compactMap { diagnostics.indices.contains($0) ? copyLine(diagnostics[$0], texts: texts) : nil } }
            .joined(separator: "\n")
    }

    /// Secondary label captions for the row hover (`.help`); nil when there
    /// are none. Primary labels are the diagnostic span itself.
    static func secondaryLabelHelp(_ d: RuntimeV1.Diagnostic) -> String? {
        guard let labels = d.labels else { return nil }
        let texts = labels.filter { !$0.primary }.map(\.text).filter { !$0.isEmpty }
        return texts.isEmpty ? nil : texts.joined(separator: "\n")
    }
}

// MARK: - Shell actions

extension ShellModel {
    /// Jumps to occurrence `k` of `group` and makes it the group's current
    /// occurrence (spoken as "k+1 of n" on the row) and the caret's current
    /// diagnostic for ⌘⇧] / ⌘⇧[. The footer names the occurrence.
    func goToOccurrence(_ k: Int, of group: EditorDiagnostics.Group, panel: DiagnosticsPanelState) {
        let diags = displayedDiagnostics
        guard group.occurrences.indices.contains(k) else { return }
        panel.selection = group.id
        panel.occurrence[group.id] = k
        let index = group.occurrences[k]
        navigate(to: EditorDiagnostics.occurrence(k, of: group, in: diags))
        if let result, let id = EditorDiagnostics.identity(resultID: resultID, index: index, in: result) {
            currentDiagnosticID = id.key
        }
        let label = EditorDiagnostics.occurrenceLabel(k, of: group, in: diags, texts: compiledDocuments)
        navigationNote = (group.count > 1 ? "\(label) — " : "") + group.message
            + (navigationNote.map { " (" + $0 + ")" } ?? "")
    }

    /// ⌘⌥] / ⌘⌥[: the next (previous) occurrence of the panel's selected
    /// group — else the group under the caret's current diagnostic, else
    /// the first group with more than one place — wrapping within the group.
    func stepOccurrence(forward: Bool, panel: DiagnosticsPanelState) {
        let diags = displayedDiagnostics
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: documents.map(\.path))
        guard !groups.isEmpty else { navigationNote = "No diagnostics to step through."; return }
        let group: EditorDiagnostics.Group
        if let g = groups.first(where: { $0.id == panel.selection }) {
            group = g
        } else if let current = currentDiagnosticID, let index = Self.diagnosticIndex(ofIdentityKey: current),
                  let g = groups.first(where: { $0.occurrences.contains(index) }) {
            group = g
            panel.occurrence[g.id] = g.occurrences.firstIndex(of: index) ?? 0
        } else if let g = groups.first(where: { $0.count > 1 }) ?? groups.first {
            group = g
            // Start before the first (or after the last) so the first step lands on an end.
            panel.occurrence[g.id] = forward ? -1 : g.count
        } else { return }
        var k = (panel.occurrence[group.id] ?? 0) + (forward ? 1 : -1)
        if k >= group.count { k = 0 }
        if k < 0 { k = group.count - 1 }
        goToOccurrence(k, of: group, panel: panel)
        if group.count == 1 { navigationNote = "Only one place: " + (navigationNote ?? group.message) }
    }

    /// Return in the panel: the selected group's current occurrence.
    func goToSelectedOccurrence(panel: DiagnosticsPanelState) {
        let diags = displayedDiagnostics
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: documents.map(\.path))
        guard let g = groups.first(where: { $0.id == panel.selection }) else {
            navigationNote = "Select a diagnostic first (↑/↓), then press Return."; return
        }
        goToOccurrence(panel.currentOccurrence(of: g), of: g, panel: panel)
    }

    /// Esc in the panel: the editor takes the keyboard back at its caret.
    /// Re-applying the caret selection with a new token makes
    /// `SourceEditorView` call `makeFirstResponder` on the text view (and
    /// announce "Line L, column C"); no other view is touched.
    func returnKeyboardToEditor() {
        let text = activeText as NSString
        let location = min(max(caretUTF16, 0), text.length)
        let length = min(max(caretLengthUTF16, 0), text.length - location)
        selection = .init(path: activePath, nsRange: NSRange(location: location, length: length),
                          token: (selection?.token ?? 0) + 1)
    }

    /// The clipboard text the panel copies (see `EditorDiagnostics.copyText`).
    func diagnosticsCopyText(panel: DiagnosticsPanelState?) -> String {
        let diags = displayedDiagnostics
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: documents.map(\.path))
        return EditorDiagnostics.copyText(groups: groups, selection: panel?.selection, in: diags, texts: compiledDocuments)
    }

    /// Edit > Copy Diagnostics as Text (⌘⌥C) and ⌘C on the focused list:
    /// writes `diagnosticsCopyText` to `pasteboard` as plain text. Returns
    /// the text written (empty: nothing was written, footer says so).
    @discardableResult
    func copyDiagnosticsAsText(panel: DiagnosticsPanelState?, pasteboard: NSPasteboard = .general) -> String {
        let text = diagnosticsCopyText(panel: panel)
        guard !text.isEmpty else { navigationNote = "No diagnostics to copy."; return "" }
        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)
        let n = text.split(separator: "\n").count
        navigationNote = "Copied \(n) diagnostic line\(n == 1 ? "" : "s") as text."
        return text
    }

    /// The `index` of an `EditorDiagnostics.Identity.key` ("id#index@path:…").
    static func diagnosticIndex(ofIdentityKey key: String) -> Int? {
        guard let hash = key.firstIndex(of: "#"), let at = key[hash...].firstIndex(of: "@") else { return nil }
        return Int(key[key.index(after: hash)..<at])
    }
}

// MARK: - Menu commands (Edit)

/// Edit > Copy Diagnostics as Text (⌘⌥C). `FlashTeXMacApp` adds
/// `DiagnosticsCommands(model: model)` to its `.commands` next to
/// `NavigationCommands`; the item is enabled while a diagnostics panel is
/// in the scene (`focusedSceneValue`).
struct DiagnosticsCommands: Commands {
    var model: ShellModel
    @FocusedValue(\.diagnosticsPanel) private var panel

    var body: some Commands {
        CommandGroup(after: .pasteboard) {
            Button("Copy Diagnostics as Text") { model.copyDiagnosticsAsText(panel: panel) }
                .keyboardShortcut("c", modifiers: [.command, .option])
                .disabled(panel == nil)
        }
    }
}

// MARK: - The panel

/// The diagnostics list under the preview: grouped rows with a selection,
/// Return / Esc / ⌘C handling and the spoken group information. Replaces
/// the body of `ContentView.diagnosticsList(_:)`.
struct DiagnosticsListView: View {
    @Environment(ShellModel.self) private var model
    let diagnostics: [RuntimeV1.Diagnostic]
    @State private var panel: DiagnosticsPanelState

    /// Identifier of the list for tests and assistive clients.
    static let listIdentifier = "diagnostics.list"

    /// - Parameter panel: the state to use (tests drive the selection through
    ///   it); the view makes its own when nil.
    /// Severity to show (nil: all); group indices stay those of `diagnostics`,
    /// so explanations, quick fixes and occurrences are unaffected by the filter.
    let severityFilter: RuntimeV1.Severity?
    /// Whether the "Diagnostics (n) — …" caption is drawn (the Problems panel draws its own header).
    let showsHeader: Bool
    let maxHeight: CGFloat

    init(diagnostics: [RuntimeV1.Diagnostic], panel: DiagnosticsPanelState? = nil,
         severityFilter: RuntimeV1.Severity? = nil, showsHeader: Bool = true, maxHeight: CGFloat = 180) {
        self.diagnostics = diagnostics
        self.severityFilter = severityFilter
        self.showsHeader = showsHeader
        self.maxHeight = maxHeight
        _panel = State(initialValue: panel ?? DiagnosticsPanelState())
    }

    var body: some View {
        let diags = diagnostics
        // Change-only / throttled reads (ShellModel mirrors, ShellChrome): `documents`,
        // `result` and `editorMarkReport` change on every keystroke or reply and
        // re-evaluated this List (an AppKit table) with each of them.
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: model.chrome.listing.map(\.path))
            .filter { severityFilter == nil || $0.severity == severityFilter }
        let status = model.resultStatus ?? .ok
        VStack(alignment: .leading, spacing: 0) {
            if showsHeader {
                Text("Diagnostics (\(diags.count)\(groups.count < diags.count ? " in \(groups.count) groups" : "")) — the preview above is still shown; errors are not hidden")
                    .font(.caption.bold()).padding(.horizontal, 8).padding(.vertical, 4)
            }
            if let carriedLine = model.chrome.carriedLine {
                Text("Underlines \(carriedLine); the list below is the failed result's.")
                    .font(.caption).foregroundStyle(.orange).padding(.horizontal, 8).padding(.bottom, 4)
            }
            List(groups, selection: $panel.selection) { g in
                row(g, in: diags, status: status)
            }
            .accessibilityIdentifier(Self.listIdentifier)
            .frame(minHeight: 80, maxHeight: maxHeight)
            .onKeyPress(.return) { model.goToSelectedOccurrence(panel: panel); return .handled }
            .onKeyPress(.escape) { model.returnKeyboardToEditor(); return .handled }
            .copyable([model.diagnosticsCopyText(panel: panel)])
        }
        .focusedSceneValue(\.diagnosticsPanel, panel)
    }

    /// "3 of 12: main.tex line 41" for a group, "main.tex line 3" for one
    /// diagnostic when the compiled text is known (an author never thinks in
    /// bytes; "bytes a..<b" stays the fallback), "no source mapping" otherwise.
    private func location(of g: EditorDiagnostics.Group, occurrence k: Int, diagnostic d: RuntimeV1.Diagnostic,
                          group: DiagnosticRowAccessibility.GroupInfo?) -> String {
        if g.count > 1 { return "\(k + 1) of \(g.count): \(group?.location ?? "no source")" }
        guard let src = d.source else { return "no source mapping" }
        if let text = model.compiledDocuments[src.path], let line = EditorDiagnostics.lineNumber(ofByte: src.startByte, in: text) {
            return "\(src.path) line \(line)"
        }
        return "\(src.path) bytes \(src.startByte)..<\(src.endByte)"
    }

    @ViewBuilder
    private func row(_ g: EditorDiagnostics.Group, in diags: [RuntimeV1.Diagnostic], status: RuntimeV1.Status) -> some View {
        let k = panel.currentOccurrence(of: g)
        let i = g.first
        let d = diags[i]
        let group = EditorDiagnostics.groupInfo(g, occurrence: k, in: diags, texts: model.compiledDocuments)
        let gap = EditorDiagnostics.isGap(d) // FlashTeX gap, not an authoring error: grey puzzle piece
        let helpFix = EditorDiagnostics.canApplyHelpReplacement(
            d, path: model.activePath, currentText: model.activeText,
            compiledRevision: model.result?.revision, editorRevision: model.editorRevision)
        let secondaryHelp = EditorDiagnostics.secondaryLabelHelp(d)
        HStack(alignment: .top) {
            Image(systemName: gap ? "puzzlepiece.extension" : d.severity == .error ? "xmark.octagon.fill" : "exclamationmark.triangle.fill")
                .foregroundStyle(gap ? Color.secondary : d.severity == .error ? .red : .orange)
            VStack(alignment: .leading) {
                // Title and location on one line: a 30-diagnostic TeX list is two lines per row, not three.
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(g.title)
                    Text(location(of: g, occurrence: k, diagnostic: d, group: group)).font(.caption2).foregroundStyle(.tertiary).lineLimit(1)
                }
                if let notes = d.notes {
                    ForEach(Array(notes.enumerated()), id: \.offset) { _, note in
                        Text("= note: \(note)").font(.caption).foregroundStyle(.secondary)
                    }
                }
                if let help = d.help?.message, !help.isEmpty {
                    Text("= help: \(help)").font(.caption).foregroundStyle(.secondary)
                }
                if let line = EditorDiagnostics.recoveryLine(recovery: d.recovery, status: status) {
                    Text("↳ \(line)").font(.caption).foregroundStyle(d.recovery == nil ? .tertiary : .secondary)
                }
                if let explain = model.explanations.explanation(resultID: model.resultID, index: i)?.line {
                    Text("↳ \(explain)").font(.caption).foregroundStyle(.secondary)
                }
                if let result = model.result,
                   let id = EditorDiagnostics.identity(resultID: model.resultID, index: i, in: result),
                   model.editorMarkReport.staleIdentities.contains(id) {
                    Text("underline withheld: span edited since the compile").font(.caption2).foregroundStyle(.orange)
                }
            }
            Spacer()
            if g.count > 1 {
                Menu("\(g.count) places") {
                    ForEach(0..<g.count, id: \.self) { j in
                        Button(EditorDiagnostics.occurrenceLabel(j, of: g, in: diags, texts: model.compiledDocuments)) {
                            model.goToOccurrence(j, of: g, panel: panel)
                        }
                        .disabled(EditorDiagnostics.occurrence(j, of: g, in: diags) == nil)
                    }
                }
                .fixedSize()
                .help("Jump to one occurrence of this diagnostic (⌘⌥] / ⌘⌥[ step through them)")
            } else if d.source != nil {
                Button("Go to source") { model.goToOccurrence(0, of: g, panel: panel) }
                    .help("Select the diagnostic's span in the editor (Return does the same)")
            }
            if helpFix {
                Button("Fix…") { model.previewQuickFix(diagnosticIndex: i) }
                    .help(d.help?.message ?? "Preview a suggested fix")
            } else if let x = model.explanations.explanation(resultID: model.resultID, index: i),
               x.suggestions.contains(where: { !$0.edits.isEmpty }) {
                Button("Fix…") { model.previewQuickFix(diagnosticIndex: i) }
                    .help(x.suggestions.first { !$0.edits.isEmpty }?.text ?? "Preview a suggested fix")
            } else if let fix = MissingIncludeFix.quickFix(for: d, projectRoot: model.project.projectRoot) { // ProjectScaffold.swift
                Button("Create \(fix.path)") { Task { await model.project.createMissingInclude(fix.argument, from: fix.from); model.navigationNote = model.project.status } }
                    .help("Create the empty file \(fix.path) under the project root and open it as included from \(fix.from)")
            }
        }
        .contextMenu {
            if d.source != nil { Button("Go to source") { model.goToOccurrence(k, of: g, panel: panel) } }
        }
        .controlSize(.small) // 30 TeX diagnostics must fit a 260 pt panel: small trailing controls, tight rows
        .tag(g.id)
        .modifier(SecondaryLabelHelp(text: secondaryHelp))
        .accessibleDiagnostic(d, index: i, total: diags.count, status: status,
                              explanation: model.explanations.explanation(resultID: model.resultID, index: i)?.line,
                              group: group) { model.goToOccurrence(k, of: g, panel: panel) } // FlashTeXAccessibility
    }
}

/// Attaches `.help` only when secondary labels have caption text, so an empty
/// hover does not override the Fix… / Go to source tooltips.
private struct SecondaryLabelHelp: ViewModifier {
    var text: String?
    @ViewBuilder func body(content: Content) -> some View {
        if let text { content.help(text) } else { content }
    }
}
