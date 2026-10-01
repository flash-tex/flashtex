import Foundation

extension TeXpand {
    /// Snippet IR (PLAN §5): the expander's output, converted by a host
    /// adapter into the editor's own snippet form.
    public indirect enum SnippetNode: Equatable, Sendable {
        case text(String)
        /// A field Tab visits; `placeholder` is its initial content.
        case tabstop(index: Int, placeholder: [SnippetNode])
        /// A copy of tabstop `index`, optionally transformed.
        case mirror(index: Int, transform: MirrorTransform)
        case choice(index: Int, options: [String])
        /// Where the caret rests after the last tabstop (`$0`).
        case final
        /// A line break; the host applies the base indent plus the depth.
        case newline
        /// One indent unit deeper for the newlines inside.
        case indent([SnippetNode])
    }

    public enum MirrorTransform: String, Equatable, Sendable {
        case none, slug, upper, lower

        public func apply(_ s: String) -> String {
            switch self {
            case .none: return s
            case .slug: return TeXpand.slug(s)
            case .upper: return s.uppercased()
            case .lower: return s.lowercased()
            }
        }
    }

    public struct Snippet: Equatable, Sendable {
        public var nodes: [SnippetNode]
        public init(_ nodes: [SnippetNode] = []) { self.nodes = nodes }

        /// A readable TextMate-like rendering for golden tests and previews:
        /// `$1`, `${1:placeholder}`, `${1/slug}`, `${1|a,b|}`, `$0`. Only `$`
        /// in text is escaped (as `\$`), so LaTeX stays legible.
        public func rendered(baseIndent: String = "", indentUnit: String = "  ") -> String {
            var out = ""
            Self.render(nodes, depth: 0, base: baseIndent, unit: indentUnit, into: &out, inPlaceholder: false)
            return out
        }

        static func render(_ nodes: [SnippetNode], depth: Int, base: String, unit: String, into out: inout String, inPlaceholder: Bool) {
            for n in nodes {
                switch n {
                case .text(let t):
                    var e = t.replacingOccurrences(of: "$", with: "\\$")
                    if inPlaceholder { e = e.replacingOccurrences(of: "}", with: "\\}") }
                    out += e
                case .tabstop(let i, let ph):
                    if ph.isEmpty { out += "$\(i)"; continue }
                    out += "${\(i):"
                    render(ph, depth: depth, base: base, unit: unit, into: &out, inPlaceholder: true)
                    out += "}"
                case .mirror(let i, let t):
                    out += t == .none ? "$\(i)" : "${\(i)/\(t.rawValue)}"
                case .choice(let i, let options):
                    out += "${\(i)|" + options.joined(separator: ",") + "|}"
                case .final:
                    out += "$0"
                case .newline:
                    out += "\n" + base + String(repeating: unit, count: depth)
                case .indent(let inner):
                    render(inner, depth: depth + 1, base: base, unit: unit, into: &out, inPlaceholder: inPlaceholder)
                }
            }
        }

        /// Tabstop indices in document order of first appearance.
        public var tabstopIndices: [Int] {
            var out: [Int] = []
            func walk(_ ns: [SnippetNode]) {
                for n in ns {
                    switch n {
                    case .tabstop(let i, let ph): out.append(i); walk(ph)
                    case .choice(let i, _): out.append(i)
                    case .indent(let inner): walk(inner)
                    default: break
                    }
                }
            }
            walk(nodes)
            return out
        }

        /// The snippet as plain text with fields resolved: what the buffer
        /// holds right after insertion, plus each field's UTF-16 range.
        public func flattened(baseIndent: String = "", indentUnit: String = "  ") -> Flat {
            var flat = Flat()
            var values: [Int: String] = [:]
            func walk(_ ns: [SnippetNode], depth: Int) {
                for n in ns {
                    switch n {
                    case .text(let t): flat.text += t
                    case .tabstop(let i, let ph):
                        let start = (flat.text as NSString).length
                        walk(ph, depth: depth)
                        let range = NSRange(location: start, length: (flat.text as NSString).length - start)
                        values[i] = (flat.text as NSString).substring(with: range)
                        flat.fields.append(Flat.Field(index: i, range: range))
                    case .choice(let i, let options):
                        let start = (flat.text as NSString).length
                        flat.text += options.first ?? ""
                        let range = NSRange(location: start, length: (flat.text as NSString).length - start)
                        values[i] = options.first ?? ""
                        flat.fields.append(Flat.Field(index: i, range: range))
                    case .mirror(let i, let t):
                        // Hosts without linked fields get the placeholder's
                        // (transformed) text, which stays correct until edited.
                        flat.text += t.apply(values[i] ?? "")
                    case .final:
                        flat.final = (flat.text as NSString).length
                    case .newline:
                        flat.text += "\n" + baseIndent + String(repeating: indentUnit, count: depth)
                    case .indent(let inner):
                        walk(inner, depth: depth + 1)
                    }
                }
            }
            walk(nodes, depth: 0)
            flat.fields.sort { $0.index == $1.index ? $0.range.location < $1.range.location : $0.index < $1.index }
            return flat
        }

        /// Plain text with resolved fields.
        public struct Flat: Equatable, Sendable {
            public struct Field: Equatable, Sendable {
                public var index: Int
                public var range: NSRange
            }
            public var text = ""
            /// Fields in Tab order (index, then position).
            public var fields: [Field] = []
            /// The `$0` position, if the snippet has one.
            public var final: Int?

            /// The editors' snippet form (`LaTeXSnippet`): the first field is
            /// selected at insertion, Tab selects each later one (so typing
            /// replaces its placeholder), then the caret goes to `$0` or the
            /// end. Mirrors are already plain text here (PLAN §5's degraded
            /// form: they copy their placeholder and do not follow edits).
            public var latexSnippet: LaTeXSnippet {
                let end = final ?? (text as NSString).length
                var seen = Set<Int>()
                let firsts = fields.filter { seen.insert($0.index).inserted }.map(\.range)
                guard let first = firsts.first else { return LaTeXSnippet(text: text, caretUTF16: end) }
                let later = firsts.dropFirst()
                return LaTeXSnippet(text: text, caretUTF16: first.location, stops: later.map(\.location) + [end],
                                    caretLength: first.length, stopLengths: later.map(\.length) + [0])
            }
        }
    }

    /// Label slug: lowercase ASCII letters and digits, runs of anything
    /// else (and control sequences' backslashes) become one `-`.
    public static func slug(_ s: String) -> String {
        var out = ""
        var dash = false
        for c in s.lowercased() {
            if c.isASCII, c.isLetter || c.isNumber {
                if dash, !out.isEmpty { out += "-" }
                out.append(c)
                dash = false
            } else {
                dash = true
            }
        }
        return out
    }
}
