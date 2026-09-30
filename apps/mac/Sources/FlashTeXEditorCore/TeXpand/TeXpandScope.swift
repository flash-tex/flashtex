import Foundation

extension TeXpand {
    /// The scope at a position (PLAN §8): a stack of frames, outermost
    /// first, from which definitions' scope flags derive. M2 builds stacks
    /// by hand (tests, headless expansion); M3's ScopeProvider fills them
    /// from the buffer with each frame's source range, which the structure
    /// editor (M10b) and the structural actions (M10) use to find the
    /// enclosing environment's body.
    public struct ScopeStack: Equatable, Sendable {
        public enum Kind: Equatable, Sendable {
            case preamble
            case document
            case environment(String)
            case inlineMath
            case displayMath
            case verbatim
            case comment
        }

        public struct Frame: Equatable, Sendable {
            public var kind: Kind
            /// UTF-16 offset of the opener (`\begin{…}`, `$`, `\[`), when known.
            public var start: Int?
            /// UTF-16 offset where the body starts (after `\begin{…}` and
            /// its arguments), when known.
            public var bodyStart: Int?
            /// UTF-16 offset of the closer (`\end{…}`), when the scanner has
            /// seen it.
            public var bodyEnd: Int?

            public init(_ kind: Kind, start: Int? = nil, bodyStart: Int? = nil, bodyEnd: Int? = nil) {
                self.kind = kind; self.start = start; self.bodyStart = bodyStart; self.bodyEnd = bodyEnd
            }

            public var environmentName: String? { if case .environment(let n) = kind { return n }; return nil }
        }

        public var frames: [Frame]
        public init(_ frames: [Frame]) { self.frames = frames }

        /// Body text (`document`), optionally inside environments.
        public static func text(_ environments: String...) -> ScopeStack {
            ScopeStack([Frame(.document)] + environments.map { Frame(.environment($0)) })
        }
        /// Inline math in body text.
        public static let math = ScopeStack([Frame(.document), Frame(.inlineMath)])
        /// Display math in body text.
        public static let displayMath = ScopeStack([Frame(.document), Frame(.displayMath)])
        public static let preamble = ScopeStack([Frame(.preamble)])

        /// The derived flags definitions match against: `preamble`, `text`,
        /// `math`, `math:inline`, `math:display`, `verbatim`, `comment`,
        /// `list`, `tikz`, `beamer`, `alg`, `table`, `float`, and `env:NAME`
        /// for every enclosing environment.
        public var flags: Set<String> {
            var f = Set<String>()
            var mode = "text"
            for frame in frames {
                switch frame.kind {
                case .preamble: mode = "preamble"
                case .document: mode = "text"
                case .inlineMath: mode = "math"; f.insert("math:inline")
                case .displayMath: mode = "math"; f.insert("math:display")
                case .verbatim: mode = "verbatim"
                case .comment: mode = "comment"
                case .environment(let raw):
                    let name = raw.hasSuffix("*") ? String(raw.dropLast()) : raw
                    f.insert("env:" + name)
                    if let cls = ScopeStack.environmentClasses[name] {
                        f.formUnion(cls)
                        if cls.contains("math") { mode = "math"; f.insert("math:display") }
                        if cls.contains("verbatim") { mode = "verbatim" }
                        if cls.contains("text") { mode = "text" }
                    } else if SyntaxHighlighter.mathEnvironments.contains(raw) || SyntaxHighlighter.mathEnvironments.contains(name) {
                        mode = "math"; f.insert("math:display") // the highlighter's list: split, aligned, gathered, …
                    } else if SyntaxHighlighter.verbatimEnvironments.contains(raw) {
                        mode = "verbatim"
                    }
                }
            }
            f.remove("math"); f.remove("text"); f.remove("verbatim")
            f.insert(mode)
            return f
        }

        /// The innermost environment whose name satisfies `match` (M10b: the
        /// matrix or tabular the caret is in).
        public func innermost(where match: (String) -> Bool) -> Frame? {
            frames.last { $0.environmentName.map(match) ?? false }
        }

        /// What an environment adds to the flags. Math environments switch the
        /// mode to math, `\text`-like bodies back to text.
        public static let environmentClasses: [String: Set<String>] = {
            var t: [String: Set<String>] = [:]
            for e in ["itemize", "enumerate", "description"] { t[e] = ["list"] }
            for e in ["tikzpicture", "scope"] { t[e] = ["tikz"] }
            t["tikzcd"] = ["tikz", "math"]
            t["frame"] = ["beamer"]
            for e in ["algorithmic"] { t[e] = ["alg"] }
            for e in ["tabular", "tabularx", "tabulary", "longtable", "tblr"] { t[e] = ["table"] }
            t["array"] = ["table", "math"]
            for e in ["figure", "table", "subfigure", "wrapfigure"] { t[e] = ["float"] }
            for e in ["equation", "align", "alignat", "flalign", "gather", "multline", "eqnarray", "displaymath", "math", "dmath"] {
                t[e] = ["math"]
            }
            for e in structureEnvironments { t[e, default: []].formUnion(["math", "structure"]) }
            t["cases"] = ["math", "structure"]
            for e in ["verbatim", "lstlisting", "minted", "Verbatim", "comment"] { t[e] = ["verbatim"] }
            for e in ["tabular", "tabularx", "array", "align", "alignat", "gather", "flalign"] { t[e, default: []].insert("structure") }
            return t
        }()

        /// Grid-shaped environments the structure editor (M10b) can open:
        /// cells split on `&`, rows on `\\`.
        public static let structureEnvironments: Set<String> = [
            "matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix", "smallmatrix",
            "cases", "dcases", "rcases",
        ]
    }

    /// A grid body — rows split on `\\`, cells on `&`, both at brace depth
    /// 0 — as matrices, tabulars, cases and aligns share it. The table
    /// generator builds with it; M10's row/column actions and M10b's grid
    /// editor round-trip through it. Row-level material that is not a cell
    /// (`\hline`, `\midrule`) is kept as a rule line.
    public struct Grid: Equatable, Sendable {
        public enum Row: Equatable, Sendable {
            case cells([String])
            /// `\hline`, `\toprule`, `\midrule`, `\bottomrule`, `\cline{…}`.
            case rule(String)
        }
        public var rows: [Row]

        public init(rows: [Row]) { self.rows = rows }

        public var columnCount: Int {
            rows.reduce(0) { m, r in if case .cells(let c) = r { return max(m, c.count) }; return m }
        }

        static let ruleCommands = ["\\hline", "\\toprule", "\\midrule", "\\bottomrule", "\\cline", "\\cmidrule"]

        /// Parses an environment body (between `\begin{…}{spec}` and `\end{…}`).
        public static func parse(_ body: String) -> Grid {
            var rows: [Row] = []
            for rawRow in split(body, on: "\\\\") {
                var rest = rawRow.trimmingCharacters(in: .whitespacesAndNewlines)
                // Optional `[len]` after `\\` belongs to the previous row break.
                if rest.hasPrefix("["), let close = rest.firstIndex(of: "]") { rest = String(rest[rest.index(after: close)...]).trimmingCharacters(in: .whitespacesAndNewlines) }
                while let rule = ruleCommands.first(where: { rest.hasPrefix($0) }) {
                    var end = rest.index(rest.startIndex, offsetBy: rule.count)
                    while end < rest.endIndex, rest[end] == "{" || rest[end] == "(" {
                        let close: Character = rest[end] == "{" ? "}" : ")"
                        guard let c = rest[end...].firstIndex(of: close) else { break }
                        end = rest.index(after: c)
                    }
                    rows.append(.rule(String(rest[..<end])))
                    rest = String(rest[end...]).trimmingCharacters(in: .whitespacesAndNewlines)
                }
                if rest.isEmpty { continue }
                rows.append(.cells(split(rest, on: "&").map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }))
            }
            return Grid(rows: rows)
        }

        /// Source lines: cells joined with ` & `, rule rows on their own
        /// lines. A cell row ends with ` \\` unless it is the very last row:
        /// a rule after it (`\bottomrule`) needs the row ended first.
        public func lines() -> [String] {
            rows.enumerated().map { k, row in
                switch row {
                case .rule(let r): return r
                case .cells(let c): return c.joined(separator: " & ") + (k == rows.count - 1 ? "" : " \\\\")
                }
            }
        }

        /// Splits on `sep` outside braces and brackets, honouring `\` escapes
        /// (so `\&` and `\\` inside a cell's `\verb`-free text stay put).
        public static func split(_ text: String, on sep: String) -> [String] {
            var out: [String] = []
            var cur = ""
            var depth = 0
            var i = text.startIndex
            while i < text.endIndex {
                let rest = text[i...]
                if depth == 0, rest.hasPrefix(sep) {
                    out.append(cur); cur = ""
                    i = text.index(i, offsetBy: sep.count)
                    continue
                }
                let c = text[i]
                if c == "\\", text.index(after: i) < text.endIndex {
                    cur.append(c); cur.append(text[text.index(after: i)])
                    i = text.index(i, offsetBy: 2)
                    continue
                }
                if c == "{" { depth += 1 } else if c == "}" { depth = max(0, depth - 1) }
                cur.append(c)
                i = text.index(after: i)
            }
            out.append(cur)
            return out
        }
    }
}
