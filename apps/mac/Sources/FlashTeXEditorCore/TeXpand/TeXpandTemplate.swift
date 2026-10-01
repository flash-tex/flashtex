import Foundation

extension TeXpand {
    /// A definition body with `<< >>` holes (PLAN §6), split into lines.
    /// Leading whitespace of each line after the first is structural: a tab
    /// or two spaces is one indent level (the host's indent unit replaces
    /// it). A line holding only whitespace and holes that all render empty
    /// is dropped with its line break (`<<label>>` with no label).
    public struct Template: Equatable, Sendable {
        public enum Hole: Equatable, Sendable {
            /// `<<1>>`, `<<1:default>>`
            case tabstop(Int, defaultText: String?)
            /// `<<1|a,b,c>>`
            case choice(Int, options: [String])
            /// `<<=1>>`, `<<=1|slug>>`
            case mirror(Int, MirrorTransform)
            /// `<<0>>`
            case final
            /// `<<children>>`: expanded children, one per line.
            case children
            /// `<<selection>>`: the wrapped text in wrap mode, else a tabstop.
            case selection
            /// `<<body>>`: in a modifier's template, the element it wraps.
            case body
            /// `<<arg.1>>`, `<<arg.title>>`, optionally `:default`.
            case arg(ArgRef, defaultText: String?)
            /// `<<opt>>`: `[value]` if given, else nothing.
            case opt
            /// `<<opt.value>>`: the bare value if given, else a tabstop.
            case optValue(defaultText: String?)
            /// `<<overlay>>`: `<spec>` if given, else nothing.
            case overlay
            /// `<<label>>`: `\label{prefix:name}` if given, else nothing.
            case label
            /// `<<star>>`: `*` if `!` was used.
            case star
            /// `<<p.NAME.FIELD>>`
            case param(name: String, field: String)
            /// `<<profile.KEY>>`
            case profile(String)
            /// `<<i>>` (1-based) or `<<i0>>` (0-based) repeat counter.
            case counter(zeroBased: Bool)
        }

        public enum ArgRef: Equatable, Sendable {
            case index(Int)
            case name(String)
        }

        public enum Part: Equatable, Sendable {
            case text(String)
            case hole(Hole)
        }

        public struct Line: Equatable, Sendable {
            public var level: Int
            public var parts: [Part]
        }

        public var lines: [Line]
        public let source: String

        public struct TemplateError: Error, Equatable, Sendable {
            public var message: String
        }

        public init(_ source: String) throws {
            self.source = source
            var parts: [Part] = []
            var text = ""
            var i = source.startIndex
            func flush() { if !text.isEmpty { parts.append(.text(text)); text = "" } }
            while i < source.endIndex {
                let rest = source[i...]
                if rest.hasPrefix("\\<<") {
                    text += "<<"
                    i = source.index(i, offsetBy: 3)
                } else if rest.hasPrefix("<<") {
                    let open = source.index(i, offsetBy: 2)
                    guard let close = source[open...].range(of: ">>") else {
                        throw TemplateError(message: "`<<` without a closing `>>` (write a literal `<<` as `\\<<`)")
                    }
                    let content = String(source[open..<close.lowerBound])
                    guard !content.contains("\n") else { throw TemplateError(message: "a hole cannot span lines: `<<\(content)`") }
                    flush()
                    parts.append(.hole(try Self.hole(content)))
                    i = close.upperBound
                } else {
                    text.append(source[i])
                    i = source.index(after: i)
                }
            }
            flush()

            // Split into lines.
            var lines: [Line] = [Line(level: 0, parts: [])]
            for part in parts {
                guard case .text(let t) = part, t.contains("\n") else { lines[lines.count - 1].parts.append(part); continue }
                let pieces = t.split(separator: "\n", omittingEmptySubsequences: false)
                for (k, piece) in pieces.enumerated() {
                    if k > 0 { lines.append(Line(level: 0, parts: [])) }
                    if !piece.isEmpty { lines[lines.count - 1].parts.append(.text(String(piece))) }
                }
            }
            // Structural indentation of the lines after the first.
            for k in lines.indices.dropFirst() {
                guard case .text(let t)? = lines[k].parts.first else { continue }
                var level = 0, spaces = 0
                var idx = t.startIndex
                while idx < t.endIndex, t[idx] == " " || t[idx] == "\t" {
                    if t[idx] == "\t" { level += 1; spaces = 0 } else { spaces += 1; if spaces == 2 { level += 1; spaces = 0 } }
                    idx = t.index(after: idx)
                }
                let remainder = String(repeating: " ", count: spaces) + t[idx...]
                lines[k].level = level
                if remainder.isEmpty { lines[k].parts.removeFirst() } else { lines[k].parts[0] = .text(remainder) }
            }
            // A trailing empty line (a body ending in a newline) is not content.
            while lines.count > 1, lines.last!.parts.isEmpty { lines.removeLast() }
            self.lines = lines
        }

        static func hole(_ raw: String) throws -> Hole {
            let c = raw.trimmingCharacters(in: .whitespaces)
            func split(_ s: Substring, at sep: Character) -> (String, String?) {
                guard let k = s.firstIndex(of: sep) else { return (String(s), nil) }
                return (String(s[..<k]), String(s[s.index(after: k)...]))
            }
            if let first = c.first, first.isNumber {
                let digits = c.prefix(while: \.isNumber)
                guard let n = Int(digits) else { throw TemplateError(message: "bad tabstop `<<\(c)>>`") }
                let rest = c.dropFirst(digits.count)
                if rest.isEmpty { return n == 0 ? .final : .tabstop(n, defaultText: nil) }
                guard n > 0 else { throw TemplateError(message: "`<<0>>` is the final caret and takes no default") }
                if rest.first == ":" { return .tabstop(n, defaultText: String(rest.dropFirst())) }
                if rest.first == "|" { return .choice(n, options: rest.dropFirst().split(separator: ",").map(String.init)) }
                throw TemplateError(message: "bad tabstop `<<\(c)>>`")
            }
            if c.hasPrefix("=") {
                let (index, transform) = split(c.dropFirst(), at: "|")
                guard let n = Int(index), n > 0 else { throw TemplateError(message: "bad mirror `<<\(c)>>`") }
                guard let t = MirrorTransform(rawValue: transform ?? "none") else {
                    throw TemplateError(message: "unknown mirror transform `\(transform ?? "")` (slug, upper, lower)")
                }
                return .mirror(n, t)
            }
            switch c {
            case "children": return .children
            case "selection": return .selection
            case "body": return .body
            case "opt": return .opt
            case "overlay": return .overlay
            case "label": return .label
            case "star": return .star
            case "i": return .counter(zeroBased: false)
            case "i0": return .counter(zeroBased: true)
            default: break
            }
            if c.hasPrefix("opt.value") {
                let rest = c.dropFirst("opt.value".count)
                if rest.isEmpty { return .optValue(defaultText: nil) }
                if rest.first == ":" { return .optValue(defaultText: String(rest.dropFirst())) }
            }
            if c.hasPrefix("arg.") {
                let (ref, def) = split(c.dropFirst(4), at: ":")
                guard !ref.isEmpty else { throw TemplateError(message: "`<<arg.>>` needs a number or a name") }
                if let n = Int(ref) {
                    guard n >= 1 else { throw TemplateError(message: "arguments count from 1: `<<\(c)>>`") }
                    return .arg(.index(n), defaultText: def)
                }
                return .arg(.name(ref), defaultText: def)
            }
            if c.hasPrefix("p.") {
                let body = c.dropFirst(2)
                guard let dot = body.firstIndex(of: "."), dot != body.startIndex, body.index(after: dot) < body.endIndex else {
                    throw TemplateError(message: "`<<\(c)>>` needs `p.NAME.FIELD`")
                }
                return .param(name: String(body[..<dot]), field: String(body[body.index(after: dot)...]))
            }
            if c.hasPrefix("profile.") {
                let key = String(c.dropFirst("profile.".count))
                guard !key.isEmpty else { throw TemplateError(message: "`<<profile.>>` needs a key") }
                return .profile(key)
            }
            throw TemplateError(message: "unknown hole `<<\(c)>>`")
        }

        /// Every hole, in order (registry lint).
        public var holes: [Hole] {
            lines.flatMap { $0.parts.compactMap { if case .hole(let h) = $0 { return h }; return nil } }
        }

        public var hasChildren: Bool { holes.contains(.children) }
        public var hasLabel: Bool { holes.contains(.label) }
        public var hasBody: Bool { holes.contains(.body) }
    }
}
