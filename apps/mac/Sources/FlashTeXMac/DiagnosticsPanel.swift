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
    /// The list's table (ProblemsTable.swift): a click on a row gives it the keyboard.
    @ObservationIgnored weak var table: NSTableView?

    init() {}

    /// A click on a row: it is selected and the list has the keyboard (as a List row's click).
    func select(_ id: String) {
        if selection != id { selection = id }
        if let table, table.window?.firstResponder !== table { table.window?.makeFirstResponder(table) }
    }

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
    /// The line of a byte of the compiled texts, for the rows' "main.tex
    /// line 41". Made with the rows (`ProblemsModel`), it looks lines up in
    /// an index of each text; made from `texts:` alone, it counts them. A
    /// row counted from the start of a 4.5 MB book for each of its labels at
    /// every update (APP-MAIN-THREAD).
    struct Lines: @unchecked Sendable {
        /// The texts the lines are of.
        let texts: [String: String]
        private let indexes: [String: EngineV3LineStarts]

        /// Counting in `texts` at each lookup (a few rows, the clipboard).
        init(texts: [String: String]) { self.texts = texts; indexes = [:] }

        /// An index of each text of `paths` (the rows' files), made here once.
        init(indexing texts: [String: String], paths: Set<String>) {
            self.texts = texts
            var ix: [String: EngineV3LineStarts] = [:]
            for p in paths { if let t = texts[p] { ix[p] = EngineV3LineStarts(t) } }
            indexes = ix
        }

        static let none = Lines(texts: [:])

        /// The one-based line of byte `offset` of `path`'s text; nil without the text or out of range.
        func line(ofByte offset: Int, in path: String) -> Int? {
            if let ix = indexes[path] { return ix.line(ofByte: offset) }
            return texts[path].flatMap { EditorDiagnostics.lineNumber(ofByte: offset, in: $0) }
        }
    }

    /// "main.tex line 41" / "main.tex bytes 10..<20" / "no source" for
    /// occurrence `k` of `group` — the location part of `occurrenceLabel`.
    static func occurrenceLocation(_ k: Int, of group: Group, in diagnostics: [RuntimeV1.Diagnostic],
                                   texts: [String: String] = [:]) -> String {
        occurrenceLocation(k, of: group, in: diagnostics, lines: Lines(texts: texts))
    }

    static func occurrenceLocation(_ k: Int, of group: Group, in diagnostics: [RuntimeV1.Diagnostic], lines: Lines) -> String {
        let label = occurrenceLabel(k, of: group, in: diagnostics, lines: lines)
        let prefix = "\(k + 1) of \(group.count): "
        return label.hasPrefix(prefix) ? String(label.dropFirst(prefix.count)) : label
    }

    /// The spoken group part of a panel row: nil for a single occurrence,
    /// else "12 places, 3 of 12, main.tex line 41".
    static func groupInfo(_ group: Group, occurrence k: Int, in diagnostics: [RuntimeV1.Diagnostic],
                          texts: [String: String] = [:]) -> DiagnosticRowAccessibility.GroupInfo? {
        groupInfo(group, occurrence: k, in: diagnostics, lines: Lines(texts: texts))
    }

    static func groupInfo(_ group: Group, occurrence k: Int, in diagnostics: [RuntimeV1.Diagnostic],
                          lines: Lines) -> DiagnosticRowAccessibility.GroupInfo? {
        guard group.count > 1 else { return nil }
        let k = min(max(k, 0), group.count - 1)
        return .init(count: group.count, occurrence: k, location: occurrenceLocation(k, of: group, in: diagnostics, lines: lines))
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

    // MARK: VoiceOver (ux-diagnostics-panel-voiceover)

    /// The count summary VoiceOver reads for the list and announces after a
    /// compile: "3 errors, 1 warning, 2 not implemented"; zero buckets are
    /// left out (unlike the header's `summary`, which keeps them), and an
    /// empty list is "No problems", the words the empty panel shows.
    static func spokenSummary(_ c: (errors: Int, warnings: Int, gaps: Int)) -> String {
        var parts: [String] = []
        if c.errors > 0 { parts.append("\(c.errors) error\(c.errors == 1 ? "" : "s")") }
        if c.warnings > 0 { parts.append("\(c.warnings) warning\(c.warnings == 1 ? "" : "s")") }
        if c.gaps > 0 { parts.append("\(c.gaps) not implemented") }
        return parts.isEmpty ? "No problems" : parts.joined(separator: ", ")
    }

    static func spokenSummary(_ diagnostics: [RuntimeV1.Diagnostic]) -> String { spokenSummary(counts(diagnostics)) }

    /// The panel row as one spoken element: "Diagnostic 2 of 5: Error: \in
    /// is not supported in math mode, main.tex line 3" — severity (or "Not
    /// implemented" for a FlashTeX gap) and line in words, never only the
    /// glyph colour or the dimmed trailing text. A grouped row speaks its
    /// count and current occurrence instead of the plain location; a row
    /// without a source speaks no location (its hint says so).
    /// `index`/`total`: the row's place in the whole list when `diagnostics`
    /// holds only the group's own (`DiagnosticRow`).
    static func rowAccessibility(_ group: Group, occurrence k: Int, in diagnostics: [RuntimeV1.Diagnostic],
                                 index: Int? = nil, total: Int? = nil,
                                 status: RuntimeV1.Status, explanation: String? = nil,
                                 texts: [String: String] = [:]) -> DiagnosticRowAccessibility {
        rowAccessibility(group, occurrence: k, in: diagnostics, index: index, total: total, status: status,
                         explanation: explanation, lines: Lines(texts: texts))
    }

    static func rowAccessibility(_ group: Group, occurrence k: Int, in diagnostics: [RuntimeV1.Diagnostic],
                                 index: Int? = nil, total: Int? = nil,
                                 status: RuntimeV1.Status, explanation: String? = nil,
                                 lines: Lines) -> DiagnosticRowAccessibility {
        let d = diagnostics[group.first]
        let info = groupInfo(group, occurrence: k, in: diagnostics, lines: lines)
        let location = d.source == nil ? nil : occurrenceLocation(0, of: group, in: diagnostics, lines: lines)
        return DiagnosticRowAccessibility(d, index: index ?? group.first, total: total ?? diagnostics.count, status: status,
                                         explanation: explanation, group: info, location: location, gap: isGap(d))
    }
}

/// Decides when a finished compile is announced to VoiceOver: only when the
/// spoken summary differs from the one last spoken, and never more often
/// than `minimumIntervalNs` — auto-compile replies arrive with every pause
/// in typing, and each would otherwise interrupt the reader. A change that
/// arrives inside the interval is kept as `pending` and spoken when the
/// interval ends (`flush`), by which time a later reply may have replaced
/// it, so the summary spoken is always the newest. Pure; the shell owns the
/// clock and the timer (`ShellModel.noteCompileCompletedForVoiceOver`).
struct DiagnosticsAnnouncer: Equatable {
    static let minimumIntervalNs: UInt64 = 2_000_000_000

    /// The least time between two announcements; tests shorten it.
    var intervalNs: UInt64 = DiagnosticsAnnouncer.minimumIntervalNs
    /// The summary last spoken. Starts as "No problems" so a project whose
    /// first compile is clean says nothing; the first problem is announced.
    private(set) var lastSpoken = EditorDiagnostics.spokenSummary([])
    private(set) var lastSpokenNs: UInt64?
    /// A changed summary waiting for the interval to end.
    private(set) var pending: String?

    init(intervalNs: UInt64 = DiagnosticsAnnouncer.minimumIntervalNs) { self.intervalNs = intervalNs }

    /// A compile finished with `summary`. Returns the text to announce now,
    /// or nil (unchanged, or throttled: `flush` after `delayNs`).
    mutating func note(summary: String, nowNs: UInt64) -> String? {
        guard summary != lastSpoken else { pending = nil; return nil }
        if let last = lastSpokenNs, nowNs &- last < intervalNs {
            pending = summary
            return nil
        }
        return speak(summary, nowNs: nowNs)
    }

    /// Nanoseconds until a pending summary may be spoken (0 when none or due).
    func delayNs(nowNs: UInt64) -> UInt64 {
        guard pending != nil, let last = lastSpokenNs else { return 0 }
        let elapsed = nowNs &- last
        return elapsed >= intervalNs ? 0 : intervalNs - elapsed
    }

    /// The pending summary once the interval has ended, if it still differs.
    /// Before the interval ends (a timer armed for an earlier announcement
    /// firing after a later one) it speaks nothing and keeps the pending
    /// summary, so the throttle holds whichever timer fires.
    mutating func flush(nowNs: UInt64) -> String? {
        guard let summary = pending, delayNs(nowNs: nowNs) == 0 else { return nil }
        pending = nil
        guard summary != lastSpoken else { return nil }
        return speak(summary, nowNs: nowNs)
    }

    private mutating func speak(_ summary: String, nowNs: UInt64) -> String {
        lastSpoken = summary
        lastSpokenNs = nowNs
        pending = nil
        return summary
    }
}

// MARK: - Shell actions

extension ShellModel {
    /// Jumps to occurrence `k` of `group` and makes it the group's current
    /// occurrence (spoken as "k+1 of n" on the row) and the caret's current
    /// diagnostic for ⌘⇧] / ⌘⇧[. The footer names the occurrence.
    func goToOccurrence(_ k: Int, of listed: EditorDiagnostics.Group, panel: DiagnosticsPanelState) {
        let diags = displayedDiagnostics
        // The panel's list may be up to `problemsListInterval` older than the
        // diagnostics: its group's indices are found again by the group's identity.
        let group = problems === engineV3Problems ? listed : engineV3Problems.group(id: listed.id) ?? listed
        guard group.occurrences.indices.contains(k) else { return }
        panel.selection = group.id
        panel.occurrence[group.id] = k
        let index = group.occurrences[k]
        let source = EditorDiagnostics.occurrence(k, of: group, in: diags)
        // A problem inside a package input that is not open (a `.sty` next
        // to the entry, a `texinputs` mount, a resolved package): open it
        // first — read-only when it has no rooted file — then select the
        // span (ShellModel+PackageNavigation.swift).
        if let source, !documents.contains(where: { $0.path == source.path }), packageInputs.contains(where: { $0.path == source.path }) {
            Task { @MainActor [weak self] in
                guard let self, await self.openPackageInput(at: source.path) else { return }
                self.navigate(to: source)
                self.noteOccurrence(k, of: group, in: diags, index: index)
            }
            return
        }
        navigate(to: source)
        noteOccurrence(k, of: group, in: diags, index: index)
    }

    private func noteOccurrence(_ k: Int, of group: EditorDiagnostics.Group, in diags: [RuntimeV1.Diagnostic], index: Int) {
        if let source = markSource, let id = EditorDiagnostics.identity(resultID: source.id, index: index, in: source.result) {
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

    // MARK: VoiceOver announcement of a finished compile

    /// The end of `bindLayout`: a compile finished (worker, helper or
    /// fixture — never an in-flight request or a keystroke) and
    /// `displayedDiagnostics` is final, layout diagnostics included.
    /// Announces the spoken summary when it changed, throttled by
    /// `DiagnosticsAnnouncer`; a throttled change is spoken once the
    /// interval ends. `nowNs` is injectable so tests drive the clock.
    func noteCompileCompletedForVoiceOver(nowNs: UInt64 = MonotonicClock.nowNs()) {
        // The call comes from the host's DONE (EngineV3Session), after its
        // rows are published.
        guard engineV3Enabled else { resetDiagnosticsAnnouncer(); return }
        let summary = engineV3Problems.spokenSummary
        if let message = diagnosticsAnnouncer.note(summary: summary, nowNs: nowNs) { announceDiagnostics(message) }
        armDiagnosticsFlushIfNeeded(nowNs: nowNs)
    }

    /// A timer fired, or a test drives the clock: speak the pending summary
    /// if the interval has ended; otherwise keep it and re-arm for the rest
    /// of the interval (the timer was armed for an earlier announcement).
    func flushDiagnosticsAnnouncement(nowNs: UInt64 = MonotonicClock.nowNs()) {
        if let message = diagnosticsAnnouncer.flush(nowNs: nowNs) { announceDiagnostics(message) }
        armDiagnosticsFlushIfNeeded(nowNs: nowNs)
    }

    /// One timer at a time, for the pending summary's remaining delay. The
    /// timer is identified by a token: a superseded or cancelled one (a
    /// new project, or a flush that already ran) does nothing when it fires,
    /// so a stale timer can neither speak early nor clear a newer one.
    private func armDiagnosticsFlushIfNeeded(nowNs: UInt64) {
        guard diagnosticsAnnouncer.pending != nil, diagnosticsAnnouncementFlush == nil else { return }
        diagnosticsFlushToken &+= 1
        let token = diagnosticsFlushToken
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.diagnosticsFlushToken == token else { return }
            self.diagnosticsAnnouncementFlush = nil
            self.flushDiagnosticsAnnouncement()
        }
        diagnosticsAnnouncementFlush = work
        let delay = diagnosticsAnnouncer.delayNs(nowNs: nowNs)
        DispatchQueue.main.asyncAfter(deadline: .now() + .nanoseconds(Int(min(delay, UInt64(Int32.max)))), execute: work)
    }

    /// The project is being replaced (File > Open through `replaceProject`,
    /// a fixture through `loadFixtures`): the spoken counts belonged to the
    /// old project, so the announcer starts clean — its first clean result
    /// is silent, its first problem is spoken at once — and any timer armed
    /// for the old project is cancelled and its token retired, so it stays
    /// silent even if already dequeued.
    func resetDiagnosticsAnnouncer() {
        diagnosticsAnnouncer = DiagnosticsAnnouncer(intervalNs: diagnosticsAnnouncer.intervalNs)
        diagnosticsFlushToken &+= 1 // a timer already dequeued but not yet run finds its token stale
        diagnosticsAnnouncementFlush?.cancel()
        diagnosticsAnnouncementFlush = nil
    }

    private func announceDiagnostics(_ message: String) {
        diagnosticAnnouncements.append(message)
        if diagnosticAnnouncements.count > 64 { diagnosticAnnouncements.removeFirst(diagnosticAnnouncements.count - 64) }
        NSAccessibility.post(element: NSApplication.shared, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.medium.rawValue])
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
    /// The finished model (`ShellModel.problems`), or nil for `diagnostics`.
    let problems: ProblemsModel?
    let diagnostics: [RuntimeV1.Diagnostic]
    @State private var panel: DiagnosticsPanelState

    /// Identifier of the list for tests and assistive clients.
    static let listIdentifier = "diagnostics.list"
    /// What VoiceOver calls the list.
    static let listLabel = "Diagnostics"

    /// - Parameter panel: the state to use (tests drive the selection through
    ///   it); the view makes its own when nil.
    /// Severity to show (nil: all); group indices stay those of `diagnostics`,
    /// so explanations, quick fixes and occurrences are unaffected by the filter.
    let severityFilter: RuntimeV1.Severity?
    /// Whether the "Diagnostics (n) — …" caption is drawn (the Problems panel draws its own header).
    let showsHeader: Bool
    let maxHeight: CGFloat

    /// The list of a finished model (the Problems panel): its groups and
    /// counts were made with its rows (`ProblemsModel`), not here.
    init(problems: ProblemsModel, panel: DiagnosticsPanelState? = nil,
         severityFilter: RuntimeV1.Severity? = nil, showsHeader: Bool = true,
         maxHeight: CGFloat = DS.Layout.diagnosticsListMaxHeight) {
        self.problems = problems
        self.diagnostics = problems.diagnostics
        self.severityFilter = severityFilter
        self.showsHeader = showsHeader
        self.maxHeight = maxHeight
        _panel = State(initialValue: panel ?? DiagnosticsPanelState())
    }

    /// The list of `diagnostics`, grouped here in the listing's order.
    init(diagnostics: [RuntimeV1.Diagnostic], panel: DiagnosticsPanelState? = nil,
         severityFilter: RuntimeV1.Severity? = nil, showsHeader: Bool = true,
         maxHeight: CGFloat = DS.Layout.diagnosticsListMaxHeight) {
        self.problems = nil
        self.diagnostics = diagnostics
        self.severityFilter = severityFilter
        self.showsHeader = showsHeader
        self.maxHeight = maxHeight
        _panel = State(initialValue: panel ?? DiagnosticsPanelState())
    }

    var body: some View {
        // Change-only / throttled reads (ShellModel mirrors, ShellChrome): `documents`,
        // `result` and `editorMarkReport` change on every keystroke or reply and
        // re-evaluated this List (an AppKit table) with each of them.
        let problems = self.problems ?? ProblemsModel(diagnostics: diagnostics, texts: model.compiledDocuments, documentOrder: model.chrome.listing.map(\.path))
        let diags = problems.diagnostics
        let groups = problems.groups(severityFilter)
        let status = model.resultStatus ?? .ok
        VStack(alignment: .leading, spacing: 0) {
            if showsHeader {
                Text("Diagnostics (\(diags.count)\(problems.groups.count < diags.count ? " in \(problems.groups.count) groups" : "")) — the preview above is still shown; errors are not hidden")
                    .font(DS.Fonts.header).padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
            }
            if let carriedLine = model.chrome.carriedLine {
                Text("Underlines \(carriedLine); the list below is the failed result's.")
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning).padding(.horizontal, DS.Space.m).padding(.bottom, DS.Space.xs)
            }
            // Every group, each row made and drawn only while it is on screen
            // (ProblemsTable.swift): a SwiftUI List made all 3,000 rows of a
            // long book at each update, so #1890 showed only 200. The table
            // is named, with the count summary as its value, so VoiceOver
            // says "Diagnostics, 3 errors, 1 warning" on landing; Return,
            // Esc and ⌘C (its text made when pressed) are the table's.
            ProblemsTable(model: model, problems: problems, groups: groups, filter: severityFilter, status: status,
                          panel: panel, selection: panel.selection, revision: model.problemsRevision)
                .background(DS.Colors.surfacePrimary)
                .frame(minHeight: DS.Layout.diagnosticsListMinHeight, maxHeight: maxHeight)
        }
        .focusedSceneValue(\.diagnosticsPanel, panel)
    }
}

/// One row of `DiagnosticsListView`: a group of diagnostics.
struct DiagnosticRow: View {
    @Environment(ShellModel.self) private var model
    /// The group, indices into the list's diagnostics (selection, navigation, quick fixes).
    let group: EditorDiagnostics.Group
    /// The group's diagnostics, in its order.
    let occurrences: [RuntimeV1.Diagnostic]
    /// How many diagnostics the list has (VoiceOver: "3 of 40").
    let total: Int
    let status: RuntimeV1.Status
    let panel: DiagnosticsPanelState

    /// `group` over `occurrences` (indices 0 ..< count): what the label helpers read.
    private var local: EditorDiagnostics.Group {
        EditorDiagnostics.Group(severity: group.severity, message: group.message, code: group.code,
                                recovery: group.recovery, occurrences: Array(occurrences.indices))
    }

    var body: some View {
        let _ = ViewBodyProbe.note("DiagnosticRow") // EngineV3ProblemsPipelineTests: only the rows on screen are made
        row(group, local: local, status: status)
    }

    /// "3 of 12: main.tex line 41" for a group, "main.tex line 3" for one
    /// diagnostic when the compiled text is known (an author never thinks in
    /// bytes; "bytes a..<b" stays the fallback), "no source mapping" otherwise.
    private func location(of g: EditorDiagnostics.Group, occurrence k: Int, diagnostic d: RuntimeV1.Diagnostic,
                          group: DiagnosticRowAccessibility.GroupInfo?) -> String {
        if g.count > 1 { return "\(k + 1) of \(g.count): \(group?.location ?? "no source")" }
        guard let src = d.source else { return "no source mapping" }
        if let line = model.problemsLines.line(ofByte: src.startByte, in: src.path) {
            return "\(src.path) line \(line)"
        }
        return "\(src.path) bytes \(src.startByte)..<\(src.endByte)"
    }

    @ViewBuilder
    private func row(_ g: EditorDiagnostics.Group, local lg: EditorDiagnostics.Group, status: RuntimeV1.Status) -> some View {
        let diags = occurrences // indexed by `lg`
        let k = panel.currentOccurrence(of: g)
        let i = g.first
        let d = diags[0]
        let selected = panel.selection == g.id
        let group = EditorDiagnostics.groupInfo(lg, occurrence: k, in: diags, lines: model.problemsLines)
        let gap = EditorDiagnostics.isGap(d) // FlashTeX gap, not an authoring error: grey puzzle piece
        // Whether Fix… can apply depends on the editor's text, which changes
        // with every key. Only a row whose diagnostic has a mechanical edit
        // for the active file reads it, in its own view
        // (`DiagnosticHelpFixSlot`): a row read here would make each key
        // re-diff every row of the list (EDITOR-FREEZE: thousands of groups
        // of a 1,152-page book held the main thread for whole seconds per key).
        let fixable = EditorDiagnostics.mechanicalEdit(for: d)?.path == model.activePath
        let secondaryHelp = EditorDiagnostics.secondaryLabelHelp(d)
        VStack(alignment: .leading, spacing: DS.Space.xxs) {
            // The IntelliJ row (F1): severity glyph, the human-readable
            // message, then the dimmed location at the trailing edge. Detail
            // lines collapse under the selected row (§9) — selecting is the
            // disclosure; Return / double-click jumps to the source.
            HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                Image(systemName: gap ? "puzzlepiece.extension" : d.severity == .error ? "xmark.octagon.fill" : "exclamationmark.triangle.fill")
                    .foregroundStyle(gap ? DS.Colors.textSecondary : d.severity == .error ? DS.Colors.severityError : DS.Colors.severityWarning)
                // One line until selected: every unselected row has one
                // height, so the table sizes thousands of rows without making
                // them (ProblemsTable); the selected row shows it all.
                Text(g.title).lineLimit(selected ? nil : 1)
                Spacer(minLength: DS.Space.m)
                // Row actions are JetBrains inline links, not bezelled push
                // buttons (owner on #653: no glass/bezel controls).
                if fixable {
                    DiagnosticHelpFixSlot(diagnostic: d, index: i)
                } else {
                    MissingIncludeFixButton(diagnostic: d)
                }
                // The compiler's missing-package diagnostic (`packages X are
                // recognised but not implemented`, `no project file found:
                // looked for X.sty`): write X.sty from the template next to
                // the entry, or ask the consent sheet to fetch it
                // (ProjectPackages.swift). Alongside the compiler's own fix
                // (remove the \usepackage), never instead of it.
                let missingFiles = ProjectPackagesState.missingFiles(for: d, projectRoot: model.project.projectRoot)
                let missing = missingFiles.map { ($0 as NSString).deletingPathExtension }
                if missingFiles.count == 1, let file = missingFiles.first, let name = missing.first {
                    let isClass = file.hasSuffix(".cls")
                    InlineActionButton(title: "Create \(file)") { Task { await model.createPackageFile(named: name, class: isClass) } }
                        .help("Write \(file) next to \(model.project.entryPath) from the \(isClass ? "class" : "package") template and open it; the next compile loads it")
                        .accessibilityIdentifier("problems.package.create")
                    InlineActionButton(title: "Fetch \(name)…") { model.projectPackages.presentFetch([name]) }
                        .help("Ask to fetch \(name) from CTAN into the package cache (nothing is fetched until you agree in the sheet)")
                        .accessibilityIdentifier("problems.package.fetch")
                } else if missing.count > 1 {
                    Menu("\(missing.count) missing packages") {
                        ForEach(missingFiles, id: \.self) { file in
                            let name = (file as NSString).deletingPathExtension
                            Button("Create \(file) next to \(model.project.entryPath)") { Task { await model.createPackageFile(named: name, class: file.hasSuffix(".cls")) } }
                            Button("Fetch \(name) from CTAN…") { model.projectPackages.presentFetch([name]) }
                        }
                        Divider()
                        Button("Fetch all \(missing.count)…") { model.projectPackages.presentFetch(missing) }
                    }
                    .menuStyle(.borderlessButton).fixedSize()
                    .font(DS.Fonts.secondary)
                    .help("Create each package from the template next to the entry, or ask the consent sheet to fetch it")
                    .accessibilityIdentifier("problems.package.menu")
                }
                if g.count > 1 {
                    Menu("\(g.count) places") {
                        ForEach(0..<g.count, id: \.self) { j in
                            Button(EditorDiagnostics.occurrenceLabel(j, of: lg, in: diags, lines: model.problemsLines)) {
                                model.goToOccurrence(j, of: g, panel: panel)
                            }
                            .disabled(EditorDiagnostics.occurrence(j, of: lg, in: diags) == nil)
                        }
                    }
                    .menuStyle(.borderlessButton).fixedSize()
                    .font(DS.Fonts.secondary)
                    .help("Jump to one occurrence of this diagnostic (⌘⌥] / ⌘⌥[ step through them)")
                }
                Text(location(of: g, occurrence: k, diagnostic: d, group: group))
                    .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary).lineLimit(1)
            }
            if selected {
                // The disclosure inside the selected problem: notes, help,
                // recovery, catalogue explanation, and withheld-underline
                // state — the raw detail, not a separate destination.
                VStack(alignment: .leading, spacing: DS.Space.xxs) {
                    if let notes = d.notes {
                        ForEach(Array(notes.enumerated()), id: \.offset) { _, note in
                            Text("= note: \(note)").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                        }
                    }
                    if let help = d.help?.message, !help.isEmpty {
                        Text("= help: \(help)").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                    }
                    if let line = EditorDiagnostics.recoveryLine(recovery: d.recovery, status: status) {
                        Text("↳ \(line)").font(DS.Fonts.secondary).foregroundStyle(d.recovery == nil ? DS.Colors.textTertiary : DS.Colors.textSecondary)
                    }
                    WithheldUnderlineNote(index: i) // reads the editor's marks, which follow every key
                }
                .padding(.leading, DS.Space.xxl)
            }
        }
        .frame(minHeight: DS.Row.problem - 2 * DS.Space.xxs)
        .contentShape(Rectangle())
        .onTapGesture(count: 2) { model.goToOccurrence(k, of: g, panel: panel) }
        .simultaneousGesture(TapGesture().onEnded { panel.select(g.id) }) // a click selects, as in a List
        .contextMenu {
            if d.source != nil { Button("Go to source") { model.goToOccurrence(k, of: g, panel: panel) } }
        }
        .controlSize(.small) // 30 TeX diagnostics must fit a 260 pt panel: small trailing controls, tight rows
        .modifier(SecondaryLabelHelp(text: secondaryHelp))
        .accessibleDiagnostic(EditorDiagnostics.rowAccessibility(lg, occurrence: k, in: diags, index: i, total: total, status: status,
                                                                 explanation: nil,
                                                                 lines: model.problemsLines)) {
            model.goToOccurrence(k, of: g, panel: panel)
        } // FlashTeXAccessibility: one spoken element, "Go to source" as the default action
    }
}

/// A row's Fix… link, shown while the diagnostic's mechanical edit still
/// applies to the editor's text (`canApplyHelpReplacement`), else the row's
/// "Create file" link if it has one. Its own view: the editor's text and
/// revision it reads change with every key, and only this view (not the
/// list) is then evaluated again.
struct DiagnosticHelpFixSlot: View {
    @Environment(ShellModel.self) private var model
    let diagnostic: RuntimeV1.Diagnostic
    let index: Int

    var body: some View {
        if EditorDiagnostics.canApplyHelpReplacement(diagnostic, path: model.activePath, currentText: model.activeText,
                                                     compiledRevision: model.fixCompiledRevision, editorRevision: model.editorRevision) {
            InlineActionButton(title: "Fix…") {
                // `index` is the panel list's, which may trail the diagnostics (`ShellModel.problemsList`).
                let diags = model.displayedDiagnostics
                let i = diags.indices.contains(index) && diags[index] == diagnostic ? index : diags.firstIndex(of: diagnostic)
                if let i { model.previewQuickFix(diagnosticIndex: i) } else { model.navigationNote = "The list is updating: try Fix… again." }
            }
                .help(diagnostic.help?.message ?? "Preview a suggested fix")
        } else {
            MissingIncludeFixButton(diagnostic: diagnostic)
        }
    }
}

/// "Create chapter.tex" for an `\include` of a missing file (ProjectScaffold.swift).
struct MissingIncludeFixButton: View {
    @Environment(ShellModel.self) private var model
    let diagnostic: RuntimeV1.Diagnostic

    var body: some View {
        if let fix = MissingIncludeFix.quickFix(for: diagnostic, projectRoot: model.project.projectRoot) {
            InlineActionButton(title: "Create \(fix.path)") { Task { _ = await model.project.createMissingInclude(fix.argument, from: fix.from); model.navigationNote = model.project.status } }
                .help("Create the empty file \(fix.path) under the project root and open it as included from \(fix.from)")
        }
    }
}

/// The selected row's note that its underline is withheld (its span was
/// edited since the compile). Its own view, as `DiagnosticHelpFixSlot`: the
/// editor's marks follow every key.
struct WithheldUnderlineNote: View {
    @Environment(ShellModel.self) private var model
    let index: Int

    var body: some View {
        if let source = model.markSource,
           let id = EditorDiagnostics.identity(resultID: source.id, index: index, in: source.result),
           model.editorMarkReport.staleIdentities.contains(id) {
            Text("underline withheld: span edited since the compile").font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityWarning)
        }
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
