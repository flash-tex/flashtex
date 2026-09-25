import Foundation
import FlashTeXProtocol

/// Screen-reader navigation over the editor buffer: lines, words (LaTeX-aware
/// tokens), user-perceived characters, diagnostics at the caret, and rotor-like
/// categories (headings, environments, diagnostics, captures). Every position
/// carries both the AppKit UTF-16 offset and the contract's UTF-8 byte offset.
/// Pure: no AppKit, no view state.
public struct AccessibleEditorModel: Equatable {
    /// A caret position in both coordinate systems.
    public struct Position: Equatable {
        public var utf16: Int
        public var utf8: Int
        public init(utf16: Int, utf8: Int) { self.utf16 = utf16; self.utf8 = utf8 }
    }

    public struct LineInfo: Equatable {
        /// 1-based.
        public var number: Int
        /// Range of the line text (newline excluded).
        public var utf8: Range<Int>
        public var utf16: NSRange
        public var text: String
    }

    /// Same shape as the shell's `EditorDiagnostics.Mark` so the hook can map
    /// field for field (`nsRange` is a UTF-16 range in `text`).
    public struct Mark: Equatable {
        public var nsRange: NSRange
        public var severity: RuntimeV1.Severity
        public var message: String
        public var recovery: String?
        /// The word spoken for the severity ("Not implemented" for a FlashTeX
        /// gap, which the shell decides); nil speaks "Error" / "Warning".
        public var spokenSeverity: String?
        public init(nsRange: NSRange, severity: RuntimeV1.Severity, message: String, recovery: String?,
                    spokenSeverity: String? = nil) {
            self.nsRange = nsRange; self.severity = severity; self.message = message; self.recovery = recovery
            self.spokenSeverity = spokenSeverity
        }

        var severityWord: String { spokenSeverity ?? (severity == .error ? "Error" : "Warning") }
    }

    /// A pinned capture insertion point (the shell's `InsertionAnchor`).
    public struct Anchor: Equatable {
        public var id: String
        public var byteOffset: Int
        public init(id: String, byteOffset: Int) { self.id = id; self.byteOffset = byteOffset }
    }

    public enum TokenKind: String, Equatable { case word, command, symbol, punctuation, whitespace }

    public struct Token: Equatable {
        public var kind: TokenKind
        public var text: String
        public var utf16: NSRange
        public var utf8: Range<Int>
    }

    public enum RotorCategory: String, CaseIterable, Equatable {
        case headings, environments, diagnostics, captures

        public var title: String {
            switch self {
            case .headings: return "Headings"
            case .environments: return "Environments"
            case .diagnostics: return "Diagnostics"
            case .captures: return "Captures"
            }
        }
    }

    public struct RotorItem: Equatable {
        public var category: RotorCategory
        public var label: String
        public var utf8: Range<Int>
        public var utf16: NSRange
        /// 1-based line of the item's start.
        public var line: Int
    }

    public let text: String
    public let path: String
    public var marks: [Mark]
    public var anchor: Anchor?
    /// Lines of `text` (always at least one; a trailing newline yields an empty last line).
    public let lines: [LineInfo]

    public init(text: String, path: String = "main.tex", marks: [Mark] = [], anchor: Anchor? = nil) {
        self.text = text
        self.path = path
        self.marks = marks
        self.anchor = anchor
        self.lines = Self.splitLines(text)
    }

    // MARK: offsets

    /// Position for a UTF-16 offset; nil when out of range or inside a
    /// surrogate pair (`Range(NSRange, in:)` would silently snap the latter).
    public func position(utf16: Int) -> Position? {
        guard let index = index(utf16: utf16) else { return nil }
        return position(at: index)
    }

    /// `String.Index` for a UTF-16 offset on a scalar boundary, else nil.
    func index(utf16: Int) -> String.Index? {
        guard utf16 >= 0, utf16 <= text.utf16.count,
              let i = text.utf16.index(text.utf16.startIndex, offsetBy: utf16, limitedBy: text.utf16.endIndex),
              i.samePosition(in: text.unicodeScalars) != nil
        else { return nil }
        return i
    }

    /// Position for a UTF-8 byte offset; nil when out of range or inside a multi-byte scalar.
    public func position(utf8: Int) -> Position? {
        guard let ns = text.nsRange(utf8Bytes: .init(path: path, startByte: utf8, endByte: utf8)) else { return nil }
        return Position(utf16: ns.location, utf8: utf8)
    }

    /// Both offsets for a `String.Index`.
    func position(at index: String.Index) -> Position {
        Position(utf16: text.utf16.distance(from: text.utf16.startIndex, to: index),
                 utf8: text.utf8.distance(from: text.utf8.startIndex, to: index))
    }

    // MARK: lines

    static func splitLines(_ text: String) -> [LineInfo] {
        var out: [LineInfo] = []
        var utf8 = 0, utf16 = 0
        var lineStart8 = 0, lineStart16 = 0
        var current = String.UnicodeScalarView()
        for s in text.unicodeScalars {
            if s == "\n" {
                out.append(LineInfo(number: out.count + 1, utf8: lineStart8..<utf8,
                                    utf16: NSRange(location: lineStart16, length: utf16 - lineStart16),
                                    text: String(current)))
                current = String.UnicodeScalarView()
                utf8 += 1; utf16 += 1
                lineStart8 = utf8; lineStart16 = utf16
            } else {
                current.append(s)
                utf8 += s.utf8.count; utf16 += s.utf16.count
            }
        }
        out.append(LineInfo(number: out.count + 1, utf8: lineStart8..<utf8,
                            utf16: NSRange(location: lineStart16, length: utf16 - lineStart16),
                            text: String(current)))
        return out
    }

    /// The line containing a UTF-16 offset (the newline belongs to the line it ends).
    public func line(containingUTF16 offset: Int) -> LineInfo? {
        guard offset >= 0, offset <= text.utf16.count else { return nil }
        return lines.first { offset <= NSMaxRange($0.utf16) } ?? lines.last
    }

    /// 1-based column in user-perceived characters.
    public func column(atUTF16 offset: Int) -> Int? {
        guard let line = line(containingUTF16: offset),
              let start = index(utf16: line.utf16.location), let end = index(utf16: offset)
        else { return nil }
        return text[start..<end].count + 1
    }

    /// "Line 3 of 10, column 5: \section{Introduction}" ("empty" for a blank line).
    public func lineDescription(atUTF16 offset: Int) -> String {
        guard let line = line(containingUTF16: offset), let col = column(atUTF16: offset) else {
            return "Caret position \(offset) is not valid in \(path)."
        }
        let body = line.text.isEmpty ? "empty" : line.text
        var s = "Line \(line.number) of \(lines.count), column \(col): \(body)"
        let diags = diagnostics(inLine: line)
        if !diags.isEmpty {
            s += " — \(diags.count) diagnostic\(diags.count == 1 ? "" : "s") on this line"
        }
        return s
    }

    /// Start of the line `delta` lines away (clamped), or nil when `offset` is invalid.
    public func lineStart(fromUTF16 offset: Int, delta: Int) -> Position? {
        guard let line = line(containingUTF16: offset) else { return nil }
        let target = lines[max(0, min(lines.count - 1, line.number - 1 + delta))]
        return Position(utf16: target.utf16.location, utf8: target.utf8.lowerBound)
    }

    // MARK: words

    /// LaTeX-aware tokens of the whole text: `\command` names (or `\` plus one
    /// non-letter), runs of letters/digits/marks, single symbols (emoji, math
    /// signs), single punctuation marks, and whitespace runs.
    public var tokens: [Token] { tokenize(text.startIndex..<text.endIndex) }

    /// Tokens of one line.
    public func tokens(in line: LineInfo) -> [Token] {
        guard let r = Range(line.utf16, in: text) else { return [] }
        return tokenize(r)
    }

    private enum CharClass { case whitespace, backslash, letter, alnum, punctuation, symbol }

    private static func classify(_ c: Character) -> CharClass {
        if c.isWhitespace || c.isNewline { return .whitespace }
        if c == "\\" { return .backslash }
        guard let first = c.unicodeScalars.first else { return .symbol }
        let p = first.properties
        if p.isAlphabetic { return .letter }
        if p.numericType != nil || c.isNumber { return .alnum }
        if c.isPunctuation { return .punctuation }
        return .symbol
    }

    private func tokenize(_ range: Range<String.Index>) -> [Token] {
        var out: [Token] = []
        var i = range.lowerBound
        func emit(_ kind: TokenKind, _ start: String.Index, _ end: String.Index) {
            let s = position(at: start), e = position(at: end)
            out.append(Token(kind: kind, text: String(text[start..<end]),
                             utf16: NSRange(location: s.utf16, length: e.utf16 - s.utf16),
                             utf8: s.utf8..<e.utf8))
        }
        while i < range.upperBound {
            let c = text[i]
            let start = i
            switch Self.classify(c) {
            case .whitespace:
                while i < range.upperBound, Self.classify(text[i]) == .whitespace { i = text.index(after: i) }
                emit(.whitespace, start, i)
            case .backslash:
                i = text.index(after: i)
                if i < range.upperBound, Self.classify(text[i]) == .letter, text[i].isASCII {
                    while i < range.upperBound, Self.classify(text[i]) == .letter, text[i].isASCII { i = text.index(after: i) }
                    emit(.command, start, i)
                } else if i < range.upperBound, Self.classify(text[i]) != .whitespace {
                    i = text.index(after: i)          // \\ \{ \% …
                    emit(.command, start, i)
                } else {
                    emit(.punctuation, start, i)      // lone backslash
                }
            case .letter, .alnum:
                while i < range.upperBound, [.letter, .alnum].contains(Self.classify(text[i])) { i = text.index(after: i) }
                emit(.word, start, i)
            case .punctuation:
                i = text.index(after: i)
                emit(.punctuation, start, i)
            case .symbol:
                i = text.index(after: i)
                emit(.symbol, start, i)
            }
        }
        return out
    }

    /// The non-whitespace token containing `offset`, or the one ending exactly
    /// there (so the caret just after a word still describes it).
    public func word(atUTF16 offset: Int) -> Token? {
        guard let line = line(containingUTF16: offset) else { return nil }
        let toks = tokens(in: line).filter { $0.kind != .whitespace }
        return toks.first { $0.utf16.location <= offset && offset < NSMaxRange($0.utf16) }
            ?? toks.first { NSMaxRange($0.utf16) == offset }
    }

    /// Kinds that word navigation stops on (punctuation is skipped, as
    /// Option-arrow does; `word(atUTF16:)` still describes it).
    static let navigableKinds: Set<TokenKind> = [.word, .command, .symbol]

    /// Start of the next word, command, or symbol after `offset` (crossing
    /// lines), or nil at the end of the text.
    public func nextWordStart(fromUTF16 offset: Int) -> Position? {
        guard position(utf16: offset) != nil else { return nil }
        return tokens.first { Self.navigableKinds.contains($0.kind) && $0.utf16.location > offset }
            .map { Position(utf16: $0.utf16.location, utf8: $0.utf8.lowerBound) }
    }

    /// Start of the token at `offset` when the caret is inside it, else the
    /// previous word/command/symbol's start; nil at the beginning.
    public func previousWordStart(fromUTF16 offset: Int) -> Position? {
        guard position(utf16: offset) != nil else { return nil }
        return tokens.last { Self.navigableKinds.contains($0.kind) && $0.utf16.location < offset }
            .map { Position(utf16: $0.utf16.location, utf8: $0.utf8.lowerBound) }
    }

    /// "word “naïve”, 5 characters" / "command \section" / "symbol 🎉, PARTY POPPER".
    public func wordDescription(atUTF16 offset: Int) -> String {
        guard position(utf16: offset) != nil else { return "Caret position \(offset) is not valid in \(path)." }
        guard let tok = word(atUTF16: offset) else { return "No word at the caret." }
        switch tok.kind {
        case .word:
            let n = tok.text.count
            return "word “\(tok.text)”, \(n) character\(n == 1 ? "" : "s")"
        case .command:
            return "command \(tok.text)"
        case .symbol:
            return "symbol \(tok.text), " + Self.scalarNames(tok.text)
        case .punctuation:
            return "punctuation \(tok.text), " + Self.scalarNames(tok.text)
        case .whitespace:
            return "whitespace"
        }
    }

    // MARK: characters

    /// Position after the user-perceived character at `offset`, or nil at the end.
    public func nextCharacter(fromUTF16 offset: Int) -> Position? {
        guard let i = index(utf16: offset), i < text.endIndex else { return nil }
        return position(at: text.index(after: i))
    }

    /// Position before the user-perceived character ending at `offset`, or nil at the start.
    public func previousCharacter(fromUTF16 offset: Int) -> Position? {
        guard let i = index(utf16: offset), i > text.startIndex else { return nil }
        return position(at: text.index(before: i))
    }

    /// Description of the character at `offset`: the grapheme, its scalar
    /// names, and its size in both encodings. "end of document" past the end.
    public func characterDescription(atUTF16 offset: Int) -> String {
        guard let i = index(utf16: offset) else {
            return "Caret position \(offset) is not valid in \(path)."
        }
        guard i < text.endIndex else { return "end of document" }
        let c = text[i]
        let scalars = c.unicodeScalars
        let head: String
        if c.isNewline { head = "line break" }
        else if c == " " { head = "space" }
        else if c == "\t" { head = "tab" }
        else if c.isWhitespace { head = "whitespace" }
        else { head = String(c) }
        let u16 = scalars.reduce(0) { $0 + $1.utf16.count }
        let u8 = scalars.reduce(0) { $0 + $1.utf8.count }
        return "\(head), \(Self.scalarNames(String(c))); \(scalars.count) scalar\(scalars.count == 1 ? "" : "s"), "
            + "\(u16) UTF-16 unit\(u16 == 1 ? "" : "s"), \(u8) UTF-8 byte\(u8 == 1 ? "" : "s")"
    }

    static func scalarNames(_ s: String) -> String {
        s.unicodeScalars.map { $0.properties.name ?? $0.properties.nameAlias ?? String(format: "U+%04X", $0.value) }
            .joined(separator: ", ")
    }

    // MARK: diagnostics

    /// Marks whose range contains `offset` (a caret at a mark's end counts).
    public func diagnostics(atUTF16 offset: Int) -> [Mark] {
        marks.filter { $0.nsRange.location <= offset && offset <= NSMaxRange($0.nsRange) }
            .sorted { $0.nsRange.location < $1.nsRange.location }
    }

    func diagnostics(inLine line: LineInfo) -> [Mark] {
        marks.filter { NSIntersectionRange($0.nsRange, line.utf16).length > 0
            || ($0.nsRange.length == 0 && NSLocationInRange($0.nsRange.location, line.utf16)) }
    }

    /// "Error: message — recovery: …" for the caret, or nil when clear.
    public func diagnosticDescription(atUTF16 offset: Int) -> String? {
        let here = diagnostics(atUTF16: offset)
        guard !here.isEmpty else { return nil }
        return here.map { m in
            (m.severity == .error ? "Error: " : "Warning: ") + m.message
                + (m.recovery.map { " — recovery: \($0)" } ?? "")
        }.joined(separator: "; ")
    }

    // MARK: rotor

    static let headingCommands = ["chapter", "section", "subsection", "subsubsection", "paragraph"]

    public func rotorItems(_ category: RotorCategory) -> [RotorItem] {
        switch category {
        case .headings: return headings()
        case .environments: return environments()
        case .diagnostics: return diagnosticItems()
        case .captures: return captureItems()
        }
    }

    /// Next item strictly after `offset` (wrapping to the first), or the
    /// previous one (wrapping to the last). Nil when the category is empty.
    public func nextRotorItem(_ category: RotorCategory, fromUTF16 offset: Int, forward: Bool) -> RotorItem? {
        let items = rotorItems(category)
        guard !items.isEmpty else { return nil }
        if forward { return items.first { $0.utf16.location > offset } ?? items[0] }
        return items.last { $0.utf16.location < offset } ?? items[items.count - 1]
    }

    /// `\name{arg}` uses for the given names, in document order.
    struct CommandUse { var name: String; var arg: String; var range: NSRange }

    func commandUses(_ names: [String]) -> [CommandUse] {
        let pattern = "\\\\(" + names.joined(separator: "|") + ")\\*?\\{([^{}\\n]*)\\}"
        guard let re = try? NSRegularExpression(pattern: pattern) else { return [] }
        let ns = text as NSString
        return re.matches(in: text, range: NSRange(location: 0, length: ns.length)).map {
            CommandUse(name: ns.substring(with: $0.range(at: 1)), arg: ns.substring(with: $0.range(at: 2)), range: $0.range)
        }
    }

    func item(_ category: RotorCategory, label: String, utf16: NSRange) -> RotorItem? {
        guard let bytes = text.utf8ByteRange(of: utf16), let line = line(containingUTF16: utf16.location) else { return nil }
        return RotorItem(category: category, label: label, utf8: bytes.start..<bytes.end, utf16: utf16, line: line.number)
    }

    func headings() -> [RotorItem] {
        commandUses(Self.headingCommands).compactMap { use in
            let level = Self.headingCommands.firstIndex(of: use.name) ?? 0   // LaTeX: chapter 0, section 1, …
            return item(.headings, label: "\(use.name.capitalized) “\(use.arg)”, level \(level)", utf16: use.range)
        }
    }

    func environments() -> [RotorItem] {
        var out: [RotorItem] = []
        var open: [CommandUse] = []
        for use in commandUses(["begin", "end"]) {
            if use.name == "begin" { open.append(use); continue }
            if let i = open.lastIndex(where: { $0.arg == use.arg }) {
                let begin = open.remove(at: i)
                let whole = NSUnionRange(begin.range, use.range)
                let l1 = line(containingUTF16: begin.range.location)?.number ?? 0
                let l2 = line(containingUTF16: use.range.location)?.number ?? 0
                let where_ = l1 == l2 ? "line \(l1)" : "lines \(l1) to \(l2)"
                if let it = item(.environments, label: "Environment \(use.arg), \(where_)", utf16: whole) { out.append(it) }
            } else if let it = item(.environments, label: "Stray \\end{\(use.arg)} with no \\begin", utf16: use.range) {
                out.append(it)
            }
        }
        for begin in open {
            if let it = item(.environments, label: "Environment \(begin.arg), unclosed", utf16: begin.range) { out.append(it) }
        }
        return out.sorted { $0.utf16.location != $1.utf16.location ? $0.utf16.location < $1.utf16.location : $0.utf16.length > $1.utf16.length }
    }

    /// One item per mark, in document order. Marks that read identically on
    /// the same range (the compiler reporting one problem twice) get
    /// ", 1 of 2" / ", 2 of 2" appended, so every item has its own label:
    /// the rotor steps by (range, label) identity and would otherwise never
    /// get past the first of them.
    func diagnosticItems() -> [RotorItem] {
        var items = marks.sorted { $0.nsRange.location < $1.nsRange.location }.compactMap { m -> RotorItem? in
            let line = line(containingUTF16: m.nsRange.location)?.number ?? 0
            return item(.diagnostics, label: "\(m.severityWord) at line \(line): \(m.message)" + (m.recovery.map { " — recovery: \($0)" } ?? ""),
                        utf16: m.nsRange)
        }
        func key(_ it: RotorItem) -> String { NSStringFromRange(it.utf16) + "|" + it.label }
        var total: [String: Int] = [:]
        for it in items { total[key(it), default: 0] += 1 }
        var seen: [String: Int] = [:]
        for i in items.indices {
            let k = key(items[i])
            guard let n = total[k], n > 1 else { continue }
            seen[k, default: 0] += 1
            items[i].label += ", \(seen[k]!) of \(n)"
        }
        return items
    }

    func captureItems() -> [RotorItem] {
        guard let anchor, let pos = position(utf8: anchor.byteOffset),
              let line = line(containingUTF16: pos.utf16), let col = column(atUTF16: pos.utf16) else { return [] }
        return [RotorItem(category: .captures,
                          label: "Insertion point \(anchor.id), line \(line.number), column \(col), byte \(anchor.byteOffset)",
                          utf8: anchor.byteOffset..<anchor.byteOffset,
                          utf16: NSRange(location: pos.utf16, length: 0), line: line.number)]
    }
}
