import Foundation
import FlashTeXProtocol

/// The Return key and line commands of the LaTeX editor, shared by the Mac
/// (`EditorIntelligence` forwards here) and the iPad. All pure functions over
/// an `NSString` and UTF-16 offsets; `EnvironmentEditingRules` decides what a
/// new body line looks like.
public enum LaTeXEditing {
    // MARK: document scans (UTF-8 bytes)

    /// A `\begin{name}` before the caret that is still open there.
    public struct OpenEnvironment: Equatable {
        public let name: String
        /// UTF-8 offset of its backslash.
        public let byte: Int
        public init(name: String, byte: Int) { self.name = name; self.byte = byte }
    }

    /// `\begin{X}` before `byte` with no matching `\end{X}` before it,
    /// outermost first — the same scan as the Mac's `Completion.openEnvironments`
    /// (control words are ASCII letters; a `{arg}` counts only when it follows
    /// the name directly and closes on the same line).
    public static func openEnvironments(in text: String, beforeByte byte: Int) -> [OpenEnvironment] {
        var stack: [OpenEnvironment] = []
        var copy = text
        copy.withUTF8 { b in
            let limit = min(max(byte, 0), b.count)
            var i = 0
            while i < limit {
                guard b[i] == 0x5C else { i += 1; continue }
                let nameStart = i + 1
                var j = nameStart
                while j < b.count, isAsciiLetter(b[j]) { j += 1 }
                guard j > nameStart else { i = j + 1; continue }
                let isBegin = j - nameStart == 5 && b[nameStart] == 0x62 && b[nameStart + 1] == 0x65 && b[nameStart + 2] == 0x67 && b[nameStart + 3] == 0x69 && b[nameStart + 4] == 0x6E
                let isEnd = j - nameStart == 3 && b[nameStart] == 0x65 && b[nameStart + 1] == 0x6E && b[nameStart + 2] == 0x64
                if (isBegin || isEnd), j < b.count, b[j] == 0x7B {
                    var k = j + 1
                    while k < b.count, b[k] != 0x7D, b[k] != 0x7B, b[k] != 0x5C, b[k] != 0x0A { k += 1 }
                    if k < b.count, b[k] == 0x7D {
                        let env = String(decoding: UnsafeBufferPointer(rebasing: b[(j + 1)..<k]), as: UTF8.self)
                        if isBegin { stack.append(OpenEnvironment(name: env, byte: i)) }
                        else if let idx = stack.lastIndex(where: { $0.name == env }) { stack.remove(at: idx) }
                        j = k + 1
                    }
                }
                i = j
            }
        }
        return stack
    }

    @inline(__always) static func isAsciiLetter(_ c: UInt8) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }

    /// UTF-8 offset of a UTF-16 caret, or nil when out of range or inside a
    /// surrogate pair (the contract coordinate; FlashTeXProtocol).
    public static func utf8Offset(of caretUTF16: Int, in text: String) -> Int? {
        guard caretUTF16 >= 0 else { return nil }
        return text.utf8ByteRange(of: NSRange(location: caretUTF16, length: 0))?.start
    }

    // MARK: Return key

    public struct NewlineInsertion: Equatable {
        /// Text replacing the caret.
        public var text: String
        /// Caret position after the insertion, relative to its start.
        public var caretOffset: Int
        /// The environment closed by this insertion, if any.
        public var closedEnvironment: String?
        public init(text: String, caretOffset: Int, closedEnvironment: String?) {
            self.text = text; self.caretOffset = caretOffset; self.closedEnvironment = closedEnvironment
        }
    }

    /// The Return-key insertion at `caret` (an empty selection): a newline
    /// plus the current line's leading whitespace; after a line that opens
    /// an environment (`\begin{env}` with optional arguments and nothing
    /// else after it) one more `indentUnit` when `rules` indent that body,
    /// then the body's line template (`\item ` in a list), and — when
    /// `closeEnvironments` and the buffer has fewer `\end{env}` than
    /// `\begin{env}` — the matching `\end{env}` on the line after the caret.
    /// Inside an environment whose template is a command, Return on a line
    /// that starts with that command and has content after it repeats the
    /// template (the next `\item`); on a bare `\item` line it just breaks.
    public static func newline(in text: NSString, caret: Int, indentUnit: String, closeEnvironments: Bool,
                               rules: EnvironmentEditingRules = .conventional) -> NewlineInsertion {
        let caret = max(0, min(caret, text.length))
        var lineStart = caret
        while lineStart > 0, text.character(at: lineStart - 1) != 0x0A { lineStart -= 1 }
        var lineEnd = caret
        while lineEnd < text.length, text.character(at: lineEnd) != 0x0A { lineEnd += 1 }
        let prefix = text.substring(with: NSRange(location: lineStart, length: caret - lineStart))
        let suffix = text.substring(with: NSRange(location: caret, length: lineEnd - caret))
        let indent = String(prefix.prefix { $0 == " " || $0 == "\t" })
        let lineIsDone = suffix.allSatisfy { $0 == " " || $0 == "\t" || $0 == "\r" }
        // `\item …` Return inside a list continues it with a new `\item ` (or
        // whatever the enclosing environment's template is); a bare `\item`
        // line (nothing typed) just breaks the line.
        if lineIsDone, let item = templateContinuation(in: text, caret: caret, linePrefix: prefix, rules: rules) {
            let insertion = "\n" + indent + item
            let caretOffset = 1 + indent.utf16.count + EnvironmentEditingRules.caretOffset(in: item)
            return NewlineInsertion(text: insertion, caretOffset: caretOffset, closedEnvironment: nil)
        }
        guard let env = openingEnvironment(inLinePrefix: prefix) else {
            return NewlineInsertion(text: "\n" + indent, caretOffset: 1 + indent.utf16.count, closedEnvironment: nil)
        }
        let template = rules.newLineText(in: env)
        var insertion = "\n" + indent + (rules.indentsBody(of: env) ? indentUnit : "") + template
        let caretOffset = insertion.utf16.count - template.utf16.count + EnvironmentEditingRules.caretOffset(in: template)
        var closed: String?
        if closeEnvironments, lineIsDone,
           occurrences(of: "\\begin{\(env)}", in: text) > occurrences(of: "\\end{\(env)}", in: text) {
            insertion += "\n" + indent + "\\end{\(env)}"
            closed = env
        }
        return NewlineInsertion(text: insertion, caretOffset: caretOffset, closedEnvironment: closed)
    }

    /// The line template to repeat after `linePrefix`: the innermost open
    /// environment's (`rules.newLineText`) when the line starts with that
    /// template's command and has content after it; failing an enclosing
    /// rule, a plain `\item …` line still continues with `\item ` (a list
    /// environment the rules do not name — `enumitem`'s `tasks`, a class's
    /// own list). Nil when the line is a bare command (the user is leaving
    /// the list) or anything else.
    public static func templateContinuation(in text: NSString, caret: Int, linePrefix prefix: String,
                                            rules: EnvironmentEditingRules) -> String? {
        let body = prefix.drop { $0 == " " || $0 == "\t" }
        guard body.hasPrefix("\\") else { return nil }
        let string = text as String
        let enclosing = utf8Offset(of: caret, in: string)
            .flatMap { openEnvironments(in: string, beforeByte: $0).last?.name }
        let template = enclosing.map(rules.newLineText(in:)) ?? ""
        if let command = EnvironmentEditingRules.command(of: template), hasContent(after: command, in: body) {
            // The optional-argument form (`\item[term] text`) continues as
            // `\item[] `, whatever the rule's own template says.
            if body.dropFirst(command.count).first == "[", !template.contains("[") { return command + "[] " }
            return template
        }
        if template.isEmpty, hasContent(after: "\\item", in: body) {
            return body.dropFirst(5).first == "[" ? "\\item[] " : "\\item "
        }
        return nil
    }

    /// Whether `body` starts with `command` (not a longer command:
    /// `\itemize` is not `\item`) followed, after any `[…]`/`{…}` groups,
    /// by something other than whitespace.
    private static func hasContent(after command: String, in body: Substring) -> Bool {
        guard body.hasPrefix(command) else { return false }
        var rest = body.dropFirst(command.count)
        if let c = rest.first, c.isLetter { return false }
        while let open = rest.first, open == "[" || open == "{" {
            let closer: Character = open == "[" ? "]" : "}"
            guard let close = rest.firstIndex(of: closer) else { return false }
            rest = rest[rest.index(after: close)...]
        }
        return rest.contains(where: { $0 != " " && $0 != "\t" })
    }

    /// `\item ` (or `\item[…] ` for a description entry) when the line prefix
    /// is a list entry with content after the `\item`; nil for a bare `\item`
    /// (the user is leaving the list) or any other line. The rule-free form
    /// of `templateContinuation`, kept for callers without a buffer.
    public static func itemContinuation(inLinePrefix prefix: String) -> String? {
        templateContinuation(in: prefix as NSString, caret: (prefix as NSString).length, linePrefix: prefix,
                             rules: EnvironmentEditingRules(indentByDefault: true, rules: []))
    }

    /// The environment a line prefix opens: its last `\begin{name}` followed
    /// only by optional `[...]`/`{...}` arguments and whitespace, unless the
    /// same prefix also closes it or is a comment.
    public static func openingEnvironment(inLinePrefix prefix: String) -> String? {
        guard let beginRange = prefix.range(of: "\\begin{", options: .backwards) else { return nil }
        let head = prefix[..<beginRange.lowerBound]
        if head.contains("%") && !head.contains("\\%") { return nil } // crude: a comment before \begin
        guard let close = prefix[beginRange.upperBound...].firstIndex(of: "}") else { return nil }
        let name = String(prefix[beginRange.upperBound..<close])
        guard !name.isEmpty, !name.contains("\n"), !name.contains("\\") else { return nil }
        if SyntaxHighlighter.verbatimEnvironments.contains(name) || name == "document" { return nil }
        // After the name: optional argument groups then whitespace only.
        var rest = Substring(prefix[prefix.index(after: close)...])
        while true {
            rest = rest.drop { $0 == " " || $0 == "\t" }
            guard let open = rest.first, open == "[" || open == "{" else { break }
            let closer: Character = open == "[" ? "]" : "}"
            var depth = 0
            var i = rest.startIndex
            var endIndex: Substring.Index?
            while i < rest.endIndex {
                let c = rest[i]
                if c == "\\" { i = rest.index(i, offsetBy: 2, limitedBy: rest.endIndex) ?? rest.endIndex; continue }
                if c == open { depth += 1 } else if c == closer { depth -= 1; if depth == 0 { endIndex = i; break } }
                i = rest.index(after: i)
            }
            guard let endIndex else { return nil }
            rest = rest[rest.index(after: endIndex)...]
        }
        guard rest.allSatisfy({ $0 == " " || $0 == "\t" || $0 == "\r" }) else { return nil }
        if prefix.contains("\\end{\(name)}") { return nil }
        return name
    }

    // MARK: ⌘/ line comment

    public struct CommentToggle: Equatable {
        public var range: NSRange
        public var replacement: String
        public var selection: NSRange
        public init(range: NSRange, replacement: String, selection: NSRange) {
            self.range = range; self.replacement = replacement; self.selection = selection
        }
    }

    /// Toggles `% ` at the indentation of every line the selection touches:
    /// when every non-blank touched line is already commented the markers are
    /// removed (`% ` or `%`), otherwise each non-blank line gets `% ` after
    /// its leading whitespace. Blank lines are left alone; the selection is
    /// moved to cover the same lines. Nil when nothing would change.
    public static func toggleComment(in text: NSString, selection: NSRange) -> CommentToggle? {
        let sel = NSRange(location: max(0, min(selection.location, text.length)),
                          length: max(0, min(selection.length, text.length - min(selection.location, text.length))))
        var start = sel.location
        while start > 0, text.character(at: start - 1) != 0x0A { start -= 1 }
        var end = NSMaxRange(sel)
        if sel.length > 0, end > start, text.character(at: end - 1) == 0x0A { end -= 1 } // a selection ending at a line start excludes that line
        while end < text.length, text.character(at: end) != 0x0A { end += 1 }
        let block = text.substring(with: NSRange(location: start, length: end - start))
        let lines = block.components(separatedBy: "\n")
        let nonBlank = lines.filter { $0.contains { $0 != " " && $0 != "\t" } }
        guard !nonBlank.isEmpty else { return nil }
        let allCommented = nonBlank.allSatisfy { $0.drop { $0 == " " || $0 == "\t" }.hasPrefix("%") }
        let out = lines.map { line -> String in
            let indent = line.prefix { $0 == " " || $0 == "\t" }
            let body = line.dropFirst(indent.count)
            if !body.contains(where: { $0 != " " && $0 != "\t" }) { return line }
            if allCommented {
                let stripped = body.hasPrefix("% ") ? body.dropFirst(2) : body.dropFirst(1)
                return String(indent) + String(stripped)
            }
            return String(indent) + "% " + String(body)
        }.joined(separator: "\n")
        let range = NSRange(location: start, length: end - start)
        let newLength = (out as NSString).length
        // A caret stays on its line (shifted by that line's change); a selection covers the toggled lines.
        let selection = sel.length == 0
            ? NSRange(location: min(max(start, sel.location + newLength - range.length), start + newLength), length: 0)
            : NSRange(location: start, length: newLength)
        return CommentToggle(range: range, replacement: out, selection: selection)
    }

    public static func occurrences(of needle: String, in text: NSString) -> Int {
        var count = 0
        var search = NSRange(location: 0, length: text.length)
        while true {
            let r = text.range(of: needle, options: .literal, range: search)
            guard r.location != NSNotFound else { return count }
            count += 1
            search = NSRange(location: NSMaxRange(r), length: text.length - NSMaxRange(r))
        }
    }

    // MARK: indent and outdent (Mac Tab / ⇧Tab and Vim `>>`; iPad ⌘] / ⌘[)

    /// One text edit in the *original* text's coordinates.
    public struct LineEdit: Equatable, Sendable {
        public var range: NSRange
        public var replacement: String
        public init(range: NSRange, replacement: String) { self.range = range; self.replacement = replacement }
    }

    /// UTF-16 offsets of the start of every line `range` touches (at least
    /// one: the line a caret sits on). A range that ends exactly at the start
    /// of a following line does not pull that line in (selecting through the
    /// end of line 1 touches line 1 only). Any line terminator counts.
    public static func lineStarts(in text: NSString, range: NSRange) -> [Int] {
        guard range.location >= 0, range.length >= 0, NSMaxRange(range) <= text.length else { return [] }
        var end = NSMaxRange(range)
        if range.length > 0, end < text.length, text.lineRange(for: NSRange(location: end, length: 0)).location == end {
            end -= 1 // ends exactly at the start of a following line: exclude that line
        }
        var starts: [Int] = []
        var loc = text.lineRange(for: NSRange(location: range.location, length: 0)).location
        while true {
            starts.append(loc)
            let lineEnd = NSMaxRange(text.lineRange(for: NSRange(location: loc, length: 0)))
            if lineEnd > end || lineEnd >= text.length || lineEnd == loc { break }
            loc = lineEnd
        }
        return starts
    }

    /// One `unit` at the start of every line `range` touches. Across several
    /// lines, blank (whitespace-only) lines are left alone, so indenting a
    /// block never leaves trailing whitespace; a caret on a blank line still
    /// indents that line. Nil when `range` or `unit` is unusable.
    public static func indentEdits(in text: NSString, range: NSRange, unit: String) -> (edits: [LineEdit], selection: NSRange)? {
        guard !unit.isEmpty else { return nil }
        let starts = lineStarts(in: text, range: range)
        guard let first = starts.first else { return nil }
        let edits = starts.filter { starts.count == 1 || !isBlankLine(at: $0, in: text) }
            .map { LineEdit(range: NSRange(location: $0, length: 0), replacement: unit) }
        return (edits, selectionAfter(edits, range: range, firstLineStart: first))
    }

    /// Up to one indent level removed from the start of every line `range`
    /// touches: the `unit` itself, else one leading tab (a tab is one level
    /// whatever the unit), else, for a spaces unit, up to its width of
    /// leading spaces. A tab unit never guesses how many spaces a tab is.
    /// Nil when `range` or `unit` is unusable; an empty (non-nil) `edits`
    /// means there was nothing to remove.
    public static func outdentEdits(in text: NSString, range: NSRange, unit: String) -> (edits: [LineEdit], selection: NSRange)? {
        guard !unit.isEmpty else { return nil }
        let starts = lineStarts(in: text, range: range)
        guard let first = starts.first else { return nil }
        let unitLength = (unit as NSString).length
        let spaceUnit = unit.allSatisfy { $0 == " " }
        var edits: [LineEdit] = []
        for start in starts {
            var count = 0
            if start + unitLength <= text.length, text.substring(with: NSRange(location: start, length: unitLength)) == unit {
                count = unitLength
            } else if start < text.length, text.character(at: start) == 0x09 {
                count = 1
            } else if spaceUnit {
                while count < unitLength, start + count < text.length, text.character(at: start + count) == 0x20 { count += 1 }
            }
            guard count > 0 else { continue }
            edits.append(LineEdit(range: NSRange(location: start, length: count), replacement: ""))
        }
        return (edits, selectionAfter(edits, range: range, firstLineStart: first))
    }

    /// `indentEdits` / `outdentEdits` as a single replacement of the span
    /// they touch (the iPad's ⌘] / ⌘[). Nil when nothing would change.
    public static func indent(in text: NSString, selection: NSRange, unit: String, outdent: Bool) -> CommentToggle? {
        let sel = NSRange(location: max(0, min(selection.location, text.length)),
                          length: max(0, min(selection.length, text.length - min(selection.location, text.length))))
        let plan = outdent ? outdentEdits(in: text, range: sel, unit: unit) : indentEdits(in: text, range: sel, unit: unit)
        guard let plan, let lo = plan.edits.map(\.range.location).min(),
              let hi = plan.edits.map({ NSMaxRange($0.range) }).max() else { return nil }
        let out = NSMutableString(string: text.substring(with: NSRange(location: lo, length: hi - lo)))
        for edit in plan.edits.sorted(by: { $0.range.location > $1.range.location }) {
            out.replaceCharacters(in: NSRange(location: edit.range.location - lo, length: edit.range.length), with: edit.replacement)
        }
        return CommentToggle(range: NSRange(location: lo, length: hi - lo), replacement: out as String, selection: plan.selection)
    }

    private static func isBlankLine(at start: Int, in text: NSString) -> Bool {
        var i = start
        while i < text.length {
            switch text.character(at: i) {
            case 0x20, 0x09: i += 1
            case 0x0A, 0x0D: return true
            default: return false
            }
        }
        return true
    }

    /// Where `offset` lands once the line-start `edits` are applied: shifted
    /// by every edit wholly before it, or — inside a run of indentation an
    /// edit removes — moved to where that run started, so it never slides
    /// into the line's text or onto the next line.
    private static func mapped(_ offset: Int, through edits: [LineEdit]) -> Int {
        var shift = 0
        for edit in edits.sorted(by: { $0.range.location < $1.range.location }) {
            if NSMaxRange(edit.range) <= offset {
                shift += (edit.replacement as NSString).length - edit.range.length
            } else {
                if edit.range.location < offset { return edit.range.location + shift }
                break
            }
        }
        return offset + shift
    }

    /// The selection after `edits`: a caret stays a caret on its line; a
    /// selection starts at its first line's start (so it covers the new
    /// indentation) and its end moves with the text, so a block selected
    /// through its last newline stays selected through it.
    private static func selectionAfter(_ edits: [LineEdit], range: NSRange, firstLineStart: Int) -> NSRange {
        guard range.length > 0 else { return NSRange(location: mapped(range.location, through: edits), length: 0) }
        let end = mapped(NSMaxRange(range), through: edits)
        return NSRange(location: firstLineStart, length: max(0, end - firstLineStart))
    }
}
