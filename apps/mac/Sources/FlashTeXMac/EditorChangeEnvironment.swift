import AppKit
import SwiftUI

/// Change Environment… (⌃⌘E): rewrite the innermost `\begin{name}` / `\end{name}`
/// pair. Pure over UTF-16 `NSString`s; the menu / sheet / linked-editing hooks
/// below stay thin. Pair matching, comment/verbatim skipping and nesting come
/// from `EditorNavigation` — this file only decides which two *name* spans to
/// replace. Optional arguments and anything after `\begin{name}` are left
/// untouched because they sit outside those spans.
enum EditorChangeEnvironment {
    /// One name-span replacement. The command applies both as a single grouped
    /// undoable edit; linked editing applies them in the open typing group.
    struct Edit: Equatable {
        var range: NSRange
        var replacement: String
    }

    /// Why the command / linked session refuses. The UI beeps and announces
    /// `message`; an unchanged name is not a refusal (empty plan).
    enum Refusal: Equatable {
        case notInside
        case unbalanced
        case verbatim
        case invalidName(String)

        var message: String {
            switch self {
            case .notInside: return "Caret is not inside an environment."
            case .unbalanced: return "The environment is unbalanced."
            case .verbatim: return "Cannot change an environment from inside a verbatim body."
            case .invalidName(let n): return "“\(n)” is not an environment name."
            }
        }
    }

    /// The innermost balanced pair whose span contains `caret`, plus the two
    /// name spans inside `\begin{…}` / `\end{…}`. Nil when the command must
    /// refuse (not inside, unbalanced, or a verbatim body / `\verb`).
    static func target(in text: NSString, caret: Int) -> (pair: EditorNavigation.EnvironmentPair, beginName: NSRange, endName: NSRange)? {
        if isInsideVerbatimBody(caret: caret, in: text) { return nil }
        guard let pair = EditorNavigation.enclosingEnvironment(at: caret, in: text),
              pair.end != nil,
              let spans = nameSpans(for: pair, in: text) else { return nil }
        return (pair, spans.begin, spans.end)
    }

    static func refusal(in text: NSString, caret: Int) -> Refusal? {
        if isInsideVerbatimBody(caret: caret, in: text) { return .verbatim }
        guard let pair = EditorNavigation.enclosingEnvironment(at: caret, in: text) else { return .notInside }
        if pair.end == nil || nameSpans(for: pair, in: text) == nil { return .unbalanced }
        return nil
    }

    /// Why `newName` cannot be an environment name (letters and `*`, including
    /// non-ASCII letters; empty after trim is empty).
    static func nameProblem(_ newName: String) -> String? {
        let name = newName.trimmingCharacters(in: .whitespaces)
        if name.isEmpty { return "the environment name is empty" }
        if !name.allSatisfy({ $0.isLetter || $0 == "*" }) { return "“\(newName)” is not an environment name" }
        return nil
    }

    /// `(text, caret, newName) → [(range, replacement)]`. Empty when refused,
    /// invalid, or the new name equals the current one. Both ranges are the
    /// name spans only, begin then end, so `*`, `[opt]` and following `{arg}`
    /// stay in the source.
    static func replacements(in text: NSString, caret: Int, newName: String) -> [(range: NSRange, replacement: String)] {
        let trimmed = newName.trimmingCharacters(in: .whitespaces)
        if nameProblem(trimmed) != nil { return [] }
        guard let t = target(in: text, caret: caret) else { return [] }
        if trimmed == t.pair.name { return [] }
        return [(t.beginName, trimmed), (t.endName, trimmed)]
    }

    /// Covering replacement for `edits` (document order): one undoable span.
    static func groupedReplacement(edits: [(range: NSRange, replacement: String)], in text: NSString) -> (range: NSRange, text: String)? {
        let ordered = edits.sorted { $0.range.location < $1.range.location }
        guard let first = ordered.first, let last = ordered.last, NSMaxRange(last.range) <= text.length else { return nil }
        let span = NSRange(location: first.range.location, length: NSMaxRange(last.range) - first.range.location)
        var out = ""
        var cursor = first.range.location
        for e in ordered {
            guard e.range.location >= cursor, NSMaxRange(e.range) <= text.length else { return nil }
            out += text.substring(with: NSRange(location: cursor, length: e.range.location - cursor))
            out += e.replacement
            cursor = NSMaxRange(e.range)
        }
        return (span, out)
    }

    /// Inventory names then names already used in `document`, unique, filtered
    /// by a fuzzy subsequence of `query` (empty query lists all).
    static func suggestions(query: String, in document: String) -> [String] {
        let known = Completion.knownEnvironments
        let used = Completion.documentEnvironments(in: document)
        var seen = Set<String>()
        var all: [String] = []
        for n in known + used where seen.insert(n).inserted { all.append(n) }
        let q = query.trimmingCharacters(in: .whitespaces)
        guard !q.isEmpty else { return all }
        return all.filter { EditorNavigation.fuzzyScore(q, in: $0) != nil }
    }

    /// Caret is inside (or at the end of) a `\begin{name}` / `\end{name}` name
    /// of a balanced pair. Unbalanced pairs never link; verbatim *bodies* never
    /// link, but the names of a verbatim environment itself do.
    static func linkedNames(at caret: Int, in text: NSString) -> (active: NSRange, partner: NSRange, name: String)? {
        if isInsideVerbatimBody(caret: caret, in: text) { return nil }
        let pairs = EditorNavigation.environmentPairs(in: text).sorted { $0.begin.location > $1.begin.location }
        for p in pairs {
            guard let spans = nameSpans(for: p, in: text) else { continue }
            if containsCaret(caret, spans.begin) { return (spans.begin, spans.end, p.name) }
            if containsCaret(caret, spans.end) { return (spans.end, spans.begin, p.name) }
        }
        return nil
    }

    /// Pre-edit buffer for linked editing: `lastKnown` when its length matches
    /// the edit, otherwise invert an insert (delete/replace still need `lastKnown`).
    static func preEditBuffer(now: NSString, lastKnown: NSString, edit: (range: NSRange, replacement: String)) -> NSString? {
        let inserted = (edit.replacement as NSString).length
        let expectedOldLength = now.length - (inserted - edit.range.length)
        if lastKnown.length == expectedOldLength { return lastKnown }
        guard edit.range.length == 0 else { return nil }
        let insertedRange = NSRange(location: edit.range.location, length: inserted)
        guard NSMaxRange(insertedRange) <= now.length, now.substring(with: insertedRange) == edit.replacement else { return nil }
        return now.replacingCharacters(in: insertedRange, with: "") as NSString
    }

    /// Partner name-span rewrite for a user edit inside a linked name, in
    /// post-edit coordinates. Nil when the edit is not in a balanced name span
    /// or the partner already matches.
    static func linkedPartnerEdit(old: NSString, edit: (range: NSRange, replacement: String)) -> (range: NSRange, replacement: String)? {
        let inserted = (edit.replacement as NSString).length
        guard let link = linkedNames(at: edit.range.location, in: old),
              edit.range.location >= link.active.location,
              NSMaxRange(edit.range) <= NSMaxRange(link.active) else { return nil }
        let delta = inserted - edit.range.length
        // Half-open: an insert at the start or end of the name is *inside* it
        // (`linkedNames` treats both carets as on the span). `<=` / `>=` would
        // shift the span past the typed character and the partner would not change.
        func shifted(_ r: NSRange) -> NSRange {
            if NSMaxRange(edit.range) < r.location { return NSRange(location: r.location + delta, length: r.length) }
            if edit.range.location > NSMaxRange(r) { return r }
            return NSRange(location: r.location, length: max(0, r.length + delta))
        }
        let applied = old.replacingCharacters(in: edit.range, with: edit.replacement) as NSString
        let active = shifted(link.active)
        let partner = shifted(link.partner)
        guard NSMaxRange(active) <= applied.length, NSMaxRange(partner) <= applied.length else { return nil }
        let newName = applied.substring(with: active)
        guard newName != applied.substring(with: partner) else { return nil }
        return (partner, newName)
    }

    // MARK: scan helpers

    /// True when `caret` sits in a `\verb` / `\verb*` argument or in the body
    /// of a verbatim-like / `comment` environment (not on the `\begin`/`\end`
    /// names themselves).
    static func isInsideVerbatimBody(caret: Int, in text: NSString) -> Bool {
        for u in EditorNavigation.uses(in: text) {
            if u.name == "verb" || u.name == "verb*" {
                if NSLocationInRange(caret, u.range) || caret == NSMaxRange(u.range) { return true }
            }
        }
        for p in EditorNavigation.environmentPairs(in: text) {
            let verbatim = SyntaxHighlighter.verbatimEnvironments.contains(p.name) || p.name == "comment"
            guard verbatim else { continue }
            let bodyStart = NSMaxRange(p.begin)
            let bodyEnd = p.end?.location ?? text.length
            if bodyEnd >= bodyStart, caret >= bodyStart, caret < bodyEnd { return true }
        }
        return false
    }

    static func nameSpans(for pair: EditorNavigation.EnvironmentPair, in text: NSString) -> (begin: NSRange, end: NSRange)? {
        guard let end = pair.end else { return nil }
        var beginArg: NSRange?
        var endArg: NSRange?
        for u in EditorNavigation.uses(in: text) {
            if u.range == pair.begin, let a = u.argRange { beginArg = a }
            if u.range == end, let a = u.argRange { endArg = a }
        }
        if let b = beginArg, let e = endArg { return (b, e) }
        return nil
    }

    private static func containsCaret(_ caret: Int, _ range: NSRange) -> Bool {
        NSLocationInRange(caret, range) || caret == NSMaxRange(range)
    }

    /// True when `caret` sits in (or at the end of) the `{name}` of `\begin` /
    /// `\end` on its line. O(line), no document pair scan — ordinary typing
    /// next to `\end{document}` must not pay `environmentPairs`.
    static func isOnEnvironmentName(in text: NSString, at caret: Int) -> Bool {
        guard text.length > 0 else { return false }
        let i = min(max(0, caret), text.length)
        var j = i
        var open = -1
        while j > 0 {
            let c = text.character(at: j - 1)
            if c == 0x0A { break }
            if c == 0x7D { return false }
            if c == 0x7B { open = j - 1; break }
            j -= 1
        }
        guard open >= 0 else { return false }
        var close = open + 1
        while close < text.length {
            let c = text.character(at: close)
            if c == 0x7D || c == 0x0A { break }
            close += 1
        }
        if caret < open + 1 || caret > close { return false }
        var k = open
        while k > 0, text.character(at: k - 1) == 0x20 { k -= 1 }
        func token(_ s: String) -> Bool {
            let n = (s as NSString).length
            return k >= n && text.substring(with: NSRange(location: k - n, length: n)) == s
        }
        return token("\\begin") || token("\\end")
    }
}

// MARK: - model

extension ShellModel {
    /// ⌃⌘E: open the Change Environment field prefilled with the innermost
    /// name, or beep + announce when the pair is missing, unbalanced, or the
    /// caret is in a verbatim body.
    func presentChangeEnvironment() {
        let text = activeText as NSString
        let caret = min(caretUTF16, text.length)
        if let why = EditorChangeEnvironment.refusal(in: text, caret: caret) {
            refuseChangeEnvironment(why.message)
            return
        }
        guard let t = EditorChangeEnvironment.target(in: text, caret: caret) else {
            refuseChangeEnvironment(EditorChangeEnvironment.Refusal.notInside.message)
            return
        }
        editorNavigation.changeName = t.pair.name
        editorNavigation.changeShown = true
    }

    /// Enter in the field: rewrite both name spans as one pending edit.
    func applyChangeEnvironment() {
        let name = editorNavigation.changeName.trimmingCharacters(in: .whitespaces)
        if EditorChangeEnvironment.nameProblem(name) != nil {
            refuseChangeEnvironment(EditorChangeEnvironment.Refusal.invalidName(editorNavigation.changeName).message)
            return
        }
        let text = activeText as NSString
        let caret = min(caretUTF16, text.length)
        if let why = EditorChangeEnvironment.refusal(in: text, caret: caret) {
            editorNavigation.changeShown = false
            refuseChangeEnvironment(why.message)
            return
        }
        let edits = EditorChangeEnvironment.replacements(in: text, caret: caret, newName: name)
        if edits.isEmpty {
            editorNavigation.changeShown = false
            return
        }
        guard let grouped = EditorChangeEnvironment.groupedReplacement(edits: edits, in: text) else {
            editorNavigation.changeShown = false
            return
        }
        pendingEdit = .init(path: activePath, nsRange: grouped.range, text: grouped.text, token: nextEditToken(), revision: editorRevision)
        let begin = NSRange(location: edits[0].range.location, length: (name as NSString).length)
        selection = .init(path: activePath, nsRange: begin, token: (selection?.token ?? 0) + 1)
        caretUTF16 = begin.location
        caretLengthUTF16 = begin.length
        editorNavigation.changeShown = false
        navigationNote = "Changed environment to \\begin{\(name)}…\\end{\(name)} (undo with ⌘Z)."
    }

    func refuseChangeEnvironment(_ message: String) {
        NSSound.beep()
        navigationNote = message
        editorNavigation.changeAnnouncements.append(message)
        NSAccessibility.post(element: NSApp as Any, notification: .announcementRequested,
                             userInfo: [.announcement: message, .priority: NSAccessibilityPriorityLevel.high.rawValue])
    }
}

// MARK: - sheet

/// Small field prefilled with the current name; Return applies, Esc dismisses.
struct ChangeEnvironmentSheet: View {
    @Environment(ShellModel.self) var model
    @FocusState private var focused: Bool
    static let identifier = "change.environment"

    var suggestions: [String] {
        EditorChangeEnvironment.suggestions(query: model.editorNavigation.changeName, in: model.activeText)
    }

    var body: some View {
        @Bindable var model = model
        VStack(alignment: .leading, spacing: 10) {
            Text("Change environment").font(.headline)
            TextField("Environment name", text: $model.editorNavigation.changeName)
                .textFieldStyle(.roundedBorder).font(.body.monospaced())
                .focused($focused)
                .accessibilityLabel("Environment name")
                .onSubmit { model.applyChangeEnvironment() }
                .onKeyPress(.escape) { model.editorNavigation.changeShown = false; return .handled }
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 100))], alignment: .leading, spacing: 4) {
                    ForEach(suggestions.prefix(24), id: \.self) { env in
                        Button(env) {
                            model.editorNavigation.changeName = env
                            model.applyChangeEnvironment()
                        }
                        .buttonStyle(.bordered).controlSize(.small).font(.body.monospaced())
                    }
                }
            }
            .frame(maxHeight: 160)
            HStack {
                Spacer()
                Button("Cancel") { model.editorNavigation.changeShown = false }.keyboardShortcut(.cancelAction)
                Button("Change") { model.applyChangeEnvironment() }.keyboardShortcut(.defaultAction)
                    .disabled(model.editorNavigation.changeName.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(16)
        .frame(width: 420)
        .onAppear { focused = true }
        .accessibilityIdentifier(Self.identifier)
    }
}

// MARK: - linked editing (SourceEditorView)

extension SourceEditorView.Coordinator {
    /// After a user edit inside a `\begin{name}` / `\end{name}` name, rewrite
    /// the partner. Recovers the pre-edit buffer (from `lastKnownText` or by
    /// inverting an insert) because once the names diverge a fresh pair scan
    /// would refuse to link. The partner write is not its own undo item: it
    /// mirrors the typed name, so ⌘Z of the typing restores both via the
    /// same linked path. Nested `insertText`/`didChangeText` at a non-caret
    /// range from inside `textDidChange` either no-ops or splits the undo group.
    func syncLinkedEnvironmentPartner(in tv: NSTextView, edit: (range: NSRange, replacement: String)?) {
        guard programmaticChanges == 0, !tv.hasMarkedText(), let edit else { return }
        if let completing = tv as? CompletingTextView, completing.isCompletionActive { return }
        // O(line) on the live storage: typing on the `\end{document}` line
        // (large-document bench) is not inside the name, so skip the O(n)
        // pair scan and nativeText copy.
        guard let storage = tv.textStorage,
              EditorChangeEnvironment.isOnEnvironmentName(in: storage.mutableString, at: edit.range.location) else { return }
        let now = SourceEditorView.nativeText(of: tv) as NSString
        guard let old = EditorChangeEnvironment.preEditBuffer(now: now, lastKnown: lastKnownText as NSString, edit: edit),
              let partnerEdit = EditorChangeEnvironment.linkedPartnerEdit(old: old, edit: edit) else { return }
        let saved = tv.selectedRange()
        programmaticChanges += 1
        tv.undoManager?.disableUndoRegistration()
        tv.textStorage?.replaceCharacters(in: partnerEdit.range, with: partnerEdit.replacement)
        tv.undoManager?.enableUndoRegistration()
        var restored = saved
        if partnerEdit.range.location < saved.location {
            restored.location += (partnerEdit.replacement as NSString).length - partnerEdit.range.length
        }
        let limit = tv.textStorage?.length ?? 0
        tv.setSelectedRange(NSRange(location: max(0, min(restored.location, limit)), length: 0))
        programmaticChanges -= 1
        lastKnownText = SourceEditorView.nativeText(of: tv)
        parent.text = lastKnownText
        refreshBraceHighlight(tv)
    }
}
