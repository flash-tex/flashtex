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
    /// link, but the names of a verbatim environment itself do. An emptied
    /// name (`\begin{}` … `\end{}`) still links, so a name deleted letter by
    /// letter can be typed anew. One lexical scan (`EditorNavigation.uses`);
    /// same-name nesting resolves by a per-name stack, innermost pair first.
    static func linkedNames(at caret: Int, in text: NSString) -> (active: NSRange, partner: NSRange, name: String)? {
        let uses = EditorNavigation.uses(in: text)
        for u in uses where u.name == "verb" || u.name == "verb*" {
            if NSLocationInRange(caret, u.range) || caret == NSMaxRange(u.range) { return nil }
        }
        var open: [String: [(use: EditorNavigation.Use, arg: NSRange)]] = [:]
        var best: (active: NSRange, partner: NSRange, name: String, begin: Int)?
        for u in uses where u.name == "begin" || u.name == "end" {
            guard let name = u.arg, let arg = u.argRange else { continue }
            if u.name == "begin" {
                open[name, default: []].append((u, arg))
                continue
            }
            guard let b = open[name]?.popLast() else { continue } // a stray `\end` never links
            let verbatim = SyntaxHighlighter.verbatimEnvironments.contains(name) || name == "comment"
            if verbatim, caret >= NSMaxRange(b.use.range), caret < u.range.location { return nil }
            let link: (NSRange, NSRange)? = containsCaret(caret, b.arg) ? (b.arg, arg)
                : containsCaret(caret, arg) ? (arg, b.arg) : nil
            if let link, best == nil || b.use.range.location > best!.begin {
                best = (link.0, link.1, name, b.use.range.location)
            }
        }
        return best.map { ($0.active, $0.partner, $0.name) }
    }

    /// A linked-editing session: the name span the user is editing, its
    /// partner and the partner's name, captured *before* the edit (in
    /// `shouldChangeTextIn`) at storage length `length`. Once the names
    /// diverge a fresh pair scan would refuse to link, so the spans are carried
    /// forward by the length change instead of rediscovered afterwards.
    struct LinkedSession: Equatable {
        var active: NSRange
        var partner: NSRange
        var name: String
        var length: Int

        /// The spans after the storage grew or shrank to `newLength`, every
        /// change having landed inside `active` (the typed edit and an
        /// auto-inserted closer at its caret).
        func advanced(to newLength: Int) -> LinkedSession {
            let delta = newLength - length
            var s = self
            s.active.length = max(0, active.length + delta)
            if partner.location >= NSMaxRange(active) { s.partner.location += delta }
            s.length = newLength
            return s
        }

        /// True when `edit` (current coordinates) lies inside the active span.
        func covers(_ edit: NSRange) -> Bool {
            edit.location >= active.location && NSMaxRange(edit) <= NSMaxRange(active)
        }
    }

    /// Starts (or, during an IME composition, continues) a linked session for
    /// a user edit of `range`. `text` is the pre-edit buffer; `continuing` is
    /// the open session when marked text is being replaced. Nil when the edit
    /// is not inside one name span of a balanced pair.
    static func linkedSession(for range: NSRange, in text: NSString, continuing: LinkedSession?) -> LinkedSession? {
        if let s = continuing?.advanced(to: text.length), s.covers(range) { return s }
        guard let link = linkedNames(at: range.location, in: text) else { return nil }
        let s = LinkedSession(active: link.active, partner: link.partner, name: link.name, length: text.length)
        return s.covers(range) ? s : nil
    }

    /// The partner rewrite once the edits of `session` are in `now`, in `now`'s
    /// coordinates. Nil when the names already match, or when `now` no longer
    /// shows the partner where the session expects it (something else changed
    /// the buffer: never write into a guessed range).
    static func partnerEdit(for session: LinkedSession, in now: NSString) -> (range: NSRange, replacement: String)? {
        let s = session.advanced(to: now.length)
        guard s.active.location >= 1, NSMaxRange(s.active) <= now.length,
              s.partner.location >= 1, NSMaxRange(s.partner) < now.length,
              now.character(at: s.active.location - 1) == 0x7B,
              now.character(at: s.partner.location - 1) == 0x7B,
              now.character(at: NSMaxRange(s.partner)) == 0x7D,
              now.substring(with: s.partner) == s.name else { return nil }
        let newName = now.substring(with: s.active)
        guard newName != s.name else { return nil }
        return (s.partner, newName)
    }

    /// Partner name-span rewrite for a user edit inside a linked name, in
    /// post-edit coordinates, from the pre-edit buffer `old`. Nil when the
    /// edit is not in a balanced name span or the partner already matches.
    static func linkedPartnerEdit(old: NSString, edit: (range: NSRange, replacement: String)) -> (range: NSRange, replacement: String)? {
        guard let session = linkedSession(for: edit.range, in: old, continuing: nil) else { return nil }
        let applied = old.replacingCharacters(in: edit.range, with: edit.replacement) as NSString
        return partnerEdit(for: session, in: applied)
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

    /// Enter in the field: rewrite both name spans as one pending grouped edit.
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
        let lineEdits = edits.map { EditorKeyHandling.LineEdit(range: $0.range, replacement: $0.replacement) }
        pendingEdit = .init(path: activePath, nsRange: edits[0].range, text: name, token: nextEditToken(),
                            revision: editorRevision, groupedEdits: lineEdits)
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
    /// `shouldChangeTextIn`, for a user edit whose location is on a
    /// `\begin{name}` / `\end{name}` name (`isOnEnvironmentName`, O(line)):
    /// captures the linked pair *before* the edit, so the partner is found
    /// whatever the edit is (typing, deletion, replacement, paste, a
    /// completion) and however stale `lastKnownText` is. Only this path pays
    /// the O(n) pair scan. During an IME composition the open session is
    /// carried forward instead: the marked text already made the names differ.
    func beginLinkedEnvironmentEdit(in tv: NSTextView, range: NSRange) {
        let continuing = tv.hasMarkedText() ? linkedSession : nil
        let text = SourceEditorView.nativeText(of: tv) as NSString
        linkedSession = EditorChangeEnvironment.linkedSession(for: range, in: text, continuing: continuing)
    }

    /// After a user edit inside a linked name, rewrite the partner from the
    /// session `beginLinkedEnvironmentEdit` captured. The partner goes through
    /// the registered `shouldChangeText` / `replaceCharacters` /
    /// `didChangeText` path in the undo group opened in `shouldChangeTextIn`,
    /// so one ⌘Z reverts both edits with ranges AppKit has already adjusted.
    /// Nested `didChangeText` is suppressed by `programmaticChanges`. Runs
    /// while the completion list is open too: the keystrokes that narrow the
    /// list are the ones that rename the environment.
    func syncLinkedEnvironmentPartner(in tv: NSTextView, edit: (range: NSRange, replacement: String)?) {
        let composing = tv.hasMarkedText()
        let session = linkedSession
        if !composing { linkedSession = nil } // a composition keeps its session until it commits
        defer {
            if openLinkedUndo {
                tv.undoManager?.endUndoGrouping()
                openLinkedUndo = false
            }
        }
        guard programmaticChanges == 0, !composing, edit != nil, let session else { return }
        if tv.undoManager?.isUndoing == true || tv.undoManager?.isRedoing == true { return }
        // O(name) on the live storage: the session says where both names are.
        guard let storage = tv.textStorage,
              let partnerEdit = EditorChangeEnvironment.partnerEdit(for: session, in: storage.mutableString) else { return }
        let saved = tv.selectedRange()
        programmaticChanges += 1
        if tv.shouldChangeText(in: partnerEdit.range, replacementString: partnerEdit.replacement) {
            tv.textStorage?.replaceCharacters(in: partnerEdit.range, with: partnerEdit.replacement)
            tv.didChangeText()
        }
        var restored = saved
        if partnerEdit.range.location < saved.location {
            restored.location += (partnerEdit.replacement as NSString).length - partnerEdit.range.length
        }
        let limit = tv.textStorage?.length ?? 0
        tv.setSelectedRange(NSRange(location: max(0, min(restored.location, limit)), length: 0))
        programmaticChanges -= 1
    }
}
