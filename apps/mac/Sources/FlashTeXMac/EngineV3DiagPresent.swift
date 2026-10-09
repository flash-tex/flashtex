import Foundation
import FlashTeXDisplayListV3
import FlashTeXProtocol

/// How the new engine's diagnostics read in the Problems panel and the error
/// lens (lane DIAG-PARITY): a short headline, rows that say the same thing
/// once, box warnings on the exact text of their line, and the definition
/// of the macro an error happened in. All of it is presentation of what the
/// host sent and the texts the compile read: the engine's output (and the
/// DIAGs themselves) are untouched. Pure.
enum EngineV3DiagPresent {
    // MARK: Headline

    /// The row's headline: TeX's message without the parts the row already
    /// shows (the "LaTeX Warning:" family prefix, "on input line N", the
    /// final period). A box warning is its kind alone ("Overfull \hbox"), so
    /// fifty of them group into one row; its amount is a note (`boxNote`).
    static func headline(code: String, message: String) -> String {
        if let kind = boxKind(code) { return kind }
        var s = message
        for p in ["LaTeX Error: ", "LaTeX3 Error: ", "LaTeX Warning: ", "LaTeX Font Warning: "] where s.hasPrefix(p) {
            s.removeFirst(p.count)
            break
        }
        // "Package hyperref Warning: x" / "Class beamer Error: x" -> "hyperref: x"
        if let r = s.range(of: #"^(Package|Class) [^ ]+ (Warning|Error): "#, options: .regularExpression) {
            let name = s[r].split(separator: " ")[1]
            s = "\(name): " + s[r.upperBound...]
        }
        s = s.replacingOccurrences(of: #"\s*on input line \d+"#, with: "", options: .regularExpression)
        s = s.replacingOccurrences(of: #"\s+"#, with: " ", options: .regularExpression)
        while let last = s.last, last == "." || last == " ", !s.hasSuffix("..."), !s.hasSuffix("\\right.") { s.removeLast() }
        return s.isEmpty ? message : s
    }

    /// "Overfull \hbox" for `tex/overfull-hbox` (and the other three), else nil.
    static func boxKind(_ code: String) -> String? {
        switch code {
        case "tex/overfull-hbox": return "Overfull \\hbox"
        case "tex/underfull-hbox": return "Underfull \\hbox"
        case "tex/overfull-vbox": return "Overfull \\vbox"
        case "tex/underfull-vbox": return "Underfull \\vbox"
        default: return nil
        }
    }

    /// "Overfull \hbox (7.77853pt too wide) in paragraph at lines 7--8" ->
    /// "7.78pt too wide, in the paragraph at lines 7–8".
    static func boxNote(_ message: String) -> String? {
        guard let open = message.firstIndex(of: "("), let close = message[open...].firstIndex(of: ")") else { return nil }
        var amount = String(message[message.index(after: open) ..< close])
        if let r = amount.range(of: #"^-?[0-9]+\.[0-9]+"#, options: .regularExpression), let v = Double(amount[r]) {
            amount = String(format: "%.2f", v) + amount[r.upperBound...]
        }
        let rest = message[message.index(after: close)...].trimmingCharacters(in: .whitespaces)
            .replacingOccurrences(of: "--", with: "–")
            .replacingOccurrences(of: "in paragraph", with: "in the paragraph")
            .replacingOccurrences(of: "in alignment", with: "in the alignment")
            .replacingOccurrences(of: "has occurred while \\output is active", with: "while a page was being output")
        return rest.isEmpty ? amount : "\(amount), \(rest)"
    }

    // MARK: Rows that say the same thing once

    /// Rows that fold into another row (index -> the row it folds into):
    /// * LaTeX's end-of-run summaries ("There were undefined references")
    ///   when a row of their kind is listed;
    /// * the stop reports ("Emergency stop", "==> Fatal error occurred")
    ///   into the error that caused them (the nearest earlier row that stays
    ///   an error), which gains the explanation that pdfLaTeX stops there;
    ///   with no such cause, the second stop report into the first;
    /// * a row identical to an earlier one (same code, message and place).
    /// `kept`: the errors that stay errors (EngineV3ErrorPolicy).
    static func folds(_ diags: [DL3Diag], kept: Set<Int>) -> [Int: Int] {
        let summaries: [String: [String]] = [
            "latex/there-were-undefined-references": ["latex/undefined-reference", "latex/undefined-citation"],
            "latex/there-were-multiply-defined-labels": ["latex/multiply-defined-label"],
            "latex-font/some-font-shapes-were-not-available-defaults-substituted": ["latex-font/font-shape-undefined"],
        ]
        var into: [Int: Int] = [:]
        var seen: [String: Int] = [:]
        var firstStop: Int?
        for (i, d) in diags.enumerated() {
            if let kinds = summaries[d.code], let t = diags.indices.first(where: { kinds.contains(diags[$0].code) }) {
                into[i] = t
                continue
            }
            if isStop(d.code) {
                if let cause = (0 ..< i).reversed().first(where: { into[$0] == nil && kept.contains($0) && diags[$0].severity == "error" && !isStop(diags[$0].code) }) {
                    into[i] = cause
                    continue
                }
                if let f = firstStop { into[i] = f; continue }
                firstStop = i
            }
            let key = [d.severity ?? "", d.code, d.message, d.file ?? "", d.line.map(String.init) ?? "", d.col.map(String.init) ?? ""].joined(separator: "\u{1}")
            if let f = seen[key] { into[i] = f } else { seen[key] = i }
        }
        return into
    }

    static func isStop(_ code: String) -> Bool { code == "tex/emergency-stop" || code == "tex/fatal-error-no-output" }

    // MARK: Lexical facts of the project (the texts the compile read)

    /// A place in one of the texts.
    struct Site: Equatable {
        var path: String
        var start: Int, end: Int
        var line: Int
        var source: RuntimeV1.SourceRange { .init(path: path, startByte: start, endByte: end) }
        var label: String { "\(path):\(line)" }
    }

    /// Every match of `pattern` in `texts` (paths in order), as sites of the
    /// whole match or of `group`.
    static func matches(_ pattern: String, in texts: [String: String], group: Int = 0) -> [Site] {
        guard let re = try? NSRegularExpression(pattern: pattern) else { return [] }
        var out: [Site] = []
        for path in texts.keys.sorted() {
            guard let text = texts[path] else { continue }
            let ns = text as NSString
            // Matches come in order: bytes and lines are counted from the last one.
            var at = 0, bytes = 0, line = 1
            for m in re.matches(in: text, range: NSRange(location: 0, length: ns.length)) {
                let r = m.range(at: group)
                guard r.location != NSNotFound, r.location >= at, !inComment(ns, at: r.location) else { continue }
                let gap = ns.substring(with: NSRange(location: at, length: r.location - at))
                bytes += gap.utf8.count
                line += gap.utf8.reduce(0) { $1 == 0x0A ? $0 + 1 : $0 }
                at = r.location
                let len = ns.substring(with: r).utf8.count
                out.append(Site(path: path, start: bytes, end: bytes + len, line: line))
            }
        }
        return out
    }

    /// Whether UTF-16 offset `at` is after an unescaped `%` on its line.
    static func inComment(_ ns: NSString, at: Int) -> Bool {
        var i = at - 1
        while i >= 0 {
            let c = ns.character(at: i)
            if c == 0x0A { return false }
            if c == 0x25 { // %
                var k = i - 1, slashes = 0
                while k >= 0, ns.character(at: k) == 0x5C { slashes += 1; k -= 1 }
                if slashes % 2 == 0 { return true }
            }
            i -= 1
        }
        return false
    }

    /// The keys of every `\label{…}` in the texts.
    static func labels(in texts: [String: String]) -> [String] {
        Array(Set(matches(#"\\label\s*\{([^{}\s]+)\}"#, in: texts, group: 1).compactMap { text(of: $0, in: texts) })).sorted()
    }

    /// The citation keys of the texts: `\bibitem{…}` and `.bib` entries.
    static func citeKeys(in texts: [String: String]) -> [String] {
        let items = matches(#"\\bibitem\s*(?:\[[^\]]*\])?\s*\{([^{}\s]+)\}"#, in: texts, group: 1)
        let bib = matches(#"@[A-Za-z]+\s*[{(]\s*([^,\s{}()]+)\s*,"#, in: texts.filter { $0.key.hasSuffix(".bib") }, group: 1)
        return Array(Set((items + bib).compactMap { text(of: $0, in: texts) })).sorted()
    }

    static func text(of s: Site, in texts: [String: String]) -> String? { texts[s.path]?.utf8Slice(s.start, s.end) }

    /// `\X` where `name` (as TeX prints a macro: `\note`, `\note `) is
    /// defined in the project: one `\newcommand`-family, `\def`-family or
    /// xparse definition of it, or (for an environment's `\X` / `\endX`) one
    /// `\newenvironment{X}`. Nil when there is none or more than one.
    ///
    /// `near`: where the engine says the definition was made (the trace's
    /// `def`: the file and the line TeX was reading, #1593). Then the
    /// definition is the last one of `name` in that file starting on or
    /// before that line (a `\newcommand` is made when its arguments have
    /// been read, so the line may be its last), however many the project
    /// has; nil when that file has none there.
    static func definition(of name: String, in texts: [String: String],
                           near: (path: String, line: Int)? = nil) -> (site: Site, body: Range<Int>?)? {
        if let near {
            guard let text = texts[near.path] else { return nil }
            return definitions(of: name, in: [near.path: text])
                .filter { $0.site.line <= near.line }
                .max { $0.site.start < $1.site.start }
                .map { ($0.site, body(in: text, after: $0.site.end, groups: $0.environment ? 2 : 1)) }
        }
        let all = definitions(of: name, in: texts)
        guard all.count == 1, let one = all.first, let text = texts[one.site.path] else { return nil }
        return (one.site, body(in: text, after: one.site.end, groups: one.environment ? 2 : 1))
    }

    /// Every project definition of `name` (see `definition(of:in:near:)`):
    /// commands and `\def`s, else environments.
    private static func definitions(of name: String, in texts: [String: String]) -> [(site: Site, environment: Bool)] {
        let cs = name.trimmingCharacters(in: .whitespaces)
        guard cs.hasPrefix("\\"), cs.count > 1, !cs.contains("@") else { return [] }
        let bare = NSRegularExpression.escapedPattern(for: String(cs.dropFirst()))
        let command = #"\\(?:(?:re)?newcommand|providecommand|DeclareRobustCommand|(?:New|Renew|Provide|Declare)DocumentCommand)\*?\s*\{?\s*(\\"# + bare + #")(?![A-Za-z@])"#
        let def = #"\\[gex]?def\s*(\\"# + bare + #")(?![A-Za-z@])"#
        let sites = matches(command, in: texts, group: 1) + matches(def, in: texts, group: 1)
        if !sites.isEmpty { return sites.map { ($0, false) } }
        let env = cs.hasPrefix("\\end") && cs.count > 4 ? String(cs.dropFirst(4)) : String(cs.dropFirst())
        let pattern = #"\\(?:(?:re)?newenvironment|(?:New|Renew|Provide)DocumentEnvironment)\*?\s*\{("# + NSRegularExpression.escapedPattern(for: env) + #")\}"#
        return matches(pattern, in: texts, group: 1).map { ($0, true) }
    }

    /// The byte range of a definition's body after `from`: optional `[…]`
    /// arguments and a `\def`'s parameter text skipped, then `groups`
    /// balanced `{…}` groups (an environment has two). Nil when unbalanced.
    static func body(in text: String, after from: Int, groups: Int) -> Range<Int>? {
        let b = Array(text.utf8)
        var i = from
        if i < b.count, b[i] == UInt8(ascii: "}") { i += 1 } // `\newcommand{\x}`
        func skipSpace() { while i < b.count, b[i] == 0x20 || b[i] == 0x0A || b[i] == 0x09 || b[i] == 0x0D { i += 1 } }
        func group() -> Range<Int>? {
            guard i < b.count, b[i] == UInt8(ascii: "{") else { return nil }
            let start = i
            var depth = 0
            while i < b.count {
                switch b[i] {
                case UInt8(ascii: "\\"): i += 1
                case UInt8(ascii: "{"): depth += 1
                case UInt8(ascii: "}"):
                    depth -= 1
                    if depth == 0 { i += 1; return start ..< i }
                default: break
                }
                i += 1
            }
            return nil
        }
        skipSpace()
        while i < b.count, b[i] == UInt8(ascii: "[") {
            guard let close = b[i...].firstIndex(of: UInt8(ascii: "]")) else { return nil }
            i = close + 1
            skipSpace()
        }
        // A `\def`'s parameter text (`#1#2`, delimiters) up to its body, within the line.
        while i < b.count, b[i] != UInt8(ascii: "{"), b[i] != 0x0A { i += 1 }
        guard let first = group() else { return nil }
        var range = first
        for _ in 1 ..< max(groups, 1) {
            skipSpace()
            guard let next = group() else { break }
            range = range.lowerBound ..< next.upperBound
        }
        return range
    }

    /// The last control word of `before` (what TeX had read of a level): the
    /// name TeX reports as undefined when that level is the innermost.
    static func lastControlWord(_ before: String) -> String? {
        let t = before.trimmingCharacters(in: .whitespaces)
        guard let slash = t.lastIndex(of: "\\") else { return nil }
        let cs = String(t[slash...])
        guard cs.count > 1, cs.dropFirst().allSatisfy({ $0.isASCII && $0.isLetter }) else { return nil }
        return cs
    }

    /// The one occurrence of control word `cs` in `range` of `path`'s text.
    static func occurrence(of cs: String, in range: Range<Int>, path: String, texts: [String: String]) -> Site? {
        guard let text = texts[path], let slice = text.utf8Slice(range.lowerBound, range.upperBound) else { return nil }
        let found = matches(NSRegularExpression.escapedPattern(for: cs) + "(?![A-Za-z])", in: [path: slice])
        guard found.count == 1, let f = found.first else { return nil }
        let before = text.utf8Slice(0, range.lowerBound + f.start) ?? ""
        return Site(path: path, start: range.lowerBound + f.start, end: range.lowerBound + f.end,
                    line: before.reduce(1) { $1 == "\n" ? $0 + 1 : $0 })
    }

    /// Where `\usepackage{…}` goes: the start of the `\begin{document}` line
    /// of the text that has one (the row's own first).
    static func preambleEnd(prefer path: String?, texts: [String: String]) -> (path: String, at: Int, eol: String)? {
        let sites = matches(#"(?m)^[ \t]*\\begin\s*\{document\}"#, in: texts)
        guard let s = sites.first(where: { $0.path == path }) ?? (sites.count == 1 ? sites.first : nil), let text = texts[s.path] else { return nil }
        return (s.path, s.start, text.contains("\r\n") ? "\r\n" : "\n")
    }
}
