import Foundation

/// What mode the document is in where a capture will land, and the wrapping
/// that makes an insertion there legal LaTeX.
///
/// This is the Swift half of the rules in `crates/bridge/src/caret.rs`. Both
/// halves are checked against the same table,
/// `protocol/fixtures/caret-context-v1.json`, by `CaretContextTests` here and
/// `crates/bridge/tests/caret_context.rs` there, and that table is in turn
/// checked against pdflatex by `scripts/caret_context_oracle.py`. Two
/// implementations exist because two paths insert: the bridge prepares the edit
/// when one is attached, and this shell prepares it when one is not.
///
/// The bug this exists for: a recogniser told only *where* the caret is, and
/// not what kind of place it is, returns `$x^2$` for a formula. Inserted inside
/// an existing `$…$` that is `$a + $x^2$ + b$`, which pdflatex rejects. The
/// FlashTeX engine currently accepts it silently, so the engine cannot catch
/// this and pdflatex is the oracle.
enum CaretMode: String, Codable, Equatable {
    case text
    case inlineMath = "inline_math"
    case displayMath = "display_math"
    case verbatim
    case comment
}

/// What an insertion at a caret must do with recognised mathematics.
enum CaretWrap: String, Codable, Equatable {
    /// Text mode where a display block is legal.
    case display
    /// Text mode where only inline math is legal (mid-sentence, tabular cell).
    case inline
    /// Already inside math: emit bare mathematics, never a delimiter.
    case alreadyMath = "already_math"
    /// Verbatim or a comment: insert exactly, wrap nothing.
    case literal
}

struct CaretContext: Codable, Equatable {
    var mode: CaretMode = .text
    /// The delimiter that opened the current math: `$`, `$$`, `\(`, `\[`, or `environment`.
    var delimiter: String?
    /// Innermost open environment.
    var environment: String?
    /// Enclosing environments, outermost first, capped at 16.
    var environments: [String] = []
    /// Whether amsmath is loaded, which decides `\text{…}` versus `\mbox{…}`.
    var amsmath: Bool = false
    var wrap: CaretWrap = .display

    enum CodingKeys: String, CodingKey {
        case mode, delimiter, environment, environments, amsmath, wrap
    }

    /// Math environments. Kept in step with `SyntaxHighlighter.mathEnvironments`;
    /// `CaretContextTests.testAgreesWithTheSyntaxHighlighter` is what enforces it.
    static let mathEnvironments = SyntaxHighlighter.mathEnvironments
    static let verbatimEnvironments = SyntaxHighlighter.verbatimEnvironments
    /// Text-mode tabular cells: `\[…\]` in one is a pdflatex error.
    static let tabularEnvironments: Set<String> = [
        "tabular", "tabular*", "tabularx", "tabulary", "longtable", "supertabular", "xtabular",
    ]
    static let maxReportedEnvironments = 16

    /// The math text-box command available here. `\text` needs amsmath; without
    /// it `\text` is a pdflatex error and the kernel's `\mbox` is the answer.
    var textBoxCommand: String { amsmath ? "\\text" : "\\mbox" }

    /// One sentence for the recogniser's prompt, the Mac's destination row and
    /// the iPad's capture screen.
    func describe() -> String {
        let place: String
        switch (mode, environment) {
        case (.verbatim, let env?): place = "inside \\begin{\(env)}, which is not LaTeX"
        case (.verbatim, nil): place = "inside a verbatim block, which is not LaTeX"
        case (.comment, _): place = "inside a comment"
        case (.inlineMath, _): place = "already inside inline math opened by \(delimiter ?? "$")"
        case (.displayMath, let env?) where Self.mathEnvironments.contains(env):
            place = "already inside \\begin{\(env)} math"
        case (.displayMath, _): place = "already inside display math opened by \(delimiter ?? "\\[")"
        case (.text, let env?) where Self.tabularEnvironments.contains(env):
            place = "in a \\begin{\(env)} cell"
        case (.text, let env?): place = "in text mode inside \\begin{\(env)}"
        case (.text, nil): place = "in text mode"
        }
        let rule: String
        switch wrap {
        case .display: rule = "Wrap a formula that stands on its own in \\[ … \\] and a formula inside a sentence in $ … $."
        case .inline: rule = "Wrap every formula in $ … $. Display math (\\[ … \\], equation) is NOT legal here."
        case .alreadyMath: rule = "Emit bare mathematics with NO delimiters: do not write $, $$, \\(, \\[ or a math environment, because the destination is already in math mode and a second delimiter would close it."
        case .literal: rule = "Emit the transcription literally with no LaTeX markup at all: the destination is not typeset."
        }
        return "The insertion point is \(place). \(rule)"
    }

    /// A short label for the destination row on the Mac and the iPad.
    var label: String {
        switch wrap {
        case .display: return "text — formulas wrapped in \\[ … \\] or $ … $"
        case .inline: return environment.map { Self.tabularEnvironments.contains($0) ? "\($0) cell — inline math only" : "text — inline math only" } ?? "text — inline math only"
        case .alreadyMath: return environment.map { "\($0) — already math, no delimiters added" } ?? "\(delimiter ?? "$") math — already math, no delimiters added"
        case .literal: return mode == .verbatim ? "verbatim — inserted literally" : "comment — inserted literally"
        }
    }
}

// MARK: - derivation

extension CaretContext {
    /// A math opener still waiting to be closed.
    private enum Open: Equatable {
        case dollar, doubleDollar, paren, bracket
        case environment(String)

        var isDisplay: Bool {
            switch self { case .dollar, .paren: false; default: true }
        }
        var spelling: String {
            switch self {
            case .dollar: "$"
            case .doubleDollar: "$$"
            case .paren: "\\("
            case .bracket: "\\["
            case .environment: "environment"
            }
        }
    }

    /// Derive the caret context from the document prefix ending at `caret`, a
    /// UTF-8 byte offset.
    ///
    /// A lexical scan, not a TeX interpreter: a `$` produced by a macro is
    /// invisible to it, exactly as it is to the syntax highlighter and to the
    /// bridge's bounded lexical context.
    static func derive(_ text: String, caretByte caret: Int) -> CaretContext {
        let bytes = Array(text.utf8)
        let caret = max(0, min(caret, bytes.count))

        var math: [Open] = []
        var environments: [String] = []
        var verbatim: String?
        var commentEnvironment = 0
        var inLineComment = false
        var amsmath = false

        func ascii(_ range: Range<Int>) -> String {
            String(decoding: bytes[range], as: UTF8.self)
        }
        /// End offset of the control sequence starting at `i` (`bytes[i] == \`).
        func controlSequenceEnd(_ i: Int) -> Int {
            var end = i + 1
            if end < caret, isLetter(bytes[end]) {
                while end < caret, isLetter(bytes[end]) || bytes[end] == UInt8(ascii: "*") { end += 1 }
                return end
            }
            if end < caret {
                end += 1
                while end < caret, bytes[end] & 0xC0 == 0x80 { end += 1 }
            }
            return end
        }
        func isLetter(_ b: UInt8) -> Bool {
            (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A)
        }
        /// `{name}` just after `at`, and the offset past the closing brace.
        func bracedName(_ at: Int) -> (String, Int)? {
            var i = at
            while i < caret, bytes[i] == UInt8(ascii: " ") || bytes[i] == UInt8(ascii: "\t") { i += 1 }
            guard i < caret, bytes[i] == UInt8(ascii: "{") else { return nil }
            var j = i + 1
            while j < caret, bytes[j] != UInt8(ascii: "}") { j += 1 }
            guard j < caret else { return nil }
            return (ascii((i + 1)..<j).trimmingCharacters(in: .whitespaces), j + 1)
        }
        func find(_ needle: [UInt8], from: Int) -> Int? {
            guard !needle.isEmpty, from + needle.count <= caret else { return nil }
            for start in from...(caret - needle.count) where Array(bytes[start..<(start + needle.count)]) == needle {
                return start
            }
            return nil
        }
        func popMath(_ opener: Open) {
            if let at = math.lastIndex(of: opener) { math.removeSubrange(at...) }
        }
        func popEnvironment(_ name: String) {
            if let at = environments.lastIndex(of: name) { environments.removeSubrange(at...) }
        }

        var i = 0
        scan: while i < caret {
            // A verbatim body is inert: only its own \end matters.
            if let name = verbatim {
                let marker = Array("\\end{\(name)}".utf8)
                guard let at = find(marker, from: i) else { break scan }
                i = at + marker.count
                verbatim = nil
                popEnvironment(name)
                continue
            }
            switch bytes[i] {
            case UInt8(ascii: "%"):
                guard let at = find([UInt8(ascii: "\n")], from: i) else {
                    inLineComment = true
                    break scan
                }
                i = at + 1
            case UInt8(ascii: "\\"):
                let end = controlSequenceEnd(i)
                let name = ascii((i + 1)..<end)
                switch name {
                case "[": math.append(.bracket); i = end
                case "]": popMath(.bracket); i = end
                case "(": math.append(.paren); i = end
                case ")": popMath(.paren); i = end
                case "verb":
                    var j = end
                    if j < caret, bytes[j] == UInt8(ascii: "*") { j += 1 }
                    guard j < caret else { i = j; break }
                    let delimiter = bytes[j]
                    j += 1
                    while j < caret, bytes[j] != delimiter, bytes[j] != UInt8(ascii: "\n") { j += 1 }
                    i = min(j + 1, caret)
                case "begin", "end":
                    guard let (env, after) = bracedName(end) else { i = end; break }
                    if name == "begin" {
                        environments.append(env)
                        if Self.verbatimEnvironments.contains(env) {
                            verbatim = env
                        } else if env == "comment" {
                            commentEnvironment += 1
                        } else if Self.mathEnvironments.contains(env) {
                            math.append(.environment(env))
                        }
                    } else {
                        popEnvironment(env)
                        if env == "comment" {
                            commentEnvironment = max(0, commentEnvironment - 1)
                        } else if Self.mathEnvironments.contains(env) {
                            popMath(.environment(env))
                        }
                    }
                    i = after
                case "usepackage", "RequirePackage":
                    guard let (list, after) = bracedName(end) else { i = end; break }
                    amsmath = amsmath || list.split(separator: ",").contains { $0.trimmingCharacters(in: .whitespaces) == "amsmath" }
                    i = after
                // Every other control sequence, `\$` and `\%` included, is
                // consumed whole: an escaped dollar never opens math.
                default: i = end
                }
            case UInt8(ascii: "$"):
                let double = i + 1 < caret && bytes[i + 1] == UInt8(ascii: "$")
                let opener: Open = double ? .doubleDollar : .dollar
                // `$` does not nest: it closes its opener, or opens one.
                if math.last == opener { math.removeLast() } else { math.append(opener) }
                i += double ? 2 : 1
            default:
                i += 1
            }
        }

        let mode: CaretMode
        if verbatim != nil {
            mode = .verbatim
        } else if commentEnvironment > 0 || inLineComment {
            mode = .comment
        } else {
            switch math.last {
            case .dollar, .paren: mode = .inlineMath
            case .some: mode = .displayMath
            case nil: mode = .text
            }
        }
        let delimiter = (mode == .inlineMath || mode == .displayMath) ? math.last?.spelling : nil
        let wrap: CaretWrap
        switch mode {
        case .verbatim, .comment: wrap = .literal
        case .inlineMath, .displayMath: wrap = .alreadyMath
        case .text:
            let inCell = environments.contains { Self.tabularEnvironments.contains($0) }
            wrap = (inCell || !atParagraphPosition(bytes, caret)) ? .inline : .display
        }
        let skip = max(0, environments.count - Self.maxReportedEnvironments)
        return CaretContext(mode: mode, delimiter: delimiter, environment: environments.last,
                            environments: Array(environments[skip...]), amsmath: amsmath, wrap: wrap)
    }

    /// Whether a display block would stand on its own here: only whitespace
    /// between the caret and a blank line (or the document edge) on each side.
    /// Mid-sentence a display block would break the sentence in two.
    private static func atParagraphPosition(_ bytes: [UInt8], _ caret: Int) -> Bool {
        func blankRun(_ range: StrideThrough<Int>) -> Bool {
            var newlines = 0
            for i in range {
                let b = bytes[i]
                if b == UInt8(ascii: "\n") {
                    newlines += 1
                    if newlines == 2 { return true }
                } else if b == UInt8(ascii: " ") || b == UInt8(ascii: "\t") || b == UInt8(ascii: "\r") {
                    continue
                } else {
                    return false
                }
            }
            return true // reached the document edge through whitespace only
        }
        let behind = caret > 0 ? blankRun(stride(from: caret - 1, through: 0, by: -1)) : true
        let ahead = caret < bytes.count ? blankRun(stride(from: caret, through: bytes.count - 1, by: 1)) : true
        return behind && ahead
    }
}

// MARK: - insertion normalisation

/// The result of making a proposal safe to insert at a caret.
struct NormalizedInsertion: Equatable {
    /// The exact text to insert, or nil when no safe insertion exists.
    var text: String?
    /// Reviewer-facing notes in the `ambiguities` vocabulary the review sheet
    /// already renders; an `UNSUPPORTED: ` entry blocks direct insertion.
    var advisories: [String] = []
}

extension CaretContext {
    /// Structural math commands: the only signal that turns undelimited output
    /// into mathematics worth wrapping. See `shape`.
    private static let mathMarkers: Set<String> = [
        "frac", "sqrt", "left", "right", "sum", "int", "prod", "lim", "cdot", "times", "div", "pm",
        "leq", "geq", "neq", "approx", "equiv", "infty", "partial", "nabla", "forall", "exists", "in",
        "subset", "cup", "cap", "to", "rightarrow", "alpha", "beta", "gamma", "delta", "theta",
        "lambda", "mu", "sigma", "phi", "omega", "pi", "binom", "overline", "underline", "hat", "vec",
    ]
    /// Text-structure commands: the transcription is a paragraph, not a formula.
    private static let textMarkers: Set<String> = [
        "section", "subsection", "subsubsection", "paragraph", "item", "textbf", "emph", "textit",
        "caption", "footnote", "par", "begin",
    ]

    private enum Shape { case bareMath, prose, delimited }

    /// Make `latex` legal at this caret.
    ///
    /// The guarantee: the text it returns never adds a math delimiter inside
    /// math and never leaves one unbalanced. That is what makes the owner's
    /// report a fixed bug rather than a better prompt — a recogniser that
    /// ignores its instructions still cannot produce `$a + $x^2$ + b$`.
    func normalize(_ latex: String) -> NormalizedInsertion {
        let body = latex.trimmingCharacters(in: .whitespacesAndNewlines)
        var advisories: [String] = []

        if wrap == .literal {
            let place = mode == .verbatim ? "verbatim block" : "comment"
            let suffix = mode == .verbatim ? " as LaTeX" : ""
            advisories.append("AMBIGUOUS: the destination is a \(place); the transcription was inserted literally and is not typeset\(suffix)")
            return NormalizedInsertion(text: body, advisories: advisories)
        }

        let scan = Self.scanMath(body)
        guard scan.balanced else {
            return NormalizedInsertion(text: nil, advisories: ["UNSUPPORTED: the proposal's math delimiters are unbalanced and it cannot be inserted safely"])
        }

        switch wrap {
        case .literal:
            preconditionFailure("handled above")
        case .alreadyMath:
            // Peel every layer the model wrapped around the formula. One `$…$`
            // inside an existing `$…$` is the reported bug; peeling makes it
            // impossible rather than unlikely.
            var inner = body
            while let whole = Self.scanMath(inner).whole {
                inner = String(inner[whole.inner]).trimmingCharacters(in: .whitespacesAndNewlines)
            }
            guard Self.scanMath(inner).spans.isEmpty else {
                return NormalizedInsertion(text: nil, advisories: ["UNSUPPORTED: text-mode content with its own math delimiters cannot be inserted at a math caret"])
            }
            // Already wrapped by an earlier pass: do not nest \mbox in \mbox.
            if Self.shape(inner) == .prose, !inner.isEmpty, !Self.isTextBoxGroup(inner) {
                advisories.append("AMBIGUOUS: prose recognised at a math caret was wrapped in \(textBoxCommand){...}")
                return gated("\(textBoxCommand){\(inner)}", advisories)
            }
            return gated(inner, advisories)
        case .inline, .display:
            if scan.spans.isEmpty {
                // Undelimited. Wrap it if it is mathematics; leave prose alone.
                if Self.shape(body) == .bareMath, !body.isEmpty {
                    return gated(wrap == .display ? "\\[ \(body) \\]" : "$\(body)$", advisories)
                }
                return gated(body, advisories)
            }
            // Already delimited. The one thing still illegal is display math
            // where only inline math fits: a tabular cell.
            if wrap == .inline, let rewritten = Self.displayToInline(body, scan) {
                advisories.append("AMBIGUOUS: display math is not allowed in a tabular cell; inserted as inline math")
                return gated(rewritten, advisories)
            }
            return gated(body, advisories)
        }
    }

    /// The already-normalised `latex` if it is still legal here, or nil.
    /// A pure predicate, so running it after `normalize` changes nothing; it is
    /// the check repeated at the moment the edit is issued.
    func insertable(_ latex: String) -> String? {
        wrap == .literal ? latex : gated(latex, []).text
    }

    /// The last line of defence: refuse anything that is not actually legal
    /// here, whatever route produced it. A bug in the wrapping logic then
    /// degrades to a refusal a reviewer sees, not to LaTeX that will not compile.
    private func gated(_ text: String, _ advisories: [String]) -> NormalizedInsertion {
        let scan = Self.scanMath(text)
        let reason: String?
        if !scan.balanced {
            reason = "its math delimiters are unbalanced"
        } else if wrap == .alreadyMath, !scan.spans.isEmpty {
            reason = "it carries math delimiters and the destination is already in math mode"
        } else if scan.maxDepth > 1 {
            reason = "it nests math inside math"
        } else if wrap == .inline, scan.spans.contains(where: \.display) {
            reason = "it uses display math where only inline math is legal"
        } else {
            reason = nil
        }
        guard let reason else { return NormalizedInsertion(text: text, advisories: advisories) }
        return NormalizedInsertion(
            text: nil,
            advisories: advisories.filter { $0.hasPrefix("UNSUPPORTED: ") }
                + ["UNSUPPORTED: the proposal cannot be inserted at this caret because \(reason)"])
    }

    /// Rewrite every top-level display group as inline math, or nil when there
    /// is no display group to rewrite.
    private static func displayToInline(_ body: String, _ scan: MathScan) -> String? {
        guard scan.spans.contains(where: \.display) else { return nil }
        var out = ""
        var at = body.startIndex
        for span in scan.spans {
            out += body[at..<span.outer.lowerBound]
            if span.display {
                out += "$" + body[span.inner].trimmingCharacters(in: .whitespacesAndNewlines) + "$"
            } else {
                out += body[span.outer]
            }
            at = span.outer.upperBound
        }
        out += body[at...]
        return out
    }

    /// Whether `content` is exactly one `\text{…}` or `\mbox{…}` group.
    private static func isTextBoxGroup(_ content: String) -> Bool {
        for command in ["\\text{", "\\mbox{"] where content.hasPrefix(command) {
            let rest = content.dropFirst(command.count)
            var depth = 1
            var i = rest.startIndex
            while i < rest.endIndex {
                switch rest[i] {
                case "\\": i = rest.index(after: i)
                case "{": depth += 1
                case "}":
                    depth -= 1
                    if depth == 0 { return rest.index(after: i) == rest.endIndex }
                default: break
                }
                if i < rest.endIndex { i = rest.index(after: i) }
            }
        }
        return false
    }

    /// Is this transcription mathematics, prose, or already delimited?
    ///
    /// Deliberately conservative: without a structural signal there is no
    /// reliable way to tell undelimited mathematics (`x = y`) from ordinary
    /// prose (`<F>`, `see figure 2`), and guessing wrong corrupts the document
    /// silently. Content with no math marker is left exactly as it came, and
    /// the prompt carries the instruction to delimit it.
    private static func shape(_ content: String) -> Shape {
        if !scanMath(content).spans.isEmpty { return .delimited }
        var hasMathMarker = false
        var hasTextMarker = false
        let units = Array(content.utf8)
        var i = 0
        while i < units.count {
            if units[i] == UInt8(ascii: "\\") {
                var end = i + 1
                if end < units.count, isAsciiLetter(units[end]) {
                    while end < units.count, isAsciiLetter(units[end]) || units[end] == UInt8(ascii: "*") { end += 1 }
                } else if end < units.count {
                    end += 1
                    while end < units.count, units[end] & 0xC0 == 0x80 { end += 1 }
                }
                let name = String(decoding: units[(i + 1)..<end], as: UTF8.self)
                hasMathMarker = hasMathMarker || mathMarkers.contains(name)
                hasTextMarker = hasTextMarker || textMarkers.contains(name)
                i = end
                continue
            }
            if units[i] == UInt8(ascii: "^") || units[i] == UInt8(ascii: "_") { hasMathMarker = true }
            i += 1
        }
        if hasTextMarker || content.contains("\n\n") { return .prose }
        return hasMathMarker ? .bareMath : .prose
    }

    private static func isAsciiLetter(_ b: UInt8) -> Bool {
        (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A)
    }

    /// One balanced math group inside a transcription.
    struct MathSpan {
        var outer: Range<String.Index>
        var inner: Range<String.Index>
        var display: Bool
    }

    struct MathScan {
        var spans: [MathSpan] = []
        /// Deepest math nesting reached; anything above 1 is a pdflatex error.
        var maxDepth = 0
        var balanced = true
        /// The single group covering the whole string, when there is one.
        var whole: MathSpan?
    }

    /// Find the top-level math groups in a transcription, honouring `\$`,
    /// comments and `\verb` exactly as `derive` does.
    static func scanMath(_ content: String) -> MathScan {
        var stack: [(Open, String.Index, String.Index)] = []
        var spans: [MathSpan] = []
        var maxDepth = 0
        var result = MathScan()

        func close(_ opener: Open, outerEnd: String.Index, innerEnd: String.Index) {
            guard let at = stack.lastIndex(where: { $0.0 == opener }) else { return }
            let (open, outerStart, innerStart) = stack[at]
            stack.removeSubrange(at...)
            if stack.isEmpty {
                spans.append(MathSpan(outer: outerStart..<outerEnd, inner: innerStart..<innerEnd, display: open.isDisplay))
            }
        }

        var i = content.startIndex
        while i < content.endIndex {
            let c = content[i]
            if c == "%" {
                guard let nl = content[i...].firstIndex(of: "\n") else { break }
                i = content.index(after: nl)
                continue
            }
            if c == "\\" {
                var end = content.index(after: i)
                if end < content.endIndex, content[end].isLetter {
                    while end < content.endIndex, content[end].isLetter || content[end] == "*" { end = content.index(after: end) }
                } else if end < content.endIndex {
                    end = content.index(after: end)
                }
                let name = String(content[content.index(after: i)..<end])
                switch name {
                case "[": stack.append((.bracket, i, end)); maxDepth = max(maxDepth, stack.count); i = end
                case "]": close(.bracket, outerEnd: end, innerEnd: i); i = end
                case "(": stack.append((.paren, i, end)); maxDepth = max(maxDepth, stack.count); i = end
                case ")": close(.paren, outerEnd: end, innerEnd: i); i = end
                case "verb":
                    var j = end
                    if j < content.endIndex, content[j] == "*" { j = content.index(after: j) }
                    guard j < content.endIndex else { i = j; break }
                    let delimiter = content[j]
                    j = content.index(after: j)
                    while j < content.endIndex, content[j] != delimiter, content[j] != "\n" { j = content.index(after: j) }
                    i = j < content.endIndex ? content.index(after: j) : j
                case "begin", "end":
                    var j = end
                    while j < content.endIndex, content[j] == " " { j = content.index(after: j) }
                    guard j < content.endIndex, content[j] == "{", let brace = content[j...].firstIndex(of: "}") else { i = end; break }
                    let env = String(content[content.index(after: j)..<brace]).trimmingCharacters(in: .whitespaces)
                    let after = content.index(after: brace)
                    if mathEnvironments.contains(env) {
                        if name == "begin" {
                            stack.append((.environment(env), i, after))
                            maxDepth = max(maxDepth, stack.count)
                        } else {
                            close(.environment(env), outerEnd: after, innerEnd: i)
                        }
                    }
                    i = after
                default: i = end
                }
                continue
            }
            if c == "$" {
                let next = content.index(after: i)
                let double = next < content.endIndex && content[next] == "$"
                let width = double ? content.index(after: next) : next
                let opener: Open = double ? .doubleDollar : .dollar
                if stack.last?.0 == opener {
                    close(opener, outerEnd: width, innerEnd: i)
                } else {
                    stack.append((opener, i, width))
                    maxDepth = max(maxDepth, stack.count)
                }
                i = width
                continue
            }
            i = content.index(after: i)
        }
        result.spans = spans
        result.maxDepth = maxDepth
        result.balanced = stack.isEmpty
        if spans.count == 1, spans[0].outer.lowerBound == content.startIndex, spans[0].outer.upperBound == content.endIndex {
            result.whole = spans[0]
        }
        return result
    }
}
