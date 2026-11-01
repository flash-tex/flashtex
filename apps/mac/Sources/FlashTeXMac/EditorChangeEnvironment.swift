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

    // MARK: linked editing (FlashTeXEditorCore/LinkedEnvironmentEditing.swift, shared with the iPad)

    typealias LinkedSession = LinkedEnvironmentEditing.LinkedSession

    static func linkedNames(at caret: Int, in text: NSString) -> (active: NSRange, partner: NSRange, name: String)? {
        LinkedEnvironmentEditing.linkedNames(at: caret, in: text)
    }

    static func linkedSession(for range: NSRange, in text: NSString, continuing: LinkedSession?) -> LinkedSession? {
        LinkedEnvironmentEditing.session(for: range, in: text, continuing: continuing)
    }

    static func partnerEdit(for session: LinkedSession, in now: NSString) -> (range: NSRange, replacement: String)? {
        LinkedEnvironmentEditing.partnerEdit(for: session, in: now)
    }

    static func linkedPartnerEdit(old: NSString, edit: (range: NSRange, replacement: String)) -> (range: NSRange, replacement: String)? {
        LinkedEnvironmentEditing.partnerEdit(old: old, edit: edit)
    }

    static func isOnEnvironmentName(in text: NSString, at caret: Int) -> Bool {
        LinkedEnvironmentEditing.isOnEnvironmentName(in: text, at: caret)
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
