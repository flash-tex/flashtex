import AppKit
import SwiftUI

/// Editor line commands (lane editor-line-commands). Pure over the buffer
/// and selection; `EditorMenu.swift` and the `CompletingTextView` hooks below
/// apply a plan as one undo step. SourceEditorView.swift is left alone
/// (folding lane).
///
/// Shortcut choice (conflicts reported, not stolen):
/// - Duplicate Line/Selection is ⌘D (Xcode Duplicate). ⇧⌘D is already
///   Navigate ▸ Go to Matching.
/// - Move Line Up/Down is ⌥⌘↑ / ⌥⌘↓. Xcode's ⌥⌘[ / ⌥⌘] are already
///   Navigate ▸ Previous/Next Occurrence; VS Code's ⌥↑ / ⌥↓ would steal
///   paragraph motion. ⌘L is Go to Line (#412); ⌃I is Re-indent (#413).
/// - Delete Line is ⌃⌘K. ⇧⌘K is File ▸ Attach Built Compiler.
/// - Join Lines is ⌃J (VS Code). ⌘J is Jump to Selection; ⌃⌘J is Go to
///   Definition. VimMode's `J` / `gJ` stay modal and are not these menus;
///   its `moveLines` scrolls by pages, it does not move text.
/// - Sort Lines Ascending/Descending and Trim Trailing Whitespace have no
///   key equivalent (palette / menu only).
enum EditorLineCommands {
    struct Plan: Equatable {
        var range: NSRange
        var replacement: String
        var selection: NSRange
    }

    struct SplitLine: Equatable {
        var content: String
        var terminator: String
    }

    // MARK: public commands

    static func duplicate(in text: String, selection: NSRange) -> Plan? {
        mutate(text, selection: selection) { lines, first, last, endedNL, term in
            var newLines = lines
            let block = Array(lines[first...last])
            newLines.insert(contentsOf: block, at: last + 1)
            let joined = join(newLines, originalEndedWithNewline: endedNL, defaultTerm: term)
            let copyStart = offset(of: last + 1, in: newLines, endedNL: endedNL, defaultTerm: term)
            let origStart = offset(of: first, in: lines, endedNL: endedNL, defaultTerm: term)
            var sel = mappedSelection(selection, from: origStart, to: copyStart)
            let n = (joined as NSString).length
            // Relative mapping of a caret at original EOF on an unterminated
            // last line lands at the new EOF, which is after the copy, not on
            // it. Snap onto the copy.
            if sel.length == 0, sel.location >= n, copyStart < n {
                sel = NSRange(location: copyStart, length: 0)
            }
            return (joined, sel)
        }
    }

    static func move(in text: String, selection: NSRange, down: Bool) -> Plan? {
        mutate(text, selection: selection) { lines, first, last, endedNL, term in
            if down {
                guard last + 1 < lines.count else { return nil }
                var newLines = lines
                let next = newLines.remove(at: last + 1)
                newLines.insert(next, at: first)
                let joined = join(newLines, originalEndedWithNewline: endedNL, defaultTerm: term)
                let origStart = offset(of: first, in: lines, endedNL: endedNL, defaultTerm: term)
                let newStart = offset(of: first + 1, in: newLines, endedNL: endedNL, defaultTerm: term)
                return (joined, mappedSelection(selection, from: origStart, to: newStart))
            } else {
                guard first > 0 else { return nil }
                var newLines = lines
                let prev = newLines.remove(at: first - 1)
                newLines.insert(prev, at: last)
                let joined = join(newLines, originalEndedWithNewline: endedNL, defaultTerm: term)
                let origStart = offset(of: first, in: lines, endedNL: endedNL, defaultTerm: term)
                let newStart = offset(of: first - 1, in: newLines, endedNL: endedNL, defaultTerm: term)
                return (joined, mappedSelection(selection, from: origStart, to: newStart))
            }
        }
    }

    static func deleteLines(in text: String, selection: NSRange) -> Plan? {
        mutate(text, selection: selection) { lines, first, last, endedNL, term in
            var newLines = lines
            newLines.removeSubrange(first...last)
            let joined = join(newLines, originalEndedWithNewline: endedNL, defaultTerm: term)
            let caret = offset(of: first, in: newLines, endedNL: endedNL, defaultTerm: term)
            let loc = min(caret, (joined as NSString).length)
            return (joined, NSRange(location: loc, length: 0))
        }
    }

    /// Joins the touched lines, or the caret's line with the next one. One
    /// space between survivors. Every line after the first is left-trimmed.
    /// Every line except the last is right-trimmed and has a trailing `%`
    /// dropped only when that `%` ends the line (TeX "comment out the
    /// newline") and is not escaped — the same rule as a two-line join's
    /// left-hand line, applied to each interior join. A `%` that does not
    /// end its line is kept, so later survivors sit in that comment. The
    /// last line's trailing whitespace is left alone.
    static func joinLines(in text: String, selection: NSRange) -> Plan? {
        mutate(text, selection: selection, extendSingleLine: true) { lines, first, last, endedNL, term in
            guard last > first else { return nil }
            let block = Array(lines[first...last])
            var acc = ""
            for (i, line) in block.enumerated() {
                var s = line.content
                if i > 0 { s = ltrim(s) }
                if i < block.count - 1 {
                    s = rtrim(stripTrailingPercentIfLineComment(s))
                }
                acc = i == 0 ? s : joinPair(acc, s)
            }
            var newLines = lines
            let folded = SplitLine(content: acc, terminator: block.last!.terminator)
            newLines.replaceSubrange(first...last, with: [folded])
            let joined = join(newLines, originalEndedWithNewline: endedNL, defaultTerm: term)
            let start = offset(of: first, in: newLines, endedNL: endedNL, defaultTerm: term)
            let caret = start + (acc as NSString).length
            return (joined, NSRange(location: min(caret, (joined as NSString).length), length: 0))
        }
    }

    static func sortLines(in text: String, selection: NSRange, descending: Bool) -> Plan? {
        mutate(text, selection: selection) { lines, first, last, endedNL, term in
            guard last > first else { return nil }
            var block = Array(lines[first...last])
            let ranked = block.enumerated().sorted { a, b in
                let c = a.element.content.localizedStandardCompare(b.element.content)
                if c == .orderedSame { return a.offset < b.offset }
                return descending ? c == .orderedDescending : c == .orderedAscending
            }
            for (i, item) in ranked.enumerated() {
                block[i].content = item.element.content
            }
            var newLines = lines
            newLines.replaceSubrange(first...last, with: block)
            let joined = join(newLines, originalEndedWithNewline: endedNL, defaultTerm: term)
            let start = offset(of: first, in: newLines, endedNL: endedNL, defaultTerm: term)
            let end = last + 1 < newLines.count
                ? offset(of: last + 1, in: newLines, endedNL: endedNL, defaultTerm: term)
                : (joined as NSString).length
            return (joined, NSRange(location: start, length: max(0, end - start)))
        }
    }

    /// Whole-document trim of trailing spaces and tabs. Bodies of
    /// `SyntaxHighlighter.verbatimEnvironments` are copied byte-for-byte; a
    /// line that is only `\\` plus whitespace is left alone (a lone `\\`
    /// line-break with padding).
    static func trimTrailingWhitespace(in text: String, selection: NSRange) -> Plan? {
        let ns = text as NSString
        let sel = clamp(selection, in: ns)
        let lines = splitLines(ns)
        guard !lines.isEmpty else { return nil }
        let endedNL = endsWithNewline(ns)
        let term = defaultTerminator(lines)
        var preserved: String?
        var newLines = lines
        var removals: [NSRange] = []
        var loc = 0
        for i in newLines.indices {
            let original = newLines[i].content
            let contentLen = (original as NSString).length
            let termLen = (newLines[i].terminator as NSString).length
            if let name = preserved {
                if isCloser(original, for: name) {
                    preserved = nil
                    let trimmed = rtrim(original)
                    let newLen = (trimmed as NSString).length
                    if newLen < contentLen {
                        removals.append(NSRange(location: loc + newLen, length: contentLen - newLen))
                    }
                    newLines[i].content = trimmed
                }
                loc += contentLen + termLen
                continue
            }
            if !isProtectedBackslashLine(original) {
                let trimmed = rtrim(original)
                let newLen = (trimmed as NSString).length
                if newLen < contentLen {
                    removals.append(NSRange(location: loc + newLen, length: contentLen - newLen))
                }
                newLines[i].content = trimmed
            }
            if let env = isOpener(newLines[i].content), SyntaxHighlighter.verbatimEnvironments.contains(env) {
                if !isCloser(newLines[i].content, for: env) { preserved = env }
            }
            loc += contentLen + termLen
        }
        let joined = join(newLines, originalEndedWithNewline: endedNL, defaultTerm: term)
        let mapped = mapSelectionSubtractingRemovals(sel, removals: removals, newLength: (joined as NSString).length)
        return makePlan(old: text, new: joined, selection: mapped)
    }

    /// The span of `old`/`new` that actually differs, comparing UTF-16 code
    /// units. `range` is the planned replacement of `old` in the buffer. Nil
    /// when they are identical (callers register no undo step).
    static func trimmedReplacement(old: String, new: String, range: NSRange) -> (range: NSRange, replacement: String)? {
        let oldNS = old as NSString
        let newNS = new as NSString
        guard oldNS.length == range.length else { return nil }
        var prefix = 0
        let shared = min(oldNS.length, newNS.length)
        while prefix < shared, oldNS.character(at: prefix) == newNS.character(at: prefix) {
            prefix += 1
        }
        var suffix = 0
        while prefix + suffix < oldNS.length, prefix + suffix < newNS.length,
              oldNS.character(at: oldNS.length - 1 - suffix) == newNS.character(at: newNS.length - 1 - suffix) {
            suffix += 1
        }
        if prefix == oldNS.length, prefix == newNS.length { return nil }
        return (NSRange(location: range.location + prefix, length: oldNS.length - prefix - suffix),
                newNS.substring(with: NSRange(location: prefix, length: newNS.length - prefix - suffix)))
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

    // MARK: reconstruct / touch

    private static func mutate(_ text: String, selection: NSRange, extendSingleLine: Bool = false,
                               _ body: ([SplitLine], Int, Int, Bool, String) -> (String, NSRange)?) -> Plan? {
        let ns = text as NSString
        let sel = clamp(selection, in: ns)
        let lines = splitLines(ns)
        guard !lines.isEmpty else { return nil }
        let touched = touchedIndices(in: text, lines: lines, selection: sel)
        guard let first = touched.first, var last = touched.last else { return nil }
        if extendSingleLine, first == last, last + 1 < lines.count {
            last += 1
        }
        let endedNL = endsWithNewline(ns)
        let term = defaultTerminator(lines)
        guard let (joined, newSel) = body(lines, first, last, endedNL, term) else { return nil }
        return makePlan(old: text, new: joined, selection: newSel)
    }

    private static func makePlan(old: String, new: String, selection: NSRange) -> Plan? {
        let oldNS = old as NSString
        guard let trimmed = trimmedReplacement(old: old, new: new,
                                               range: NSRange(location: 0, length: oldNS.length)) else { return nil }
        let n = (new as NSString).length
        let loc = max(0, min(selection.location, n))
        let len = max(0, min(selection.length, n - loc))
        return Plan(range: trimmed.range, replacement: trimmed.replacement, selection: NSRange(location: loc, length: len))
    }

    static func touchedIndices(in text: String, lines: [SplitLine], selection: NSRange) -> [Int] {
        let starts = EditorKeyHandling.lineStarts(in: text, range: selection)
        guard !starts.isEmpty else { return [] }
        var indices: [Int] = []
        var loc = 0
        var i = 0
        for start in starts {
            while i < lines.count, loc < start {
                loc += (lines[i].content as NSString).length + (lines[i].terminator as NSString).length
                i += 1
            }
            if i < lines.count, loc == start { indices.append(i) }
        }
        return indices
    }

    static func join(_ lines: [SplitLine], originalEndedWithNewline: Bool, defaultTerm: String) -> String {
        guard !lines.isEmpty else { return "" }
        var s = ""
        for (i, line) in lines.enumerated() {
            s += line.content
            let isLast = i == lines.count - 1
            if isLast {
                if originalEndedWithNewline {
                    s += line.terminator.isEmpty ? defaultTerm : line.terminator
                }
            } else {
                s += line.terminator.isEmpty ? defaultTerm : line.terminator
            }
        }
        return s
    }

    static func offset(of index: Int, in lines: [SplitLine], endedNL: Bool, defaultTerm: String) -> Int {
        guard index > 0 else { return 0 }
        if index >= lines.count {
            return (join(lines, originalEndedWithNewline: endedNL, defaultTerm: defaultTerm) as NSString).length
        }
        return (join(Array(lines[..<index]), originalEndedWithNewline: true, defaultTerm: defaultTerm) as NSString).length
    }

    private static func mappedSelection(_ sel: NSRange, from oldStart: Int, to newStart: Int) -> NSRange {
        let delta = newStart - oldStart
        return NSRange(location: max(0, sel.location + delta), length: sel.length)
    }

    /// Maps `sel` through independent deletions: a location inside a removed
    /// range clamps to that range's start, then every removal that ends at or
    /// before the (clamped) location is subtracted.
    private static func mapSelectionSubtractingRemovals(_ sel: NSRange, removals: [NSRange], newLength: Int) -> NSRange {
        func map(_ pos: Int) -> Int {
            var p = max(0, pos)
            for r in removals {
                if p >= r.location && p < NSMaxRange(r) {
                    p = r.location
                    break
                }
            }
            var subtract = 0
            for r in removals {
                if NSMaxRange(r) <= p { subtract += r.length }
            }
            return max(0, min(p - subtract, newLength))
        }
        if sel.length == 0 { return NSRange(location: map(sel.location), length: 0) }
        let a = map(sel.location)
        let b = map(NSMaxRange(sel))
        return NSRange(location: min(a, b), length: abs(b - a))
    }

    private static func clamp(_ selection: NSRange, in ns: NSString) -> NSRange {
        let loc = max(0, min(selection.location, ns.length))
        return NSRange(location: loc, length: max(0, min(selection.length, ns.length - loc)))
    }

    private static func endsWithNewline(_ ns: NSString) -> Bool {
        guard ns.length > 0 else { return false }
        let c = ns.character(at: ns.length - 1)
        return c == 0x0A || c == 0x0D
    }

    private static func defaultTerminator(_ lines: [SplitLine]) -> String {
        lines.first(where: { !$0.terminator.isEmpty })?.terminator ?? "\n"
    }

    // MARK: join helpers

    static func joinPair(_ left: String, _ right: String) -> String {
        if left.isEmpty { return right }
        if right.isEmpty { return left }
        return left + " " + right
    }

    /// Drops a trailing `%` comment that ends the line (`foo %` / `foo%`),
    /// then the whitespace that followed it. `\%` (odd backslashes) stays.
    static func stripTrailingPercentIfLineComment(_ line: String) -> String {
        let ns = line as NSString
        var end = ns.length
        while end > 0 {
            let c = ns.character(at: end - 1)
            if c == 0x20 || c == 0x09 { end -= 1; continue }
            break
        }
        guard end > 0, ns.character(at: end - 1) == 0x25 else { return line }
        var slashes = 0
        var i = end - 2
        while i >= 0, ns.character(at: i) == 0x5C { slashes += 1; i -= 1 }
        if slashes % 2 == 1 { return line }
        return ns.substring(to: end - 1)
    }

    static func ltrim(_ s: String) -> String {
        let ns = s as NSString
        var i = 0
        while i < ns.length {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        return ns.substring(from: i)
    }

    static func rtrim(_ s: String) -> String {
        let ns = s as NSString
        var end = ns.length
        while end > 0 {
            let c = ns.character(at: end - 1)
            if c != 0x20 && c != 0x09 { break }
            end -= 1
        }
        return end == ns.length ? s : ns.substring(to: end)
    }

    /// A line whose non-whitespace content is exactly `\\`.
    static func isProtectedBackslashLine(_ content: String) -> Bool {
        let ns = content as NSString
        var i = 0
        while i < ns.length {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        guard i + 1 < ns.length, ns.character(at: i) == 0x5C, ns.character(at: i + 1) == 0x5C else { return false }
        i += 2
        while i < ns.length {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { return false }
            i += 1
        }
        return true
    }

    private static func isOpener(_ line: String) -> String? {
        environmentName(line, command: "begin")
    }

    static func isCloser(_ line: String, for name: String) -> Bool {
        environmentName(line, command: "end") == name
    }

    private static func environmentName(_ line: String, command: String) -> String? {
        let ns = line as NSString
        let n = ns.length
        var i = 0
        while i < n {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        guard i < n, ns.character(at: i) == 0x5C else { return nil }
        i += 1
        let start = i
        while i < n, isLetter(ns.character(at: i)) { i += 1 }
        guard ns.substring(with: NSRange(location: start, length: i - start)) == command else { return nil }
        while i < n {
            let c = ns.character(at: i)
            if c != 0x20 && c != 0x09 { break }
            i += 1
        }
        guard i < n, ns.character(at: i) == 0x7B else { return nil }
        i += 1
        let nameStart = i
        while i < n {
            let c = ns.character(at: i)
            if c == 0x7D || c == 0x0A { break }
            i += 1
        }
        guard i < n, ns.character(at: i) == 0x7D else { return nil }
        let name = ns.substring(with: NSRange(location: nameStart, length: i - nameStart))
        return name.isEmpty ? nil : name
    }

    private static func isLetter(_ c: unichar) -> Bool {
        (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A)
    }
}

// MARK: - menu / first-responder hooks

enum EditorLineCommandAction {
    static func duplicate() { NSApp.sendAction(#selector(CompletingTextView.duplicateLines(_:)), to: nil, from: nil) }
    static func moveUp() { NSApp.sendAction(#selector(CompletingTextView.moveLinesUp(_:)), to: nil, from: nil) }
    static func moveDown() { NSApp.sendAction(#selector(CompletingTextView.moveLinesDown(_:)), to: nil, from: nil) }
    static func deleteLines() { NSApp.sendAction(#selector(CompletingTextView.deleteLines(_:)), to: nil, from: nil) }
    static func joinLines() { NSApp.sendAction(#selector(CompletingTextView.joinSelectedLines(_:)), to: nil, from: nil) }
    static func sortAscending() { NSApp.sendAction(#selector(CompletingTextView.sortLinesAscending(_:)), to: nil, from: nil) }
    static func sortDescending() { NSApp.sendAction(#selector(CompletingTextView.sortLinesDescending(_:)), to: nil, from: nil) }
    static func trimTrailingWhitespace() { NSApp.sendAction(#selector(CompletingTextView.trimTrailingWhitespace(_:)), to: nil, from: nil) }
}

extension CompletingTextView {
    @objc func duplicateLines(_ sender: Any?) { applyLineCommand(EditorLineCommands.duplicate(in: string, selection: selectedRange()), actionName: "Duplicate Line") }
    @objc func moveLinesUp(_ sender: Any?) { applyLineCommand(EditorLineCommands.move(in: string, selection: selectedRange(), down: false), actionName: "Move Line Up") }
    @objc func moveLinesDown(_ sender: Any?) { applyLineCommand(EditorLineCommands.move(in: string, selection: selectedRange(), down: true), actionName: "Move Line Down") }
    @objc func deleteLines(_ sender: Any?) { applyLineCommand(EditorLineCommands.deleteLines(in: string, selection: selectedRange()), actionName: "Delete Line") }
    @objc func joinSelectedLines(_ sender: Any?) { applyLineCommand(EditorLineCommands.joinLines(in: string, selection: selectedRange()), actionName: "Join Lines") }
    @objc func sortLinesAscending(_ sender: Any?) { applyLineCommand(EditorLineCommands.sortLines(in: string, selection: selectedRange(), descending: false), actionName: "Sort Lines") }
    @objc func sortLinesDescending(_ sender: Any?) { applyLineCommand(EditorLineCommands.sortLines(in: string, selection: selectedRange(), descending: true), actionName: "Sort Lines") }
    @objc func trimTrailingWhitespace(_ sender: Any?) { applyLineCommand(EditorLineCommands.trimTrailingWhitespace(in: string, selection: selectedRange()), actionName: "Trim Trailing Whitespace") }

    fileprivate func applyLineCommand(_ plan: EditorLineCommands.Plan?, actionName: String) {
        guard !hasMarkedText(), let plan else { return }
        // Structural rewrite of existing source. `insertText` is AppKit's
        // unwrapped edit path; do not send this through `CaretContext.normalize`.
        breakUndoCoalescing()
        insertText(plan.replacement, replacementRange: plan.range)
        setSelectedRange(plan.selection)
        undoManager?.setActionName(actionName)
        breakUndoCoalescing()
    }
}
