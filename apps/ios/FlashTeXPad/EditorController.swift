import FlashTeXEditorCore
import FlashTeXPadKit
import FlashTeXProtocol
import GameController
import UIKit

/// Hardware-keyboard commands of the editor. Each is a `UIKeyCommand` on the
/// text view (so it works whenever the editor has focus and shows up, with
/// its title, in the ⌘-hold HUD); ⌘Z / ⇧⌘Z are the system's.
enum EditorCommand: String, CaseIterable {
    case showCompletions, dismiss, acceptOrNextStop, previousStop
    case toggleComment, indent, outdent, bold, italic, toggleDiagnostics

    var input: String {
        switch self {
        case .showCompletions: return " "
        case .dismiss: return UIKeyCommand.inputEscape
        case .acceptOrNextStop, .previousStop: return "\t"
        case .toggleComment: return "/"
        case .indent: return "]"
        case .outdent: return "["
        case .bold: return "b"
        case .italic: return "i"
        case .toggleDiagnostics: return "e"
        }
    }

    var modifiers: UIKeyModifierFlags {
        switch self {
        case .showCompletions: return .control
        case .dismiss, .acceptOrNextStop: return []
        case .previousStop: return .shift
        case .bold: return [.command, .shift]
        case .toggleComment, .indent, .outdent, .italic, .toggleDiagnostics: return .command
        }
    }

    var title: String {
        switch self {
        case .showCompletions: return "Show Completions"
        case .dismiss: return "Dismiss Completions"
        case .acceptOrNextStop: return "Accept Completion / Next Placeholder"
        case .previousStop: return "Previous Placeholder"
        case .toggleComment: return "Toggle Comment"
        case .indent: return "Indent"
        case .outdent: return "Outdent"
        case .bold: return "Bold (\\textbf)"
        case .italic: return "Italic (\\textit)"
        case .toggleDiagnostics: return "Toggle Diagnostics"
        }
    }

    /// Whether the command handles a key the system would otherwise use
    /// (Tab inserts a tab, Esc dismisses the keyboard).
    var overridesSystem: Bool { self == .acceptOrNextStop || self == .previousStop || self == .dismiss }

    var keyCommand: UIKeyCommand {
        let c = UIKeyCommand(title: title, action: #selector(EditorTextView.editorCommand(_:)), input: input, modifierFlags: modifiers,
                             propertyList: rawValue)
        c.wantsPriorityOverSystemBehavior = overridesSystem
        return c
    }
}

/// The text view: forwards its key commands to the controller.
final class EditorTextView: UITextView {
    weak var controller: EditorController?

    override var keyCommands: [UIKeyCommand]? { EditorCommand.allCases.map(\.keyCommand) }

    @objc func editorCommand(_ sender: UIKeyCommand) {
        guard let raw = sender.propertyList as? String, let command = EditorCommand(rawValue: raw) else { return }
        controller?.perform(command)
    }
}

/// Owns the editor's `UITextView`, its `EditorTextStorage` and every editing
/// behaviour the delegate adds on top of UIKit: auto-close with type-over
/// and pair deletion, the Return key, bracket-match highlighting, snippet
/// stops, line commands and the accessory bar — each decided by the shared
/// FlashTeXEditorCore helpers the Mac editor uses, so the two agree.
///
/// Programmatic edits go through `UITextInput.replace(_:withText:)`, which
/// keeps them in the text view's undo stack; the `programmatic` counter
/// stops the delegate from treating them as keystrokes. A change reaches
/// the model once (`handleChange`, keyed on the storage's edit serial)
/// whether UIKit or this controller made it.
@MainActor
final class EditorController: NSObject, UITextViewDelegate {
    let storage: EditorTextStorage
    let textView: EditorTextView
    let accessoryBar: EditorAccessoryBar

    var indentUnit = "    "
    var rules = EnvironmentEditingRules.conventional
    var autoClosePairs = AutoClose.conventionalPairs
    var closeEnvironments = true

    /// The text changed (a keystroke, a command, an accepted completion).
    var onChange: ((_ text: String, _ caretUTF16: Int, _ mathMode: Bool) -> Void)?
    /// The caret moved without a text change.
    var onCaret: ((_ caretUTF16: Int, _ mathMode: Bool) -> Void)?
    /// Commands the model answers: show/dismiss completions, toggle the
    /// diagnostics panel, and Tab while a completion list is showing
    /// (return true when a completion was accepted).
    var onCommand: ((EditorCommand) -> Bool)?

    /// UTF-16 offsets of auto-inserted closers not yet typed over or edited
    /// away (the type-over authority; kept aligned with edits).
    private(set) var pendingClosers: [Int] = []
    /// Snippet placeholders still to visit with Tab, absolute UTF-16 offsets.
    private(set) var snippetStops: [Int] = []
    /// The delimiter pair highlighted around the caret.
    private(set) var matchRanges: [NSRange] = []
    /// The document revision the text view currently shows (set by the
    /// SwiftUI layer; a different model revision reloads the text).
    var loadedRevision = 0

    private var programmatic = 0
    private var lastEdit: (range: NSRange, replacement: String)?
    private var handledSerial = 0
    private var keyboardObservers: [NSObjectProtocol] = []
    /// Tests: pretend a hardware keyboard is (not) attached.
    var hardwareKeyboardOverride: Bool? { didSet { updateAccessoryVisibility() } }

    init(theme: EditorTheme = .standard) {
        storage = EditorTextStorage(theme: theme)
        let layoutManager = NSLayoutManager()
        layoutManager.allowsNonContiguousLayout = true
        storage.addLayoutManager(layoutManager)
        let container = NSTextContainer(size: CGSize(width: 0, height: CGFloat.greatestFiniteMagnitude))
        container.widthTracksTextView = true
        layoutManager.addTextContainer(container)
        textView = EditorTextView(frame: CGRect(x: 0, y: 0, width: 600, height: 400), textContainer: container)
        accessoryBar = EditorAccessoryBar()
        super.init()
        textView.controller = self
        textView.delegate = self
        textView.font = theme.font
        textView.textColor = theme.text
        textView.typingAttributes = theme.baseAttributes
        textView.autocorrectionType = .no
        textView.autocapitalizationType = .none
        textView.smartQuotesType = .no
        textView.smartDashesType = .no
        textView.smartInsertDeleteType = .no
        textView.spellCheckingType = .no
        textView.keyboardType = .asciiCapable
        textView.alwaysBounceVertical = true
        textView.textContainerInset = UIEdgeInsets(top: 12, left: 8, bottom: 12, right: 8)
        textView.accessibilityIdentifier = "editor.textview"
        accessoryBar.onItem = { [weak self] item in self?.insert(item) }
        let center = NotificationCenter.default
        for name in [Notification.Name.GCKeyboardDidConnect, .GCKeyboardDidDisconnect] {
            keyboardObservers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated { self?.updateAccessoryVisibility() }
            })
        }
        updateAccessoryVisibility()
    }

    deinit {
        for o in keyboardObservers { NotificationCenter.default.removeObserver(o) }
    }

    // MARK: text access

    var text: String { storage.string }
    var length: Int { storage.length }
    var selectedRange: NSRange { textView.selectedRange }
    private var ns: NSString { storage.string as NSString }

    /// Replaces the whole text (a document was opened or changed behind the
    /// editor's back): a fresh highlight, no pending closers, undo cleared.
    func load(text: String, caret: Int, revision: Int) {
        programmatic += 1
        storage.replaceCharacters(in: NSRange(location: 0, length: storage.length), with: text)
        textView.undoManager?.removeAllActions()
        pendingClosers = []
        snippetStops = []
        matchRanges = []
        textView.selectedRange = NSRange(location: max(0, min(caret, storage.length)), length: 0)
        handledSerial = storage.editSerial
        loadedRevision = revision
        programmatic -= 1
        noteSelectionChanged()
    }

    // MARK: UITextViewDelegate

    func textView(_ tv: UITextView, shouldChangeTextIn range: NSRange, replacementText text: String) -> Bool {
        guard programmatic == 0, tv.markedTextRange == nil else { return true }
        let ns = self.ns
        // Type-over: the closer the user types is the one auto-inserted here.
        if range.length == 0, let prefix = AutoClose.overtypePrefix(typing: text, at: range.location, in: ns, pending: pendingClosers) {
            if prefix > 0 { replace(NSRange(location: range.location - prefix, length: prefix), with: "", caret: nil) }
            let start = range.location - prefix
            pendingClosers.removeAll { $0 >= start && $0 <= start + prefix }
            setCaret(start + prefix + 1)
            noteSelectionChanged()
            return false
        }
        // Return: indent, line templates, `\end{…}` (EnvironmentEditingRules).
        if text == "\n", range.length == 0 {
            let insertion = LaTeXEditing.newline(in: ns, caret: range.location, indentUnit: indentUnit,
                                                 closeEnvironments: closeEnvironments, rules: rules)
            replace(range, with: insertion.text, caret: range.location + insertion.caretOffset)
            return false
        }
        // Backspace between an auto-closed pair removes both halves.
        if text.isEmpty, range.length == 1, AutoClose.backspaceRemovesPair(at: range.location + 1, in: ns, pending: pendingClosers) {
            replace(NSRange(location: range.location, length: 2), with: "", caret: range.location)
            return false
        }
        lastEdit = (range, text)
        return true
    }

    func textViewDidChange(_ tv: UITextView) {
        guard programmatic == 0 else { return }
        if let edit = lastEdit {
            lastEdit = nil
            shiftTracking(edit: edit.range, replacementLength: (edit.replacement as NSString).length)
            autoClose(after: edit)
        } else {
            // An edit the delegate did not see (undo, redo, dictation):
            // the tracked offsets may be stale, so forget them.
            pendingClosers = []
            snippetStops = []
        }
        handleChange()
    }

    func textViewDidChangeSelection(_ tv: UITextView) {
        guard programmatic == 0 else { return }
        noteSelectionChanged()
    }

    // MARK: edits

    /// A programmatic, undoable edit: shifts the tracked offsets, moves the
    /// caret and reports the change once.
    private func replace(_ range: NSRange, with replacement: String, caret: Int?) {
        programmatic += 1
        replaceRaw(range, with: replacement)
        shiftTracking(edit: range, replacementLength: (replacement as NSString).length)
        if let caret { textView.selectedRange = NSRange(location: max(0, min(caret, storage.length)), length: 0) }
        programmatic -= 1
        handleChange()
        noteSelectionChanged()
    }

    private func replaceRaw(_ range: NSRange, with replacement: String) {
        guard let start = textView.position(from: textView.beginningOfDocument, offset: range.location),
              let end = textView.position(from: start, offset: range.length),
              let textRange = textView.textRange(from: start, to: end) else { return }
        textView.replace(textRange, withText: replacement)
    }

    private func setCaret(_ location: Int) {
        programmatic += 1
        textView.selectedRange = NSRange(location: max(0, min(location, storage.length)), length: 0)
        programmatic -= 1
    }

    private func shiftTracking(edit range: NSRange, replacementLength: Int) {
        pendingClosers = AutoClose.shifted(pendingClosers, edit: range, replacementLength: replacementLength)
        snippetStops = AutoClose.shifted(snippetStops, edit: range, replacementLength: replacementLength)
    }

    /// After a single unit was typed at the caret: the closer it earns, if any.
    private func autoClose(after edit: (range: NSRange, replacement: String)) {
        guard edit.range.length == 0, edit.replacement.count == 1, let opener = edit.replacement.first else { return }
        let caret = edit.range.location + (edit.replacement as NSString).length
        guard textView.selectedRange == NSRange(location: caret, length: 0) else { return }
        let math = storage.mode(at: caret).isMath
        guard let closer = AutoClose.closer(afterTyping: opener, in: text, caretUTF16: caret, mathMode: math, pairs: autoClosePairs) else { return }
        programmatic += 1
        replaceRaw(NSRange(location: caret, length: 0), with: closer)
        textView.selectedRange = NSRange(location: caret, length: 0)
        programmatic -= 1
        let length = (closer as NSString).length
        snippetStops = AutoClose.shifted(snippetStops, edit: NSRange(location: caret, length: 0), replacementLength: length)
        pendingClosers += (0..<length).map { caret + $0 }
    }

    private func handleChange() {
        guard storage.editSerial != handledSerial else { return }
        handledSerial = storage.editSerial
        let caret = textView.selectedRange.location
        onChange?(text, caret, storage.mode(at: caret).isMath)
    }

    private func noteSelectionChanged() {
        updateMatchHighlight()
        textView.typingAttributes = storage.theme.baseAttributes
        let caret = textView.selectedRange.location
        onCaret?(caret, storage.mode(at: caret).isMath)
    }

    // MARK: bracket / environment match

    private static let delimiterUnits: Set<unichar> = [0x7B, 0x7D, 0x5B, 0x5D, 0x24]

    private func updateMatchHighlight() {
        let ns = self.ns
        for r in matchRanges where NSMaxRange(r) <= ns.length { storage.removeAttribute(.backgroundColor, range: r) }
        matchRanges = []
        let sel = textView.selectedRange
        guard sel.length == 0 else { return }
        let caret = sel.location
        // O(1) gate before the byte scan: a delimiter must touch the caret.
        let adjacent = (caret > 0 && Self.delimiterUnits.contains(ns.character(at: caret - 1)))
            || (caret < ns.length && Self.delimiterUnits.contains(ns.character(at: caret)))
        guard adjacent, let match = BraceMatcher.match(in: text, caretUTF16: caret) else { return }
        matchRanges = [match.open, match.close]
        for r in matchRanges { storage.addAttribute(.backgroundColor, value: storage.theme.matchBackground, range: r) }
    }

    // MARK: commands

    func perform(_ command: EditorCommand) {
        switch command {
        case .showCompletions, .toggleDiagnostics:
            _ = onCommand?(command)
        case .dismiss:
            pressEscape()
        case .acceptOrNextStop:
            pressTab()
        case .previousStop:
            break // stops are visited forward; ⇧Tab is reserved
        case .toggleComment:
            apply(LaTeXEditing.toggleComment(in: ns, selection: textView.selectedRange))
        case .indent:
            apply(LaTeXEditing.indent(in: ns, selection: textView.selectedRange, unit: indentUnit, outdent: false))
        case .outdent:
            apply(LaTeXEditing.indent(in: ns, selection: textView.selectedRange, unit: indentUnit, outdent: true))
        case .bold:
            wrap("\\textbf{", "}")
        case .italic:
            wrap("\\textit{", "}")
        }
    }

    private func apply(_ edit: LaTeXEditing.CommentToggle?) {
        guard let edit else { return }
        programmatic += 1
        replaceRaw(edit.range, with: edit.replacement)
        shiftTracking(edit: edit.range, replacementLength: (edit.replacement as NSString).length)
        textView.selectedRange = edit.selection
        programmatic -= 1
        handleChange()
        noteSelectionChanged()
    }

    /// `\textbf{…}` around the selection; an empty selection gets the caret
    /// inside the braces with the `}` typable-over like an auto-closed one.
    private func wrap(_ open: String, _ close: String) {
        let sel = textView.selectedRange
        let inner = ns.substring(with: sel)
        let openLength = (open as NSString).length
        programmatic += 1
        replaceRaw(sel, with: open + inner + close)
        shiftTracking(edit: sel, replacementLength: openLength + sel.length + (close as NSString).length)
        if sel.length == 0 {
            textView.selectedRange = NSRange(location: sel.location + openLength, length: 0)
            pendingClosers.append(sel.location + openLength)
        } else {
            textView.selectedRange = NSRange(location: sel.location + openLength, length: sel.length)
        }
        programmatic -= 1
        handleChange()
        noteSelectionChanged()
    }

    /// Tab: accept the showing completion, else jump to the next snippet
    /// placeholder, else insert one indent unit.
    func pressTab() {
        if onCommand?(.acceptOrNextStop) == true { return }
        if !snippetStops.isEmpty {
            let next = snippetStops.removeFirst()
            setCaret(next)
            noteSelectionChanged()
            return
        }
        replace(textView.selectedRange, with: indentUnit, caret: textView.selectedRange.location + (indentUnit as NSString).length)
    }

    /// Esc: leave the snippet and dismiss the completion list.
    func pressEscape() {
        snippetStops = []
        _ = onCommand?(.dismiss)
    }

    // MARK: typing (the accessory bar and the tests use the keystroke path)

    /// As if the keyboard typed `s` at the selection: the same delegate
    /// decisions (type-over, Return, auto-close) as a real keystroke.
    func type(_ s: String) {
        let sel = textView.selectedRange
        guard textView(textView, shouldChangeTextIn: sel, replacementText: s) else { return }
        programmatic += 1
        replaceRaw(sel, with: s)
        textView.selectedRange = NSRange(location: sel.location + (s as NSString).length, length: 0)
        programmatic -= 1
        textViewDidChange(textView)
        noteSelectionChanged()
    }

    /// As if Backspace was pressed with the current selection.
    func backspace() {
        let sel = textView.selectedRange
        let range = sel.length > 0 ? sel : NSRange(location: max(0, sel.location - 1), length: sel.location > 0 ? 1 : 0)
        guard range.length > 0 else { return }
        guard textView(textView, shouldChangeTextIn: range, replacementText: "") else { return }
        programmatic += 1
        replaceRaw(range, with: "")
        textView.selectedRange = NSRange(location: range.location, length: 0)
        programmatic -= 1
        textViewDidChange(textView)
        noteSelectionChanged()
    }

    func pressReturn() { type("\n") }

    func select(_ range: NSRange) {
        textView.selectedRange = range
        noteSelectionChanged()
    }

    // MARK: completion acceptance

    /// Inserts a completion: its snippet (caret inside, later placeholders
    /// as Tab stops) or its text. `range` is the UTF-16 range of the prefix
    /// the suggestion replaces. An environment skeleton supplies its own
    /// `}`, so a pending `}` right after the prefix (from the auto-closed
    /// `\begin{`) is eaten rather than doubled.
    func accept(_ suggestion: LocalCompletion.Suggestion, replacing range: NSRange) {
        var range = range
        let ns = self.ns
        if suggestion.kind == .environment, suggestion.snippet != nil, NSMaxRange(range) < ns.length,
           ns.character(at: NSMaxRange(range)) == 0x7D, pendingClosers.contains(NSMaxRange(range)) {
            range.length += 1
        }
        let insertion = suggestion.insertion
        let caret = range.location + (suggestion.snippet?.caretUTF16 ?? (insertion as NSString).length)
        programmatic += 1
        replaceRaw(range, with: insertion)
        shiftTracking(edit: range, replacementLength: (insertion as NSString).length)
        textView.selectedRange = NSRange(location: caret, length: 0)
        if let snippet = suggestion.snippet {
            snippetStops = snippet.stops.map { range.location + $0 }
            // The placeholder's closer overtypes like a hand-typed pair's.
            let after = self.ns
            if caret < after.length, [0x7D, 0x5D, 0x29].contains(after.character(at: caret)) {
                pendingClosers.append(caret)
            }
        }
        programmatic -= 1
        handleChange()
        noteSelectionChanged()
    }

    // MARK: accessory bar

    private var hardwareKeyboardAttached: Bool { hardwareKeyboardOverride ?? (GCKeyboard.coalesced != nil) }

    /// Shown with the on-screen keyboard only: a hardware keyboard has the
    /// key commands, and the bar would just eat a row.
    func updateAccessoryVisibility() {
        let wanted: UIView? = hardwareKeyboardAttached ? nil : accessoryBar
        guard textView.inputAccessoryView !== wanted else { return }
        textView.inputAccessoryView = wanted
        if textView.isFirstResponder { textView.reloadInputViews() }
    }

    var accessoryBarShown: Bool { textView.inputAccessoryView === accessoryBar }

    func insert(_ item: EditorAccessoryBar.Item) {
        switch item {
        case .backslash: type("\\")
        case .braces: type("{")
        case .brackets: type("[")
        case .dollar: type("$")
        case .caret: type("^")
        case .underscore: type("_")
        case .frac: insertSnippet(LaTeXSnippets.argument(name: "frac", arguments: "{numerator}{denominator}"))
        case .sqrt: insertSnippet(LaTeXSnippets.argument(name: "sqrt", arguments: "[root]{radicand}"))
        case .begin:
            type("\\begin")
            type("{")
            _ = onCommand?(.showCompletions)
        case .item: type("\\item ")
        case .tab: pressTab()
        case .undo: textView.undoManager?.undo()
        case .redo: textView.undoManager?.redo()
        }
    }

    private func insertSnippet(_ snippet: LaTeXSnippet?) {
        guard let snippet else { return }
        let sel = textView.selectedRange
        accept(LocalCompletion.Suggestion(kind: .command, text: snippet.text, detail: "", replaceStart: 0, replaceEnd: 0, snippet: snippet), replacing: sel)
    }
}
