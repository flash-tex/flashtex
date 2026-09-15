import Foundation
import FlashTeXProtocol

/// Editor navigation intelligence (lane mac-editor-dx-3): environment pair
/// matching for highlight / select / wrap, project-wide symbol rename plans
/// for `\label` keys and user commands, user command definitions for
/// go-to-definition and the hover peek, the fuzzy symbol picker, and Go to
/// Line (`resolveLineTarget`). All pure over UTF-16 `NSString`s; the model
/// glue lives in `ShellModel+EditorNavigation.swift`, the editor glue in
/// `SourceEditorView`.
///
/// The scanners share one lexical rule set: a `%` that is not `\%` comments
/// the rest of its line, `\begin{verbatim}`-like environments (the
/// `SyntaxHighlighter.verbatimEnvironments` set) and `\verb<d>…<d>` are
/// skipped whole, and `\\` / `\{` / `\%` are escapes. Nothing here parses TeX.
enum EditorNavigation {
    // MARK: lexical scan

    /// One control sequence with its complete braced argument, in document
    /// order: `\name{arg}` (`range` spans the backslash through `}`), or a
    /// bare `\name` (`argRange` nil) for every other command.
    struct Use: Equatable {
        var name: String
        /// The whole `\name` (or `\name{arg}`) span.
        var range: NSRange
        /// The text between the braces, when the command has a braced argument right after its name.
        var arg: String?
        var argRange: NSRange?
    }

    /// Every command of `text` outside comments and verbatim, with its
    /// immediate braced argument when one follows (whitespace allowed
    /// between name and `{` only for `\begin`/`\end`, which LaTeX permits).
    static func uses(in text: NSString) -> [Use] {
        var out: [Use] = []
        let n = text.length
        var i = 0
        var verbatimUntil: String? // `\end{name}` that closes the skipped block
        func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
        while i < n {
            if let closer = verbatimUntil {
                // Skip to the matching `\end{name}`; the `\end` itself is emitted.
                let r = text.range(of: closer, options: .literal, range: NSRange(location: i, length: n - i))
                guard r.location != NSNotFound else { break }
                i = r.location
                verbatimUntil = nil
                // fall through to lex the `\end`
            }
            let c = text.character(at: i)
            if c == 0x25 { // '%'
                while i < n, text.character(at: i) != 0x0A { i += 1 }
                continue
            }
            guard c == 0x5C else { i += 1; continue } // '\'
            var j = i + 1
            while j < n, isLetter(text.character(at: j)) { j += 1 }
            if j == i + 1 { // control symbol: `\\`, `\%`, `\{`
                i = min(n, i + 2)
                continue
            }
            if j < n, text.character(at: j) == 0x2A { j += 1 } // `\newcommand*`, `\section*`
            let name = text.substring(with: NSRange(location: i + 1, length: j - i - 1))
            if name == "verb" || name == "verb*", j < n { // `\verb<d>…<d>` to the delimiter or line end
                let d = text.character(at: j)
                var k = j + 1
                while k < n, text.character(at: k) != d, text.character(at: k) != 0x0A { k += 1 }
                out.append(Use(name: name, range: NSRange(location: i, length: min(n, k + 1) - i), arg: nil, argRange: nil))
                i = min(n, k + 1)
                continue
            }
            var k = j
            if name == "begin" || name == "end" { while k < n, text.character(at: k) == 0x20 { k += 1 } }
            if k < n, text.character(at: k) == 0x7B { // '{'
                var m = k + 1
                var depth = 1
                while m < n {
                    let d = text.character(at: m)
                    if d == 0x5C { m += 2; continue }
                    if d == 0x7B { depth += 1 } else if d == 0x7D { depth -= 1; if depth == 0 { break } }
                    if d == 0x0A, depth > 0, name == "begin" || name == "end" || name == "label" { break } // an env/label name never spans lines
                    m += 1
                }
                if m < n, text.character(at: m) == 0x7D {
                    let arg = text.substring(with: NSRange(location: k + 1, length: m - k - 1))
                    out.append(Use(name: name, range: NSRange(location: i, length: m + 1 - i), arg: arg, argRange: NSRange(location: k + 1, length: m - k - 1)))
                    if name == "begin", SyntaxHighlighter.verbatimEnvironments.contains(arg) || arg == "comment" { verbatimUntil = "\\end{\(arg)}" }
                    i = k + 1 // keep lexing inside the argument: `\newcommand{\foo}` also yields the `\foo` use
                    continue
                }
            }
            out.append(Use(name: name, range: NSRange(location: i, length: j - i), arg: nil, argRange: nil))
            i = j
        }
        return out
    }

    // MARK: environment pairs

    struct EnvironmentPair: Equatable {
        var name: String
        /// `\begin{name}` span.
        var begin: NSRange
        /// `\end{name}` span; nil when the environment is never closed.
        var end: NSRange?

        /// From the `\begin` through the `\end` (or to `limit` when unclosed).
        func whole(limit: Int) -> NSRange {
            let stop = end.map(NSMaxRange) ?? limit
            return NSRange(location: begin.location, length: max(0, stop - begin.location))
        }
    }

    /// Every `\begin{X}` paired with its `\end{X}` by a per-name stack, so
    /// nesting of the same name and unbalanced text both resolve: a stray
    /// `\end` is ignored, an unclosed `\begin` keeps `end == nil`.
    static func environmentPairs(in text: NSString) -> [EnvironmentPair] {
        var pairs: [EnvironmentPair] = []
        var open: [String: [Int]] = [:] // name → indices into pairs
        for u in uses(in: text) {
            guard let arg = u.arg, !arg.isEmpty else { continue }
            if u.name == "begin" {
                open[arg, default: []].append(pairs.count)
                pairs.append(EnvironmentPair(name: arg, begin: u.range, end: nil))
            } else if u.name == "end", let i = open[arg]?.popLast() {
                pairs[i].end = u.range
            }
        }
        return pairs
    }

    /// The pair whose `\begin` or `\end` contains `caret` (a caret right after
    /// the closing brace counts as on it, like bracket matching).
    static func environmentPair(at caret: Int, in text: NSString) -> EnvironmentPair? {
        environmentPairs(in: text).first { p in
            NSLocationInRange(caret, p.begin) || caret == NSMaxRange(p.begin)
                || (p.end.map { NSLocationInRange(caret, $0) || caret == NSMaxRange($0) } ?? false)
        }
    }

    /// The innermost environment whose span contains `caret` (the `document`
    /// environment included: selecting it selects the body).
    static func enclosingEnvironment(at caret: Int, in text: NSString) -> EnvironmentPair? {
        var best: EnvironmentPair?
        for p in environmentPairs(in: text) {
            let whole = p.whole(limit: text.length)
            guard whole.location <= caret, caret <= NSMaxRange(whole) else { continue }
            if best == nil || whole.location >= best!.begin.location { best = p }
        }
        return best
    }

    /// ⌘⇧A: the whole environment around `selection` — the innermost one
    /// when the selection is a caret, and when the selection already is a
    /// whole environment the next one out (repeat to widen).
    static func selectEnvironment(around selection: NSRange, in text: NSString) -> EnvironmentPair? {
        let pairs = environmentPairs(in: text)
        let enclosing = pairs.filter { p in
            let whole = p.whole(limit: text.length)
            return whole.location <= selection.location && NSMaxRange(selection) <= NSMaxRange(whole)
        }.sorted { $0.begin.location > $1.begin.location } // innermost first
        // An exactly selected environment widens to its parent.
        if let i = enclosing.firstIndex(where: { $0.whole(limit: text.length) == selection }) {
            return i + 1 < enclosing.count ? enclosing[i + 1] : nil
        }
        return enclosing.first
    }

    struct Wrap: Equatable {
        var range: NSRange
        var replacement: String
        /// Where the caret goes: at the start of the wrapped body.
        var selection: NSRange
    }

    /// ⌘⇧W: `\begin{env}` … `\end{env}` around `selection`. A selection that
    /// covers whole lines (or is empty) becomes a block: the environment on
    /// its own lines at the first line's indentation, every non-blank body
    /// line indented one more `indentUnit` (none for verbatim-like
    /// environments); anything else is wrapped inline. The caret lands at
    /// the start of the body.
    static func wrap(selection: NSRange, in text: NSString, environment env: String, indentUnit: String) -> Wrap {
        let sel = NSRange(location: max(0, min(selection.location, text.length)),
                          length: max(0, min(selection.length, text.length - min(selection.location, text.length))))
        var lineStart = sel.location
        while lineStart > 0, text.character(at: lineStart - 1) != 0x0A { lineStart -= 1 }
        let prefix = text.substring(with: NSRange(location: lineStart, length: sel.location - lineStart))
        let blank: (Character) -> Bool = { $0 == " " || $0 == "\t" }
        let atLineStart = prefix.allSatisfy(blank)
        var end = NSMaxRange(sel)
        if sel.length > 0, text.character(at: end - 1) == 0x0A { end -= 1 } // a selection ending after a newline excludes the next line
        let atLineEnd = end == text.length || text.character(at: end) == 0x0A
        let begin = "\\begin{\(env)}", close = "\\end{\(env)}"
        guard sel.length == 0 || (atLineStart && atLineEnd) else {
            let body = text.substring(with: sel)
            return Wrap(range: sel, replacement: begin + body + close, selection: NSRange(location: sel.location + begin.utf16.count, length: 0))
        }
        var lineEnd = end
        while lineEnd < text.length, text.character(at: lineEnd) != 0x0A { lineEnd += 1 }
        let block = NSRange(location: lineStart, length: lineEnd - lineStart)
        var lines = text.substring(with: block).components(separatedBy: "\n")
        let indent = String((lines.first ?? "").prefix(while: blank)) // the block's own indentation
        let unit = SyntaxHighlighter.verbatimEnvironments.contains(env) ? "" : indentUnit
        if sel.length == 0, lines.allSatisfy({ $0.allSatisfy(blank) }) { lines = [indent] } // a blank line becomes the (indented) body line
        let body = lines.map { $0.allSatisfy(blank) && sel.length > 0 ? "" : unit + $0 }.joined(separator: "\n")
        let head = indent + begin + "\n"
        let out = head + body + "\n" + indent + close
        let firstIndent = (lines.first ?? "").prefix(while: blank).utf16.count
        let caret = lineStart + head.utf16.count + unit.utf16.count + firstIndent
        return Wrap(range: block, replacement: out, selection: NSRange(location: caret, length: 0))
    }

    // MARK: symbol rename

    enum Symbol: Equatable {
        /// A `\label` key (defined by `\label{key}`, used by `\ref`-family commands).
        case label(String)
        /// A user control sequence `\name` (defined by `\newcommand`/`\def`/…).
        case command(String)

        var displayName: String {
            switch self {
            case .label(let k): k
            case .command(let n): "\\" + n
            }
        }
    }

    /// Commands whose braced argument is a comma-separated list of label keys.
    static let labelCommands: Set<String> = [
        "label", "ref", "eqref", "pageref", "autoref", "cref", "Cref", "cpageref", "Cpageref", "nameref", "vref", "Vref",
        "fref", "Fref", "labelcref", "refstepcounter",
    ]

    /// The renameable symbol under `caret`: a label key when the caret is in a
    /// `\label`/`\ref`-family argument, else the user command under it.
    /// Built-ins (`\section`, `\begin`) are still returned as commands — the
    /// rename refuses them when no definition exists in the project.
    static func symbol(at caret: Int, in text: NSString) -> Symbol? {
        // Innermost first: a `\foo` inside `\newcommand{\foo}` comes after its definer in document order.
        for u in uses(in: text).reversed() where u.range.location <= caret && caret <= NSMaxRange(u.range) {
            if labelCommands.contains(u.name), let argRange = u.argRange,
               NSLocationInRange(caret, argRange) || caret == NSMaxRange(argRange) {
                // The comma-separated key the caret is in.
                var start = argRange.location
                var at = argRange.location
                while at < caret, at < NSMaxRange(argRange) { if text.character(at: at) == 0x2C { start = at + 1 }; at += 1 } // ','
                var end = start
                while end < NSMaxRange(argRange), text.character(at: end) != 0x2C { end += 1 }
                let key = text.substring(with: NSRange(location: start, length: end - start)).trimmingCharacters(in: .whitespaces)
                return key.isEmpty ? nil : .label(key)
            }
            if u.name == "begin" || u.name == "end" { return nil }
            // On the command name itself (not inside its argument: keep looking for an inner use).
            let nameEnd = u.range.location + 1 + (u.name as NSString).length
            if caret <= nameEnd { return .command(u.name.hasSuffix("*") ? String(u.name.dropLast()) : u.name) }
        }
        return nil
    }

    /// Every occurrence of `symbol` in `text` outside comments/verbatim: the
    /// key spans inside label-family arguments, or the `\name` spans (word
    /// boundary: `\foo` never matches `\foobar`, and `\foo*` is `\foo`).
    static func occurrences(of symbol: Symbol, in text: NSString) -> [NSRange] {
        var out: [NSRange] = []
        for u in uses(in: text) {
            switch symbol {
            case .label(let key):
                guard labelCommands.contains(u.name), let argRange = u.argRange, let arg = u.arg else { continue }
                var start = 0
                let ns = arg as NSString
                for i in 0...ns.length where i == ns.length || ns.character(at: i) == 0x2C {
                    let raw = ns.substring(with: NSRange(location: start, length: i - start))
                    let trimmed = raw.trimmingCharacters(in: .whitespaces)
                    if trimmed == key, let sub = raw.range(of: trimmed) {
                        let lead = raw.utf16.distance(from: raw.utf16.startIndex, to: sub.lowerBound.samePosition(in: raw.utf16)!)
                        out.append(NSRange(location: argRange.location + start + lead, length: (trimmed as NSString).length))
                    }
                    start = i + 1
                }
            case .command(let name):
                let base = u.name.hasSuffix("*") ? String(u.name.dropLast()) : u.name
                guard base == name else { continue }
                out.append(NSRange(location: u.range.location, length: 1 + (name as NSString).length))
            }
        }
        return out
    }

    /// One document's part of a rename: nonoverlapping edits in text order.
    struct DocumentEdits: Equatable {
        var path: String
        var ranges: [NSRange]
        var replacement: String
        /// The single grouped edit (first through last occurrence) that applies
        /// every range as one undoable replacement, and the resulting text.
        func grouped(in text: NSString) -> (range: NSRange, text: String)? {
            guard let first = ranges.first, let last = ranges.last, NSMaxRange(last) <= text.length else { return nil }
            let span = NSRange(location: first.location, length: NSMaxRange(last) - first.location)
            var out = ""
            var cursor = first.location
            for r in ranges {
                out += text.substring(with: NSRange(location: cursor, length: r.location - cursor)) + replacement
                cursor = NSMaxRange(r)
            }
            return (span, out)
        }
        func applied(to text: NSString) -> String? {
            guard let g = grouped(in: text) else { return nil }
            return text.replacingCharacters(in: g.range, with: g.text)
        }
    }

    struct RenamePlan: Equatable {
        var symbol: Symbol
        var newName: String
        var documents: [DocumentEdits]
        var count: Int { documents.reduce(0) { $0 + $1.ranges.count } }
        var summary: String {
            "\(count) occurrence\(count == 1 ? "" : "s") in \(documents.count) file\(documents.count == 1 ? "" : "s")"
        }
    }

    /// Why `newName` cannot name `symbol` (mirrors LaTeX's rules well enough
    /// to refuse before an edit; nothing here is validated by the compiler).
    static func nameProblem(_ newName: String, for symbol: Symbol) -> String? {
        switch symbol {
        case .label:
            if newName.isEmpty { return "the new key is empty" }
            if let bad = newName.first(where: { $0.isWhitespace || "{}\\%#,".contains($0) }) {
                return "the new key contains “\(bad.isWhitespace ? "whitespace" : String(bad))”"
            }
        case .command:
            let name = newName.hasPrefix("\\") ? String(newName.dropFirst()) : newName
            if name.isEmpty { return "the new command name is empty" }
            if !name.allSatisfy({ $0.isASCII && $0.isLetter }) { return "a command name is letters only (\\\(name) is not)" }
        }
        return nil
    }

    /// The project-wide plan: every occurrence in every document (the active
    /// buffer's text first, in the order given). Nil when nothing would change.
    static func renamePlan(_ symbol: Symbol, to newName: String, in documents: [(path: String, text: String)]) -> RenamePlan? {
        let replacement: String
        switch symbol {
        case .label: replacement = newName
        case .command: replacement = "\\" + (newName.hasPrefix("\\") ? String(newName.dropFirst()) : newName)
        }
        var out: [DocumentEdits] = []
        for doc in documents {
            let ranges = occurrences(of: symbol, in: doc.text as NSString)
            if !ranges.isEmpty { out.append(DocumentEdits(path: doc.path, ranges: ranges, replacement: replacement)) }
        }
        guard !out.isEmpty else { return nil }
        return RenamePlan(symbol: symbol, newName: newName, documents: out)
    }

    // MARK: user command definitions

    struct Definition: Equatable {
        var name: String
        /// The defining command (`newcommand`, `def`, `DeclareMathOperator`, `newenvironment`, `let`, …).
        var via: String
        /// Whole definition span (for go-to-definition selection).
        var range: NSRange
        /// The replacement text / body, when the definition has one (`\let` has none).
        var body: String?
        /// 1-based line of the definition.
        var line: Int

        var isEnvironment: Bool { via.hasPrefix("newenvironment") || via.hasPrefix("renewenvironment") || via == "NewDocumentEnvironment" || via.hasPrefix("newtheorem") }
        /// One-line summary for the hover: `\newcommand{\R}[1]{…}`.
        var summary: String {
            let head = "\\\(via)" + (isEnvironment ? "{\(name)}" : "{\\\(name)}")
            guard let body else { return head }
            let flat = body.replacingOccurrences(of: "\n", with: " ")
            return head + "{" + (flat.count > 120 ? String(flat.prefix(117)) + "…" : flat) + "}"
        }
    }

    /// Definition commands that define a control sequence (`\newcommand{\foo}` / `\newcommand\foo`).
    static let commandDefiners: Set<String> = [
        "newcommand", "renewcommand", "providecommand", "def", "gdef", "edef", "xdef", "let", "DeclareMathOperator",
        "NewDocumentCommand", "RenewDocumentCommand", "ProvideDocumentCommand", "DeclareDocumentCommand", "DeclareRobustCommand",
        "DeclarePairedDelimiter", "newcommandx",
    ]
    /// Definition commands whose argument is an environment name.
    static let environmentDefiners: Set<String> = [
        "newenvironment", "renewenvironment", "NewDocumentEnvironment", "newtheorem",
    ]

    /// Every user definition in `text`, in document order.
    static func definitions(in text: NSString) -> [Definition] {
        var out: [Definition] = []
        let n = text.length
        func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
        func skipSpaces(_ p: inout Int) { while p < n, text.character(at: p) == 0x20 || text.character(at: p) == 0x09 { p += 1 } }
        /// A balanced `{…}` group starting at `p` (or nil): its inner range and the index after `}`.
        func group(at p: Int) -> (inner: NSRange, after: Int)? {
            guard p < n, text.character(at: p) == 0x7B else { return nil }
            var m = p + 1, depth = 1
            while m < n {
                let d = text.character(at: m)
                if d == 0x5C { m += 2; continue }
                if d == 0x7B { depth += 1 } else if d == 0x7D { depth -= 1; if depth == 0 { return (NSRange(location: p + 1, length: m - p - 1), m + 1) } }
                m += 1
            }
            return nil
        }
        func bracket(at p: Int) -> Int? { // `[...]` → index after `]`
            guard p < n, text.character(at: p) == 0x5B else { return nil }
            var m = p + 1
            while m < n, text.character(at: m) != 0x5D, text.character(at: m) != 0x0A { m += 1 }
            return m < n && text.character(at: m) == 0x5D ? m + 1 : nil
        }
        var lineStarts: [Int] = [0]
        for i in 0..<n where text.character(at: i) == 0x0A { lineStarts.append(i + 1) }
        for u in uses(in: text) {
            let base = u.name.hasSuffix("*") ? String(u.name.dropLast()) : u.name
            let definesCommand = commandDefiners.contains(base), definesEnvironment = environmentDefiners.contains(base)
            guard definesCommand || definesEnvironment else { continue }
            var p = u.range.location + 1 + (u.name as NSString).length
            skipSpaces(&p)
            let name: String
            var after: Int
            if definesEnvironment {
                guard let g = group(at: p) else { continue }
                name = text.substring(with: g.inner).trimmingCharacters(in: .whitespaces)
                after = g.after
            } else if let g = group(at: p) { // `{\foo}`
                let inner = text.substring(with: g.inner).trimmingCharacters(in: .whitespaces)
                guard inner.hasPrefix("\\"), inner.count > 1 else { continue }
                name = String(inner.dropFirst())
                after = g.after
            } else { // `\foo` bare (`\def\foo`, `\newcommand\foo`, `\let\foo`)
                guard p < n, text.character(at: p) == 0x5C else { continue }
                var q = p + 1
                while q < n, isLetter(text.character(at: q)) { q += 1 }
                guard q > p + 1 else { continue }
                name = text.substring(with: NSRange(location: p + 1, length: q - p - 1))
                after = q
            }
            guard !name.isEmpty else { continue }
            // Body: skip `[n]`/`[default]` groups, `\def` parameter text, `\let`'s `=`; the next `{…}` is the body.
            var body: String?
            var end = after
            if base == "let" {
                skipSpaces(&end)
                if end < n, text.character(at: end) == 0x3D { end += 1 }
                skipSpaces(&end)
                var q = end
                if q < n, text.character(at: q) == 0x5C { q += 1; while q < n, isLetter(text.character(at: q)) { q += 1 }; if q == end + 1 { q = min(n, q + 1) } }
                body = q > end ? text.substring(with: NSRange(location: end, length: q - end)) : nil
                end = q
            } else {
                var q = after
                while true {
                    skipSpaces(&q)
                    if let b = bracket(at: q) { q = b; continue }
                    break
                }
                if base.hasSuffix("DocumentCommand") || base == "NewDocumentEnvironment", let spec = group(at: q) { // `{m m}` argument spec
                    q = spec.after
                    skipSpaces(&q)
                }
                if base == "def" || base == "gdef" || base == "edef" || base == "xdef" {
                    while q < n, text.character(at: q) != 0x7B, text.character(at: q) != 0x0A { q += 1 } // parameter text `#1#2`
                }
                if base == "DeclareMathOperator", definesCommand, let first = group(at: q) { // `{\op}{text}` — the first group is the name (handled) so this is the body
                    body = text.substring(with: first.inner); end = first.after
                } else if let g = group(at: q) {
                    body = text.substring(with: g.inner); end = g.after
                    if definesEnvironment, let e = group(at: { var r = g.after; skipSpaces(&r); return r }()) { end = e.after } // `\newenvironment{x}{begin}{end}`
                }
            }
            let line = (lineStarts.lastIndex { $0 <= u.range.location } ?? 0) + 1
            out.append(Definition(name: name, via: u.name, range: NSRange(location: u.range.location, length: max(end, NSMaxRange(u.range)) - u.range.location), body: body, line: line))
        }
        return out
    }

    /// The first definition of `\name` (or environment `name`) in `text`.
    static func definition(of name: String, in text: NSString, environment: Bool = false) -> Definition? {
        definitions(in: text).first { $0.name == name && $0.isEnvironment == environment }
    }

    // MARK: symbol quick open

    /// Subsequence match with a small ranking: consecutive and word-start
    /// hits score higher, an exact prefix highest. Nil when `query` is not a
    /// subsequence of `candidate` (case-insensitive).
    static func fuzzyScore(_ query: String, in candidate: String) -> Int? {
        let q = Array(query.lowercased()), c = Array(candidate.lowercased())
        guard !q.isEmpty else { return 0 }
        if c.starts(with: q) { return 1000 - c.count }
        var score = 0, qi = 0, last = -2
        for (i, ch) in c.enumerated() where qi < q.count && ch == q[qi] {
            score += 10
            if i == last + 1 { score += 5 }
            if i == 0 || !(c[i - 1].isLetter || c[i - 1].isNumber) { score += 8 }
            last = i
            qi += 1
        }
        guard qi == q.count else { return nil }
        return score - c.count / 4
    }

    // MARK: go to line

    /// A resolved caret: UTF-16 offset on `NSString` coordinates, 1-based line,
    /// 1-based column counting extended grapheme clusters (so a flag emoji or
    /// `e\u{0301}` is one column).
    struct LineTarget: Equatable {
        var utf16: Int
        var line: Int
        var column: Int
    }

    /// Parsed Go to Line input, before it is applied to a buffer.
    enum LineSpec: Equatable {
        /// 1-based line, optional 1-based grapheme column (`42` / `42:7`).
        case absolute(line: Int, column: Int?)
        /// Added to the caret's 1-based line (`+5` / `-5`).
        case relative(delta: Int)
    }

    static let emptyLineTargetHint = "Type a line number, line:column, or +N/−N."
    static let invalidLineTargetHint = "Not a line number. Try 42, 42:7, or +5/−5."

    /// Inline-hint payload for a failed parse/resolve (`Result`'s Failure must be `Error`).
    struct LineHint: Error, Equatable {
        var message: String
    }

    /// `42`, `42:7`, `+5`, `-5`; a leading `:` is ignored so the command
    /// palette can pass `:42` through the same parser. Empty / junk fail
    /// with a short hint for the sheet.
    static func parseLineTarget(_ input: String) -> Result<LineSpec, LineHint> {
        var s = input.trimmingCharacters(in: .whitespacesAndNewlines)
        if s.hasPrefix(":") { s = String(s.dropFirst()).trimmingCharacters(in: .whitespaces) }
        guard !s.isEmpty else { return .failure(LineHint(message: emptyLineTargetHint)) }
        if s.first == "+" || s.first == "-" {
            let negative = s.first == "-"
            let digits = String(s.dropFirst())
            guard let n = saturatedDecimal(digits) else { return .failure(LineHint(message: invalidLineTargetHint)) }
            let delta = negative ? (n == Int.max ? Int.min : -n) : n
            return .success(.relative(delta: delta))
        }
        let parts = s.split(separator: ":", maxSplits: 2, omittingEmptySubsequences: false)
        if parts.count == 1 {
            guard let line = saturatedDecimal(String(parts[0])) else { return .failure(LineHint(message: invalidLineTargetHint)) }
            return .success(.absolute(line: line, column: nil))
        }
        if parts.count == 2 {
            guard let line = saturatedDecimal(String(parts[0])), let column = saturatedDecimal(String(parts[1])) else {
                return .failure(LineHint(message: invalidLineTargetHint))
            }
            return .success(.absolute(line: line, column: column))
        }
        return .failure(LineHint(message: invalidLineTargetHint))
    }

    /// Maps `input` onto `text`: out-of-range line/column clamp to the last
    /// line or the last grapheme column of that line; invalid text is a hint.
    /// `caret` is a UTF-16 offset (clamped) used only for `+N`/`-N`.
    static func resolveLineTarget(_ text: String, input: String, caret: Int) -> Result<LineTarget, LineHint> {
        switch parseLineTarget(input) {
        case .failure(let hint): return .failure(hint)
        case .success(let spec):
            let ns = text as NSString
            let lines = lineSpans(in: ns)
            let last = lines.count
            let caretLine: Int = {
                let c = min(max(caret, 0), ns.length)
                if let i = lines.firstIndex(where: { c < $0.end || (c == ns.length && $0.start == ns.length) }) {
                    return i + 1
                }
                return last
            }()
            let requested: (line: Int, column: Int?)
            switch spec {
            case .absolute(let line, let column): requested = (line, column)
            case .relative(let delta):
                let sum = caretLine.addingReportingOverflow(delta)
                requested = (sum.overflow ? (delta > 0 ? Int.max : 1) : sum.partialValue, nil)
            }
            let line = min(max(requested.line, 1), last)
            let span = lines[line - 1]
            let content = ns.substring(with: NSRange(location: span.start, length: span.contentsEnd - span.start))
            let graphemes = Array(content)
            let maxColumn = graphemes.count + 1
            let column = min(max(requested.column ?? 1, 1), maxColumn)
            var utf16 = span.start
            for g in graphemes.prefix(column - 1) { utf16 += String(g).utf16.count }
            return .success(LineTarget(utf16: utf16, line: line, column: column))
        }
    }

    /// Non-empty all-digits string → Int, saturating at `Int.max`; nil otherwise.
    private static func saturatedDecimal(_ s: String) -> Int? {
        guard !s.isEmpty else { return nil }
        var n = 0
        for ch in s.unicodeScalars {
            guard ch >= "0" && ch <= "9" else { return nil }
            let d = Int(ch.value - 48)
            if n > (Int.max - d) / 10 { return Int.max }
            n = n * 10 + d
        }
        return n
    }

    /// Every Cocoa line of `ns`, including an empty last line after a trailing
    /// terminator. `end` includes the delimiter; `contentsEnd` does not.
    private static func lineSpans(in ns: NSString) -> [(start: Int, contentsEnd: Int, end: Int)] {
        let n = ns.length
        if n == 0 { return [(0, 0, 0)] }
        var out: [(start: Int, contentsEnd: Int, end: Int)] = []
        var loc = 0
        while loc < n {
            var start = 0, end = 0, contentsEnd = 0
            ns.getLineStart(&start, end: &end, contentsEnd: &contentsEnd, for: NSRange(location: loc, length: 0))
            out.append((start, contentsEnd, end))
            if end <= loc { break }
            loc = end
        }
        if let last = out.last, last.contentsEnd < last.end {
            out.append((n, n, n))
        }
        return out
    }
}
