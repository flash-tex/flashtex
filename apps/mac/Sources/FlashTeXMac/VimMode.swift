import Foundation

/// Vim keybinding emulation for the source editor — the pure half.
///
/// Shaped after `EditorKeyHandling.swift`: every decision is made here, in
/// `NSTextView`-free code over a `String` plus a UTF-16 caret offset, and only
/// three small hooks connect it to AppKit (`VimModeEditor.swift`). Tests drive
/// `VimMachine` directly — key sequence in, buffer and caret out — so they need
/// no window, no first responder and no synthesized key events.
///
/// The machine never edits anything itself in the app: it reports the edits it
/// wants as `EditorKeyHandling.LineEdit` values, which the host applies through
/// the editor's existing `applyLineEdits` (a `shouldChangeText` /
/// `didChangeText` pass per edit, inside one undo group), so `u` / `⌃R` and
/// ⌘Z / ⇧⌘Z drive the *same* `NSUndoManager` — there is no parallel stack.
/// Actions only the app can perform (save, close, the Find bar, Go to Matching)
/// come back as `VimRequest` values.
///
/// Off by default: `VimModeFeature.isEnabled` gates the single `keyDown` hook,
/// so with the preference off not one keystroke reaches this file.
///
/// ## What is implemented
///
/// - Modes: normal, insert, visual (`v`), visual-line (`V`); Escape (or `⌃[`)
///   returns to normal from any of them.
/// - Motions: `h j k l w b e 0 ^ $ gg G { } f F t T ; ,` and `%`, plus the
///   arrow keys, Return and Backspace.
/// - Operators `d c y` over any motion, doubled (`dd cc yy`) for whole lines,
///   `>>` / `<<` (and `>` / `<` on a visual selection) for indent, plus
///   `D` (`d$`), `C` (`c$`), `Y` (`yy`), `x`, `s`, `r<char>`, `p`, `P`,
///   `o`, `O`, `a`, `A`, `i`, `I`.
/// - Counts before and inside a command: `3dd`, `5j`, `d3w`, `2d3w` (= `d6w`).
/// - `u` and `⌃R`; `/` `?` `n` `N`; `:w` `:q` `:wq` `:x` `:q!` and `:<line>`.
///
/// ## What is *not* implemented (deliberately)
///
/// One unnamed register only — no named registers, no yank ring. No marks, no
/// macros (`q`/`@`), no `.` repeat, no text objects (`ciw`, `di{`), no `*`/`#`,
/// no `J`, `~`, `R`, `gJ`, no WORD motions (`W B E`), no `H M L` or
/// `zz zt zb`, no jump list (`⌃O`/`⌃I`), no window/tab/buffer commands, and no
/// `:` command beyond the ones listed. `/` hands the query to the app's Find
/// bar rather than reimplementing search, so it is the Find bar's matching
/// (literal, its own case rules), not vim regex. An unrecognised key in normal
/// or visual mode is swallowed and beeps rather than being typed into the
/// buffer.
enum VimMode: String, Equatable, Sendable, CaseIterable {
    case normal, insert, visual, visualLine

    /// The word the editor's mode indicator shows.
    var indicator: String {
        switch self {
        case .normal: "NORMAL"
        case .insert: "INSERT"
        case .visual: "VISUAL"
        case .visualLine: "V-LINE"
        }
    }

    var isVisual: Bool { self == .visual || self == .visualLine }
}

/// One keystroke as the machine sees it. The host builds these from `NSEvent`
/// (`VimModeEditor.swift`); tests build them literally.
struct VimKey: Equatable, Sendable {
    enum Special: Equatable, Sendable { case escape, enter, backspace }

    var character: Character?
    var special: Special?
    /// Control was held. Command and Option keys never reach the machine.
    var control: Bool

    init(character: Character? = nil, special: Special? = nil, control: Bool = false) {
        self.character = character
        self.special = special
        self.control = control
    }

    static func c(_ ch: Character) -> VimKey { VimKey(character: ch) }
    static func ctrl(_ ch: Character) -> VimKey { VimKey(character: ch, control: true) }
    static let escape = VimKey(special: .escape)
    static let enter = VimKey(special: .enter)
    static let backspace = VimKey(special: .backspace)

    /// `VimKey.keys("3dd")` — every character of `s` as a plain keystroke.
    static func keys(_ s: String) -> [VimKey] { s.map { VimKey(character: $0) } }
}

/// Something only the app can do; the host routes each one to the real command
/// (`VimModeEditor.swift`, `ContentView.swift`).
enum VimRequest: Equatable, Sendable {
    /// `u` / `⌃R`: the editor's own `NSUndoManager`, never a parallel stack.
    case undo, redo
    /// `:w` → `ShellModel.saveTexInteractive()`.
    case save
    /// `:q` / `:q!` → detach the active document, or close the window.
    case close, saveAndClose, discardAndClose
    /// `%` with no bracket or math pair in reach: the app's `\begin`/`\end`,
    /// `\label`/`\ref` jump (`ShellModel.goToMatching`, ⌘⇧D).
    case goToMatching
    /// `/` and `?`: the standard Find bar, not a parallel search.
    case openFind(backward: Bool)
    /// `n` / `N`: Find Next / Find Previous, direction-corrected after `?`.
    case findNext, findPrevious
}

/// What one keystroke produced.
struct VimOutcome: Equatable {
    /// `false` means "not a vim key": the host passes the event straight to
    /// AppKit, exactly as if vim mode were off. Insert mode returns this for
    /// everything but Escape, which is what keeps input methods, dead keys,
    /// auto-close, completion and every other editor behaviour intact while
    /// typing.
    var handled: Bool = true
    /// Edits in the *pre-key* buffer's coordinates, applied last-first as one
    /// undo step (`EditorKeyHandling.applyLineEdits`).
    var edits: [EditorKeyHandling.LineEdit] = []
    /// Where the selection/caret ends up once `edits` are applied.
    var selection: NSRange?
    var requests: [VimRequest] = []
    /// Undo action name for `edits` ("Delete", "Change", "Indent", …).
    var actionName: String?
    /// An unknown or impossible command: the host beeps, nothing changes.
    var beep: Bool = false

    static let ignored = VimOutcome(handled: false)
}

// MARK: - pure text helpers

/// UTF-16 offset arithmetic over the buffer: lines, columns, words,
/// paragraphs and in-line character search. All pure, all tested.
enum VimText {
    enum CharClass: Equatable { case whitespace, word, punctuation }

    static func isNewline(_ c: unichar) -> Bool {
        c == 0x0A || c == 0x0D || c == 0x2028 || c == 0x2029 || c == 0x85
    }

    static func clampOffset(_ ns: NSString, _ p: Int) -> Int { min(max(p, 0), ns.length) }

    /// Offset of the first character of the line `p` sits on.
    static func lineStart(_ ns: NSString, _ p: Int) -> Int {
        ns.lineRange(for: NSRange(location: clampOffset(ns, p), length: 0)).location
    }

    /// Offset just past the last character of the line `p` sits on, *not*
    /// counting the terminator (so it equals `lineStart` on an empty line).
    static func lineEnd(_ ns: NSString, _ p: Int) -> Int {
        let r = ns.lineRange(for: NSRange(location: clampOffset(ns, p), length: 0))
        var end = NSMaxRange(r)
        while end > r.location, isNewline(ns.character(at: end - 1)) { end -= 1 }
        return end
    }

    /// Offset just past the line `p` sits on, terminator included (the start
    /// of the next line, or the buffer end on the last line).
    static func lineEndWithNewline(_ ns: NSString, _ p: Int) -> Int {
        NSMaxRange(ns.lineRange(for: NSRange(location: clampOffset(ns, p), length: 0)))
    }

    /// First non-blank character of `p`'s line, or its end when blank (`^`).
    static func firstNonBlank(_ ns: NSString, _ p: Int) -> Int {
        let start = lineStart(ns, p), end = lineEnd(ns, p)
        var i = start
        while i < end, ns.character(at: i) == 0x20 || ns.character(at: i) == 0x09 { i += 1 }
        return i
    }

    /// The line's leading whitespace, copied onto the lines `o`/`O` open.
    static func leadingWhitespace(_ ns: NSString, _ p: Int) -> String {
        let start = lineStart(ns, p)
        return ns.substring(with: NSRange(location: start, length: firstNonBlank(ns, p) - start))
    }

    static func column(_ ns: NSString, _ p: Int) -> Int { clampOffset(ns, p) - lineStart(ns, p) }

    /// Start of the line `delta` lines away, or nil when that runs off an end.
    static func line(_ ns: NSString, from p: Int, delta: Int) -> Int? {
        var at = lineStart(ns, p)
        if delta >= 0 {
            for _ in 0..<delta {
                let next = lineEndWithNewline(ns, at)
                // The last line has no terminator of its own: `next` is the
                // buffer end and there is no line after it.
                if next >= ns.length || next == at { return nil }
                at = next
            }
        } else {
            for _ in 0..<(-delta) {
                if at == 0 { return nil }
                at = lineStart(ns, at - 1)
            }
        }
        return at
    }

    /// Start of the last line that holds characters.
    static func lastLineStart(_ ns: NSString) -> Int {
        ns.length == 0 ? 0 : lineStart(ns, ns.length - 1)
    }

    /// Start of the `n`-th line, 1-based, clamped to the buffer (`5G`, `:5`).
    static func start(ofLine n: Int, in ns: NSString) -> Int {
        guard n > 1 else { return 0 }
        var at = 0
        for _ in 1..<n {
            let next = lineEndWithNewline(ns, at)
            if next >= ns.length { return lastLineStart(ns) }
            at = next
        }
        return at
    }

    static func charClass(_ ns: NSString, _ p: Int) -> CharClass {
        guard p >= 0, p < ns.length else { return .whitespace }
        let c = ns.character(at: p)
        if c == 0x20 || c == 0x09 || isNewline(c) { return .whitespace }
        if c == UInt16(UInt8(ascii: "_")) { return .word }
        guard let scalar = Unicode.Scalar(c) else { return .word }
        return CharacterSet.alphanumerics.contains(scalar) ? .word : .punctuation
    }

    /// `w`: start of the next word. An empty line counts as a word.
    static func wordForward(_ ns: NSString, from p: Int) -> Int {
        var i = clampOffset(ns, p)
        guard i < ns.length else { return ns.length }
        let cls = charClass(ns, i)
        if cls != .whitespace { while i < ns.length, charClass(ns, i) == cls { i += 1 } }
        while i < ns.length, charClass(ns, i) == .whitespace {
            if isNewline(ns.character(at: i)), i + 1 < ns.length, isNewline(ns.character(at: i + 1)) { return i + 1 }
            i += 1
        }
        return i
    }

    /// `b`: start of the previous word.
    static func wordBackward(_ ns: NSString, from p: Int) -> Int {
        var i = clampOffset(ns, p) - 1
        while i >= 0, charClass(ns, i) == .whitespace { i -= 1 }
        guard i >= 0 else { return 0 }
        let cls = charClass(ns, i)
        while i > 0, charClass(ns, i - 1) == cls { i -= 1 }
        return i
    }

    /// `e`: last character of the current or next word (an inclusive motion).
    static func wordEnd(_ ns: NSString, from p: Int) -> Int {
        var i = clampOffset(ns, p) + 1
        while i < ns.length, charClass(ns, i) == .whitespace { i += 1 }
        guard i < ns.length else { return max(0, ns.length - 1) }
        let cls = charClass(ns, i)
        while i + 1 < ns.length, charClass(ns, i + 1) == cls { i += 1 }
        return i
    }

    static func isBlankLine(_ ns: NSString, _ p: Int) -> Bool { lineStart(ns, p) == lineEnd(ns, p) }

    /// `}`: start of the next empty line after `p`'s line, else the buffer end.
    static func paragraphForward(_ ns: NSString, from p: Int) -> Int {
        var at = lineEndWithNewline(ns, p)
        while at < ns.length {
            if isBlankLine(ns, at) { return at }
            let next = lineEndWithNewline(ns, at)
            if next == at { break }
            at = next
        }
        return ns.length
    }

    /// `{`: start of the previous empty line before `p`'s line, else 0.
    static func paragraphBackward(_ ns: NSString, from p: Int) -> Int {
        var at = lineStart(ns, p)
        while at > 0 {
            at = lineStart(ns, at - 1)
            if isBlankLine(ns, at) { return at }
        }
        return 0
    }

    /// `f` / `F` / `t` / `T` within `p`'s line; `till` stops one short.
    /// `skipAdjacent` is what makes `;` after a `t` step on instead of
    /// standing still on the character before the same target.
    static func find(_ ns: NSString, from p: Int, target: Character, forward: Bool, till: Bool,
                     skipAdjacent: Bool = false, count: Int = 1) -> Int? {
        let units = Array(String(target).utf16)
        guard units.count == 1 else { return nil }
        let unit = units[0]
        let start = lineStart(ns, p), end = lineEnd(ns, p)
        var i = clampOffset(ns, p)
        if till, skipAdjacent { i = forward ? i + 1 : i - 1 }
        var found = i
        for _ in 0..<max(1, count) {
            if forward {
                var j = i + 1
                while j < end, ns.character(at: j) != unit { j += 1 }
                guard j < end else { return nil }
                found = j; i = j
            } else {
                var j = i - 1
                while j >= start, ns.character(at: j) != unit { j -= 1 }
                guard j >= start else { return nil }
                found = j; i = j
            }
        }
        return till ? (forward ? found - 1 : found + 1) : found
    }
}

// MARK: - the machine

/// The mode machine. One per editor (held by the view's coordinator); tests
/// make one directly and feed it keys.
final class VimMachine {
    /// The single unnamed register — this stage has no named registers.
    struct Register: Equatable { var text: String = ""; var linewise: Bool = false }

    enum MotionKind: Equatable { case exclusive, inclusive, linewise }
    struct Motion: Equatable { var target: Int; var kind: MotionKind }

    private(set) var mode: VimMode = .normal
    private(set) var text: String
    /// UTF-16 offset. In normal/visual mode this is the character the block
    /// cursor covers; in insert mode it is an ordinary caret.
    private(set) var caret: Int
    private(set) var register = Register()
    /// The `:`, `/` or `?` line being typed, prefix included; nil otherwise.
    private(set) var commandLine: String?
    /// Last status/error line for the indicator.
    private(set) var message: String?

    /// What one `>>` inserts; the host sets it from `EditorPreferences`.
    var indentUnit: String = "    "

    private var visualAnchor = 0
    private var count: Int?
    private var pendingOperator: Character?
    private var operatorCount: Int?
    /// `f F t T` awaiting their target, or `r` awaiting its replacement.
    private var awaiting: Character?
    /// The `g` of `gg`.
    private var prefix: Character?
    private var lastFind: (kind: Character, target: Character)?
    private var lastSearchBackward = false
    private var desiredColumn: Int?

    private var ns: NSString { text as NSString }

    init(text: String = "", caret: Int = 0) {
        self.text = text
        self.caret = 0
        self.caret = clampNormal(min(max(caret, 0), (text as NSString).length))
    }

    /// The selection the editor should show for the current state.
    var selection: NSRange {
        let ns = self.ns
        switch mode {
        case .normal, .insert:
            return NSRange(location: VimText.clampOffset(ns, caret), length: 0)
        case .visual:
            let lo = min(caret, visualAnchor), hi = max(caret, visualAnchor)
            return NSRange(location: lo, length: max(0, min(hi + 1, ns.length) - lo))
        case .visualLine:
            let lo = VimText.lineStart(ns, min(caret, visualAnchor))
            let hi = VimText.lineEndWithNewline(ns, max(caret, visualAnchor))
            return NSRange(location: lo, length: max(0, hi - lo))
        }
    }

    /// What the indicator shows after the mode word: the count and operator
    /// typed so far (`3d`), or the command line being entered (`:wq`).
    var pendingDisplay: String {
        if let commandLine { return commandLine }
        var s = ""
        if let operatorCount { s += String(operatorCount) }
        if let pendingOperator { s.append(pendingOperator) }
        if let count { s += String(count) }
        if let prefix { s.append(prefix) }
        if let awaiting { s.append(awaiting) }
        return s
    }

    /// The host calls this before every key so the machine sees exactly what
    /// the text view holds (the user may have clicked, or another feature may
    /// have edited the buffer).
    func sync(text: String, caret: Int) {
        if self.text != text { self.text = text; desiredColumn = nil }
        let clamped = VimText.clampOffset(ns, caret)
        if clamped != self.caret { desiredColumn = nil }
        self.caret = mode == .insert ? clamped : clampNormal(clamped)
        if mode.isVisual { visualAnchor = VimText.clampOffset(ns, visualAnchor) }
    }

    /// Applies an outcome's edits to the machine's own copy of the buffer. In
    /// the app the host does this by re-syncing from the text view after the
    /// real edit; tests call it so a whole sequence runs with no AppKit at all.
    func applyToOwnBuffer(_ outcome: VimOutcome) {
        // Without edits the caret is already where `handle` put it (and in
        // visual mode `selection.location` is the *anchor* end, not the caret).
        guard !outcome.edits.isEmpty else { return }
        var s = ns
        for edit in outcome.edits.sorted(by: { $0.range.location > $1.range.location }) {
            s = s.replacingCharacters(in: edit.range, with: edit.replacement) as NSString
        }
        text = s as String
        guard let selection = outcome.selection else { return }
        caret = VimText.clampOffset(ns, selection.location)
        if mode == .normal { caret = clampNormal(caret) }
    }

    /// Feeds one key and applies the result to the machine's own buffer.
    @discardableResult
    func send(_ key: VimKey) -> VimOutcome {
        let outcome = handle(key)
        applyToOwnBuffer(outcome)
        return outcome
    }

    /// `machine.send("3dd")` — every character as a plain keystroke.
    func send(_ s: String) { for key in VimKey.keys(s) { send(key) } }

    // MARK: dispatch

    func handle(_ key: VimKey) -> VimOutcome {
        message = nil
        if commandLine != nil { return handleCommandLine(key) }
        if mode == .insert { return handleInsert(key) }
        return handleNormal(key)
    }

    /// Insert mode is deliberately transparent: only Escape (and `⌃[`) is
    /// taken, so AppKit's own `keyDown` path — auto-close, the completion
    /// list, snippet stops, dead keys and every input method — behaves exactly
    /// as it does with vim mode off.
    private func handleInsert(_ key: VimKey) -> VimOutcome {
        guard isEscape(key) else { return .ignored }
        mode = .normal
        caret = clampNormal(max(VimText.lineStart(ns, caret), caret - 1))
        resetPending()
        return VimOutcome(selection: selection)
    }

    private func handleNormal(_ key: VimKey) -> VimOutcome {
        if isEscape(key) {
            resetPending()
            if mode.isVisual { mode = .normal; caret = clampNormal(caret) }
            return VimOutcome(selection: selection)
        }
        if key.control, let ch = key.character {
            guard ch == "r" else { return .ignored } // ⌃Space and friends stay the app's
            resetPending()
            return VimOutcome(requests: [.redo])
        }
        if key.special == .enter { return runMotion("+") }
        if key.special == .backspace { return runMotion("h") }
        guard let ch = key.character else { return VimOutcome(beep: true) }

        // A pending character argument: an f/F/t/T target, or r's replacement.
        if let pending = awaiting {
            awaiting = nil
            if pending == "r" { return replaceCharacter(with: ch) }
            lastFind = (pending, ch)
            return findMotion(kind: pending, target: ch, skipAdjacent: false)
        }
        // `g` prefix: only `gg` at this stage.
        if prefix == "g" {
            prefix = nil
            guard ch == "g" else { return beep() }
            return runMotion("g")
        }
        // Counts. A leading `0` is the motion, not a count digit.
        if let digit = ch.wholeNumberValue, (0...9).contains(digit), !(digit == 0 && count == nil) {
            count = (count ?? 0) * 10 + digit
            return VimOutcome(selection: selection)
        }
        if ch == "g" { prefix = "g"; return VimOutcome(selection: selection) }
        if "fFtT".contains(ch) { awaiting = ch; return VimOutcome(selection: selection) }
        if ch == "r", pendingOperator == nil, !mode.isVisual { awaiting = "r"; return VimOutcome(selection: selection) }

        if let op = pendingOperator { return continueOperator(op, ch) }
        if mode.isVisual { return visualCommand(ch) }
        return normalCommand(ch)
    }

    // MARK: operators

    /// `d c y > <` start an operator in normal mode.
    private func startOperator(_ ch: Character) -> VimOutcome {
        pendingOperator = ch
        operatorCount = count
        count = nil
        return VimOutcome(selection: selection)
    }

    /// The key after an operator: repeating it makes a line command (`dd`),
    /// anything else must be a motion.
    private func continueOperator(_ op: Character, _ ch: Character) -> VimOutcome {
        if ch == op {
            let n = rawCount() ?? 1
            resetPending()
            return applyLinewise(op, lines: n)
        }
        return runMotion(ch)
    }

    /// `dd` / `cc` / `yy` / `>>` / `<<` over `lines` lines from the caret.
    private func applyLinewise(_ op: Character, lines: Int) -> VimOutcome {
        let last = VimText.line(ns, from: caret, delta: max(1, lines) - 1) ?? VimText.lastLineStart(ns)
        return apply(op, motion: Motion(target: last, kind: .linewise), from: caret)
    }

    /// A motion's target is always an offset *inside* the last line it reaches,
    /// so a linewise range is simply "the whole lines from one to the other".
    private func range(for motion: Motion, from: Int) -> NSRange {
        let ns = self.ns
        let lo = min(from, motion.target), hi = max(from, motion.target)
        switch motion.kind {
        case .linewise:
            let start = VimText.lineStart(ns, lo)
            return NSRange(location: start, length: max(0, VimText.lineEndWithNewline(ns, hi) - start))
        case .inclusive:
            return NSRange(location: lo, length: max(0, min(hi + 1, ns.length) - lo))
        case .exclusive:
            return NSRange(location: lo, length: max(0, hi - lo))
        }
    }

    private func apply(_ op: Character, motion: Motion, from: Int) -> VimOutcome {
        applyTo(op, range: range(for: motion, from: from), linewise: motion.kind == .linewise)
    }

    private func applyTo(_ op: Character, range rawRange: NSRange, linewise: Bool) -> VimOutcome {
        let ns = self.ns
        let loc = VimText.clampOffset(ns, rawRange.location)
        let range = NSRange(location: loc, length: max(0, min(NSMaxRange(rawRange), ns.length) - loc))
        let wasVisual = mode.isVisual
        switch op {
        case "y":
            register = Register(text: ns.substring(with: range), linewise: linewise)
            if wasVisual { mode = .normal }
            caret = clampNormal(linewise ? VimText.firstNonBlank(ns, range.location) : range.location)
            return VimOutcome(selection: NSRange(location: caret, length: 0))

        case "d":
            register = Register(text: ns.substring(with: range), linewise: linewise)
            if wasVisual { mode = .normal }
            let after = ns.replacingCharacters(in: range, with: "") as NSString
            let at = min(range.location, after.length)
            let landing = linewise ? VimText.firstNonBlank(after, at) : clampNormal(at, in: after)
            return VimOutcome(edits: [.init(range: range, replacement: "")],
                              selection: NSRange(location: landing, length: 0), actionName: "Delete")

        case "c":
            register = Register(text: ns.substring(with: range), linewise: linewise)
            mode = .insert
            guard linewise else {
                return VimOutcome(edits: [.init(range: range, replacement: "")],
                                  selection: NSRange(location: range.location, length: 0), actionName: "Change")
            }
            // `cc` empties the line but keeps it, and keeps its indentation.
            let indent = VimText.leadingWhitespace(ns, range.location)
            let endedWithNewline = range.length > 0 && VimText.isNewline(ns.character(at: NSMaxRange(range) - 1))
            let replacement = indent + (endedWithNewline ? "\n" : "")
            return VimOutcome(edits: [.init(range: range, replacement: replacement)],
                              selection: NSRange(location: range.location + (indent as NSString).length, length: 0),
                              actionName: "Change")

        case ">", "<":
            if wasVisual { mode = .normal }
            let result = op == ">"
                ? EditorKeyHandling.indentEdits(in: text, range: range, unit: indentUnit)
                : EditorKeyHandling.outdentEdits(in: text, range: range, unit: indentUnit)
            guard let (edits, _) = result, !edits.isEmpty else {
                caret = clampNormal(VimText.firstNonBlank(ns, range.location))
                return VimOutcome(selection: NSRange(location: caret, length: 0))
            }
            var after = ns
            for edit in edits.sorted(by: { $0.range.location > $1.range.location }) {
                after = after.replacingCharacters(in: edit.range, with: edit.replacement) as NSString
            }
            let landing = VimText.firstNonBlank(after, min(range.location, after.length))
            return VimOutcome(edits: edits, selection: NSRange(location: landing, length: 0),
                              actionName: op == ">" ? "Indent" : "Outdent")

        default:
            return beep()
        }
    }

    // MARK: motions

    /// Runs `ch` as a motion: with an operator pending it becomes that
    /// operator's range, otherwise it just moves (or extends a selection).
    private func runMotion(_ ch: Character) -> VimOutcome {
        let raw = rawCount()
        guard let motion = resolveMotion(ch, count: raw) else { resetPending(); return beep() }
        let op = pendingOperator
        let from = caret
        resetPending()
        if let op { return apply(op, motion: motion, from: from) }
        return move(to: motion)
    }

    private func move(to motion: Motion) -> VimOutcome {
        let ns = self.ns
        let target = motion.kind == .linewise ? VimText.firstNonBlank(ns, motion.target) : motion.target
        caret = mode.isVisual ? VimText.clampOffset(ns, target) : clampNormal(target)
        return VimOutcome(selection: selection)
    }

    private func resolveMotion(_ ch: Character, count: Int?) -> Motion? {
        let ns = self.ns
        let n = max(1, count ?? 1)
        if !"jk+-".contains(ch) { desiredColumn = nil }
        switch ch {
        case "h":
            return Motion(target: max(VimText.lineStart(ns, caret), caret - n), kind: .exclusive)
        case "l", " ":
            return Motion(target: min(VimText.lineEnd(ns, caret), caret + n), kind: .exclusive)
        case "j", "k", "+", "-":
            let delta = (ch == "j" || ch == "+") ? n : -n
            guard let lineStart = VimText.line(ns, from: caret, delta: delta) else { return nil }
            if pendingOperator != nil || ch == "+" || ch == "-" { return Motion(target: lineStart, kind: .linewise) }
            let wanted = desiredColumn ?? VimText.column(ns, caret)
            desiredColumn = wanted
            let target = min(lineStart + wanted, max(lineStart, VimText.lineEnd(ns, lineStart) - 1))
            return Motion(target: target, kind: .exclusive)
        case "0":
            return Motion(target: VimText.lineStart(ns, caret), kind: .exclusive)
        case "^":
            return Motion(target: VimText.firstNonBlank(ns, caret), kind: .exclusive)
        case "$":
            let line = VimText.line(ns, from: caret, delta: n - 1) ?? VimText.lastLineStart(ns)
            // Inclusive, so `d$` takes the last character of the line too.
            return Motion(target: max(VimText.lineStart(ns, line), VimText.lineEnd(ns, line) - 1), kind: .inclusive)
        case "w":
            // Vim's one famous special case: `cw` on a non-blank changes to
            // the end of the word, like `ce`, instead of eating the space.
            if pendingOperator == "c", VimText.charClass(ns, caret) != .whitespace {
                var end = caret
                for _ in 0..<n { end = max(end, VimText.wordEnd(ns, from: end)) }
                // On the word's last character `cw` changes just that character.
                if n == 1, VimText.charClass(ns, caret + 1) != VimText.charClass(ns, caret) { end = caret }
                return Motion(target: end, kind: .inclusive)
            }
            var at = caret
            for _ in 0..<n { at = VimText.wordForward(ns, from: at) }
            // Vim's rule: `dw` on the last word of a line stops at the line
            // end instead of swallowing the newline and joining two lines.
            if pendingOperator != nil {
                let end = VimText.lineEnd(ns, caret)
                if at > end, caret <= end { at = end }
            }
            return Motion(target: at, kind: .exclusive)
        case "b":
            var at = caret
            for _ in 0..<n { at = VimText.wordBackward(ns, from: at) }
            return Motion(target: at, kind: .exclusive)
        case "e":
            var at = caret
            for _ in 0..<n { at = VimText.wordEnd(ns, from: at) }
            return Motion(target: at, kind: .inclusive)
        case "{":
            return Motion(target: VimText.paragraphBackward(ns, from: caret), kind: .exclusive)
        case "}":
            return Motion(target: VimText.paragraphForward(ns, from: caret), kind: .exclusive)
        case "g": // `gg`
            return Motion(target: VimText.start(ofLine: count ?? 1, in: ns), kind: .linewise)
        case "G":
            return Motion(target: count.map { VimText.start(ofLine: $0, in: ns) } ?? VimText.lastLineStart(ns),
                          kind: .linewise)
        default:
            return nil
        }
    }

    private func findMotion(kind: Character, target: Character, skipAdjacent: Bool) -> VimOutcome {
        let n = rawCount() ?? 1
        let forward = kind == "f" || kind == "t"
        let till = kind == "t" || kind == "T"
        guard let at = VimText.find(ns, from: caret, target: target, forward: forward, till: till,
                                    skipAdjacent: skipAdjacent, count: n) else {
            resetPending()
            return beep()
        }
        let motion = Motion(target: at, kind: forward ? .inclusive : .exclusive)
        let op = pendingOperator
        let from = caret
        resetPending()
        if let op { return apply(op, motion: motion, from: from) }
        return move(to: motion)
    }

    // MARK: normal-mode commands

    private func normalCommand(_ ch: Character) -> VimOutcome {
        let ns = self.ns
        let n = rawCount() ?? 1
        switch ch {
        case "h", "j", "k", "l", "0", "^", "$", "w", "b", "e", "{", "}", "G", " ":
            return runMotion(ch)
        case "d", "c", "y", ">", "<":
            return startOperator(ch)
        case "D", "C":
            count = nil
            guard let motion = resolveMotion("$", count: n) else { return beep() }
            return apply(ch == "D" ? "d" : "c", motion: motion, from: caret)
        case "Y": count = nil; return applyLinewise("y", lines: n)
        case "x":
            count = nil
            let end = VimText.lineEnd(ns, caret)
            guard caret < end else { return VimOutcome(selection: selection) }
            let range = NSRange(location: caret, length: min(n, end - caret))
            register = Register(text: ns.substring(with: range), linewise: false)
            let after = ns.replacingCharacters(in: range, with: "") as NSString
            return VimOutcome(edits: [.init(range: range, replacement: "")],
                              selection: NSRange(location: clampNormal(range.location, in: after), length: 0),
                              actionName: "Delete")
        case "s":
            count = nil
            let end = VimText.lineEnd(ns, caret)
            let range = NSRange(location: caret, length: min(n, max(0, end - caret)))
            register = Register(text: ns.substring(with: range), linewise: false)
            mode = .insert
            return VimOutcome(edits: [.init(range: range, replacement: "")],
                              selection: NSRange(location: caret, length: 0), actionName: "Change")
        case "i":
            count = nil; mode = .insert
            return VimOutcome(selection: NSRange(location: caret, length: 0))
        case "a":
            count = nil; mode = .insert
            caret = min(VimText.lineEnd(ns, caret), caret + 1)
            return VimOutcome(selection: NSRange(location: caret, length: 0))
        case "I":
            count = nil; mode = .insert; caret = VimText.firstNonBlank(ns, caret)
            return VimOutcome(selection: NSRange(location: caret, length: 0))
        case "A":
            count = nil; mode = .insert; caret = VimText.lineEnd(ns, caret)
            return VimOutcome(selection: NSRange(location: caret, length: 0))
        case "o", "O": count = nil; return openLine(below: ch == "o")
        case "p", "P": count = nil; return paste(after: ch == "p", times: n)
        case "u": count = nil; return VimOutcome(requests: [.undo])
        case "v":
            count = nil; mode = .visual; visualAnchor = caret
            return VimOutcome(selection: selection)
        case "V":
            count = nil; mode = .visualLine; visualAnchor = caret
            return VimOutcome(selection: selection)
        case "%": count = nil; return matchingDelimiter()
        case "/", "?":
            // The Find bar takes the keyboard the moment it opens, so there is
            // no vim command line to keep here — only the direction `n`/`N`
            // should step in afterwards.
            count = nil
            lastSearchBackward = ch == "?"
            return VimOutcome(requests: [.openFind(backward: ch == "?")])
        case "n": count = nil; return VimOutcome(requests: [lastSearchBackward ? .findPrevious : .findNext])
        case "N": count = nil; return VimOutcome(requests: [lastSearchBackward ? .findNext : .findPrevious])
        case ";", ",":
            guard let last = lastFind else { return beep() }
            let kind = ch == ";" ? last.kind : reversedFind(last.kind)
            return findMotion(kind: kind, target: last.target, skipAdjacent: true)
        case ":": count = nil; commandLine = ":"; return VimOutcome(selection: selection)
        default: return beep()
        }
    }

    private func reversedFind(_ kind: Character) -> Character {
        switch kind { case "f": "F"; case "F": "f"; case "t": "T"; default: "t" }
    }

    // MARK: visual-mode commands

    private func visualCommand(_ ch: Character) -> VimOutcome {
        switch ch {
        case "h", "j", "k", "l", "0", "^", "$", "w", "b", "e", "{", "}", "G", " ":
            return runMotion(ch)
        case "v":
            count = nil
            if mode == .visual { mode = .normal; caret = clampNormal(caret) } else { mode = .visual }
            return VimOutcome(selection: selection)
        case "V":
            count = nil
            if mode == .visualLine { mode = .normal; caret = clampNormal(caret) } else { mode = .visualLine }
            return VimOutcome(selection: selection)
        case "o":
            count = nil
            swap(&caret, &visualAnchor)
            return VimOutcome(selection: selection)
        case "d", "x", "c", "s", "y", ">", "<":
            count = nil
            let op: Character = (ch == "x") ? "d" : (ch == "s") ? "c" : ch
            return applyTo(op, range: selection, linewise: mode == .visualLine)
        case "p", "P": count = nil; return visualPaste()
        case "%": count = nil; return matchingDelimiter()
        case ":": count = nil; commandLine = ":"; return VimOutcome(selection: selection)
        // `u`/`U` are case conversion in vim, which this stage does not have,
        // so they beep rather than quietly meaning undo.
        default: return beep()
        }
    }

    /// Visual `p`: the selection is replaced by the register and, vim-like,
    /// the replaced text becomes the register.
    private func visualPaste() -> VimOutcome {
        let range = selection
        let replaced = ns.substring(with: range)
        let wasLinewise = register.linewise
        let insert = wasLinewise && !register.text.hasSuffix("\n") ? register.text + "\n" : register.text
        register = Register(text: replaced, linewise: mode == .visualLine)
        mode = .normal
        let after = ns.replacingCharacters(in: range, with: insert) as NSString
        let landing = wasLinewise ? VimText.firstNonBlank(after, min(range.location, after.length))
                                  : clampNormal(range.location, in: after)
        return VimOutcome(edits: [.init(range: range, replacement: insert)],
                          selection: NSRange(location: landing, length: 0), actionName: "Paste")
    }

    // MARK: single commands

    private func replaceCharacter(with ch: Character) -> VimOutcome {
        let ns = self.ns
        let n = rawCount() ?? 1
        resetPending()
        guard caret + n <= VimText.lineEnd(ns, caret) else { return beep() }
        return VimOutcome(edits: [.init(range: NSRange(location: caret, length: n),
                                        replacement: String(repeating: String(ch), count: n))],
                          selection: NSRange(location: caret + n - 1, length: 0), actionName: "Replace")
    }

    private func openLine(below: Bool) -> VimOutcome {
        let ns = self.ns
        let indent = VimText.leadingWhitespace(ns, caret)
        let width = (indent as NSString).length
        mode = .insert
        if below {
            let at = VimText.lineEnd(ns, caret)
            return VimOutcome(edits: [.init(range: NSRange(location: at, length: 0), replacement: "\n" + indent)],
                              selection: NSRange(location: at + 1 + width, length: 0), actionName: "Open Line")
        }
        let at = VimText.lineStart(ns, caret)
        return VimOutcome(edits: [.init(range: NSRange(location: at, length: 0), replacement: indent + "\n")],
                          selection: NSRange(location: at + width, length: 0), actionName: "Open Line")
    }

    private func paste(after: Bool, times: Int) -> VimOutcome {
        guard !register.text.isEmpty else { return VimOutcome(selection: selection) }
        let ns = self.ns
        let body = String(repeating: register.text, count: max(1, times))
        guard register.linewise else {
            let end = VimText.lineEnd(ns, caret)
            let at = (after && caret < end) ? caret + 1 : caret
            let length = (body as NSString).length
            return VimOutcome(edits: [.init(range: NSRange(location: at, length: 0), replacement: body)],
                              selection: NSRange(location: max(at, at + length - 1), length: 0), actionName: "Paste")
        }
        var payload = body.hasSuffix("\n") ? body : body + "\n"
        var at = after ? VimText.lineEndWithNewline(ns, caret) : VimText.lineStart(ns, caret)
        var landing = at
        // Pasting below a last line that has no terminator needs one first.
        if after, at == ns.length, ns.length > 0, !VimText.isNewline(ns.character(at: ns.length - 1)) {
            payload = "\n" + (payload.hasSuffix("\n") ? String(payload.dropLast()) : payload)
            landing = at + 1
        }
        at = min(at, ns.length)
        let edit = EditorKeyHandling.LineEdit(range: NSRange(location: at, length: 0), replacement: payload)
        let afterText = ns.replacingCharacters(in: edit.range, with: payload) as NSString
        return VimOutcome(edits: [edit],
                          selection: NSRange(location: VimText.firstNonBlank(afterText, min(landing, afterText.length)), length: 0),
                          actionName: "Paste")
    }

    /// `%`: the app's own LaTeX-aware delimiter matcher
    /// (`SourceEditorView.BraceMatcher` — `{}`, `[]`, `$`, `$$`, minding
    /// escapes, `%` comments and `\verb`). When the caret is not on such a
    /// delimiter the app's Go to Matching command (⌘⇧D) takes over, which is
    /// what handles `\begin`/`\end` and `\label`/`\ref`.
    private func matchingDelimiter() -> VimOutcome {
        let ns = self.ns
        if let match = SourceEditorView.BraceMatcher.match(in: text, caretUTF16: caret) {
            return jump(match: match, from: caret)
        }
        var probe = caret
        let end = VimText.lineEnd(ns, caret)
        while probe < end {
            let c = ns.character(at: probe)
            if c == 0x7B || c == 0x7D || c == 0x5B || c == 0x5D || c == 0x24,
               let match = SourceEditorView.BraceMatcher.match(in: text, caretUTF16: probe) {
                return jump(match: match, from: probe)
            }
            probe += 1
        }
        return VimOutcome(requests: [.goToMatching])
    }

    private func jump(match: SourceEditorView.BraceMatcher.Match, from probe: Int) -> VimOutcome {
        let onOpener = probe >= match.open.location && probe < NSMaxRange(match.open)
        let target = onOpener ? match.close.location : match.open.location
        caret = mode.isVisual ? VimText.clampOffset(ns, target) : clampNormal(target)
        return VimOutcome(selection: selection)
    }

    // MARK: the `:` / `/` / `?` line

    private func handleCommandLine(_ key: VimKey) -> VimOutcome {
        guard var line = commandLine else { return .ignored }
        if isEscape(key) { commandLine = nil; return VimOutcome(selection: selection) }
        if key.special == .backspace {
            line.removeLast()
            commandLine = line.isEmpty ? nil : line
            return VimOutcome(selection: selection)
        }
        if key.special == .enter {
            commandLine = nil
            return runEx(String(line.dropFirst()))
        }
        guard let ch = key.character, !key.control else { return VimOutcome(beep: true) }
        line.append(ch)
        commandLine = line
        return VimOutcome(selection: selection)
    }

    /// The commands mapped onto the app's real actions. Anything else is
    /// reported as unknown rather than silently ignored.
    private func runEx(_ command: String) -> VimOutcome {
        let trimmed = command.trimmingCharacters(in: .whitespaces)
        switch trimmed {
        case "w", "write": return VimOutcome(requests: [.save])
        case "q", "quit": return VimOutcome(requests: [.close])
        case "wq", "x", "xit", "wq!": return VimOutcome(requests: [.saveAndClose])
        case "q!", "quit!": return VimOutcome(requests: [.discardAndClose])
        default:
            if let n = Int(trimmed), n > 0 {
                caret = clampNormal(VimText.firstNonBlank(ns, VimText.start(ofLine: n, in: ns)))
                return VimOutcome(selection: selection)
            }
            message = "E492: Not an editor command: \(trimmed)"
            return VimOutcome(beep: true)
        }
    }

    // MARK: helpers

    private func isEscape(_ key: VimKey) -> Bool {
        key.special == .escape || (key.control && key.character == "[")
    }

    /// Multiplies the operator's count by the motion's, vim-style; nil when
    /// neither was typed (so `G` can tell "last line" from "line 1").
    private func rawCount() -> Int? {
        guard operatorCount != nil || count != nil else { return nil }
        return max(1, (operatorCount ?? 1) * (count ?? 1))
    }

    /// Normal mode never sits on the line terminator: the caret is *on* a
    /// character, so it stops at the last one.
    private func clampNormal(_ p: Int) -> Int { clampNormal(p, in: ns) }

    private func clampNormal(_ p: Int, in ns: NSString) -> Int {
        let at = VimText.clampOffset(ns, p)
        return min(at, max(VimText.lineStart(ns, at), VimText.lineEnd(ns, at) - 1))
    }

    private func beep() -> VimOutcome {
        resetPending()
        return VimOutcome(selection: selection, beep: true)
    }

    private func resetPending() {
        count = nil; operatorCount = nil; pendingOperator = nil; awaiting = nil; prefix = nil
    }
}
