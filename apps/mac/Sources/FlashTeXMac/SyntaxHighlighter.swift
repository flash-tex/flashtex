import AppKit

/// LaTeX syntax highlighting for the source editor (lane mac-syntax-highlight).
///
/// Two layers:
///
/// - `SyntaxHighlighter` is a pure, testable model: a lexer over UTF-16 units
///   that produces attribute runs (`Run`) for a range of text given the mode
///   the range starts in, plus a per-line table of start modes so that an
///   edit re-lexes only the changed lines (and the lines after them until
///   the line-start mode converges with what was stored, i.e. the enclosing
///   math span or environment). `runs(in:)` after `edit` equals the runs a
///   fresh `reset` would give: that invariant is tested.
/// - `SyntaxPainter` (below) applies the runs to an `NSTextView` as layout-
///   manager temporary attributes (`.foregroundColor` only), never touching
///   the text storage, so undo, the `text` binding and the marks lane are
///   unaffected. Colours are dynamic `NSColor`s resolved per appearance, so
///   light/dark switches need no repaint. Painting is windowed around the
///   visible text like `SourceEditorView.MarkPainter`; an edit repaints the
///   changed lines only.
///
/// Lexing rules (deliberately lexical; the compiler owns the semantics):
/// - `%` starts a comment to the end of the line unless escaped.
/// - `\` + letters is a control word; `\` + one other unit is a control
///   symbol (`\\`, `\$`, `\{`, `\%`, `\(`, `\[`). A backslash before a
///   surrogate pair takes both units.
/// - `\begin{name}` / `\end{name}` colour `\begin`/`\end` as commands and the
///   name as an environment name. Math environments (`equation`, `align`,
///   …) switch to math mode until their `\end`; verbatim-like environments
///   (`verbatim`, `lstlisting`, `minted`, …) leave everything plain until
///   their `\end`; `comment` is a comment until `\end{comment}`.
/// - `$…$`, `$$…$$`, `\(…\)`, `\[…\]` are math. Inline `$` math (and `$$`)
///   never crosses a blank line (LaTeX's paragraph rule; also the rule the
///   brace matcher uses), so an unbalanced `$` colours at most a paragraph.
/// - In math mode: commands are `mathCommand`, digit runs (with `.`) are
///   `number`, everything else is `math`; braces stay `brace`.
/// - The argument of `\label`/`\ref`/`\cite`-like commands is `reference`;
///   of `\input`/`\include`/`\includegraphics`/`\usepackage`-like commands
///   `file`; the control sequence defined by `\newcommand`-like commands is
///   `definition`.
/// - `\verb<d>…<d>` is `verbatim` (plain) to the delimiter or the line end.
/// - Line breaks are `\n`; a `\r` is whitespace (CRLF sources).
struct SyntaxHighlighter {
    enum Kind: UInt8, CaseIterable, Sendable {
        case command, mathCommand, environment, math, mathDelimiter, number, comment
        case brace, bracket, reference, file, definition, verbatim
    }

    struct Run: Equatable, Sendable {
        var range: NSRange
        var kind: Kind
        init(_ location: Int, _ length: Int, _ kind: Kind) { range = NSRange(location: location, length: length); self.kind = kind }
        init(range: NSRange, kind: Kind) { self.range = range; self.kind = kind }
    }

    /// Mode a line starts in.
    enum Mode: Equatable, Sendable {
        case text
        /// `$…$`.
        case inlineMath
        /// `$$…$$`.
        case dollarDisplayMath
        /// `\[…\]`.
        case displayMath
        /// `\(…\)`.
        case parenMath
        /// Inside math environments (`equation`, `align`, …), `depth` of them
        /// deep. Math environments nest in real documents — `cases` inside
        /// `align`, `split` inside `equation`, `array` inside `equation` — so
        /// the mode counts them; a flat flag left everything between the inner
        /// `\end{cases}` and the outer `\end{align}` lexed as text.
        case mathEnvironment(depth: Int)
        /// Inside a verbatim-like environment; ends at `\end{name}`.
        case verbatim(String)
        /// Inside `\begin{comment}`.
        case commentEnvironment

        var isMath: Bool {
            switch self {
            case .inlineMath, .dollarDisplayMath, .displayMath, .parenMath, .mathEnvironment: true
            default: false
            }
        }
    }

    static let mathEnvironments: Set<String> = [
        "math", "displaymath", "equation", "equation*", "align", "align*", "alignat", "alignat*", "gather", "gather*",
        "multline", "multline*", "flalign", "flalign*", "eqnarray", "eqnarray*", "split", "aligned", "gathered",
        "cases", "dcases", "matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix", "smallmatrix", "array",
        "subequations", "empheq", "IEEEeqnarray", "IEEEeqnarray*",
    ]
    static let verbatimEnvironments: Set<String> = [
        "verbatim", "verbatim*", "Verbatim", "BVerbatim", "LVerbatim", "lstlisting", "minted", "alltt", "filecontents",
        "filecontents*", "tikzpicture-verbatim",
    ]
    static let referenceCommands: Set<String> = [
        "label", "ref", "eqref", "pageref", "autoref", "cref", "Cref", "cpageref", "Cpageref", "nameref", "vref", "hyperref",
        "cite", "citep", "citet", "citeauthor", "citeyear", "citealp", "citealt", "nocite", "parencite", "textcite",
        "autocite", "footcite", "fullcite", "Cite", "Parencite", "Textcite", "Autocite",
    ]
    static let fileCommands: Set<String> = [
        "input", "include", "includeonly", "includegraphics", "bibliography", "bibliographystyle", "usepackage",
        "documentclass", "RequirePackage", "addbibresource", "graphicspath", "lstinputlisting", "inputminted",
        "subfile", "import", "subimport", "includepdf", "InputIfFileExists",
    ]
    static let definitionCommands: Set<String> = [
        "newcommand", "renewcommand", "providecommand", "newcommand*", "renewcommand*", "providecommand*", "def", "gdef",
        "edef", "xdef", "let", "DeclareMathOperator", "DeclareMathOperator*", "newenvironment", "renewenvironment",
        "newenvironment*", "renewenvironment*", "NewDocumentCommand", "RenewDocumentCommand", "ProvideDocumentCommand",
        "DeclareDocumentCommand", "NewDocumentEnvironment", "newtheorem", "newtheorem*", "newlength", "newcounter",
        "newif", "newcolumntype", "DeclareRobustCommand", "DeclarePairedDelimiter", "newcommandx",
    ]

    // MARK: lexer

    /// Lexes `units[from..<to]` (UTF-16, with `base` the offset of `units[0]`
    /// in the document) starting in `mode`. Runs are appended to `runs` when
    /// given; the mode at `to` is returned. `to` should be a line end (or the
    /// end of the text) for the returned mode to be a line-start mode.
    static func lex(_ units: UnsafeBufferPointer<UInt16>, from: Int, to: Int, base: Int, mode: Mode,
                    runs: inout [Run], collect: Bool = true) -> Mode {
        var state = Lexer(units: units, end: to, base: base, mode: mode, collect: collect)
        state.run(from: from)
        if collect { runs.append(contentsOf: state.runs) }
        return state.mode
    }

    private struct Lexer {
        let units: UnsafeBufferPointer<UInt16>
        let end: Int
        let base: Int
        var mode: Mode
        let collect: Bool
        var runs: [Run] = []
        /// Start of an open `math` run (merged across characters), or nil.
        var mathRunStart: Int?
        /// Current line has only whitespace so far (for the blank-line rule).
        var lineBlank = true

        init(units: UnsafeBufferPointer<UInt16>, end: Int, base: Int, mode: Mode, collect: Bool) {
            self.units = units; self.end = end; self.base = base; self.mode = mode; self.collect = collect
        }

        @inline(__always) static func isLetter(_ c: UInt16) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
        @inline(__always) static func isDigit(_ c: UInt16) -> Bool { c >= 0x30 && c <= 0x39 }
        @inline(__always) static func isSpace(_ c: UInt16) -> Bool { c == 0x20 || c == 0x09 || c == 0x0D }
        @inline(__always) static func isHighSurrogate(_ c: UInt16) -> Bool { c >= 0xD800 && c <= 0xDBFF }

        mutating func emit(_ start: Int, _ endExclusive: Int, _ kind: Kind) {
            guard collect, endExclusive > start else { return }
            runs.append(Run(base + start, endExclusive - start, kind))
        }

        mutating func closeMathRun(at i: Int) {
            if let s = mathRunStart { emit(s, i, .math); mathRunStart = nil }
        }

        /// A unit of math content: extends the open math run.
        mutating func mathContent(at i: Int) {
            if mathRunStart == nil { mathRunStart = i }
        }

        /// Control word letters after the backslash at `i`; returns the end.
        func controlWordEnd(after i: Int) -> Int {
            var j = i + 1
            while j < end, Self.isLetter(units[j]) { j += 1 }
            if j < end, j > i + 1, units[j] == 0x2A { j += 1 } // starred form: \newcommand*, \begin{align*} is the env
            return j
        }

        func name(_ s: Int, _ e: Int) -> String {
            String(utf16CodeUnits: units.baseAddress! + s, count: e - s)
        }

        /// `{name}` after optional spaces at `i`; returns (open brace, name start, name end, close brace + 1).
        func bracedName(at i: Int) -> (open: Int, nameStart: Int, nameEnd: Int, next: Int)? {
            var j = i
            while j < end, Self.isSpace(units[j]) { j += 1 }
            guard j < end, units[j] == 0x7B else { return nil }
            var k = j + 1
            while k < end, units[k] != 0x7D, units[k] != 0x0A, units[k] != 0x7B, units[k] != 0x5C { k += 1 }
            guard k < end, units[k] == 0x7D else { return nil }
            return (j, j + 1, k, k + 1)
        }

        /// Skips `[...]` (one level) then colours `{...}` content as `kind`;
        /// returns the index after the argument or `i` when none follows.
        mutating func colourArgument(at i: Int, kind: Kind) -> Int {
            var j = i
            while j < end, Self.isSpace(units[j]) { j += 1 }
            if j < end, units[j] == 0x5B {
                var k = j + 1
                while k < end, units[k] != 0x5D, units[k] != 0x0A { k += 1 }
                guard k < end, units[k] == 0x5D else { return i }
                emit(j, j + 1, .bracket); emit(k, k + 1, .bracket)
                j = k + 1
                while j < end, Self.isSpace(units[j]) { j += 1 }
            }
            guard j < end, units[j] == 0x7B else { return i }
            var depth = 1
            var k = j + 1
            while k < end, units[k] != 0x0A {
                if units[k] == 0x5C { k += 2; continue }
                if units[k] == 0x7B { depth += 1 }
                if units[k] == 0x7D { depth -= 1; if depth == 0 { break } }
                k += 1
            }
            guard k < end, units[k] == 0x7D else { return i }
            emit(j, j + 1, .brace)
            emit(j + 1, k, kind)
            emit(k, k + 1, .brace)
            return k + 1
        }

        mutating func run(from start: Int) {
            var i = start
            while i < end {
                switch mode {
                case .verbatim(let env): i = verbatimBody(from: i, env: env, kind: .verbatim)
                case .commentEnvironment: i = verbatimBody(from: i, env: "comment", kind: .comment)
                default: i = codeUnit(at: i)
                }
            }
            closeMathRun(at: end)
        }

        /// Plain (or comment-coloured) text until `\end{env}`.
        mutating func verbatimBody(from start: Int, env: String, kind: Kind) -> Int {
            var i = start
            while i < end {
                if units[i] == 0x5C, let m = endOfEnvironment(at: i, named: env) {
                    emit(start, i, kind)
                    emit(i, m.nameStart - 1, .command)
                    emit(m.nameStart - 1, m.nameStart, .brace)
                    emit(m.nameStart, m.nameEnd, .environment)
                    emit(m.nameEnd, m.next, .brace)
                    mode = .text
                    lineBlank = false
                    return m.next
                }
                if units[i] == 0x0A { lineBlank = true } else if !Self.isSpace(units[i]) { lineBlank = false }
                i += 1
            }
            emit(start, end, kind)
            return end
        }

        /// `\end{env}` at `i`.
        func endOfEnvironment(at i: Int, named env: String) -> (nameStart: Int, nameEnd: Int, next: Int)? {
            let e = controlWordEnd(after: i)
            guard e - i == 4, units[i + 1] == 0x65, units[i + 2] == 0x6E, units[i + 3] == 0x64,
                  let b = bracedName(at: e), name(b.nameStart, b.nameEnd) == env else { return nil }
            return (b.nameStart, b.nameEnd, b.next)
        }

        mutating func codeUnit(at i: Int) -> Int {
            let c = units[i]
            switch c {
            case 0x0A:
                closeMathRun(at: i)
                if lineBlank, mode == .inlineMath || mode == .dollarDisplayMath { mode = .text }
                lineBlank = true
                return i + 1
            case 0x25: // %
                closeMathRun(at: i)
                var j = i + 1
                while j < end, units[j] != 0x0A { j += 1 }
                emit(i, j, .comment)
                lineBlank = false
                return j
            case 0x5C: // backslash
                closeMathRun(at: i)
                lineBlank = false
                return controlSequence(at: i)
            case 0x24: // $
                closeMathRun(at: i)
                lineBlank = false
                let double = i + 1 < end && units[i + 1] == 0x24
                switch mode {
                case .text:
                    if double { emit(i, i + 2, .mathDelimiter); mode = .dollarDisplayMath; return i + 2 }
                    emit(i, i + 1, .mathDelimiter); mode = .inlineMath; return i + 1
                case .inlineMath:
                    emit(i, i + 1, .mathDelimiter); mode = .text; return i + 1
                case .dollarDisplayMath:
                    if double { emit(i, i + 2, .mathDelimiter); mode = .text; return i + 2 }
                    emit(i, i + 1, .mathDelimiter); return i + 1 // stray $ inside $$…$$
                default:
                    emit(i, i + 1, .mathDelimiter); return i + 1 // $ inside \[…\] or an environment: shown as a delimiter
                }
            case 0x7B, 0x7D:
                closeMathRun(at: i); emit(i, i + 1, .brace); lineBlank = false; return i + 1
            case 0x5B, 0x5D:
                closeMathRun(at: i); emit(i, i + 1, .bracket); lineBlank = false; return i + 1
            default:
                if Self.isSpace(c) {
                    if mode.isMath { mathContent(at: i) }
                    return i + 1
                }
                lineBlank = false
                if mode.isMath {
                    if Self.isDigit(c) {
                        closeMathRun(at: i)
                        var j = i + 1
                        while j < end, Self.isDigit(units[j]) || (units[j] == 0x2E && j + 1 < end && Self.isDigit(units[j + 1])) { j += 1 }
                        emit(i, j, .number)
                        return j
                    }
                    mathContent(at: i)
                }
                return i + 1
            }
        }

        mutating func controlSequence(at i: Int) -> Int {
            let next = i + 1 < end ? units[i + 1] : 0
            let inMath = mode.isMath
            let commandKind: Kind = inMath ? .mathCommand : .command
            guard Self.isLetter(next) else {
                // Control symbol (or a trailing lone backslash).
                var j = min(end, i + 2)
                if i + 1 < end, Self.isHighSurrogate(next), i + 2 < end { j = i + 3 }
                if next == 0x28 { // \(
                    if mode == .text { mode = .parenMath }
                    emit(i, j, .mathDelimiter)
                } else if next == 0x29 { // \)
                    if mode == .parenMath { mode = .text }
                    emit(i, j, .mathDelimiter)
                } else if next == 0x5B { // \[
                    if mode == .text { mode = .displayMath }
                    emit(i, j, .mathDelimiter)
                } else if next == 0x5D { // \]
                    if mode == .displayMath { mode = .text }
                    emit(i, j, .mathDelimiter)
                } else if next == 0x0A || i + 1 >= end {
                    emit(i, i + 1, commandKind)
                    return i + 1
                } else {
                    emit(i, j, commandKind)
                }
                return j
            }
            let e = controlWordEnd(after: i)
            let word = name(i + 1, e)
            switch word {
            case "begin", "end":
                emit(i, e, .command) // structural, never a math command
                guard let b = bracedName(at: e) else { return e }
                emit(b.open, b.open + 1, .brace)
                emit(b.nameStart, b.nameEnd, .environment)
                emit(b.nameEnd, b.next, .brace)
                let env = name(b.nameStart, b.nameEnd)
                if word == "begin" {
                    if SyntaxHighlighter.verbatimEnvironments.contains(env) { mode = .verbatim(env) }
                    else if env == "comment" { mode = .commentEnvironment }
                    else if SyntaxHighlighter.mathEnvironments.contains(env) {
                        // Entering math, or nesting one math environment inside
                        // another (`cases` in `align`, `split` in `equation`).
                        switch mode {
                        case .text: mode = .mathEnvironment(depth: 1)
                        case .mathEnvironment(let depth): mode = .mathEnvironment(depth: depth + 1)
                        default: break // `$…$`, verbatim and comment bodies are not entered
                        }
                    }
                } else if case .mathEnvironment(let depth) = mode, SyntaxHighlighter.mathEnvironments.contains(env) {
                    // Only the outermost `\end` leaves math.
                    mode = depth <= 1 ? .text : .mathEnvironment(depth: depth - 1)
                }
                return b.next
            case "verb", "verb*":
                emit(i, e, commandKind)
                guard e < end, !Self.isSpace(units[e]), units[e] != 0x0A else { return e }
                let d = units[e]
                var k = e + 1
                while k < end, units[k] != d, units[k] != 0x0A { k += 1 }
                let stop = k < end && units[k] == d ? k + 1 : k
                emit(e, stop, .verbatim)
                return stop
            default:
                emit(i, e, commandKind)
                if SyntaxHighlighter.referenceCommands.contains(word) { return colourArgument(at: e, kind: .reference) }
                if SyntaxHighlighter.fileCommands.contains(word) { return colourArgument(at: e, kind: .file) }
                if SyntaxHighlighter.definitionCommands.contains(word) { return definedName(after: e) }
                return e
            }
        }

        /// `\newcommand{\foo}` / `\newcommand\foo` / `\newenvironment{foo}`:
        /// the defined name is `definition`.
        mutating func definedName(after e: Int) -> Int {
            var j = e
            while j < end, Self.isSpace(units[j]) { j += 1 }
            guard j < end else { return e }
            var braced = false
            if units[j] == 0x7B { emit(j, j + 1, .brace); braced = true; j += 1 }
            var k = j
            if k < end, units[k] == 0x5C {
                k = controlWordEnd(after: k)
                if k == j + 1, k < end { k += 1 } // control symbol
            } else {
                while k < end, Self.isLetter(units[k]) || units[k] == 0x2A { k += 1 }
            }
            guard k > j else { return e }
            emit(j, k, .definition)
            if braced, k < end, units[k] == 0x7D { emit(k, k + 1, .brace); k += 1 }
            return k
        }
    }

    // MARK: incremental line model

    /// UTF-16 offset of each line start (`[0]` is 0); a trailing `\n` opens
    /// one more (empty) line.
    private(set) var lineStarts: [Int] = [0]
    /// Mode at the start of each line (`count == lineStarts.count`).
    private(set) var modes: [Mode] = [.text]
    private(set) var length = 0
    /// Lines re-lexed by the last `edit` (evidence for tests/benchmarks).
    private(set) var lastEditLinesLexed = 0

    var lineCount: Int { lineStarts.count }

    /// Index of the line containing `utf16` (the last line for offsets at or past the end).
    func line(at utf16: Int) -> Int {
        var lo = 0, hi = lineStarts.count - 1
        while lo < hi {
            let mid = (lo + hi + 1) / 2
            if lineStarts[mid] <= utf16 { lo = mid } else { hi = mid - 1 }
        }
        return lo
    }

    /// Range of line `index` including its terminator.
    func lineRange(_ index: Int) -> NSRange {
        let start = lineStarts[index]
        let end = index + 1 < lineStarts.count ? lineStarts[index + 1] : length
        return NSRange(location: start, length: end - start)
    }

    /// Rebuilds the line table and every line-start mode for `text`.
    mutating func reset(_ text: NSString) {
        length = text.length
        lineStarts = [0]
        modes = [.text]
        lastEditLinesLexed = 0
        guard length > 0 else { return }
        withUnits(of: text, range: NSRange(location: 0, length: length)) { units in
            var mode = Mode.text
            var lineStart = 0
            var runs: [Run] = []
            for i in 0..<length where units[i] == 0x0A {
                mode = Self.lex(units, from: lineStart, to: i + 1, base: 0, mode: mode, runs: &runs, collect: false)
                lineStart = i + 1
                lineStarts.append(lineStart)
                modes.append(mode)
            }
        }
    }

    /// The storage replaced `range` (old coordinates) with `replacementLength`
    /// units; `text` is the new text. Re-lexes from the first affected line
    /// until the line-start modes converge. Returns the line-aligned range
    /// (new coordinates) whose runs may have changed.
    mutating func edit(range: NSRange, replacementLength: Int, text: NSString) -> NSRange {
        let newLength = text.length
        precondition(newLength == length - range.length + replacementLength, "edit out of sync with the text")
        let first = line(at: range.location)
        let lastOld = line(at: NSMaxRange(range))
        let delta = replacementLength - range.length
        // Line starts of the replaced region: rescan from the start of `first`
        // to the end of the line containing the end of the replacement.
        let scanStart = lineStarts[first]
        var scanEnd = range.location + replacementLength
        while scanEnd < newLength, text.character(at: scanEnd) != 0x0A { scanEnd += 1 }
        if scanEnd < newLength { scanEnd += 1 }
        var fresh: [Int] = []
        withUnits(of: text, range: NSRange(location: scanStart, length: scanEnd - scanStart)) { units in
            for i in 0..<units.count where units[i] == 0x0A { fresh.append(scanStart + i + 1) }
        }
        // Lines strictly inside the old region (first+1 ... lastOld) are replaced by `fresh`
        // (the line starting at scanEnd, if any, belongs to the old lastOld+1 unless it was lastOld's terminator).
        var tail = lastOld + 1
        if fresh.last == scanEnd, scanEnd <= newLength, tail < lineStarts.count, lineStarts[tail] + delta == scanEnd {
            // The line after lastOld starts exactly at scanEnd: it is an old line, not a fresh one.
            fresh.removeLast()
        } else if let last = fresh.last, last == scanEnd, tail < lineStarts.count {
            // Shouldn't happen (scanEnd is a line start of an old line); keep consistent.
            fresh.removeLast()
        }
        // Splice the line table in place; shift the old tail by the edit's delta.
        lineStarts.replaceSubrange((first + 1)..<tail, with: fresh)
        modes.replaceSubrange((first + 1)..<tail, with: repeatElement(.text, count: fresh.count))
        if delta != 0 {
            lineStarts.withUnsafeMutableBufferPointer { b in
                var i = first + 1 + fresh.count
                while i < b.count { b[i] += delta; i += 1 }
            }
        }
        length = newLength
        // Re-lex from `first` until convergence.
        let freshEndLine = first + fresh.count // first line whose stored mode is old
        var lineIndex = first
        var mode = modes[first]
        var lexed = 0
        var dirtyEnd = lineStarts[first]
        while lineIndex < lineStarts.count {
            let r = lineRange(lineIndex)
            var runs: [Run] = []
            withUnits(of: text, range: r) { units in
                mode = Self.lex(units, from: 0, to: units.count, base: r.location, mode: mode, runs: &runs, collect: false)
            }
            lexed += 1
            dirtyEnd = NSMaxRange(r)
            let next = lineIndex + 1
            guard next < lineStarts.count else { break }
            if next > freshEndLine, modes[next] == mode { break } // converged with an old line-start mode
            modes[next] = mode
            lineIndex = next
        }
        lastEditLinesLexed = lexed
        return NSRange(location: lineStarts[first], length: dirtyEnd - lineStarts[first])
    }

    /// Runs intersecting `range`, lexed from the start of the line containing
    /// `range.location` with its stored mode through the end of the line
    /// containing the range end. Runs are clipped to `range`.
    func runs(in range: NSRange, text: NSString) -> [Run] {
        guard range.length > 0, text.length == length else { return [] }
        let first = line(at: range.location)
        let last = line(at: max(range.location, NSMaxRange(range) - 1))
        let start = lineStarts[first]
        let end = NSMaxRange(lineRange(last))
        var runs: [Run] = []
        withUnits(of: text, range: NSRange(location: start, length: end - start)) { units in
            _ = Self.lex(units, from: 0, to: units.count, base: start, mode: modes[first], runs: &runs)
        }
        if start == range.location, end == NSMaxRange(range) { return runs }
        return runs.compactMap { run in
            let clipped = NSIntersectionRange(run.range, range)
            return clipped.length > 0 ? Run(range: clipped, kind: run.kind) : nil
        }
    }

    /// Full lex of `text` from a fresh model (tests; the incremental invariant).
    static func runs(of text: NSString) -> [Run] {
        var h = SyntaxHighlighter()
        h.reset(text)
        return h.runs(in: NSRange(location: 0, length: text.length), text: text)
    }

    /// Kind of the run at `utf16` after a full lex (hover/tests), or nil for plain text.
    func kind(at utf16: Int, text: NSString) -> Kind? {
        guard utf16 >= 0, utf16 < length else { return nil }
        let r = lineRange(line(at: utf16))
        return runs(in: r, text: text).first { NSLocationInRange(utf16, $0.range) }?.kind
    }

    private func withUnits<T>(of text: NSString, range: NSRange, _ body: (UnsafeBufferPointer<UInt16>) -> T) -> T {
        let buffer = UnsafeMutablePointer<UInt16>.allocate(capacity: max(range.length, 1))
        defer { buffer.deallocate() }
        text.getCharacters(buffer, range: range)
        return body(UnsafeBufferPointer(start: buffer, count: range.length))
    }
}

// MARK: - theme

/// Semantic colours for the runs. Every colour is a dynamic `NSColor`
/// resolved per drawing appearance (light and dark variants; the system
/// accent colour for references), so an appearance change needs no repaint.
/// Restraint on purpose: plain text keeps the text view's colour, braces and
/// brackets are only slightly dimmed.
struct SyntaxTheme: Sendable {
    static func dynamic(light: (CGFloat, CGFloat, CGFloat), dark: (CGFloat, CGFloat, CGFloat)) -> NSColor {
        NSColor(name: nil) { appearance in
            let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            let c = isDark ? dark : light
            return NSColor(srgbRed: c.0 / 255, green: c.1 / 255, blue: c.2 / 255, alpha: 1)
        }
    }

    static let command = dynamic(light: (155, 35, 147), dark: (252, 95, 163))          // Xcode keyword
    static let mathCommand = dynamic(light: (50, 109, 116), dark: (103, 183, 164))     // teal
    static let environment = dynamic(light: (11, 79, 121), dark: (93, 216, 255))       // type
    static let math = dynamic(light: (28, 0, 207), dark: (208, 168, 255))              // indigo
    static let mathDelimiter = dynamic(light: (28, 0, 207), dark: (208, 168, 255))
    static let number = dynamic(light: (28, 0, 207), dark: (208, 191, 105))            // Xcode number
    static let comment = dynamic(light: (93, 108, 121), dark: (108, 121, 134))         // Xcode comment
    static let brace = NSColor.secondaryLabelColor
    static let reference = dynamic(light: (196, 26, 22), dark: (252, 106, 93))         // Xcode string
    static let file = dynamic(light: (196, 26, 22), dark: (252, 106, 93))
    static let definition = dynamic(light: (15, 104, 160), dark: (65, 161, 192))       // Xcode declaration
    static let currentLine = NSColor(name: nil) { appearance in
        let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
        return NSColor.labelColor.withAlphaComponent(isDark ? 0.06 : 0.045)
    }
    static let gutterText = NSColor.tertiaryLabelColor
    static let gutterCurrentText = NSColor.secondaryLabelColor

    static func color(for kind: SyntaxHighlighter.Kind) -> NSColor? {
        switch kind {
        case .command: command
        case .mathCommand: mathCommand
        case .environment: environment
        case .math: math
        case .mathDelimiter: mathDelimiter
        case .number: number
        case .comment: comment
        case .brace, .bracket: brace
        case .reference: reference
        case .file: file
        case .definition: definition
        case .verbatim: nil // plain
        }
    }
}

// MARK: - painter

/// Applies `SyntaxHighlighter` runs to a text view as `.foregroundColor`
/// temporary attributes over a window around the visible text (extended on
/// scroll); an edit repaints only the lines the highlighter re-lexed.
/// Nothing is painted while the view has marked text (IME composition): the
/// dirty range is kept and painted at the next flush after the commit.
@MainActor
final class SyntaxPainter {
    static let key = NSAttributedString.Key.foregroundColor
    static let padding = 4_000

    private(set) var highlighter = SyntaxHighlighter()
    /// Disjoint, sorted ranges whose temporary colours match the highlighter.
    private(set) var painted: [NSRange] = []
    /// Range whose paint is stale (kept while marked text exists).
    private(set) var pendingDirty: NSRange?
    /// Evidence: paint passes, runs painted, thread CPU of the last edit.
    private(set) var paints = 0
    private(set) var runsPainted = 0
    private(set) var lastEditCpuNs: UInt64 = 0
    private(set) var lastFlushCpuNs: UInt64 = 0
    private(set) var lastEditLinesLexed = 0
    var enabled = true { didSet { if !enabled { clear() } } }
    private weak var textView: NSTextView?
    private var observer: NSObjectProtocol?
    private var flushScheduled = false

    deinit { if let observer { NotificationCenter.default.removeObserver(observer) } }

    /// Starts following `tv`'s storage (every character edit, whatever its
    /// origin) and paints the current window.
    func attach(_ tv: NSTextView) {
        textView = tv
        guard let storage = tv.textStorage else { return }
        observer = NotificationCenter.default.addObserver(forName: NSTextStorage.didProcessEditingNotification,
                                                          object: storage, queue: nil) { [weak self] note in
            MainActor.assumeIsolated {
                guard let self, let storage = note.object as? NSTextStorage, storage.editedMask.contains(.editedCharacters) else { return }
                let edited = storage.editedRange
                let delta = storage.changeInLength
                self.storageEdited(range: NSRange(location: edited.location, length: edited.length - delta), replacementLength: edited.length)
            }
        }
        reset()
    }

    /// The whole text changed (or the view was reset, dropping temporary attributes).
    func reset() {
        guard let tv = textView else { return }
        highlighter.reset(tv.textStorage?.string as NSString? ?? "")
        painted = []
        pendingDirty = nil
        guard enabled else { return }
        extend(to: Self.window(for: tv))
    }

    func clear() {
        guard let tv = textView, let lm = tv.layoutManager else { return }
        let whole = NSRange(location: 0, length: tv.textStorage?.length ?? 0)
        for range in painted {
            let r = NSIntersectionRange(range, whole)
            if r.length > 0 { lm.removeTemporaryAttribute(Self.key, forCharacterRange: r) }
        }
        painted = []
    }

    /// The view scrolled: paint the part of the new window not yet painted.
    func scrolled() {
        guard enabled, let tv = textView else { return }
        let window = Self.window(for: tv)
        guard window.length > 0, !gaps(in: window).isEmpty else { return }
        extend(to: window)
    }

    private func storageEdited(range: NSRange, replacementLength: Int) {
        guard let tv = textView else { return }
        let t0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        let text = tv.textStorage?.string as NSString? ?? ""
        if text.length != highlighter.length - range.length + replacementLength {
            reset(); return // out of sync (should not happen): start over
        }
        let dirty = highlighter.edit(range: range, replacementLength: replacementLength, text: text)
        lastEditLinesLexed = highlighter.lastEditLinesLexed
        shiftPainted(edit: range, replacementLength: replacementLength)
        pendingDirty = pendingDirty.map { NSUnionRange(Self.shifted($0, edit: range, replacementLength: replacementLength), dirty) } ?? dirty
        lastEditCpuNs = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - t0
        // Painting waits until the layout manager has processed this edit
        // (it shifts the existing temporary attributes): the editor's
        // `textDidChange` flushes right away; this covers edits that post none.
        if !flushScheduled {
            flushScheduled = true
            DispatchQueue.main.async { [weak self] in self?.flushScheduled = false; self?.flush() }
        }
    }

    /// Repaints the pending dirty range (unless marked text exists; then a
    /// later flush after the commit does it).
    func flush() {
        guard enabled, let tv = textView, let dirty = pendingDirty else { return }
        if tv.hasMarkedText() {
            if !flushScheduled {
                flushScheduled = true
                DispatchQueue.main.async { [weak self] in self?.flushScheduled = false; self?.flush() }
            }
            return
        }
        pendingDirty = nil
        guard let lm = tv.layoutManager else { return }
        let t0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        defer { lastFlushCpuNs = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - t0 }
        let text = tv.textStorage?.string as NSString? ?? ""
        let whole = NSRange(location: 0, length: text.length)
        var count = 0
        for range in painted {
            let r = NSIntersectionRange(NSIntersectionRange(range, dirty), whole)
            guard r.length > 0 else { continue }
            lm.removeTemporaryAttribute(Self.key, forCharacterRange: r)
            count += paint(highlighter.runs(in: r, text: text), layoutManager: lm)
        }
        paints += 1
        runsPainted += count
    }

    /// `r` after replacing `edit` with `replacementLength` characters. An edit
    /// that touches `r` (ends at its start or starts at its end) joins it, so
    /// text typed at either edge of a painted range is repainted with it
    /// (GH#280: typing at the end of the painted window stayed uncoloured).
    static func shifted(_ r: NSRange, edit: NSRange, replacementLength: Int) -> NSRange {
        let delta = replacementLength - edit.length
        if NSMaxRange(edit) < r.location { return NSRange(location: r.location + delta, length: r.length) }
        if edit.location > NSMaxRange(r) { return r }
        let start = min(r.location, edit.location)
        let end = max(NSMaxRange(r), NSMaxRange(edit)) + delta
        return NSRange(location: start, length: max(0, end - start))
    }

    private func shiftPainted(edit: NSRange, replacementLength: Int) {
        painted = Self.merged(painted.map { Self.shifted($0, edit: edit, replacementLength: replacementLength) })
    }

    func gaps(in window: NSRange) -> [NSRange] {
        var result: [NSRange] = []
        var cursor = window.location
        for range in painted where NSMaxRange(range) > cursor {
            if range.location >= NSMaxRange(window) { break }
            if range.location > cursor { result.append(NSRange(location: cursor, length: range.location - cursor)) }
            cursor = max(cursor, NSMaxRange(range))
        }
        if cursor < NSMaxRange(window) { result.append(NSRange(location: cursor, length: NSMaxRange(window) - cursor)) }
        return result
    }

    private func extend(to window: NSRange) {
        guard let tv = textView, let lm = tv.layoutManager else { return }
        let text = tv.textStorage?.string as NSString? ?? ""
        let whole = NSRange(location: 0, length: text.length)
        var count = 0
        for gap in gaps(in: window) {
            let r = NSIntersectionRange(gap, whole)
            guard r.length > 0 else { continue }
            count += paint(highlighter.runs(in: r, text: text), layoutManager: lm)
        }
        if count > 0 { paints += 1; runsPainted += count }
        painted = Self.merged(painted + [window])
    }

    @discardableResult
    private func paint(_ runs: [SyntaxHighlighter.Run], layoutManager lm: NSLayoutManager) -> Int {
        var n = 0
        for run in runs {
            guard let color = SyntaxTheme.color(for: run.kind) else { continue }
            lm.addTemporaryAttribute(Self.key, value: color, forCharacterRange: run.range)
            n += 1
        }
        return n
    }

    static func merged(_ ranges: [NSRange]) -> [NSRange] {
        SourceEditorView.MarkPainter.merged(ranges)
    }

    static func window(for tv: NSTextView) -> NSRange {
        let length = tv.textStorage?.length ?? 0
        guard let lm = tv.layoutManager, let container = tv.textContainer, tv.window != nil,
              !tv.visibleRect.isEmpty else { return NSRange(location: 0, length: length) }
        let glyphs = lm.glyphRange(forBoundingRect: tv.visibleRect, in: container)
        let visible = lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        let start = max(0, visible.location - padding)
        let end = min(length, NSMaxRange(visible) + padding)
        return NSRange(location: start, length: max(0, end - start))
    }
}
