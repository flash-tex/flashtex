import Foundation

/// The lexical scan and `\begin`/`\end` pairing both editors share, moved
/// here from the Mac's `EditorNavigation` (which forwards to it) so the iPad
/// and TeXpand's scope provider use the same rules.
///
/// The rule set: a `%` that is not `\%` comments the rest of its line,
/// `\begin{verbatim}`-like environments (`SyntaxHighlighter.verbatimEnvironments`)
/// and `\verb<d>…<d>` are skipped whole, and `\\` / `\{` / `\%` are escapes.
/// Nothing here parses TeX. All offsets are UTF-16.
public enum LaTeXScan {
    // MARK: lexical scan

    /// One control sequence with its complete braced argument, in document
    /// order: `\name{arg}` (`range` spans the backslash through `}`), or a
    /// bare `\name` (`argRange` nil) for every other command.
    public struct Use: Equatable, Sendable {
        public var name: String
        /// The whole `\name` (or `\name{arg}`) span.
        public var range: NSRange
        /// The text between the braces, when the command has a braced argument right after its name.
        public var arg: String?
        public var argRange: NSRange?
        public init(name: String, range: NSRange, arg: String?, argRange: NSRange?) {
            self.name = name; self.range = range; self.arg = arg; self.argRange = argRange
        }
    }

    /// Every command of `text` outside comments and verbatim, with its
    /// immediate braced argument when one follows (whitespace allowed
    /// between name and `{` only for `\begin`/`\end`, which LaTeX permits).
    public static func uses(in text: NSString) -> [Use] {
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

    public struct EnvironmentPair: Equatable, Sendable {
        public var name: String
        /// `\begin{name}` span.
        public var begin: NSRange
        /// `\end{name}` span; nil when the environment is never closed.
        public var end: NSRange?

        public init(name: String, begin: NSRange, end: NSRange?) { self.name = name; self.begin = begin; self.end = end }

        /// From the `\begin` through the `\end` (or to `limit` when unclosed).
        public func whole(limit: Int) -> NSRange {
            let stop = end.map(NSMaxRange) ?? limit
            return NSRange(location: begin.location, length: max(0, stop - begin.location))
        }
    }

    /// Every `\begin{X}` paired with its `\end{X}` by a per-name stack, so
    /// nesting of the same name and unbalanced text both resolve: a stray
    /// `\end` is ignored, an unclosed `\begin` keeps `end == nil`.
    public static func environmentPairs(in text: NSString) -> [EnvironmentPair] {
        var pairs: [EnvironmentPair] = []
        var open: [String: [Int]] = [:] // name → indices into pairs
        forEachEnvironmentUse(in: text) { isBegin, range, arg in
            guard !arg.isEmpty else { return }
            if isBegin {
                open[arg, default: []].append(pairs.count)
                pairs.append(EnvironmentPair(name: arg, begin: range, end: nil))
            } else if let i = open[arg]?.popLast() {
                pairs[i].end = range
            }
        }
        return pairs
    }

    /// Exactly the `\begin{arg}` / `\end{arg}` uses that `uses(in:)` yields,
    /// in order, from the same lexical walk — but over one copy of the UTF-16
    /// units, allocating a `String` only for those arguments. `uses(in:)`
    /// reads the text through `character(at:)` and materialises every
    /// command's name and argument; on the editor's bridged buffer that cost
    /// 2.4 ms per keystroke at 500 KB, 12 ms once the text held a non-ASCII
    /// character (APP-PERF-AUDIT), and the brace highlight runs this scan on
    /// every keystroke on a `\begin`/`\end` line. `AppPerfAuditTests`
    /// checks the pairs equal the `uses(in:)`-based construction.
    public static func forEachEnvironmentUse(in text: NSString, _ body: (_ isBegin: Bool, _ range: NSRange, _ arg: String) -> Void) {
        let n = text.length
        guard n > 0 else { return }
        let buffer = UnsafeMutableBufferPointer<unichar>.allocate(capacity: n)
        defer { buffer.deallocate() }
        text.getCharacters(buffer.baseAddress!, range: NSRange(location: 0, length: n))
        let t = UnsafeBufferPointer(buffer)
        func isLetter(_ c: unichar) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
        /// Whether units `from..<to` spell `word` (ASCII).
        func spells(_ word: StaticString, _ from: Int, _ to: Int) -> Bool {
            guard to - from == word.utf8CodeUnitCount else { return false }
            return word.withUTF8Buffer { w in
                for (o, b) in w.enumerated() where t[from + o] != unichar(b) { return false }
                return true
            }
        }
        /// First index >= `from` where the units of `closer` start (literal), or nil.
        func find(_ closer: [unichar], from: Int) -> Int? {
            guard let first = closer.first, closer.count <= n else { return nil }
            var p = from
            while p + closer.count <= n {
                if t[p] == first {
                    var q = 1
                    while q < closer.count, t[p + q] == closer[q] { q += 1 }
                    if q == closer.count { return p }
                }
                p += 1
            }
            return nil
        }
        var i = 0
        var verbatimUntil: [unichar]? // `\end{name}` that closes the skipped block
        while i < n {
            if let closer = verbatimUntil {
                guard let r = find(closer, from: i) else { break }
                i = r
                verbatimUntil = nil
            }
            let c = t[i]
            if c == 0x25 { // '%'
                while i < n, t[i] != 0x0A { i += 1 }
                continue
            }
            guard c == 0x5C else { i += 1; continue } // '\'
            var j = i + 1
            while j < n, isLetter(t[j]) { j += 1 }
            if j == i + 1 { i = min(n, i + 2); continue } // control symbol
            if j < n, t[j] == 0x2A { j += 1 } // `\name*`
            let nameStart = i + 1, nameEnd = j
            if (spells("verb", nameStart, nameEnd) || spells("verb*", nameStart, nameEnd)), j < n {
                let d = t[j]
                var k = j + 1
                while k < n, t[k] != d, t[k] != 0x0A { k += 1 }
                i = min(n, k + 1)
                continue
            }
            let isBegin = spells("begin", nameStart, nameEnd)
            let isEnd = !isBegin && spells("end", nameStart, nameEnd)
            let isLabel = !isBegin && !isEnd && spells("label", nameStart, nameEnd)
            var k = j
            if isBegin || isEnd { while k < n, t[k] == 0x20 { k += 1 } }
            if k < n, t[k] == 0x7B { // '{'
                var m = k + 1
                var depth = 1
                while m < n {
                    let d = t[m]
                    if d == 0x5C { m += 2; continue }
                    if d == 0x7B { depth += 1 } else if d == 0x7D { depth -= 1; if depth == 0 { break } }
                    if d == 0x0A, depth > 0, isBegin || isEnd || isLabel { break } // an env/label name never spans lines
                    m += 1
                }
                if m < n, t[m] == 0x7D {
                    if isBegin || isEnd {
                        let arg = text.substring(with: NSRange(location: k + 1, length: m - k - 1))
                        body(isBegin, NSRange(location: i, length: m + 1 - i), arg)
                        if isBegin, SyntaxHighlighter.verbatimEnvironments.contains(arg) || arg == "comment" {
                            verbatimUntil = Array("\\end{\(arg)}".utf16)
                        }
                    }
                    i = k + 1 // keep lexing inside the argument, as `uses(in:)` does
                    continue
                }
            }
            i = j
        }
    }

    /// The pair whose `\begin` or `\end` contains `caret` (a caret right after
    /// the closing brace counts as on it, like bracket matching).
    public static func environmentPair(at caret: Int, in text: NSString) -> EnvironmentPair? {
        environmentPairs(in: text).first { p in
            NSLocationInRange(caret, p.begin) || caret == NSMaxRange(p.begin)
                || (p.end.map { NSLocationInRange(caret, $0) || caret == NSMaxRange($0) } ?? false)
        }
    }

    /// The innermost environment whose span contains `caret` (the `document`
    /// environment included: selecting it selects the body).
    public static func enclosingEnvironment(at caret: Int, in text: NSString) -> EnvironmentPair? {
        var best: EnvironmentPair?
        for p in environmentPairs(in: text) {
            let whole = p.whole(limit: text.length)
            guard whole.location <= caret, caret <= NSMaxRange(whole) else { continue }
            if best == nil || whole.location >= best!.begin.location { best = p }
        }
        return best
    }
}
