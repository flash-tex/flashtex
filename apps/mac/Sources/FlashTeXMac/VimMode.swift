import AppKit
import Observation

/// Vim keybinding emulation for the source editor: a modal state machine the
/// editor text view (`CompletingTextView`, Completion.swift) consults from
/// `keyDown` when the "Vim keybindings" preference (EditorPreferences.swift,
/// default off) is on. Off, nothing here runs and the editor keys are exactly
/// what they were.
///
/// Boundaries, on purpose:
/// - Insert mode intercepts only Esc / ⌃[ (and only when no marked text
///   exists); every other key takes the editor's normal path, so input
///   methods, dead keys, completion, snippets and signature help are untouched.
/// - ⌘-shortcuts are never intercepted in any mode (⌘Z, ⌘S, ⌘F keep working).
/// - Every buffer change goes through `insertText(_:replacementRange:)`, the
///   same reviewed edit path as typing (delegate `shouldChangeText`,
///   `didChangeText`, the owner's `textDidChange`), so the model, undo
///   manager and diagnostics see Vim edits as ordinary edits. Undo/redo (`u`,
///   ⌃R) are the editor's own undo manager; an insert session is one step
///   because coalescing is broken at its boundaries.
/// - `:` commands that need the app (`:w`, `:q`, `:e`, `:set nu`) are handed
///   to `exCommandHandler` (wired by the owner to `ShellModel`); `:%s` and
///   `:noh` act on the buffer directly.
///
/// Positions are UTF-16 offsets into the text view's storage, like every
/// other editor helper. The caret in normal mode sits *on* a character
/// (offset = that character's index), so `x` deletes `[caret, caret+1)` and
/// `$` lands on the last character of the line, not after it.
@MainActor
final class VimMode {
    enum Mode: Equatable {
        case normal, insert, visual, visualLine

        var indicator: String {
            switch self {
            case .normal: "-- NORMAL --"
            case .insert: "-- INSERT --"
            case .visual: "-- VISUAL --"
            case .visualLine: "-- VISUAL LINE --"
            }
        }
    }

    /// One keystroke as the state machine sees it (built from an `NSEvent`
    /// by `Key.init(event:)`, or directly by tests and `.` repeat).
    struct Key: Equatable {
        var char: Character?
        var control = false
        var escape = false
        var `return` = false
        var backspace = false

        static func c(_ ch: Character) -> Key { Key(char: ch) }
        static func ctrl(_ ch: Character) -> Key { Key(char: ch, control: true) }
        static let esc = Key(escape: true)
        static let enter = Key(return: true)
        static let delete = Key(backspace: true)

        init(char: Character? = nil, control: Bool = false, escape: Bool = false, return: Bool = false, backspace: Bool = false) {
            self.char = char; self.control = control; self.escape = escape; self.return = `return`; self.backspace = backspace
        }

        /// nil for events Vim never handles (⌘/⌥ shortcuts, function keys, arrows).
        init?(event: NSEvent) {
            let mods = event.modifierFlags.intersection([.command, .option, .control, .shift])
            if mods.contains(.command) || mods.contains(.option) { return nil }
            switch event.keyCode {
            case 53: self = .esc; return
            case 36, 76: self = .enter; return
            case 51: self = .delete; return
            case 123, 124, 125, 126, 115, 119, 116, 121, 48: return nil // arrows, Home/End, Page, Tab: the editor's own
            default: break
            }
            if mods.contains(.control) {
                guard let ch = event.charactersIgnoringModifiers?.first else { return nil }
                if ch == "[" { self = .esc; return }
                self.init(char: Character(ch.lowercased()), control: true)
                return
            }
            guard let ch = event.characters?.first, !ch.isNewline else { return nil }
            self.init(char: ch)
        }
    }

    /// Commands that need the application (the owner wires them to the shell
    /// model). The handler returns a status message, or nil when it acted silently.
    enum ExCommand: Equatable {
        case write
        case quit(force: Bool)
        case writeQuit
        case edit(String)
        case setNumber(Bool)
    }

    enum Operator: Equatable { case delete, change, yank, indent, outdent }

    private enum Pending: Equatable {
        case none
        case find(forward: Bool, till: Bool)
        case g
        case replaceChar
        case mark
        case jumpMark(exact: Bool)
        case textObject(inner: Bool)
        case register
        case z
    }

    struct Register: Equatable {
        var text: String
        var linewise: Bool
    }

    /// What the status bar shows (`VimModeStatusItem`, ContentView.swift).
    @Observable
    final class Status {
        static let shared = Status()
        /// nil while Vim keybindings are off.
        var indicator: String?
        /// The command line being typed (`:wq`, `/pattern`), or a message.
        var commandLine: String?
    }

    weak var textView: NSTextView?
    var exCommandHandler: ((ExCommand) -> String?)?
    /// Called after every handled key (mode changes, caret shape, status).
    var onStateChange: (() -> Void)?

    private(set) var mode: Mode = .normal
    private var count: Int?
    private var pendingOperator: Operator?
    private var operatorCount: Int?
    private var pending: Pending = .none
    private var register: Register?
    private var namedRegisters: [Character: Register] = [:]
    private var selectedRegister: Character?
    private var marks: [Character: Int] = [:]
    private var lastFind: (char: Character, forward: Bool, till: Bool)?
    private(set) var lastSearch: (pattern: String, forward: Bool)?
    private var visualAnchor = 0
    private var preferredColumn: Int?
    /// Column kept across `gj`/`gk`, counted from the start of the *visual*
    /// row rather than the logical line. Independent of `preferredColumn`:
    /// mixing `j` and `gj` must not make either drift.
    private var preferredVisualColumn: Int?
    private var insertStart: Int?
    private var replayingDot = false
    private var recording: [Key] = []
    private var recordingChange = false
    private var lastChange: (keys: [Key], insertedText: String)?
    private var lastInsertedText = ""
    /// The command line being edited (`:`/`/`/`?` prefix + text) and the caret before it started.
    private var commandLine: (prefix: Character, text: String, origin: Int)?
    private(set) var message: String?
    /// > 0 while a key is being handled: selection changes then are Vim's own.
    private var applying = 0

    init(textView: NSTextView) { self.textView = textView }

    var wantsBlockCaret: Bool { mode != .insert }
    var isCommandLineActive: Bool { commandLine != nil }

    /// The text of the unnamed register (what `p` would paste).
    var unnamedRegister: Register? { register }

    // MARK: entry points

    /// Handles one key. Returns false when the key is not Vim's (insert-mode
    /// typing, ⌘-shortcuts, arrows): the caller passes it to the editor.
    @discardableResult
    func handle(_ key: Key) -> Bool {
        guard let tv = textView else { return false }
        if tv.hasMarkedText() { return false } // an input-method composition owns the keys
        applying += 1
        defer { applying -= 1 }
        if commandLine != nil { handleCommandLineKey(key); publish(); return true }
        let handled: Bool
        switch mode {
        case .insert: handled = handleInsertKey(key)
        case .normal: handled = handleNormalKey(key)
        case .visual, .visualLine: handled = handleVisualKey(key)
        }
        if handled { publish() }
        return handled
    }

    /// The preference was switched (or the view was made): reset to normal mode.
    func activate() {
        applying += 1
        defer { applying -= 1 }
        mode = .normal
        resetPending()
        commandLine = nil
        if let tv = textView, tv.selectedRange().length == 0 { clampNormalCaret() }
        publish()
    }

    func deactivate() {
        Status.shared.indicator = nil
        Status.shared.commandLine = nil
        onStateChange?()
    }

    /// A selection made by the mouse (or a navigation command) while in normal
    /// mode enters visual mode, like `selectmode=mouse`; an empty selection
    /// in visual mode leaves it.
    func selectionDidChange(_ range: NSRange) {
        guard applying == 0, commandLine == nil else { return }
        switch mode {
        case .normal where range.length > 0:
            visualAnchor = range.location
            mode = .visual
            publish()
        case .visual, .visualLine:
            if range.length == 0 { mode = .normal; publish() }
        default: break
        }
    }

    private func publish() {
        Status.shared.indicator = mode.indicator + pendingIndicator
        if let commandLine {
            Status.shared.commandLine = String(commandLine.prefix) + commandLine.text
        } else {
            Status.shared.commandLine = message
        }
        onStateChange?()
    }

    private var pendingIndicator: String {
        var s = ""
        if let c = count { s += " \(c)" }
        if let op = pendingOperator {
            switch op {
            case .delete: s += " d"
            case .change: s += " c"
            case .yank: s += " y"
            case .indent: s += " >"
            case .outdent: s += " <"
            }
        }
        return s
    }

    private func resetPending() {
        count = nil
        pendingOperator = nil
        operatorCount = nil
        pending = .none
        selectedRegister = nil
    }

    // MARK: text access

    private var text: NSString { (textView?.string ?? "") as NSString }
    private var length: Int { text.length }
    private var caret: Int { textView?.selectedRange().location ?? 0 }

    private func char(at i: Int) -> unichar? {
        guard i >= 0, i < length else { return nil }
        return text.character(at: i)
    }

    private func lineStart(_ p: Int) -> Int {
        var i = min(max(p, 0), length)
        while i > 0, text.character(at: i - 1) != 0x0A { i -= 1 }
        return i
    }

    /// Index of the line's `\n` (or `length` on the last line).
    private func lineEnd(_ p: Int) -> Int {
        var i = min(max(p, 0), length)
        while i < length, text.character(at: i) != 0x0A { i += 1 }
        return i
    }

    private func firstNonBlank(fromLineStart s: Int) -> Int {
        var i = s
        while i < length, let c = char(at: i), c == 0x20 || c == 0x09 { i += 1 }
        return i
    }

    private func lineNumber(of p: Int) -> Int {
        var n = 0, i = 0
        let stop = min(p, length)
        while i < stop { if text.character(at: i) == 0x0A { n += 1 }; i += 1 }
        return n
    }

    private func lineStart(ofLine n: Int) -> Int {
        var i = 0, line = 0
        while line < n, i < length {
            if text.character(at: i) == 0x0A { line += 1 }
            i += 1
        }
        return i
    }

    private var lineCount: Int { lineNumber(of: length) + 1 }

    /// Full-line range of every line touched by `range`, the trailing newline included.
    private func lineRange(_ range: NSRange, includeTrailingNewline: Bool) -> NSRange {
        let start = lineStart(range.location)
        var end = lineEnd(max(range.location, NSMaxRange(range) - (range.length > 0 ? 1 : 0)))
        if includeTrailingNewline {
            if end < length { end += 1 } else if start > 0 { return NSRange(location: start - 1, length: end - start + 1) } // last line: eat the newline before it
        }
        return NSRange(location: start, length: end - start)
    }

    private func setCaret(_ p: Int) {
        let clamped = min(max(p, 0), length)
        textView?.setSelectedRange(NSRange(location: clamped, length: 0))
        textView?.scrollRangeToVisible(NSRange(location: clamped, length: 0))
    }

    /// Normal mode never rests past the last character of a non-empty line.
    private func clampNormalCaret() {
        let c = caret
        if c > 0, c == lineEnd(c), c > lineStart(c) { setCaret(c - 1) }
        else if c > length { setCaret(length) }
    }

    // MARK: edits (all through the reviewed edit path)

    private func replace(_ range: NSRange, with s: String, actionName: String) {
        guard let tv = textView else { return }
        tv.breakUndoCoalescing()
        tv.insertText(s, replacementRange: range)
        tv.undoManager?.setActionName(actionName)
        tv.breakUndoCoalescing()
    }

    private func store(_ range: NSRange, linewise: Bool) {
        let s = text.substring(with: range)
        let r = Register(text: s, linewise: linewise)
        register = r
        if let reg = selectedRegister {
            if reg == "+" || reg == "*" {
                let pb = NSPasteboard.general
                pb.clearContents()
                pb.setString(s, forType: .string)
            } else {
                namedRegisters[reg] = r
            }
        }
    }

    private func registerForPaste() -> Register? {
        guard let reg = selectedRegister else { return register }
        if reg == "+" || reg == "*" {
            guard let s = NSPasteboard.general.string(forType: .string) else { return nil }
            return Register(text: s, linewise: s.hasSuffix("\n"))
        }
        return namedRegisters[reg]
    }

    // MARK: insert mode

    private func handleInsertKey(_ key: Key) -> Bool {
        guard key.escape else { return false }
        leaveInsert()
        return true
    }

    private func enterInsert(at p: Int? = nil) {
        if let p { setCaret(p) }
        textView?.breakUndoCoalescing()
        insertStart = caret
        mode = .insert
    }

    private func leaveInsert() {
        guard let tv = textView else { return }
        (tv as? CompletingTextView)?.dismissCompletionForVim()
        tv.breakUndoCoalescing()
        if let start = insertStart, caret >= start, start <= length, caret <= length {
            lastInsertedText = text.substring(with: NSRange(location: start, length: caret - start))
        } else {
            lastInsertedText = ""
        }
        insertStart = nil
        mode = .normal
        if recordingChange { finishRecording() }
        let c = caret
        if c > lineStart(c) { setCaret(c - 1) }
        preferredColumn = nil
    }

    // MARK: `.` repeat

    private func beginRecording(_ key: Key) {
        guard !replayingDot else { return }
        recording = [key]
        recordingChange = true
    }

    private func record(_ key: Key) {
        guard recordingChange, !replayingDot else { return }
        recording.append(key)
    }

    private func finishRecording() {
        guard recordingChange, !replayingDot else { recordingChange = false; return }
        lastChange = (recording, mode == .insert ? "" : lastInsertedText)
        recordingChange = false
        recording = []
    }

    private func repeatLastChange() {
        guard let change = lastChange else { return }
        replayingDot = true
        defer { replayingDot = false }
        for key in change.keys { _ = handle(key) }
        if mode == .insert {
            if !change.insertedText.isEmpty { textView?.insertText(change.insertedText, replacementRange: textView!.selectedRange()) }
            leaveInsert()
        }
    }

    // MARK: normal mode

    private func handleNormalKey(_ key: Key) -> Bool {
        if key.escape { resetPending(); message = nil; clampNormalCaret(); return true }
        if pending != .none { return handlePendingKey(key) }
        guard let ch = key.char else { return false }

        // Counts (a leading 0 is the motion).
        if !key.control, let d = ch.wholeNumberValue, ch.isASCII, d != 0 || count != nil {
            count = (count ?? 0) * 10 + d
            if recordingChange { record(key) }
            return true
        }
        let n = count
        count = nil

        if key.control {
            switch ch {
            case "d": moveLines(by: pageLines() / 2 * (n ?? 1)); return true
            case "u": moveLines(by: -(pageLines() / 2) * (n ?? 1)); return true
            case "f": moveLines(by: pageLines() * (n ?? 1)); return true
            case "b": moveLines(by: -pageLines() * (n ?? 1)); return true
            case "r": textView?.undoManager?.redo(); clampNormalCaret(); return true
            default: return false
            }
        }

        if let op = pendingOperator {
            return handleOperatorTarget(ch, key: key, count: n, op: op)
        }

        switch ch {
        case "i": beginRecording(key); enterInsert()
        case "a": beginRecording(key); enterInsert(at: caret < lineEnd(caret) ? caret + 1 : caret)
        case "I": beginRecording(key); enterInsert(at: firstNonBlank(fromLineStart: lineStart(caret)))
        case "A": beginRecording(key); enterInsert(at: lineEnd(caret))
        case "o": beginRecording(key); openLine(below: true)
        case "O": beginRecording(key); openLine(below: false)
        case "v": mode = .visual; visualAnchor = caret; textView?.setSelectedRange(NSRange(location: caret, length: min(1, length - caret)))
        case "V": mode = .visualLine; visualAnchor = caret; updateVisualSelection()
        case "d", "c", "y", ">", "<":
            beginRecording(key)
            pendingOperator = operatorFor(ch)
            operatorCount = n
        case "x": beginRecording(key); deleteChars(count: n ?? 1, forward: true); finishRecording()
        case "X": beginRecording(key); deleteChars(count: n ?? 1, forward: false); finishRecording()
        case "s": beginRecording(key); deleteChars(count: n ?? 1, forward: true, thenInsert: true)
        case "S": beginRecording(key); changeLines(count: n ?? 1)
        case "D": beginRecording(key); finishOperator(.delete, motion: .toLineEnd(count: n ?? 1))
        case "C": beginRecording(key); finishOperator(.change, motion: .toLineEnd(count: n ?? 1))
        case "Y": yankLines(count: n ?? 1)
        case "p": beginRecording(key); paste(after: true, count: n ?? 1); finishRecording()
        case "P": beginRecording(key); paste(after: false, count: n ?? 1); finishRecording()
        case "J": beginRecording(key); joinLines(count: max(2, n ?? 2)); finishRecording()
        case "u": textView?.undoManager?.undo(); clampNormalCaret()
        case ".": repeatLastChange()
        case "~": beginRecording(key); toggleCase(count: n ?? 1); finishRecording()
        case "r": beginRecording(key); pending = .replaceChar; count = n
        case "m": pending = .mark
        case "'": pending = .jumpMark(exact: false)
        case "`": pending = .jumpMark(exact: true)
        case "\"": pending = .register
        case "g": pending = .g; count = n
        case "z": pending = .z
        case "f", "F", "t", "T": pending = .find(forward: ch == "f" || ch == "t", till: ch == "t" || ch == "T"); count = n
        case ":": commandLine = (":", "", caret)
        case "/", "?": commandLine = (ch, "", caret)
        case "n": searchAgain(reverse: false, count: n ?? 1)
        case "N": searchAgain(reverse: true, count: n ?? 1)
        case "*": searchWordUnderCaret(count: n ?? 1)
        default:
            guard let m = motion(for: ch, count: n) else { return false }
            move(m)
        }
        return true
    }

    private func operatorFor(_ ch: Character) -> Operator {
        switch ch {
        case "d": .delete
        case "c": .change
        case "y": .yank
        case ">": .indent
        default: .outdent
        }
    }

    private func handlePendingKey(_ key: Key) -> Bool {
        let p = pending
        pending = .none
        let n = count
        count = nil
        guard let ch = key.char, !key.control else { resetPending(); return true }
        switch p {
        case .none: return false
        case .find(let forward, let till):
            lastFind = (ch, forward, till)
            let m = Motion.findChar(ch, forward: forward, till: till, count: n ?? 1)
            if let op = pendingOperator { finishOperator(op, motion: m) } else { move(m) }
        case .g:
            switch ch {
            case "g":
                let m = Motion.line(number: (n ?? 1) - 1)
                if let op = pendingOperator { finishOperator(op, motion: m) } else { move(m) }
            // `gj`/`gk` and `g0`/`g^`/`g$`: the visual row, which is the only
            // way to move by what you see once a paragraph wraps — and line
            // wrapping is on by default, so plain `j` skips whole paragraphs.
            case "j", "k", "0", "^", "$":
                let c = n ?? 1
                let m: Motion = switch ch {
                case "j": .visualDown(count: c)
                case "k": .visualUp(count: c)
                case "0": .rowStart
                case "^": .rowFirstNonBlank
                default: .rowEnd
                }
                if let op = pendingOperator { finishOperator(op, motion: m) } else { move(m) }
            case "J": beginRecording(.c("g")); record(key); joinLines(count: max(2, n ?? 2), spaces: false); finishRecording()
            case "v": mode = .visual; textView?.setSelectedRange(NSRange(location: caret, length: min(1, length - caret)))
            case "u", "U", "~" where pendingOperator == nil: break // gu/gU need an operator target: not supported yet
            default: resetPending()
            }
        case .replaceChar:
            let c = n ?? 1
            guard caret + c <= lineEnd(caret) else { resetPending(); return true }
            record(key)
            replace(NSRange(location: caret, length: c), with: String(repeating: ch, count: c), actionName: "Replace Character")
            setCaret(caret - 1)
            finishRecording()
        case .mark:
            if ch.isLetter { marks[ch] = caret }
        case .jumpMark(let exact):
            guard let target = marks[ch], target <= length else { message = "E20: Mark not set"; return true }
            let m: Motion = exact ? .absolute(target, linewise: false) : .absolute(firstNonBlank(fromLineStart: lineStart(target)), linewise: true)
            if let op = pendingOperator { finishOperator(op, motion: m) } else { move(m) }
        case .textObject(let inner):
            guard let op = pendingOperator, let range = textObject(ch, inner: inner) else { resetPending(); return true }
            record(key)
            finishOperator(op, range: range, linewise: false)
        case .register:
            selectedRegister = ch
        case .z:
            guard let tv = textView else { return true }
            switch ch {
            case "z", ".": tv.centerSelectionInVisibleArea(nil)
            case "t", "\n": tv.scrollRangeToVisible(NSRange(location: caret, length: 0))
            default: break
            }
        }
        return true
    }

    /// The key after `d`/`c`/`y`/`>`/`<`: a motion, a doubled operator (line), or `i`/`a` for a text object.
    private func handleOperatorTarget(_ ch: Character, key: Key, count n: Int?, op: Operator) -> Bool {
        record(key)
        let total = (operatorCount ?? 1) * (n ?? 1)
        let doubled: Character = switch op { case .delete: "d"; case .change: "c"; case .yank: "y"; case .indent: ">"; case .outdent: "<" }
        if ch == doubled {
            operateOnLines(op, count: total)
            return true
        }
        switch ch {
        case "i", "a": pending = .textObject(inner: ch == "i"); count = n; return true
        case "f", "F", "t", "T": pending = .find(forward: ch == "f" || ch == "t", till: ch == "t" || ch == "T"); count = total; return true
        case "g": pending = .g; count = n; return true
        case "'", "`": pending = .jumpMark(exact: ch == "`"); return true
        case "w" where op == .change:
            // `cw` behaves like `ce` on a non-blank character (Vim special case).
            if let c = char(at: caret), !isBlank(c) { finishOperator(op, motion: .wordEnd(count: total, big: false)); return true }
            fallthrough
        default:
            guard let m = motion(for: ch, count: total) else { resetPending(); return true }
            finishOperator(op, motion: m)
            return true
        }
    }

    private func operateOnLines(_ op: Operator, count: Int) {
        let start = lineStart(caret)
        var end = start
        for _ in 0..<count { end = lineEnd(end); if end < length { end += 1 } }
        switch op {
        case .delete:
            store(NSRange(location: start, length: end - start), linewise: true)
            // The last line has no trailing newline: take the one before it instead.
            let r = end == length && start > 0 ? NSRange(location: start - 1, length: end - start + 1) : NSRange(location: start, length: end - start)
            replace(r, with: "", actionName: "Delete Line")
            setCaret(firstNonBlank(fromLineStart: lineStart(min(start, length))))
            finishRecording()
        case .change:
            changeLines(count: count)
        case .yank:
            store(NSRange(location: start, length: end - start), linewise: true)
            resetPending()
            finishRecording()
        case .indent, .outdent:
            shiftLines(NSRange(location: start, length: max(0, end - start - (end < length ? 1 : 0))), outdent: op == .outdent)
            finishRecording()
        }
        pendingOperator = nil
        operatorCount = nil
    }

    private func finishOperator(_ op: Operator, motion: Motion) {
        guard let target = resolve(motion) else { resetPending(); return }
        let from = caret
        var lo = min(from, target.position), hi = max(from, target.position)
        if target.linewise {
            // Whole lines from the caret's to the target's; a delete takes a
            // newline with them (the one before on the last line), a yank keeps
            // the trailing newline in the register, indent/change keep the lines.
            lo = lineStart(lo)
            hi = lineEnd(hi)
            switch op {
            case .delete:
                if hi < length { hi += 1 } else if lo > 0 { lo -= 1 }
            case .yank:
                if hi < length { hi += 1 }
            case .change:
                lo = firstNonBlank(fromLineStart: lo)
            case .indent, .outdent: break
            }
        } else if target.inclusive, hi < length {
            hi += 1
        }
        finishOperator(op, range: NSRange(location: lo, length: hi - lo), linewise: target.linewise)
    }

    private func finishOperator(_ op: Operator, range: NSRange, linewise: Bool) {
        pendingOperator = nil
        operatorCount = nil
        apply(op, to: range, linewise: linewise)
        if op != .change { finishRecording() }
    }

    private func apply(_ op: Operator, to range: NSRange, linewise: Bool = false) {
        switch op {
        case .delete:
            store(range, linewise: linewise)
            replace(range, with: "", actionName: "Delete")
            setCaret(range.location)
            if linewise { setCaret(firstNonBlank(fromLineStart: lineStart(min(range.location, length)))) } else { clampNormalCaret() }
        case .change:
            store(range, linewise: linewise)
            replace(range, with: "", actionName: "Change")
            enterInsert(at: range.location)
        case .yank:
            store(range, linewise: linewise)
            setCaret(range.location)
            clampNormalCaret()
        case .indent, .outdent:
            shiftLines(range, outdent: op == .outdent)
        }
    }

    private func shiftLines(_ range: NSRange, outdent: Bool) {
        guard let tv = textView else { return }
        let unit = EditorPreferences.shared.indentString
        let s = tv.string
        let plan = outdent ? EditorKeyHandling.outdentEdits(in: s, range: range, unit: unit) : EditorKeyHandling.indentEdits(in: s, range: range, unit: unit)
        guard let (edits, _) = plan, !edits.isEmpty else { return }
        let start = lineStart(range.location)
        tv.breakUndoCoalescing()
        tv.undoManager?.beginUndoGrouping()
        for e in edits.sorted(by: { $0.range.location > $1.range.location }) {
            tv.insertText(e.replacement, replacementRange: e.range)
        }
        tv.undoManager?.setActionName(outdent ? "Outdent" : "Indent")
        tv.undoManager?.endUndoGrouping()
        tv.breakUndoCoalescing()
        setCaret(firstNonBlank(fromLineStart: start))
    }

    private func deleteChars(count: Int, forward: Bool, thenInsert: Bool = false) {
        let c = caret
        let range: NSRange
        if forward {
            let end = min(lineEnd(c), c + count)
            guard end > c else { if thenInsert { enterInsert() }; return }
            range = NSRange(location: c, length: end - c)
        } else {
            let start = max(lineStart(c), c - count)
            guard start < c else { return }
            range = NSRange(location: start, length: c - start)
        }
        store(range, linewise: false)
        replace(range, with: "", actionName: "Delete")
        setCaret(range.location)
        if thenInsert { enterInsert() } else { clampNormalCaret() }
    }

    private func changeLines(count: Int) {
        let start = lineStart(caret)
        var end = start
        for i in 0..<count { if i > 0, end < length { end += 1 }; end = lineEnd(end) }
        let indent = firstNonBlank(fromLineStart: start)
        store(NSRange(location: start, length: min(length, end + 1) - start), linewise: true)
        replace(NSRange(location: indent, length: end - indent), with: "", actionName: "Change Line")
        pendingOperator = nil
        enterInsert(at: indent)
    }

    private func yankLines(count: Int) {
        let start = lineStart(caret)
        var end = start
        for _ in 0..<count { end = lineEnd(end); if end < length { end += 1 } }
        store(NSRange(location: start, length: end - start), linewise: true)
    }

    private func openLine(below: Bool) {
        let c = caret
        let indent = text.substring(with: NSRange(location: lineStart(c), length: firstNonBlank(fromLineStart: lineStart(c)) - lineStart(c)))
        if below {
            let end = lineEnd(c)
            replace(NSRange(location: end, length: 0), with: "\n" + indent, actionName: "Open Line")
            enterInsert(at: end + 1 + (indent as NSString).length)
        } else {
            let start = lineStart(c)
            replace(NSRange(location: start, length: 0), with: indent + "\n", actionName: "Open Line")
            enterInsert(at: start + (indent as NSString).length)
        }
    }

    private func paste(after: Bool, count: Int) {
        guard let reg = registerForPaste(), !reg.text.isEmpty else { return }
        let body = String(repeating: reg.text, count: count)
        if reg.linewise {
            var s = body
            if !s.hasSuffix("\n") { s += "\n" }
            if after {
                let end = lineEnd(caret)
                if end == length { // last line: the pasted block goes on a new line below it
                    let trimmed = String(s.dropLast())
                    replace(NSRange(location: end, length: 0), with: "\n" + trimmed, actionName: "Paste")
                    setCaret(firstNonBlank(fromLineStart: end + 1))
                } else {
                    replace(NSRange(location: end + 1, length: 0), with: s, actionName: "Paste")
                    setCaret(firstNonBlank(fromLineStart: end + 1))
                }
            } else {
                let start = lineStart(caret)
                replace(NSRange(location: start, length: 0), with: s, actionName: "Paste")
                setCaret(firstNonBlank(fromLineStart: start))
            }
        } else {
            let at = after && caret < lineEnd(caret) ? caret + 1 : caret
            replace(NSRange(location: at, length: 0), with: body, actionName: "Paste")
            setCaret(at + (body as NSString).length - 1)
        }
    }

    private func joinLines(count: Int, spaces: Bool = true) {
        guard let tv = textView else { return }
        let start = lineStart(caret)
        tv.breakUndoCoalescing()
        tv.undoManager?.beginUndoGrouping()
        var joinAt = start
        var lastJoin: Int?
        for _ in 1..<count {
            let end = lineEnd(joinAt)
            guard end < length else { break }
            var next = end + 1
            var replacement = ""
            if spaces {
                while next < length, let c = char(at: next), c == 0x20 || c == 0x09 { next += 1 }
                let endsWithSpace = end > start && (char(at: end - 1) == 0x20 || char(at: end - 1) == 0x09)
                let nextIsCloser = char(at: next) == 0x29 // `)`
                replacement = end == lineStart(end) || endsWithSpace || nextIsCloser || next >= length ? "" : " "
            }
            tv.insertText(replacement, replacementRange: NSRange(location: end, length: next - end))
            lastJoin = end
            joinAt = end
        }
        tv.undoManager?.setActionName("Join Lines")
        tv.undoManager?.endUndoGrouping()
        tv.breakUndoCoalescing()
        if let lastJoin { setCaret(lastJoin) }
        clampNormalCaret()
    }

    private func toggleCase(count: Int) {
        let c = caret
        let end = min(lineEnd(c), c + count)
        guard end > c else { return }
        let range = NSRange(location: c, length: end - c)
        let toggled = String(text.substring(with: range).map { ch -> Character in
            ch.isUppercase ? Character(ch.lowercased()) : Character(ch.uppercased())
        })
        replace(range, with: toggled, actionName: "Toggle Case")
        setCaret(end)
        clampNormalCaret()
    }

    // MARK: visual mode

    private func handleVisualKey(_ key: Key) -> Bool {
        if key.escape { leaveVisual(); return true }
        if pending != .none {
            if case .textObject(let inner) = pending, let ch = key.char, let r = textObject(ch, inner: inner) {
                pending = .none
                visualAnchor = r.location
                mode = .visual
                setSelection(NSRange(location: r.location, length: r.length))
                return true
            }
            return handlePendingKey(key)
        }
        guard let ch = key.char else { return false }
        if !key.control, let d = ch.wholeNumberValue, ch.isASCII, d != 0 || count != nil { count = (count ?? 0) * 10 + d; return true }
        let n = count
        count = nil
        if key.control {
            switch ch {
            case "d": moveLines(by: pageLines() / 2); return true
            case "u": moveLines(by: -(pageLines() / 2)); return true
            case "f": moveLines(by: pageLines()); return true
            case "b": moveLines(by: -pageLines()); return true
            default: return false
            }
        }
        let sel = textView?.selectedRange() ?? NSRange(location: caret, length: 0)
        let linewise = mode == .visualLine
        switch ch {
        case "v": if mode == .visual { leaveVisual() } else { mode = .visual; updateVisualSelection() }
        case "V": if mode == .visualLine { leaveVisual() } else { mode = .visualLine; updateVisualSelection() }
        case "o": let head = visualHead; visualAnchor = head; setCaretKeepingVisual(sel.location == head ? NSMaxRange(sel) - 1 : sel.location)
        case "d", "x": beginRecording(key); mode = .normal; apply(.delete, to: sel, linewise: linewise); finishRecording()
        case "c", "s": beginRecording(key); mode = .normal; apply(.change, to: sel, linewise: linewise)
        case "y": mode = .normal; apply(.yank, to: sel, linewise: linewise)
        case ">": beginRecording(key); mode = .normal; shiftLines(sel, outdent: false); finishRecording()
        case "<": beginRecording(key); mode = .normal; shiftLines(sel, outdent: true); finishRecording()
        case "~":
            beginRecording(key); mode = .normal
            let toggled = String(text.substring(with: sel).map { c -> Character in c.isUppercase ? Character(c.lowercased()) : Character(c.uppercased()) })
            replace(sel, with: toggled, actionName: "Toggle Case"); setCaret(sel.location); finishRecording()
        case "J": beginRecording(key); mode = .normal; setCaret(sel.location); joinLines(count: max(2, lineNumber(of: NSMaxRange(sel) - 1) - lineNumber(of: sel.location) + 1)); finishRecording()
        case "p", "P":
            beginRecording(key); mode = .normal
            let reg = registerForPaste()
            store(sel, linewise: linewise)
            if let reg { replace(sel, with: reg.text, actionName: "Paste"); setCaret(sel.location + (reg.text as NSString).length - 1) }
            clampNormalCaret(); finishRecording()
        case "i", "a": pending = .textObject(inner: ch == "i")
        case "f", "F", "t", "T": pending = .find(forward: ch == "f" || ch == "t", till: ch == "t" || ch == "T"); count = n
        case "g": pending = .g; count = n
        case "'", "`": pending = .jumpMark(exact: ch == "`")
        case "\"": pending = .register
        case "m": pending = .mark
        case ":": commandLine = (":", "'<,'>", sel.location)
        case "/", "?": commandLine = (ch, "", caret)
        case "n": searchAgain(reverse: false, count: n ?? 1)
        case "N": searchAgain(reverse: true, count: n ?? 1)
        case "*": searchWordUnderCaret(count: n ?? 1)
        default:
            guard let m = motion(for: ch, count: n) else { return false }
            move(m)
        }
        return true
    }

    /// The moving end of the visual selection (the caret in Vim terms).
    private var visualHead: Int {
        let sel = textView?.selectedRange() ?? NSRange(location: 0, length: 0)
        if sel.length == 0 { return sel.location }
        return sel.location == visualAnchor ? max(sel.location, NSMaxRange(sel) - 1) : sel.location
    }

    private func setSelection(_ r: NSRange) {
        textView?.setSelectedRange(r)
        textView?.scrollRangeToVisible(NSRange(location: r.location, length: 0))
    }

    private func setCaretKeepingVisual(_ p: Int) {
        let head = min(max(p, 0), max(0, length))
        let lo = min(visualAnchor, head), hi = max(visualAnchor, head)
        if mode == .visualLine {
            setSelection(lineRange(NSRange(location: lo, length: hi - lo + 1), includeTrailingNewline: true))
        } else {
            setSelection(NSRange(location: lo, length: min(length, hi + 1) - lo))
        }
    }

    private func updateVisualSelection() { setCaretKeepingVisual(visualHead) }

    private func leaveVisual() {
        let head = visualHead
        mode = .normal
        resetPending()
        setCaret(head)
        clampNormalCaret()
    }

    // MARK: motions

    enum Motion: Equatable {
        case left(count: Int), right(count: Int), up(count: Int), down(count: Int)
        /// `gj`/`gk`: one *visual* row (line fragment), which is what the user
        /// sees when a paragraph wraps. Exclusive, like Vim's.
        case visualDown(count: Int), visualUp(count: Int)
        /// `g0`/`g^`/`g$`: the ends of the visual row.
        case rowStart, rowFirstNonBlank, rowEnd
        case wordStart(count: Int, big: Bool), wordEnd(count: Int, big: Bool), wordBack(count: Int, big: Bool)
        case lineStart, firstNonBlank, lineEnd(count: Int), toLineEnd(count: Int)
        case line(number: Int), lastLine
        case paragraphForward(count: Int), paragraphBack(count: Int)
        case sentenceForward(count: Int), sentenceBack(count: Int)
        case findChar(Character, forward: Bool, till: Bool, count: Int)
        case repeatFind(count: Int, reverse: Bool)
        case matchPair
        case screenTop, screenMiddle, screenBottom
        case absolute(Int, linewise: Bool)
    }

    struct Target: Equatable {
        var position: Int
        var linewise = false
        var inclusive = false
    }

    private func motion(for ch: Character, count n: Int?) -> Motion? {
        let c = n ?? 1
        switch ch {
        case "h": return .left(count: c)
        case "l", " ": return .right(count: c)
        case "j": return .down(count: c)
        case "k": return .up(count: c)
        case "w": return .wordStart(count: c, big: false)
        case "W": return .wordStart(count: c, big: true)
        case "e": return .wordEnd(count: c, big: false)
        case "E": return .wordEnd(count: c, big: true)
        case "b": return .wordBack(count: c, big: false)
        case "B": return .wordBack(count: c, big: true)
        case "0": return .lineStart
        case "^": return .firstNonBlank
        case "$": return .lineEnd(count: c)
        case "G": return n.map { .line(number: $0 - 1) } ?? .lastLine
        case "{": return .paragraphBack(count: c)
        case "}": return .paragraphForward(count: c)
        case "(": return .sentenceBack(count: c)
        case ")": return .sentenceForward(count: c)
        case ";": return .repeatFind(count: c, reverse: false)
        case ",": return .repeatFind(count: c, reverse: true)
        case "%": return .matchPair
        case "H": return .screenTop
        case "M": return .screenMiddle
        case "L": return .screenBottom
        default: return nil
        }
    }

    private func move(_ m: Motion) {
        guard let t = resolve(m) else { return }
        switch m {
        case .up, .down: preferredVisualColumn = nil
        case .visualDown, .visualUp: preferredColumn = nil
        default: preferredColumn = nil; preferredVisualColumn = nil
        }
        if mode == .visual || mode == .visualLine {
            setCaretKeepingVisual(t.position)
        } else {
            setCaret(t.position)
            clampNormalCaret()
        }
    }

    private func moveLines(by delta: Int) {
        guard delta != 0 else { return }
        move(delta > 0 ? .down(count: delta) : .up(count: -delta))
    }

    private func pageLines() -> Int {
        guard let tv = textView, let lm = tv.layoutManager else { return 20 }
        let lineHeight = lm.defaultLineHeight(for: tv.font ?? NSFont.systemFont(ofSize: 13))
        let visible = tv.enclosingScrollView?.documentVisibleRect.height ?? tv.visibleRect.height
        return max(1, Int(visible / max(lineHeight, 1)))
    }

    /// Where a motion lands from the current caret (nil: cannot move).
    func resolve(_ m: Motion) -> Target? {
        let c = mode == .visual || mode == .visualLine ? visualHead : caret
        switch m {
        case .left(let n): return Target(position: max(lineStart(c), c - n))
        case .right(let n):
            let limit = mode == .insert ? lineEnd(c) : max(lineStart(c), lineEnd(c) - 1)
            return Target(position: min(limit, c + n))
        case .down(let n), .up(let n):
            let column = preferredColumn ?? (c - lineStart(c))
            preferredColumn = column
            let current = lineNumber(of: c)
            let target: Int
            if case .down = m { target = min(lineCount - 1, current + n) } else { target = max(0, current - n) }
            if target == current { return nil }
            let s = lineStart(ofLine: target)
            let e = lineEnd(s)
            let last = mode == .insert ? e : max(s, e - 1)
            var position = min(last, s + column)
            if let tv = textView as? CompletingTextView {
                position = tv.folds.adjustLinewise(from: c, to: position)
            }
            return Target(position: position, linewise: true)
        case .visualDown(let n), .visualUp(let n):
            var down = true
            if case .visualUp = m { down = false }
            guard var row = visualRow(containing: c) else { return nil }
            let column = preferredVisualColumn ?? (c - row.location)
            preferredVisualColumn = column
            var moved = 0
            for _ in 0..<n {
                guard let next = visualRow(adjacentTo: row, down: down) else { break }
                row = next
                moved += 1
            }
            guard moved > 0 else { return nil }
            return Target(position: min(lastCaretPosition(inRow: row), row.location + column))
        case .rowStart, .rowFirstNonBlank, .rowEnd:
            guard let row = visualRow(containing: c) else { return nil }
            switch m {
            case .rowEnd: return Target(position: lastCaretPosition(inRow: row), inclusive: true)
            case .rowFirstNonBlank:
                var i = row.location
                let last = lastCaretPosition(inRow: row)
                while i < last, isBlank(text.character(at: i)) { i += 1 }
                return Target(position: i)
            default: return Target(position: row.location)
            }
        case .wordStart(let n, let big):
            var p = c
            for _ in 0..<n { p = nextWordStart(from: p, big: big) }
            return Target(position: p)
        case .wordEnd(let n, let big):
            var p = c
            for _ in 0..<n { p = nextWordEnd(from: p, big: big) }
            return Target(position: p, inclusive: true)
        case .wordBack(let n, let big):
            var p = c
            for _ in 0..<n { p = previousWordStart(from: p, big: big) }
            return Target(position: p)
        case .lineStart: return Target(position: lineStart(c))
        case .firstNonBlank: return Target(position: firstNonBlank(fromLineStart: lineStart(c)))
        case .lineEnd(let n):
            var e = lineEnd(c)
            for _ in 1..<max(1, n) where e < length { e = lineEnd(e + 1) }
            return Target(position: mode == .insert ? e : max(lineStart(e), e - 1), inclusive: true)
        case .toLineEnd(let n):
            var e = lineEnd(c)
            for _ in 1..<max(1, n) where e < length { e = lineEnd(e + 1) }
            return Target(position: e)
        case .line(let number):
            let s = lineStart(ofLine: min(max(0, number), lineCount - 1))
            return Target(position: firstNonBlank(fromLineStart: s), linewise: true)
        case .lastLine:
            return Target(position: firstNonBlank(fromLineStart: lineStart(length)), linewise: true)
        case .paragraphForward(let n):
            var p = c
            for _ in 0..<n { p = nextParagraphBoundary(from: p) }
            return Target(position: p)
        case .paragraphBack(let n):
            var p = c
            for _ in 0..<n { p = previousParagraphBoundary(from: p) }
            return Target(position: p)
        case .sentenceForward(let n):
            var p = c
            for _ in 0..<n { p = nextSentenceStart(from: p) }
            return Target(position: p)
        case .sentenceBack(let n):
            var p = c
            for _ in 0..<n { p = previousSentenceStart(from: p) }
            return Target(position: p)
        case .findChar(let ch, let forward, let till, let n):
            guard let p = findChar(ch, from: c, forward: forward, till: till, count: n) else { return nil }
            return Target(position: p, inclusive: forward)
        case .repeatFind(let n, let reverse):
            guard let f = lastFind else { return nil }
            let forward = reverse ? !f.forward : f.forward
            guard let p = findChar(f.char, from: c, forward: forward, till: f.till, count: n, repeating: true) else { return nil }
            return Target(position: p, inclusive: forward)
        case .matchPair:
            guard let p = matchingPair(from: c) else { return nil }
            return Target(position: p, inclusive: true)
        case .screenTop, .screenMiddle, .screenBottom:
            guard let tv = textView else { return nil }
            let visible = tv.enclosingScrollView?.documentVisibleRect ?? tv.visibleRect
            let y: CGFloat = switch m {
            case .screenTop: visible.minY + 1
            case .screenMiddle: visible.midY
            default: visible.maxY - 1
            }
            let idx = tv.characterIndexForInsertion(at: NSPoint(x: visible.minX + tv.textContainerInset.width, y: y))
            return Target(position: firstNonBlank(fromLineStart: lineStart(min(idx, length))), linewise: true)
        case .absolute(let p, let linewise):
            return Target(position: min(max(0, p), length), linewise: linewise)
        }
    }

    // MARK: visual rows (line fragments)

    /// The character range of the line fragment `p` sits on — one *visual*
    /// row, which is a whole logical line when nothing wraps and a slice of
    /// one when it does. Nil when there is no laid-out text to ask (no text
    /// view, an empty buffer, TextKit 2 without a layout manager): every
    /// caller then treats the motion as "cannot move", never as a silent
    /// fall-back to logical lines, which would move the caret somewhere the
    /// user did not ask for.
    func visualRow(containing p: Int) -> NSRange? {
        guard let tv = textView, let lm = tv.layoutManager, tv.textContainer != nil, lm.numberOfGlyphs > 0 else { return nil }
        let clamped = max(0, min(p, length))
        // The empty line after a trailing newline has no glyphs at all, so the
        // layout manager would answer with the row above it. `j` reaches that
        // line, so the visual-row motions must see it as its own row.
        if clamped == length, length > 0, text.character(at: length - 1) == 0x0A { return NSRange(location: length, length: 0) }
        // A caret at the very end of the buffer has no glyph of its own.
        let glyph = min(lm.glyphIndexForCharacter(at: clamped), lm.numberOfGlyphs - 1)
        var fragment = NSRange()
        _ = lm.lineFragmentRect(forGlyphAt: glyph, effectiveRange: &fragment)
        guard fragment.length > 0 else { return nil }
        return lm.characterRange(forGlyphRange: fragment, actualGlyphRange: nil)
    }

    /// The row directly above or below `row`, or nil at the buffer's ends.
    private func visualRow(adjacentTo row: NSRange, down: Bool) -> NSRange? {
        guard down else {
            let probe = row.location - 1
            guard probe >= 0, let next = visualRow(containing: probe), next != row else { return nil }
            return next
        }
        let probe = NSMaxRange(row)
        guard probe <= length else { return nil }
        if probe == length {
            // The empty line after a trailing newline has no glyphs of its own,
            // so `visualRow` cannot find it; `j` reaches it and `gj` must too.
            guard length > 0, row.location < length, text.character(at: length - 1) == 0x0A else { return nil }
            return NSRange(location: length, length: 0)
        }
        guard let next = visualRow(containing: probe), next != row else { return nil }
        return next
    }

    /// The rightmost position the normal-mode caret may take on `row`: the
    /// row's last character, with a trailing newline excluded. Insert mode may
    /// sit one past it, as everywhere else here.
    private func lastCaretPosition(inRow row: NSRange) -> Int {
        guard row.length > 0 else { return row.location } // the empty final line
        var end = NSMaxRange(row)
        if end <= length, text.character(at: end - 1) == 0x0A { end -= 1 }
        return mode == .insert ? end : max(row.location, end - 1)
    }

    // MARK: character classes and scanning

    private func isBlank(_ c: unichar) -> Bool { c == 0x20 || c == 0x09 || c == 0x0A || c == 0x0D }
    private func isWordChar(_ c: unichar) -> Bool {
        (c >= 0x30 && c <= 0x39) || (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) || c == 0x5F || c > 0x7F
    }
    /// 0 blank, 1 word, 2 punctuation (big words: everything non-blank is class 1).
    private func cls(_ c: unichar, big: Bool) -> Int {
        if isBlank(c) { return 0 }
        if big || isWordChar(c) { return 1 }
        return 2
    }

    private func nextWordStart(from p: Int, big: Bool) -> Int {
        guard p < length else { return length }
        var i = p
        let k = cls(text.character(at: i), big: big)
        if k != 0 { while i < length, cls(text.character(at: i), big: big) == k { i += 1 } }
        // Skip blanks, but an empty line is a word.
        while i < length, isBlank(text.character(at: i)) {
            if text.character(at: i) == 0x0A, i + 1 < length, text.character(at: i + 1) == 0x0A, i > p { return i + 1 }
            i += 1
        }
        return i
    }

    private func nextWordEnd(from p: Int, big: Bool) -> Int {
        var i = p + 1
        while i < length, isBlank(text.character(at: i)) { i += 1 }
        guard i < length else { return max(0, length - 1) }
        let k = cls(text.character(at: i), big: big)
        while i + 1 < length, cls(text.character(at: i + 1), big: big) == k { i += 1 }
        return i
    }

    private func previousWordStart(from p: Int, big: Bool) -> Int {
        var i = p - 1
        while i > 0, isBlank(text.character(at: i)) { i -= 1 }
        guard i > 0 else { return 0 }
        let k = cls(text.character(at: i), big: big)
        while i > 0, cls(text.character(at: i - 1), big: big) == k { i -= 1 }
        return i
    }

    private func isBlankLine(at s: Int) -> Bool { s >= length || text.character(at: s) == 0x0A }

    private func nextParagraphBoundary(from p: Int) -> Int {
        var s = lineStart(p)
        // Skip blank lines, then non-blank lines; stop on the next blank line.
        while s < length, isBlankLine(at: s) { s = lineEnd(s) + 1 }
        while s < length, !isBlankLine(at: s) { s = lineEnd(s) + 1 }
        return min(s, length)
    }

    private func previousParagraphBoundary(from p: Int) -> Int {
        var s = lineStart(p)
        if s == 0 { return 0 }
        s = lineStart(s - 1)
        while s > 0, isBlankLine(at: s) { s = lineStart(s - 1) }
        while s > 0, !isBlankLine(at: lineStart(s - 1)) { s = lineStart(s - 1) }
        if s > 0, isBlankLine(at: lineStart(s - 1)) { return lineStart(s - 1) }
        return 0
    }

    private func isSentenceEnd(_ i: Int) -> Bool {
        guard let c = char(at: i), c == 0x2E || c == 0x21 || c == 0x3F else { return false }
        var j = i + 1
        while let d = char(at: j), d == 0x29 || d == 0x5D || d == 0x22 || d == 0x27 { j += 1 }
        guard let d = char(at: j) else { return true }
        return isBlank(d)
    }

    private func nextSentenceStart(from p: Int) -> Int {
        var i = p
        while i < length {
            if isSentenceEnd(i) {
                var j = i + 1
                while let d = char(at: j), d == 0x29 || d == 0x5D || d == 0x22 || d == 0x27 { j += 1 }
                while j < length, isBlank(text.character(at: j)) { j += 1 }
                if j > p { return min(j, length) }
            }
            if text.character(at: i) == 0x0A, i + 1 < length, text.character(at: i + 1) == 0x0A {
                var j = i + 1
                while j < length, isBlank(text.character(at: j)) { j += 1 }
                if j > p { return j }
            }
            i += 1
        }
        return length
    }

    private func previousSentenceStart(from p: Int) -> Int {
        var i = p - 1
        while i > 0, isBlank(text.character(at: i)) { i -= 1 }
        while i > 0 {
            i -= 1
            if isSentenceEnd(i) || (text.character(at: i) == 0x0A && i + 1 < length && text.character(at: i + 1) == 0x0A) {
                var j = i + 1
                while j < length, isBlank(text.character(at: j)) || text.character(at: j) == 0x29 || text.character(at: j) == 0x22 { j += 1 }
                if j < p { return j }
            }
        }
        return 0
    }

    private func findChar(_ ch: Character, from p: Int, forward: Bool, till: Bool, count: Int, repeating: Bool = false) -> Int? {
        let target = String(ch) as NSString
        guard target.length == 1 else { return nil }
        let u = target.character(at: 0)
        var found = p
        for k in 0..<count {
            var i = forward ? found + 1 : found - 1
            if till, repeating, k == 0 { i += forward ? 1 : -1 } // `;` after `t` must not stick
            let lo = lineStart(p), hi = lineEnd(p)
            while i >= lo, i < hi {
                if text.character(at: i) == u { break }
                i += forward ? 1 : -1
            }
            guard i >= lo, i < hi else { return nil }
            found = i
        }
        return till ? found + (forward ? -1 : 1) : found
    }

    /// `%`: the bracket pair at or after the caret on its line, or the
    /// `\begin`/`\end` partner when the caret is on one (EditorNavigation.swift).
    private func matchingPair(from p: Int) -> Int? {
        if let pair = EditorNavigation.environmentPair(at: p, in: text), let end = pair.end {
            let onBegin = NSLocationInRange(p, pair.begin) || p == NSMaxRange(pair.begin)
            return onBegin ? end.location : pair.begin.location
        }
        let openers: [unichar: unichar] = [0x28: 0x29, 0x5B: 0x5D, 0x7B: 0x7D]
        let closers: [unichar: unichar] = [0x29: 0x28, 0x5D: 0x5B, 0x7D: 0x7B]
        var i = p
        let end = lineEnd(p)
        while i < end, openers[text.character(at: i)] == nil, closers[text.character(at: i)] == nil { i += 1 }
        guard i < end else { return nil }
        let c = text.character(at: i)
        if let close = openers[c] {
            var depth = 0, j = i
            while j < length {
                let d = text.character(at: j)
                if d == c { depth += 1 } else if d == close { depth -= 1; if depth == 0 { return j } }
                j += 1
            }
        } else if let open = closers[c] {
            var depth = 0, j = i
            while j >= 0 {
                let d = text.character(at: j)
                if d == c { depth += 1 } else if d == open { depth -= 1; if depth == 0 { return j } }
                j -= 1
            }
        }
        return nil
    }

    // MARK: text objects

    /// `iw aw i( a( i[ a[ i{ a{ i" a" i$ a$ ie ae` (and `ib ab iB aB`) around the caret.
    func textObject(_ ch: Character, inner: Bool) -> NSRange? {
        let c = caret
        switch ch {
        case "w", "W":
            guard c < length else { return nil }
            let big = ch == "W"
            let k = cls(text.character(at: c), big: big)
            var s = c, e = c
            while s > 0, cls(text.character(at: s - 1), big: big) == k, text.character(at: s - 1) != 0x0A { s -= 1 }
            while e + 1 < length, cls(text.character(at: e + 1), big: big) == k, text.character(at: e + 1) != 0x0A { e += 1 }
            if !inner {
                var f = e + 1
                if k != 0 {
                    while f < length, let d = char(at: f), d == 0x20 || d == 0x09 { f += 1 }
                    if f == e + 1 { // no trailing blanks: take the leading ones
                        while s > 0, let d = char(at: s - 1), d == 0x20 || d == 0x09 { s -= 1 }
                    }
                } else {
                    let k2 = f < length ? cls(text.character(at: f), big: big) : 0
                    while f < length, cls(text.character(at: f), big: big) == k2, text.character(at: f) != 0x0A { f += 1 }
                }
                e = f - 1
            }
            return NSRange(location: s, length: e - s + 1)
        case "(", ")", "b": return bracketObject(open: 0x28, close: 0x29, inner: inner)
        case "[", "]": return bracketObject(open: 0x5B, close: 0x5D, inner: inner)
        case "{", "}", "B": return bracketObject(open: 0x7B, close: 0x7D, inner: inner)
        case "\"", "'", "`": return quoteObject(String(ch).utf16.first!, inner: inner)
        case "$": return mathObject(inner: inner)
        case "e": return environmentObject(inner: inner)
        default: return nil
        }
    }

    private func bracketObject(open: unichar, close: unichar, inner: Bool) -> NSRange? {
        let c = caret
        // Find the innermost unmatched opener before (or at) the caret.
        var depth = 0, i = min(c, length - 1)
        guard i >= 0 else { return nil }
        if text.character(at: i) == close { i -= 1 }
        var openAt: Int?
        while i >= 0 {
            let d = text.character(at: i)
            if d == close { depth += 1 } else if d == open { if depth == 0 { openAt = i; break }; depth -= 1 }
            i -= 1
        }
        guard let s = openAt else { return nil }
        depth = 0
        var j = s
        var closeAt: Int?
        while j < length {
            let d = text.character(at: j)
            if d == open { depth += 1 } else if d == close { depth -= 1; if depth == 0 { closeAt = j; break } }
            j += 1
        }
        guard let e = closeAt else { return nil }
        return inner ? NSRange(location: s + 1, length: e - s - 1) : NSRange(location: s, length: e - s + 1)
    }

    private func quoteObject(_ q: unichar, inner: Bool) -> NSRange? {
        let c = caret
        let ls = lineStart(c), le = lineEnd(c)
        var quotes: [Int] = []
        var i = ls
        while i < le { if text.character(at: i) == q, i == ls || text.character(at: i - 1) != 0x5C { quotes.append(i) }; i += 1 }
        guard quotes.count >= 2 else { return nil }
        var pair: (Int, Int)?
        var k = 0
        while k + 1 < quotes.count {
            if c <= quotes[k + 1] { pair = (quotes[k], quotes[k + 1]); break }
            k += 2
        }
        guard let (s, e) = pair else { return nil }
        return inner ? NSRange(location: s + 1, length: e - s - 1) : NSRange(location: s, length: e - s + 1)
    }

    /// `$…$` (or `$$…$$`) around the caret on its line.
    private func mathObject(inner: Bool) -> NSRange? {
        let c = caret
        let ls = lineStart(c), le = lineEnd(c)
        var dollars: [NSRange] = []
        var i = ls
        while i < le {
            if text.character(at: i) == 0x24, i == ls || text.character(at: i - 1) != 0x5C {
                if i + 1 < le, text.character(at: i + 1) == 0x24 { dollars.append(NSRange(location: i, length: 2)); i += 2; continue }
                dollars.append(NSRange(location: i, length: 1))
            }
            i += 1
        }
        var k = 0
        while k + 1 < dollars.count {
            let open = dollars[k], close = dollars[k + 1]
            if c < NSMaxRange(close) {
                return inner ? NSRange(location: NSMaxRange(open), length: close.location - NSMaxRange(open))
                    : NSRange(location: open.location, length: NSMaxRange(close) - open.location)
            }
            k += 2
        }
        return nil
    }

    /// `ie`/`ae`: the innermost `\begin{X}…\end{X}` around the caret
    /// (`EditorNavigation.enclosingEnvironment`); inner is the body between them.
    private func environmentObject(inner: Bool) -> NSRange? {
        guard let pair = EditorNavigation.enclosingEnvironment(at: caret, in: text), let end = pair.end else { return nil }
        if !inner { return NSRange(location: pair.begin.location, length: NSMaxRange(end) - pair.begin.location) }
        var s = NSMaxRange(pair.begin), e = end.location
        // Whole body lines when `\begin` and `\end` sit on their own lines.
        if let c = char(at: s), c == 0x0A, lineStart(e) == firstNonBlank(fromLineStart: lineStart(e)) || e == firstNonBlank(fromLineStart: lineStart(e)) {
            s += 1
            e = lineStart(e)
        }
        return NSRange(location: s, length: max(0, e - s))
    }

    // MARK: search (`/ ? n N *`)

    private func search(_ pattern: String, from p: Int, forward: Bool, wrap: Bool = true) -> NSRange? {
        guard !pattern.isEmpty else { return nil }
        let opts: NSString.CompareOptions = forward ? [] : [.backwards]
        let caseOpts: NSString.CompareOptions = pattern.lowercased() == pattern ? [.caseInsensitive] : [] // smartcase
        if forward {
            let start = min(p + 1, length)
            let r = text.range(of: pattern, options: opts.union(caseOpts), range: NSRange(location: start, length: length - start))
            if r.location != NSNotFound { return r }
            guard wrap else { return nil }
            let w = text.range(of: pattern, options: opts.union(caseOpts), range: NSRange(location: 0, length: length))
            return w.location == NSNotFound ? nil : w
        } else {
            let r = text.range(of: pattern, options: opts.union(caseOpts), range: NSRange(location: 0, length: max(0, min(p, length))))
            if r.location != NSNotFound { return r }
            guard wrap else { return nil }
            let w = text.range(of: pattern, options: opts.union(caseOpts), range: NSRange(location: 0, length: length))
            return w.location == NSNotFound ? nil : w
        }
    }

    private func jumpToMatch(_ pattern: String, forward: Bool, count: Int) {
        var p = mode == .visual || mode == .visualLine ? visualHead : caret
        var found: NSRange?
        for _ in 0..<count {
            guard let r = search(pattern, from: p, forward: forward) else { break }
            found = r; p = r.location
        }
        guard let r = found else { message = "E486: Pattern not found: \(pattern)"; return }
        if mode == .visual || mode == .visualLine { setCaretKeepingVisual(r.location) } else { setCaret(r.location) }
    }

    private func searchAgain(reverse: Bool, count: Int) {
        guard let s = lastSearch else { message = "E35: No previous regular expression"; return }
        jumpToMatch(s.pattern, forward: reverse ? !s.forward : s.forward, count: count)
    }

    private func searchWordUnderCaret(count: Int) {
        guard let r = textObject("w", inner: true), r.length > 0, let c = char(at: r.location), isWordChar(c) else { return }
        let word = text.substring(with: r)
        lastSearch = (word, true)
        shareWithFindBar(word)
        jumpToMatch(word, forward: true, count: count)
    }

    /// The search term also becomes the find bar's (⌘G / Edit ▸ Find ▸ Find Next continue it).
    private func shareWithFindBar(_ pattern: String) {
        let pb = NSPasteboard(name: .find)
        pb.clearContents()
        pb.setString(pattern, forType: .string)
    }

    // MARK: command line (`:` `/` `?`)

    private func handleCommandLineKey(_ key: Key) {
        guard var line = commandLine else { return }
        if key.escape {
            commandLine = nil
            if line.prefix != ":" { setCaret(line.origin) } // incremental search: back to where it started
            if mode == .visual || mode == .visualLine { updateVisualSelection() }
            return
        }
        if key.backspace {
            if line.text.isEmpty { commandLine = nil; if line.prefix != ":" { setCaret(line.origin) }; return }
            line.text.removeLast()
        } else if key.return {
            commandLine = nil
            if line.prefix == ":" { runExCommand(line.text) } else { commitSearch(line.text, forward: line.prefix == "/", origin: line.origin) }
            return
        } else if let ch = key.char, !key.control {
            line.text.append(ch)
        } else {
            return
        }
        commandLine = line
        if line.prefix != ":" { previewSearch(line.text, forward: line.prefix == "/", origin: line.origin) }
    }

    private func previewSearch(_ pattern: String, forward: Bool, origin: Int) {
        guard let r = search(pattern, from: origin, forward: forward) else { setCaret(origin); return }
        setSelection(r)
    }

    private func commitSearch(_ pattern: String, forward: Bool, origin: Int) {
        let p = pattern.isEmpty ? lastSearch?.pattern : pattern
        guard let p, !p.isEmpty else { setCaret(origin); return }
        lastSearch = (p, forward)
        shareWithFindBar(p)
        setCaret(origin)
        jumpToMatch(p, forward: forward, count: 1)
    }

    /// `:w :q :q! :wq :x :e file :[range]s/a/b/[g] :noh :set nu|nonu|number|nonumber`.
    func runExCommand(_ raw: String) {
        message = nil
        let line = raw.trimmingCharacters(in: .whitespaces)
        guard !line.isEmpty else { return }
        // A substitute with an address: `%s`, `'<,'>s`, `3,5s`, `s`.
        if let r = line.range(of: #"^(%|'<,'>|\d+,\d+|\d+)?s(?=[/#|])"#, options: .regularExpression) {
            substitute(address: String(line[r.lowerBound..<line.index(before: r.upperBound)]), spec: String(line[r.upperBound...]))
            return
        }
        let parts = line.split(separator: " ", maxSplits: 1).map(String.init)
        let cmd = parts[0], arg = parts.count > 1 ? parts[1].trimmingCharacters(in: .whitespaces) : ""
        switch cmd {
        case "w", "write": message = dispatch(.write)
        case "q", "quit": message = dispatch(.quit(force: false))
        case "q!", "quit!": message = dispatch(.quit(force: true))
        case "wq", "x", "xit", "exit": message = dispatch(.writeQuit)
        case "e", "edit": message = arg.isEmpty ? "E32: No file name" : dispatch(.edit(arg))
        case "noh", "nohlsearch": clearFindHighlight()
        case "set", "se":
            switch arg {
            case "nu", "number": message = dispatch(.setNumber(true))
            case "nonu", "nonumber": message = dispatch(.setNumber(false))
            default: message = "E518: Unknown option: \(arg)"
            }
        default:
            message = "E492: Not an editor command: \(line)"
        }
    }

    private func dispatch(_ command: ExCommand) -> String? {
        guard let handler = exCommandHandler else { return "E319: Command not available here" }
        return handler(command)
    }

    private func clearFindHighlight() {
        textView?.setSelectedRange(NSRange(location: caret, length: 0))
        message = nil
    }

    /// `:[range]s/pattern/replacement/[g]` — literal text, one undo step, via
    /// the editor's edit path. Default range: the caret's line.
    private func substitute(address: String, spec: String) {
        guard let sep = spec.first else { return }
        let fields = spec.dropFirst().split(separator: sep, maxSplits: 2, omittingEmptySubsequences: false).map(String.init)
        guard fields.count >= 2, !fields[0].isEmpty else { message = "E486: Pattern not found"; return }
        let pattern = fields[0], replacement = fields[1]
        let flags = fields.count > 2 ? fields[2] : ""
        let global = flags.contains("g")
        let range: NSRange
        switch address {
        case "%": range = NSRange(location: 0, length: length)
        case "'<,'>":
            let sel = textView?.selectedRange() ?? NSRange(location: caret, length: 0)
            range = lineRange(sel.length > 0 ? sel : NSRange(location: caret, length: 0), includeTrailingNewline: false)
        case "":
            range = NSRange(location: lineStart(caret), length: lineEnd(caret) - lineStart(caret))
        default:
            let nums = address.split(separator: ",").compactMap { Int($0) }
            guard let a = nums.first, let b = nums.last, a >= 1, b >= a else { message = "E16: Invalid range"; return }
            let s = lineStart(ofLine: a - 1), e = lineEnd(lineStart(ofLine: b - 1))
            range = NSRange(location: s, length: max(0, e - s))
        }
        let caseOpts: NSString.CompareOptions = pattern.lowercased() == pattern ? [.caseInsensitive] : []
        var edits: [NSRange] = []
        var lastLineStart = -1
        var i = range.location
        while i < NSMaxRange(range) {
            let r = text.range(of: pattern, options: caseOpts, range: NSRange(location: i, length: NSMaxRange(range) - i))
            guard r.location != NSNotFound else { break }
            let ls = lineStart(r.location)
            if global || ls != lastLineStart { edits.append(r); lastLineStart = ls }
            i = NSMaxRange(r) == r.location ? r.location + 1 : NSMaxRange(r)
        }
        guard !edits.isEmpty, let tv = textView else { message = "E486: Pattern not found: \(pattern)"; return }
        tv.breakUndoCoalescing()
        tv.undoManager?.beginUndoGrouping()
        for r in edits.reversed() { tv.insertText(replacement, replacementRange: r) }
        tv.undoManager?.setActionName("Substitute")
        tv.undoManager?.endUndoGrouping()
        tv.breakUndoCoalescing()
        setCaret(firstNonBlank(fromLineStart: lineStart(min(edits.last!.location, length))))
        message = edits.count > 1 ? "\(edits.count) substitutions" : nil
    }
}
