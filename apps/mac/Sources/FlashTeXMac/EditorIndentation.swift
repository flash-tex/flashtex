import AppKit

/// LaTeX-aware reindent (lane editor-reindent). Pure over line arrays; the
/// Editor menu and `CompletingTextView` hooks below apply a plan as one undo
/// step. SourceEditorView.swift is left alone (folding lane).
///
/// Shortcut choice: Editor ▸ Re-indent Lines is ⌃I (Xcode's re-indent). It
/// does not collide with Tab (keyCode 48) or with ⇧Tab. VimMode does not bind
/// ⌃I (normal/visual only handle ⌃d/⌃u/⌃f/⌃b/⌃r); insert mode lets menu key
/// equivalents through, and ⌘-shortcuts are never intercepted. ⌘⇧I is already
/// View ▸ Toggle Captures. Re-indent Document has no key equivalent.
enum EditorIndentation {
    /// Bodies of these environments are copied byte-for-byte, including their
    /// existing indentation. Starred / package variants are not in this set.
    static let preservedBodyEnvironments: Set<String> = [
        "verbatim", "lstlisting", "minted", "comment",
    ]

    /// These environments do not indent their body. Indenting a whole
    /// `document` body is the common complaint this constant exists to avoid.
    static let flatEnvironments: Set<String> = ["document"]

    /// Incoming scanner state for a run of lines. `reindent(_:baseDepth:unit:)`
    /// starts at `depth = baseDepth` with no preserved environment.
    struct State: Equatable {
        var depth: Int
        /// Environment whose body is currently frozen, if any.
        var preserved: String?
        /// Depth of the `\begin` that opened `preserved` (used to indent `\end`).
        var preserveDepth: Int
        /// Continuation lines under a preceding `\item` get one extra unit.
        var itemContinuation: Bool

        init(depth: Int = 0, preserved: String? = nil, preserveDepth: Int = 0, itemContinuation: Bool = false) {
            self.depth = max(0, depth)
            self.preserved = preserved
            self.preserveDepth = max(0, preserveDepth)
            self.itemContinuation = itemContinuation
        }
    }

    /// One replacement of the touched lines plus the selection mapped onto
    /// non-whitespace content in the result.
    struct Plan: Equatable {
        var range: NSRange
        var replacement: String
        var selection: NSRange
    }

    /// Reindents `lines` (no terminators) at `baseDepth` units of `unit`.
    /// Blank lines become empty. Lines are never joined or split. Depth
    /// increases after `\begin{x}` (except `flatEnvironments` and preserved
    /// begins) and decreases on the line with `\end{x}`. `\item` lines stay at
    /// the environment depth; continuation lines under an item get one extra
    /// unit. Brace groups `{…}` that span lines add a unit. Escaped `\{`/`\%`
    /// and `%` comments are ignored when counting.
    static func reindent(_ lines: [Substring], baseDepth: Int, unit: String) -> [String] {
        reindent(lines, unit: unit, incoming: State(depth: baseDepth))
    }

    static func reindent(_ lines: [Substring], unit: String, incoming: State) -> [String] {
        let unit = unit.isEmpty ? "  " : unit
        var state = incoming
        state.depth = max(0, state.depth)
        var out: [String] = []
        out.reserveCapacity(lines.count)
        for line in lines {
            var raw = line
            if raw.hasSuffix("\r") { raw = raw.dropLast() }
            out.append(reindentLine(String(raw), unit: unit, state: &state))
        }
        return out
    }

    /// Infers the depth at which `selection`'s first line should start: the
    /// visual indent of the first non-blank line above it, plus that line's
    /// outgoing opens, so a partial selection matches its neighbour. Preserved
    /// bodies are detected by scanning from the start of `text`.
    static func plan(in text: String, selection: NSRange, unit: String, tabWidth: Int, wholeDocument: Bool) -> Plan? {
        let unit = unit.isEmpty ? "  " : unit
        let tabWidth = max(1, tabWidth)
        let ns = text as NSString
        let n = ns.length
        let sel = NSRange(location: max(0, min(selection.location, n)),
                          length: max(0, min(selection.length, n - min(selection.location, n))))
        let lines = splitLines(ns)
        guard !lines.isEmpty else { return nil }

        let touched: [Int]
        if wholeDocument {
            touched = Array(0..<lines.count)
        } else {
            let starts = EditorKeyHandling.lineStarts(in: text, range: sel)
            guard !starts.isEmpty else { return nil }
            var indices: [Int] = []
            var lineStart = 0
            var i = 0
            for start in starts {
                while i < lines.count, lineStart < start {
                    lineStart += (lines[i].content as NSString).length + (lines[i].terminator as NSString).length
                    i += 1
                }
                if i < lines.count { indices.append(i) }
            }
            touched = indices
        }
        guard let first = touched.first, let last = touched.last else { return nil }

        var state = State()
        for i in 0..<first {
            _ = reindentLine(lines[i].content, unit: unit, state: &state)
        }

        if !wholeDocument, let neighbor = (0..<first).reversed().first(where: { !isBlank($0, lines: lines) }) {
            let visual = leadingUnits(lines[neighbor].content, unit: unit, tabWidth: tabWidth)
            let scan = scanLine(lines[neighbor].content)
            state.depth = max(0, visual + scan.leadingCloses + scan.indentOpens - scan.indentCloses)
            state.itemContinuation = scan.opensItemContinuation
        } else if first == 0 {
            state.depth = 0
            state.itemContinuation = false
        }

        let oldContents = touched.map { lines[$0].content }
        let newContents = reindent(oldContents.map { $0[...] }, unit: unit, incoming: state)
        let oldBlock = zip(oldContents, touched.map { lines[$0].terminator }).map { $0 + $1 }.joined()
        let newBlock = zip(newContents, touched.map { lines[$0].terminator }).map { $0 + $1 }.joined()

        var loc = 0
        for i in 0..<first {
            loc += (lines[i].content as NSString).length + (lines[i].terminator as NSString).length
        }
        var end = loc
        for i in first...last {
            end += (lines[i].content as NSString).length + (lines[i].terminator as NSString).length
        }
        let range = NSRange(location: loc, length: end - loc)
        guard (oldBlock as NSString).length == range.length else { return nil }
        if oldBlock == newBlock { return nil }

        let mapped = mapSelection(sel, replaceStart: loc, oldLines: Array(lines[first...last]), newContents: newContents)
        return Plan(range: range, replacement: newBlock, selection: mapped)
    }

    // MARK: line rewrite

    private static func reindentLine(_ line: String, unit: String, state: inout State) -> String {
        if let name = state.preserved {
            if isCloser(line, for: name) {
                let rewritten = indent(line, depth: state.preserveDepth, unit: unit)
                state.preserved = nil
                state.itemContinuation = false
                return rewritten
            }
            return line
        }

        let scan = scanLine(line)
        if scan.isBlank { return "" }

        let thisDepth = max(0, state.depth - scan.leadingCloses)
        // Nested `\begin` under an `\item` stays at environment depth, not continuation depth.
        let extra = (state.itemContinuation && !scan.isItem && !scan.closesEnvironment
                     && scan.indentOpens == 0 && scan.preservedBegin == nil) ? 1 : 0
        let rewritten = indent(line, depth: thisDepth + extra, unit: unit)

        state.depth = max(0, state.depth + scan.indentOpens - scan.indentCloses)
        if let begin = scan.preservedBegin, scan.preservedEnd != begin {
            state.preserved = begin
            state.preserveDepth = thisDepth
        }
        if scan.isItem {
            state.itemContinuation = true
        } else if scan.closesEnvironment || scan.indentOpens > 0 {
            state.itemContinuation = false
        }
        return rewritten
    }

    private static func indent(_ line: String, depth: Int, unit: String) -> String {
        let ns = line as NSString
        let n = ns.length
        var i = 0
        while i < n {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        if i == n { return "" }
        let body = ns.substring(from: i)
        if depth <= 0 { return body }
        return String(repeating: unit, count: depth) + body
    }

    // MARK: scan

    struct LineScan: Equatable {
        var indentOpens = 0
        var indentCloses = 0
        var leadingCloses = 0
        var isItem = false
        var isBlank = false
        var closesEnvironment = false
        var opensItemContinuation = false
        var preservedBegin: String?
        var preservedEnd: String?
    }

    static func scanLine(_ line: String) -> LineScan {
        var scan = LineScan()
        let ns = line as NSString
        let n = ns.length
        var i = 0
        while i < n {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        if i == n {
            scan.isBlank = true
            return scan
        }
        var leading = true
        while i < n {
            let c = ns.character(at: i)
            if c == 0x20 || c == 0x09 { i += 1; continue }
            if c == 0x25 { break } // '%'
            if c == 0x5C { // '\'
                i += 1
                guard i < n else { break }
                if !isLetter(ns.character(at: i)) {
                    i += 1 // `\{`, `\%`, `\\`
                    leading = false
                    continue
                }
                let start = i
                while i < n, isLetter(ns.character(at: i)) { i += 1 }
                var name = ns.substring(with: NSRange(location: start, length: i - start))
                if name == "verb", i < n, ns.character(at: i) == 0x2A {
                    name = "verb*"
                    i += 1
                }
                if name == "verb" || name == "verb*" {
                    skipVerb(ns, n, &i)
                    leading = false
                    continue
                }
                if name == "begin" || name == "end" {
                    if let env = readEnvName(ns, n, &i) {
                        if name == "begin" {
                            leading = false
                            if preservedBodyEnvironments.contains(env) {
                                scan.preservedBegin = env
                            } else if !flatEnvironments.contains(env) {
                                scan.indentOpens += 1
                            }
                        } else {
                            scan.closesEnvironment = true
                            if preservedBodyEnvironments.contains(env) {
                                scan.preservedEnd = env
                                leading = false
                            } else if !flatEnvironments.contains(env) {
                                scan.indentCloses += 1
                                if leading { scan.leadingCloses += 1 }
                            } else {
                                leading = false
                            }
                        }
                    } else {
                        leading = false
                    }
                    continue
                }
                if name == "item" {
                    if leading { scan.isItem = true }
                    leading = false
                    continue
                }
                leading = false
                continue
            }
            if c == 0x7B { // '{'
                scan.indentOpens += 1
                leading = false
                i += 1
                continue
            }
            if c == 0x7D { // '}'
                scan.indentCloses += 1
                if leading { scan.leadingCloses += 1 }
                i += 1
                continue
            }
            leading = false
            i += 1
        }
        scan.opensItemContinuation = scan.isItem
        return scan
    }

    private static func isCloser(_ line: String, for name: String) -> Bool {
        let ns = line as NSString
        let n = ns.length
        var i = 0
        while i < n {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        guard i < n, ns.character(at: i) == 0x5C else { return false }
        i += 1
        let start = i
        while i < n, isLetter(ns.character(at: i)) { i += 1 }
        guard ns.substring(with: NSRange(location: start, length: i - start)) == "end" else { return false }
        return readEnvName(ns, n, &i) == name
    }

    private static func readEnvName(_ ns: NSString, _ n: Int, _ i: inout Int) -> String? {
        while i < n {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        guard i < n, ns.character(at: i) == 0x7B else { return nil }
        i += 1
        let start = i
        while i < n {
            let c = ns.character(at: i)
            if c == 0x7D || c == 0x0A { break }
            i += 1
        }
        guard i < n, ns.character(at: i) == 0x7D else { return nil }
        let name = ns.substring(with: NSRange(location: start, length: i - start))
        i += 1
        return name.isEmpty ? nil : name
    }

    private static func skipVerb(_ ns: NSString, _ n: Int, _ i: inout Int) {
        guard i < n else { return }
        let d = ns.character(at: i)
        i += 1
        while i < n, ns.character(at: i) != d { i += 1 }
        if i < n { i += 1 }
    }

    private static func isLetter(_ c: unichar) -> Bool {
        (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A)
    }

    private static func isBlank(_ i: Int, lines: [SplitLine]) -> Bool {
        let ns = lines[i].content as NSString
        let n = ns.length
        var k = 0
        while k < n {
            let c = ns.character(at: k)
            if c != 0x20 && c != 0x09 { return false }
            k += 1
        }
        return true
    }

    static func leadingUnits(_ line: String, unit: String, tabWidth: Int) -> Int {
        let ns = line as NSString
        var cols = 0
        for k in 0..<ns.length {
            let c = ns.character(at: k)
            if c == 0x20 { cols += 1 }
            else if c == 0x09 { cols += tabWidth }
            else { break }
        }
        let unitCols = unit == "\t" ? tabWidth : max(1, unit.count)
        return cols / unitCols
    }

    // MARK: split / map

    struct SplitLine {
        var content: String
        var terminator: String
    }

    static func splitLines(_ ns: NSString) -> [SplitLine] {
        let n = ns.length
        guard n > 0 else { return [] }
        var out: [SplitLine] = []
        out.reserveCapacity(max(1, n / 40))
        var loc = 0
        while loc < n {
            var end = loc
            while end < n {
                let c = ns.character(at: end)
                if c == 0x0D {
                    let termLen = (end + 1 < n && ns.character(at: end + 1) == 0x0A) ? 2 : 1
                    out.append(SplitLine(content: ns.substring(with: NSRange(location: loc, length: end - loc)),
                                         terminator: ns.substring(with: NSRange(location: end, length: termLen))))
                    loc = end + termLen
                    end = loc
                    break
                }
                if c == 0x0A {
                    out.append(SplitLine(content: ns.substring(with: NSRange(location: loc, length: end - loc)),
                                         terminator: "\n"))
                    loc = end + 1
                    end = loc
                    break
                }
                end += 1
            }
            if end == n, loc < n {
                out.append(SplitLine(content: ns.substring(from: loc), terminator: ""))
                break
            }
        }
        return out
    }

    private static func mapSelection(_ sel: NSRange, replaceStart: Int, oldLines: [SplitLine], newContents: [String]) -> NSRange {
        func map(_ pos: Int) -> Int {
            var rel = pos - replaceStart
            if rel < 0 { return replaceStart }
            var newOffset = 0
            for (old, newContent) in zip(oldLines, newContents) {
                let oldNS = old.content as NSString
                let termNS = old.terminator as NSString
                let lineLen = oldNS.length + termNS.length
                let newNS = newContent as NSString
                if rel < lineLen {
                    if rel <= oldNS.length {
                        let anchor = contentAnchor(in: old.content, utf16Offset: rel)
                        let stripped = old.content.drop { $0 == " " || $0 == "\t" }
                        let cap = (String(stripped) as NSString).length
                        let indent = newNS.length - cap
                        return replaceStart + newOffset + max(0, indent) + min(anchor, cap)
                    }
                    let intoTerm = rel - oldNS.length
                    return replaceStart + newOffset + newNS.length + min(intoTerm, termNS.length)
                }
                rel -= lineLen
                newOffset += newNS.length + termNS.length
            }
            return replaceStart + newOffset + rel
        }
        if sel.length == 0 {
            return NSRange(location: map(sel.location), length: 0)
        }
        let a = map(sel.location)
        let b = map(NSMaxRange(sel))
        return NSRange(location: min(a, b), length: abs(b - a))
    }

    /// Offset of `utf16Offset` into the non-whitespace tail of `line`. An
    /// offset in the indent (or at the first content character) maps to 0.
    static func contentAnchor(in line: String, utf16Offset: Int) -> Int {
        let ns = line as NSString
        var i = 0
        while i < ns.length {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        if utf16Offset <= i { return 0 }
        return utf16Offset - i
    }
}

// MARK: - menu / first-responder hooks

enum EditorIndentationAction {
    static func reindentLines() {
        NSApp.sendAction(#selector(CompletingTextView.reindentSelectedLines(_:)), to: nil, from: nil)
    }
    static func reindentDocument() {
        NSApp.sendAction(#selector(CompletingTextView.reindentWholeDocument(_:)), to: nil, from: nil)
    }
}

extension CompletingTextView {
    /// Reindents the selected lines (or the caret's line) as one undo step.
    @objc func reindentSelectedLines(_ sender: Any?) {
        applyReindent(wholeDocument: false)
    }

    /// Reindents the whole buffer as one undo step.
    @objc func reindentWholeDocument(_ sender: Any?) {
        applyReindent(wholeDocument: true)
    }

    fileprivate func applyReindent(wholeDocument: Bool) {
        guard !hasMarkedText() else { return }
        let unit = EditorPreferences.shared.indentString
        let tabWidth = EditorPreferences.shared.tabWidth
        guard let plan = EditorIndentation.plan(in: string, selection: selectedRange(), unit: unit,
                                                tabWidth: tabWidth, wholeDocument: wholeDocument) else { return }
        breakUndoCoalescing()
        insertText(plan.replacement, replacementRange: plan.range)
        setSelectedRange(plan.selection)
        undoManager?.setActionName(wholeDocument ? "Re-indent Document" : "Re-indent Lines")
        breakUndoCoalescing()
    }
}
