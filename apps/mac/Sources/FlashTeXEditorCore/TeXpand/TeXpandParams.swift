import Foundation

extension TeXpand {
    /// Declared param types (PLAN §4.4). Definitions stay declarative: the
    /// engine parses a param into named fields and templates read them as
    /// `<<p.NAME.FIELD>>`.
    public enum ParamType: String, CaseIterable, Sendable {
        case raw, int, range, ratio, list, arrow, pred, shape, colspec
    }

    /// A parsed param: named string fields. List fields are also exposed
    /// element-wise (`dens.0`, `items.1`) and with `.count`.
    public struct TypedParam: Equatable, Sendable {
        public var type: ParamType
        public var fields: [String: String]
        public var lists: [String: [String]]

        public init(type: ParamType, fields: [String: String] = [:], lists: [String: [String]] = [:]) {
            self.type = type; self.fields = fields; self.lists = lists
        }

        /// `value`, `lo`, `dens.0`, `items.count`, …; nil when absent.
        public func field(_ path: String) -> String? {
            if let v = fields[path] { return v }
            let parts = path.split(separator: ".", maxSplits: 1).map(String.init)
            guard parts.count == 2, let list = lists[parts[0]] else {
                if let list = lists[path] { return list.joined(separator: ",") }
                return nil
            }
            if parts[1] == "count" { return String(list.count) }
            guard let k = Int(parts[1]), k >= 0, k < list.count else { return nil }
            return list[k]
        }

        /// The fields a type defines (for registry lint of `<<p.X.FIELD>>`).
        public static func fieldNames(of type: ParamType) -> Set<String> {
            switch type {
            case .raw: return ["value"]
            case .int: return ["value"]
            case .range: return ["var", "lo", "hi", "value"]
            case .ratio: return ["num", "dens", "value"]
            case .list: return ["items", "value"]
            case .arrow: return ["lhs", "rhs", "kind", "cmd", "value"]
            case .pred: return ["lhs", "rhs", "value"]
            case .shape: return ["rows", "cols", "symbolic", "value"]
            case .colspec: return ["value", "ncols"]
            }
        }
    }

    public struct ParamError: Error, Equatable, Sendable {
        public var message: String
    }

    /// Parses `text` as `type`. Every result also carries `value`: the text itself.
    public static func parseParam(_ text: String, as type: ParamType) -> Result<TypedParam, ParamError> {
        var p = TypedParam(type: type, fields: ["value": text])
        func fail(_ m: String) -> Result<TypedParam, ParamError> { .failure(ParamError(message: m)) }
        switch type {
        case .raw:
            break
        case .int:
            guard !text.isEmpty, text.allSatisfy({ $0.isASCII && $0.isNumber }) else { return fail("`\(text)` is not a whole number") }
        case .range:
            // [var=]lo..hi
            var body = Substring(text)
            if let eq = topLevel(text, find: "="), topLevel(text, find: "..").map({ $0 > eq }) ?? true {
                p.fields["var"] = String(text[..<eq])
                body = text[text.index(after: eq)...]
            }
            guard let dots = topLevel(String(body), find: "..") else { return fail("`\(text)` is not a range (`lo..hi` or `i=lo..hi`)") }
            let b = String(body)
            p.fields["lo"] = String(b[..<dots])
            p.fields["hi"] = String(b[b.index(dots, offsetBy: 2)...])
        case .ratio:
            // num/den[,den2,...]
            guard let slash = topLevel(text, find: "/") else { return fail("`\(text)` is not a ratio (`num/den`)") }
            p.fields["num"] = String(text[..<slash])
            let dens = splitTopLevel(String(text[text.index(after: slash)...]), on: ",")
            guard !dens.contains(where: \.isEmpty) else { return fail("`\(text)` has an empty denominator") }
            p.lists["dens"] = dens
        case .list:
            p.lists["items"] = text.isEmpty ? [] : splitTopLevel(text, on: ",")
        case .arrow:
            if let r = topLevel(text, find: "|->") {
                p.fields["lhs"] = String(text[..<r]); p.fields["rhs"] = String(text[text.index(r, offsetBy: 3)...]); p.fields["kind"] = "mapsto"; p.fields["cmd"] = "\\mapsto"
            } else if let r = topLevel(text, find: "->") {
                p.fields["lhs"] = String(text[..<r]); p.fields["rhs"] = String(text[text.index(r, offsetBy: 2)...]); p.fields["kind"] = "to"; p.fields["cmd"] = "\\to"
            } else {
                return fail("`\(text)` is not an arrow (`a->b` or `a|->b`)")
            }
        case .pred:
            guard let bar = topLevel(text, find: "|") else { return fail("`\(text)` is not a predicate (`x|condition`)") }
            p.fields["lhs"] = String(text[..<bar]); p.fields["rhs"] = String(text[text.index(after: bar)...])
        case .shape:
            let parts = text.split(separator: "x", omittingEmptySubsequences: false).map(String.init)
            if parts.count == 1, let n = Int(parts[0]), n >= 1 {
                p.fields["rows"] = String(n); p.fields["cols"] = String(n); p.fields["symbolic"] = "false"
            } else if parts.count == 2, let r = Int(parts[0]), let c = Int(parts[1]), r >= 1, c >= 1 {
                p.fields["rows"] = String(r); p.fields["cols"] = String(c); p.fields["symbolic"] = "false"
            } else if parts.count == 2, !parts[0].isEmpty, !parts[1].isEmpty,
                      (parts[0] + parts[1]).allSatisfy({ $0.isLetter || $0.isNumber }) {
                p.fields["rows"] = parts[0]; p.fields["cols"] = parts[1]; p.fields["symbolic"] = "true"
            } else {
                return fail("`\(text)` is not a size (`3`, `3x4` or `mxn`)")
            }
        case .colspec:
            guard let n = columnCount(text) else { return fail("`\(text)` is not a column specification") }
            p.fields["ncols"] = String(n)
        }
        return .success(p)
    }

    /// Index of the first `needle` at bracket depth 0 (`()`, `[]`, `{}`).
    static func topLevel(_ text: String, find needle: String) -> String.Index? {
        var depth = 0
        var i = text.startIndex
        while i < text.endIndex {
            let c = text[i]
            if c == "\\" { i = text.index(i, offsetBy: 2, limitedBy: text.endIndex) ?? text.endIndex; continue }
            if "([{".contains(c) { depth += 1 } else if ")]}".contains(c) { depth -= 1 }
            if depth == 0, text[i...].hasPrefix(needle) { return i }
            i = text.index(after: i)
        }
        return nil
    }

    /// Splits on `sep` at bracket depth 0.
    public static func splitTopLevel(_ text: String, on sep: Character) -> [String] {
        var out: [String] = []
        var depth = 0
        var cur = ""
        var escape = false
        for c in text {
            if escape { cur.append(c); escape = false; continue }
            if c == "\\" { escape = true; cur.append(c); continue }
            if "([{".contains(c) { depth += 1 } else if ")]}".contains(c) { depth -= 1 }
            if c == sep, depth == 0 { out.append(cur); cur = ""; continue }
            cur.append(c)
        }
        out.append(cur)
        return out
    }

    /// Columns of a tabular preamble: `l c r`, `p{…} m{…} b{…}`, `X`, `S`,
    /// `*{n}{spec}`; `|`, `@{…}`, `!{…}`, `>{…}`, `<{…}` and spaces add none.
    /// Letters defined by `\newcolumntype` count as one column each. Nil when
    /// the spec is malformed or has no column.
    public static func columnCount(_ spec: String) -> Int? {
        let cs = Array(spec)
        var i = 0
        func group() -> String? {
            while i < cs.count, cs[i] == " " { i += 1 }
            guard i < cs.count, cs[i] == "{" else { return nil }
            var depth = 0
            let a = i
            while i < cs.count {
                if cs[i] == "{" { depth += 1 } else if cs[i] == "}" { depth -= 1; if depth == 0 { i += 1; return String(cs[(a + 1)..<(i - 1)]) } }
                i += 1
            }
            return nil
        }
        var n = 0
        while i < cs.count {
            let c = cs[i]
            i += 1
            switch c {
            case " ", "|", "\t": continue
            case "@", "!", ">", "<":
                guard group() != nil else { return nil }
            case "p", "m", "b":
                guard group() != nil else { return nil }
                n += 1
            case "*":
                guard let count = group().flatMap({ Int($0.trimmingCharacters(in: .whitespaces)) }), let inner = group(),
                      let k = columnCount(inner) else { return nil }
                n += count * k
            case _ where c.isLetter:
                n += 1
            default:
                return nil
            }
        }
        return n > 0 ? n : nil
    }
}
