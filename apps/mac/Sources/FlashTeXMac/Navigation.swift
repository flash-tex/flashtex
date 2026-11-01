import AppKit
import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility

/// Source-aware navigation. The text functions are pure and return UTF-8 byte
/// ranges into the current buffers (no compile result involved, so they are
/// never stale). Result-backed navigation (preview click, diagnostics, caret
/// reveal) goes through `ShellModel.navigateExactly`, which maps runtime-v1
/// UTF-8 byte spans onto the editor's UTF-16 selection exactly: a span whose
/// bytes were edited since the compile is refused with an explanation, a span
/// is never split inside a scalar or a composed character sequence, and a span
/// in another open document switches the active document.
enum Navigation {
    struct ByteRange: Equatable {
        let start: Int
        let end: Int
    }

    /// One `\name{arg}` occurrence: `range` spans the backslash through `}`,
    /// `argRange` the argument text.
    struct CommandUse: Equatable {
        let name: String
        let arg: String
        let range: ByteRange
        let argRange: ByteRange
    }

    static let referenceCommands: Set<String> = ["ref", "eqref", "pageref", "autoref"]
    private static let interestingCommands: Set<String> = referenceCommands.union(["begin", "end", "label"])

    /// Every `\begin{…}`, `\end{…}`, `\label{…}`, and reference command with a
    /// complete braced argument, in document order. Commands inside a `%`
    /// comment (to the end of the line) are skipped; `\%` and `\\` are escapes.
    static func commandUses(in text: String) -> [CommandUse] {
        var out: [CommandUse] = []
        Completion.withBytes(text) { b in
            guard let p = b.baseAddress else { return }
            let n = b.count
            let table = Completion.wordByteClass
            let backslash = Completion.backslash, percent = UInt8(ascii: "%"), newline = UInt8(ascii: "\n")
            let open = UInt8(ascii: "{"), close = UInt8(ascii: "}")
            var i = 0
            while i < n {
                let c = p[i]
                if c == percent {
                    while i < n, p[i] != newline { i += 1 }
                    continue
                }
                guard c == backslash else { i += 1; continue }
                var j = i + 1
                while j < n, table[Int(p[j])] == 1 { j += 1 }
                guard j > i + 1 else { i = min(n, i + 2); continue } // `\%`, `\\`, `\{`, trailing `\`
                let nameBytes = UnsafeBufferPointer(start: p + i + 1, count: j - i - 1)
                let name = String(decoding: nameBytes, as: UTF8.self)
                guard interestingCommands.contains(name), j < n, p[j] == open else { i = j; continue }
                var k = j + 1
                while k < n, p[k] != close, p[k] != open, p[k] != backslash, p[k] != newline, p[k] != percent { k += 1 }
                guard k < n, p[k] == close else { i = j; continue }
                out.append(CommandUse(name: name,
                                      arg: String(decoding: UnsafeBufferPointer(start: p + j + 1, count: k - j - 1), as: UTF8.self),
                                      range: ByteRange(start: i, end: k + 1), argRange: ByteRange(start: j + 1, end: k)))
                i = k + 1
            }
        }
        return out
    }

    enum Target: Equatable {
        case found(ByteRange, note: String)
        case notFound(String)
    }

    /// Counterpart of the command under the caret (`caretByte`, UTF-8) within
    /// one document: `\ref{X}` → its `\label{X}`; `\label{X}` → the next
    /// reference to it (wrapping); `\begin{X}` ↔ `\end{X}` honouring nesting
    /// of the same name. See `matchingRange(in:activePath:caretByte:)` for
    /// labels and references that live in other open documents.
    static func matchingRange(in text: String, caretByte: Int) -> Target {
        switch matchingRange(in: [.init(path: "", text: text)], activePath: "", caretByte: caretByte) {
        case .found(_, let range, let note): return .found(range, note: note)
        case .notFound(let why): return .notFound(why)
        }
    }

    enum DocumentTarget: Equatable {
        case found(path: String, ByteRange, note: String)
        case notFound(String)
    }

    /// Multi-document counterpart lookup. `\begin`/`\end` match within the
    /// active document. `\ref{X}` finds `\label{X}` in the active document
    /// first, then the other open documents in project order. `\label{X}`
    /// cycles through every reference to it: those after the caret in the
    /// active document, then the following documents, wrapping around.
    static func matchingRange(in documents: [RuntimeV1.Document], activePath: String, caretByte: Int) -> DocumentTarget {
        guard let active = documents.firstIndex(where: { $0.path == activePath }) else {
            return .notFound("No open document named \(activePath).")
        }
        let uses = commandUses(in: documents[active].text)
        // A caret at the byte where one command ends and the next begins
        // (`\end{a}\begin{b}`) belongs to the one that starts there.
        let here = uses.firstIndex(where: { $0.range.start == caretByte })
            ?? uses.firstIndex(where: { $0.range.start <= caretByte && caretByte <= $0.range.end })
        guard let here else {
            return .notFound("Caret is not inside \\begin, \\end, \\label, or a \\ref-style command.")
        }
        let use = uses[here]
        let elsewhere = documents.count > 1 ? " or any open document" : ""
        switch use.name {
        case "begin":
            var depth = 0
            for other in uses[(here + 1)...] where other.arg == use.arg {
                if other.name == "begin" { depth += 1 }
                else if other.name == "end" {
                    if depth == 0 { return .found(path: activePath, other.range, note: "Matched \\begin{\(use.arg)} → \\end{\(use.arg)} at byte \(other.range.start).") }
                    depth -= 1
                }
            }
            return .notFound("\\begin{\(use.arg)} at byte \(use.range.start) has no matching \\end{\(use.arg)}.")
        case "end":
            var depth = 0
            for other in uses[..<here].reversed() where other.arg == use.arg {
                if other.name == "end" { depth += 1 }
                else if other.name == "begin" {
                    if depth == 0 { return .found(path: activePath, other.range, note: "Matched \\end{\(use.arg)} → \\begin{\(use.arg)} at byte \(other.range.start).") }
                    depth -= 1
                }
            }
            return .notFound("\\end{\(use.arg)} at byte \(use.range.start) has no matching \\begin{\(use.arg)}.")
        case "label":
            // All references in project order, starting with the active document.
            var refs: [(path: String, use: CommandUse)] = []
            for offset in 0..<documents.count {
                let doc = documents[(active + offset) % documents.count]
                let docUses = offset == 0 ? uses : commandUses(in: doc.text)
                for r in docUses where referenceCommands.contains(r.name) && r.arg == use.arg {
                    refs.append((doc.path, r))
                }
            }
            guard !refs.isEmpty else { return .notFound("No reference to label \(use.arg) in this document\(elsewhere).") }
            let after = refs.firstIndex { $0.path != activePath || $0.use.range.start > use.range.start }
            let next = refs[after ?? 0]
            let index = (after ?? 0) + 1
            let place = next.path == activePath ? "" : " in \(next.path)"
            return .found(path: next.path, next.use.range,
                          note: "Reference \(index) of \(refs.count) to label \(use.arg) at byte \(next.use.range.start)\(place).")
        default:
            for offset in 0..<documents.count {
                let doc = documents[(active + offset) % documents.count]
                let docUses = offset == 0 ? uses : commandUses(in: doc.text)
                if let label = docUses.first(where: { $0.name == "label" && $0.arg == use.arg }) {
                    let place = doc.path == activePath ? "" : " in \(doc.path)"
                    return .found(path: doc.path, label.range, note: "Definition of \(use.arg): \\label at byte \(label.range.start)\(place).")
                }
            }
            return .notFound("No \\label{\(use.arg)} in this document\(elsewhere).")
        }
    }

    // MARK: - byte span → editor selection

    enum RangeMapping: Equatable {
        /// `widenedFrom` is set when the span started or ended inside a
        /// composed character sequence (`e` + U+0301, a ZWJ emoji, a ligature
        /// glyph is one scalar and never splits) and was extended to cover it.
        case selected(NSRange, widenedFrom: NSRange?)
        case refused(String)
    }

    /// Exact UTF-16 selection for UTF-8 bytes `start..<end` of `text`.
    /// Refused when out of range, reversed, or inside a multi-byte scalar;
    /// widened outward to whole composed character sequences (the units the
    /// text view selects and moves the caret by), so the selection is never a
    /// split cluster. An empty span at a cluster-interior position is moved
    /// to the start of that cluster and stays empty.
    static func editorRange(start: Int, end: Int, in text: String, path: String = "") -> RangeMapping {
        let byteCount = text.utf8.count
        let label = path.isEmpty ? "" : " in \(path)"
        guard start >= 0, end >= start, end <= byteCount else {
            return .refused("Bytes \(start)..<\(end) are not a valid range\(label) (buffer is \(byteCount) bytes).")
        }
        guard let r = text.rangeOfUTF8(start: start, end: end) else {
            return .refused("Bytes \(start)..<\(end)\(label) start or end inside a multi-byte character; refusing to split it.")
        }
        let ns = NSRange(r, in: text)
        let nsText = text as NSString
        let whole: NSRange
        if ns.length == 0 {
            whole = ns.location < nsText.length
                ? NSRange(location: nsText.rangeOfComposedCharacterSequence(at: ns.location).location, length: 0)
                : ns
        } else {
            whole = nsText.rangeOfComposedCharacterSequences(for: ns)
        }
        return .selected(whole, widenedFrom: whole == ns ? nil : ns)
    }

    // MARK: - byte span across an edit

    enum Rebased: Equatable {
        /// The span in `current`; `note` names the shift when there was one.
        case mapped(start: Int, end: Int, note: String?)
        /// Why the span cannot be mapped (which bytes overlap the edit).
        case refused(String)
    }

    /// Maps bytes `start..<end` of `baseline` onto `current` across the single
    /// changed region between them (byte-for-byte comparison), refusing a span
    /// that overlaps the edit and verifying that the mapped bytes still spell
    /// the baseline bytes so a wrong span can never be selected silently.
    static func rebaseExactly(start: Int, end: Int, from baseline: String, to current: String, path: String) -> Rebased {
        if baseline.sameBytes(as: current) { return .mapped(start: start, end: end, note: nil) }
        let region = SourceMapping.changedRegion(from: baseline, to: current)
        switch SourceMapping.rebase(start: start, end: end, across: region) {
        case .overlapsEdit:
            return .refused("bytes \(start)..<\(end) of \(path) overlap the edit at \(region.startByte)..<\(region.oldEndByte), now \(region.startByte)..<\(region.newEndByte)")
        case .rebased(let s, let e):
            guard let was = baseline.rangeOfUTF8(start: start, end: end),
                  let now = current.rangeOfUTF8(start: s, end: e),
                  String(baseline[was]).sameBytes(as: String(current[now])) else {
                return .refused("bytes \(start)..<\(end) of \(path) no longer spell the same text after rebasing to \(s)..<\(e)")
            }
            return .mapped(start: s, end: e, note: s != start ? "rebased from \(start)..<\(end) across edits" : nil)
        case .unchanged:
            return .mapped(start: start, end: end, note: nil)
        }
    }
}

// MARK: - model actions

extension ShellModel {
    /// Exact preview → source navigation. Same contract as `navigate(to:expectedText:)`
    /// with these guarantees:
    /// - staleness is decided byte-for-byte (`sameBytes`), so a normalization-only
    ///   edit (é → e + U+0301) is an edit, not a match;
    /// - a span overlapping the edited region is refused, and the note says
    ///   which bytes were edited;
    /// - a rebased span is verified to spell the same bytes it did at compile
    ///   time (item text is informational: generated text such as a section
    ///   number legitimately differs from its source);
    /// - the selection covers whole composed character sequences;
    /// - the active document switches when the span lives in another open one.
    ///
    /// `compiledText` overrides the recorded baseline for `source.path` (a v2
    /// display list that attests the buffer's exact digest passes the buffer).
    func navigateExactly(to source: RuntimeV1.SourceRange?, expectedText: String? = nil, compiledText: String? = nil) {
        guard let source else {
            navigationNote = "This item has no source mapping."
            return
        }
        guard let doc = documents.first(where: { $0.path == source.path }) else {
            let open = documents.map(\.path).joined(separator: ", ")
            navigationNote = "No open document named \(source.path) (open: \(open))."
            return
        }
        var start = source.startByte, end = source.endByte
        var notes: [String] = []
        if let compiled = compiledText ?? self.compiledText(for: source.path) {
            switch Navigation.rebaseExactly(start: start, end: end, from: compiled, to: doc.text, path: source.path) {
            case .refused(let why):
                navigationNote = "Source for this item was edited since the last compile (\(why)); recompile to navigate."
                return
            case .mapped(let s, let e, let note):
                if let note { notes.append(note) }
                start = s; end = e
            }
        }
        let ns: NSRange
        switch Navigation.editorRange(start: start, end: end, in: doc.text, path: source.path) {
        case .refused(let why):
            navigationNote = why
            return
        case .selected(let range, let widenedFrom):
            ns = range
            if let widenedFrom {
                notes.append("widened from UTF-16 \(widenedFrom.location)..<\(NSMaxRange(widenedFrom)) to whole characters")
            }
        }
        if let expectedText, !(doc.text as NSString).substring(with: ns).sameBytes(as: expectedText) {
            notes.append("“\(expectedText)” is generated from this source")
        }
        if activePath != source.path {
            activePath = source.path
            notes.append("switched to \(source.path)")
        }
        selection = .init(path: source.path, nsRange: ns, token: (selection?.token ?? 0) + 1)
        caretUTF16 = ns.location
        caretLengthUTF16 = ns.length
        navigationNote = "Selected \(source.path) bytes \(start)..<\(end) → UTF-16 \(ns.location)..<\(NSMaxRange(ns))"
            + (notes.isEmpty ? "" : " (" + notes.joined(separator: "; ") + ")")
    }

    /// The exact text `path` was compiled from by the engine's last
    /// completed compile (`compiledDocuments`), if it read that file.
    func compiledText(for path: String) -> String? { compiledDocuments[path] }

    /// ⌘⇧D: select the counterpart of the `\begin`/`\end`/`\label`/`\ref`
    /// under the caret; a label or reference in another open document
    /// switches to it.
    func goToMatching() {
        guard let caretByte = CaretSync.byteOffset(ofCaretUTF16: caretUTF16, in: activeText) else {
            navigationNote = "Caret position \(caretUTF16) is not valid in \(activePath)."
            return
        }
        goToMatchingInBuffer(caretByte: caretByte)
    }

    /// In-buffer matching across the open documents.
    func goToMatchingInBuffer(caretByte: Int) {
        switch Navigation.matchingRange(in: documents, activePath: activePath, caretByte: caretByte) {
        case .notFound(let why):
            navigationNote = why
        case .found(let path, let range, let note):
            guard let text = documents.first(where: { $0.path == path })?.text,
                  case .selected(let ns, _) = Navigation.editorRange(start: range.start, end: range.end, in: text, path: path) else {
                navigationNote = "Bytes \(range.start)..<\(range.end) are not a valid range in \(path)."
                return
            }
            activePath = path
            selection = .init(path: path, nsRange: ns, token: (selection?.token ?? 0) + 1)
            caretUTF16 = ns.location
            caretLengthUTF16 = ns.length
            navigationNote = note
        }
    }

    /// ⌘⇧] / ⌘⇧[: step through the underlined diagnostics (`editorMarkReport`,
    /// exact mark identities, stale spans withheld) of the active document in
    /// document order; past its last mark the step continues into the next
    /// open document with marks (project order, wrapping), switching to it.
    /// Diagnostics under edited text are skipped and counted in the note.
    func goToDiagnostic(forward: Bool) {
        guard let result = markSource?.result else {
            navigationNote = "No compile result loaded; nothing to navigate to."
            return
        }
        let report = editorMarkReport
        let here = EditorDiagnostics.step(report.marks, fromUTF16: caretUTF16, forward: forward,
                                          currentID: currentDiagnosticID, in: activeText)
        var chosen: (path: String, step: EditorDiagnosticNavigation.Step, report: EditorDiagnostics.Report)?
        if let here, !here.wrapped || documents.count == 1 {
            chosen = (activePath, here, report)
        } else if let active = documents.firstIndex(where: { $0.path == activePath }), documents.count > 1 {
            // Leaving the active document at either end: the first (or last)
            // mark of the next open document that has one.
            for offset in 1..<documents.count {
                let doc = documents[(active + offset) % documents.count]
                let other = diagnosticReport(for: doc.path, currentText: doc.text)
                if let step = EditorDiagnostics.step(other.marks, fromUTF16: forward ? -1 : Int.max, forward: forward, in: doc.text) {
                    chosen = (doc.path, step, other)
                    break
                }
            }
            if chosen == nil, let here { chosen = (activePath, here, report) }
        }
        guard let chosen else {
            let total = result.diagnostics.count
            let open = documents.count > 1 ? "an open document (\(documents.map(\.path).joined(separator: ", ")))" : activePath
            navigationNote = total == 0 ? "The last compile has no diagnostics."
                : report.staleNote.map { "No diagnostic can be selected: " + $0 + "." }
                ?? "None of the \(total) diagnostic\(total == 1 ? "" : "s") has a source in \(open)."
            return
        }
        let switched = chosen.path != activePath
        activePath = chosen.path
        currentDiagnosticID = chosen.step.item.id
        selection = .init(path: chosen.path, nsRange: chosen.step.item.nsRange, token: (selection?.token ?? 0) + 1)
        caretUTF16 = chosen.step.item.nsRange.location
        caretLengthUTF16 = chosen.step.item.nsRange.length
        navigationNote = chosen.step.announcement
            + (switched ? " (in \(chosen.path))" : "")
            + (chosen.report.staleNote.map { "; " + $0 } ?? "")
    }

    /// ⌘⇧J: select the full source span of the preview item under the caret
    /// (so the preview's caret highlight and page scroll follow) and say where
    /// it landed. It is also the manual override for automatic following
    /// (`CaretFollow.swift`): asking for the caret scrolls to it at once —
    /// without waiting for the debounce, and without regard to the "Preview
    /// follows the caret" preference, on or off — and resumes following (once
    /// the preference is on) if a manual preview scroll had stopped it.
    func revealCaretInPreview() {
        caretFollow.note(.explicit)
        // The pane scrolls to and flashes the target (EngineV3Preview.swift).
        if engineV3CaretTarget() == nil { navigationNote = "The caret maps to no glyph in the preview (a comment, the preamble, or not typeset yet)." }
    }
}

// MARK: - menu

/// `Navigate` menu, added from `FlashTeXMacApp` with one line.
struct NavigationCommands: Commands {
    var model: ShellModel
    /// The diagnostics panel's selection/occurrence cursor (DiagnosticsPanel.swift), while one is in the scene.
    @FocusedValue(\.diagnosticsPanel) private var diagnosticsPanel

    var body: some Commands {
        CommandMenu("Navigate") {
            Button("Go to Line…") { model.presentGoToLine() }
                .keyboardShortcut("l", modifiers: [.command])
            Button("Go to Matching \\begin/\\end or \\label/\\ref") { model.goToMatching() }
                .keyboardShortcut("d", modifiers: [.command, .shift])
            // Editor navigation lane (ShellModel+EditorNavigation.swift / EditorNavigation.swift).
            Button("Go to Definition") { model.goToDefinition() }
                .keyboardShortcut("j", modifiers: [.command, .control])
            Button("Go to Symbol…") { model.editorNavigation.symbolPickerShown = true }
                .keyboardShortcut("t", modifiers: [.command, .shift])
            Divider()
            Button("Select Environment") { model.selectEnvironment() }
                .keyboardShortcut("a", modifiers: [.command, .shift])
            Button("Wrap Selection in Environment…") { model.editorNavigation.wrapShown = true }
                .keyboardShortcut("w", modifiers: [.command, .shift])
            Button("Rename Symbol…") { model.presentRenameSymbol() }
                .keyboardShortcut("r", modifiers: [.option, .shift])
            Divider()
            Button("Next Diagnostic") { model.goToDiagnostic(forward: true) }
                .keyboardShortcut("]", modifiers: [.command, .shift])
                .disabled(model.toolbarProblemCount == 0) // change-only mirror: a per-compile diagnostics read here re-evaluates the App scene (FlashTeXMacApp.commands)
            Button("Previous Diagnostic") { model.goToDiagnostic(forward: false) }
                .keyboardShortcut("[", modifiers: [.command, .shift])
                .disabled(model.toolbarProblemCount == 0) // change-only mirror: a per-compile diagnostics read here re-evaluates the App scene (FlashTeXMacApp.commands)
            Button("Next Occurrence") { if let p = diagnosticsPanel { model.stepOccurrence(forward: true, panel: p) } }
                .keyboardShortcut("]", modifiers: [.command, .option])
                .disabled(diagnosticsPanel == nil)
            Button("Previous Occurrence") { if let p = diagnosticsPanel { model.stepOccurrence(forward: false, panel: p) } }
                .keyboardShortcut("[", modifiers: [.command, .option])
                .disabled(diagnosticsPanel == nil)
            Divider()
            Button("Reveal Caret in Preview") { model.revealCaretInPreview() }
                .keyboardShortcut("j", modifiers: [.command, .shift])
                .disabled(!model.engineV3Enabled)
        }
    }
}
