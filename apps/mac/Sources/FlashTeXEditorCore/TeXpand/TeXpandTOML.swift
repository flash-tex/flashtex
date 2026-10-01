import Foundation

/// TeXpand's configuration reader: the subset of TOML 1.0 that definition
/// files and `texpand.toml` use. The app has no Swift TOML parser (the
/// project manifest, `flashtex.toml`, is read by the Rust helper), and the
/// iPad has no helper at all, so the core carries this small one instead of
/// a package dependency.
///
/// Supported: comments; `[table]`, `[a.b]`, `[[array.of.tables]]`; bare,
/// quoted and dotted keys; basic and literal strings, both single-line and
/// multi-line (`"""`, `'''`); integers; floats; booleans; arrays (multi-line,
/// trailing comma); inline tables. Not supported: dates and times (an error
/// names them). Errors carry the 1-based line.
extension TeXpand {
    public enum TOMLValue: Equatable, Sendable {
        case string(String)
        case integer(Int)
        case float(Double)
        case bool(Bool)
        case array([TOMLValue])
        case table(TOMLTable)

        public var string: String? { if case .string(let s) = self { return s }; return nil }
        public var integer: Int? { if case .integer(let i) = self { return i }; return nil }
        public var bool: Bool? { if case .bool(let b) = self { return b }; return nil }
        public var array: [TOMLValue]? { if case .array(let a) = self { return a }; return nil }
        public var table: TOMLTable? { if case .table(let t) = self { return t }; return nil }

        /// The value as it would be written in TOML (inline form).
        public var tomlDescription: String {
            switch self {
            case .string(let s): return TOMLValue.quote(s)
            case .integer(let i): return String(i)
            case .float(let d): return String(d)
            case .bool(let b): return b ? "true" : "false"
            case .array(let a): return "[" + a.map(\.tomlDescription).joined(separator: ", ") + "]"
            case .table(let t): return "{ " + t.entries.map { "\($0.key) = \($0.value.tomlDescription)" }.joined(separator: ", ") + " }"
            }
        }

        static func quote(_ s: String) -> String {
            var out = "\""
            for c in s.unicodeScalars {
                switch c {
                case "\"": out += "\\\""
                case "\\": out += "\\\\"
                case "\n": out += "\\n"
                case "\t": out += "\\t"
                default: out.unicodeScalars.append(c)
                }
            }
            return out + "\""
        }
    }

    /// An ordered table; `line` is where it was opened (its header, or the
    /// line of an inline table), for diagnostics.
    public struct TOMLTable: Equatable, Sendable {
        public var entries: [(key: String, value: TOMLValue)] = []
        public var line: Int = 1

        public init(line: Int = 1) { self.line = line }

        public subscript(key: String) -> TOMLValue? {
            get { entries.first(where: { $0.key == key })?.value }
            set {
                if let i = entries.firstIndex(where: { $0.key == key }) {
                    if let newValue { entries[i].value = newValue } else { entries.remove(at: i) }
                } else if let newValue {
                    entries.append((key, newValue))
                }
            }
        }

        public var keys: [String] { entries.map(\.key) }

        public static func == (a: TOMLTable, b: TOMLTable) -> Bool {
            a.entries.count == b.entries.count && zip(a.entries, b.entries).allSatisfy { $0.key == $1.key && $0.value == $1.value }
        }
    }

    public struct TOMLError: Error, Equatable, Sendable, CustomStringConvertible {
        public var line: Int
        public var message: String
        public var description: String { "line \(line): \(message)" }
    }

    /// Parses a whole document into its root table.
    public static func parseTOML(_ text: String) throws -> TOMLTable {
        var p = TOMLParser(text)
        return try p.document()
    }
}

private struct TOMLParser {
    let s: [Unicode.Scalar]
    var i = 0
    var line = 1

    init(_ text: String) { s = Array(text.unicodeScalars) }

    typealias Table = TeXpand.TOMLTable
    typealias Value = TeXpand.TOMLValue

    func fail(_ message: String) -> TeXpand.TOMLError { TeXpand.TOMLError(line: line, message: message) }

    var atEnd: Bool { i >= s.count }
    var cur: Unicode.Scalar? { i < s.count ? s[i] : nil }
    func peek(_ k: Int = 1) -> Unicode.Scalar? { i + k < s.count ? s[i + k] : nil }

    mutating func advance() {
        if s[i] == "\n" { line += 1 }
        i += 1
    }

    mutating func skipSpaces() { while let c = cur, c == " " || c == "\t" { i += 1 } }

    mutating func skipComment() {
        if cur == "#" { while let c = cur, c != "\n" { i += 1 } }
    }

    /// Spaces, comments and newlines (inside arrays).
    mutating func skipWhitespaceAndNewlines() {
        while let c = cur {
            if c == " " || c == "\t" || c == "\r" || c == "\n" { advance() } else if c == "#" { skipComment() } else { break }
        }
    }

    mutating func endOfLine() throws {
        skipSpaces()
        skipComment()
        if cur == "\r" { i += 1 }
        guard atEnd || cur == "\n" else { throw fail("expected the end of the line, found `\(Character(cur!))`") }
        if !atEnd { advance() }
    }

    // MARK: document

    mutating func document() throws -> Table {
        var root = Table(line: 1)
        // Path of the table the following key/values go into; arrays of
        // tables are addressed by their last element.
        var current: [String] = []
        while true {
            skipWhitespaceAndNewlines()
            guard let c = cur else { break }
            if c == "[" {
                let headerLine = line
                let isArray = peek() == "["
                i += isArray ? 2 : 1
                skipSpaces()
                let path = try key()
                skipSpaces()
                guard cur == "]" else { throw fail("expected `]` after the table name") }
                i += 1
                if isArray {
                    guard cur == "]" else { throw fail("expected `]]` after the array-of-tables name") }
                    i += 1
                }
                try endOfLine()
                if isArray {
                    try Self.appendTable(to: &root, path: path, line: headerLine, parser: self)
                } else {
                    try Self.ensureTable(in: &root, path: path, line: headerLine, parser: self)
                }
                current = path
            } else {
                let path = try key()
                skipSpaces()
                guard cur == "=" else { throw fail("expected `=` after the key `\(path.joined(separator: "."))`") }
                i += 1
                skipSpaces()
                let v = try value()
                try Self.insert(v, at: current, key: path, into: &root, parser: self)
                try endOfLine()
            }
        }
        return root
    }

    // MARK: keys

    mutating func key() throws -> [String] {
        var parts: [String] = []
        while true {
            skipSpaces()
            guard let c = cur else { throw fail("expected a key") }
            if c == "\"" {
                parts.append(try basicString())
            } else if c == "'" {
                parts.append(try literalString())
            } else if Self.isBareKey(c) {
                var k = ""
                while let c = cur, Self.isBareKey(c) { k.unicodeScalars.append(c); i += 1 }
                parts.append(k)
            } else {
                throw fail("expected a key, found `\(Character(c))`")
            }
            skipSpaces()
            if cur == "." { i += 1; continue }
            return parts
        }
    }

    static func isBareKey(_ c: Unicode.Scalar) -> Bool {
        (c >= "a" && c <= "z") || (c >= "A" && c <= "Z") || (c >= "0" && c <= "9") || c == "_" || c == "-"
    }

    // MARK: values

    mutating func value() throws -> Value {
        guard let c = cur else { throw fail("expected a value") }
        switch c {
        case "\"":
            if peek() == "\"", peek(2) == "\"" { return .string(try multilineBasic()) }
            return .string(try basicString())
        case "'":
            if peek() == "'", peek(2) == "'" { return .string(try multilineLiteral()) }
            return .string(try literalString())
        case "[": return try arrayValue()
        case "{": return try inlineTable()
        case "t", "f":
            if matchWord("true") { return .bool(true) }
            if matchWord("false") { return .bool(false) }
            throw fail("expected a value")
        default:
            return try number()
        }
    }

    mutating func matchWord(_ w: String) -> Bool {
        let ws = Array(w.unicodeScalars)
        guard i + ws.count <= s.count, Array(s[i..<(i + ws.count)]) == ws else { return false }
        if i + ws.count < s.count, Self.isBareKey(s[i + ws.count]) { return false }
        i += ws.count
        return true
    }

    mutating func number() throws -> Value {
        var text = ""
        while let c = cur, Self.isBareKey(c) || c == "+" || c == "." || c == ":" { text.unicodeScalars.append(c); i += 1 }
        let clean = text.replacingOccurrences(of: "_", with: "")
        if clean.contains(":") || (clean.filter({ $0 == "-" }).count >= 2 && !clean.hasPrefix("-")) {
            throw fail("dates and times are not supported (`\(text)`)")
        }
        if let n = Int(clean) { return .integer(n) }
        if clean.hasPrefix("0x"), let n = Int(clean.dropFirst(2), radix: 16) { return .integer(n) }
        if let d = Double(clean), clean.contains(where: { $0 == "." || $0 == "e" || $0 == "E" }) { return .float(d) }
        if text.isEmpty, let c = cur { throw fail("expected a value, found `\(Character(c))`") }
        throw fail("expected a value, found `\(text)` (strings need quotes)")
    }

    mutating func basicString() throws -> String {
        i += 1 // "
        var out = ""
        while true {
            guard let c = cur, c != "\n" else { throw fail("unterminated string") }
            i += 1
            if c == "\"" { return out }
            if c == "\\" { out.unicodeScalars.append(contentsOf: try escape().unicodeScalars) } else { out.unicodeScalars.append(c) }
        }
    }

    mutating func escape() throws -> String {
        guard let e = cur else { throw fail("unterminated string") }
        i += 1
        switch e {
        case "n": return "\n"
        case "t": return "\t"
        case "r": return "\r"
        case "b": return "\u{8}"
        case "f": return "\u{C}"
        case "\"": return "\""
        case "\\": return "\\"
        case "u", "U":
            let n = e == "u" ? 4 : 8
            guard i + n <= s.count else { throw fail("short unicode escape") }
            let hex = String(String.UnicodeScalarView(s[i..<(i + n)]))
            i += n
            guard let v = UInt32(hex, radix: 16), let u = Unicode.Scalar(v) else { throw fail("bad unicode escape `\\\(e)\(hex)`") }
            return String(Character(u))
        default:
            throw fail("unknown escape `\\\(Character(e))` (use a literal string, '…', for LaTeX)")
        }
    }

    mutating func literalString() throws -> String {
        i += 1 // '
        var out = ""
        while true {
            guard let c = cur, c != "\n" else { throw fail("unterminated literal string") }
            i += 1
            if c == "'" { return out }
            out.unicodeScalars.append(c)
        }
    }

    mutating func multilineBasic() throws -> String {
        i += 3
        if cur == "\r" { i += 1 }
        if cur == "\n" { advance() }
        var out = ""
        while true {
            guard let c = cur else { throw fail("unterminated multi-line string") }
            if c == "\"", peek() == "\"", peek(2) == "\"" {
                i += 3
                // Up to two further quotes belong to the content.
                while cur == "\"" { out += "\""; i += 1 }
                return out
            }
            if c == "\\" {
                i += 1
                // Line-ending backslash: trim the newline and leading whitespace.
                var j = i
                while j < s.count, s[j] == " " || s[j] == "\t" { j += 1 }
                if j < s.count, s[j] == "\n" || s[j] == "\r" {
                    i = j
                    while let w = cur, w == " " || w == "\t" || w == "\n" || w == "\r" { advance() }
                    continue
                }
                out.unicodeScalars.append(contentsOf: try escape().unicodeScalars)
                continue
            }
            out.unicodeScalars.append(c)
            advance()
        }
    }

    mutating func multilineLiteral() throws -> String {
        i += 3
        if cur == "\r" { i += 1 }
        if cur == "\n" { advance() }
        var out = ""
        while true {
            guard let c = cur else { throw fail("unterminated multi-line literal string") }
            if c == "'", peek() == "'", peek(2) == "'" {
                i += 3
                while cur == "'" { out += "'"; i += 1 }
                return out
            }
            out.unicodeScalars.append(c)
            advance()
        }
    }

    mutating func arrayValue() throws -> Value {
        i += 1 // [
        var items: [Value] = []
        while true {
            skipWhitespaceAndNewlines()
            guard let c = cur else { throw fail("unterminated array") }
            if c == "]" { i += 1; return .array(items) }
            items.append(try value())
            skipWhitespaceAndNewlines()
            if cur == "," { i += 1; continue }
            skipWhitespaceAndNewlines()
            guard cur == "]" else { throw fail("expected `,` or `]` in an array") }
        }
    }

    mutating func inlineTable() throws -> Value {
        var t = Table(line: line)
        i += 1 // {
        skipWhitespaceAndNewlines()
        if cur == "}" { i += 1; return .table(t) }
        while true {
            skipWhitespaceAndNewlines()
            let path = try key()
            skipSpaces()
            guard cur == "=" else { throw fail("expected `=` in an inline table") }
            i += 1
            skipSpaces()
            let v = try value()
            try Self.insertInline(v, key: path, into: &t, parser: self)
            skipWhitespaceAndNewlines()
            if cur == "," { i += 1; continue }
            if cur == "}" { i += 1; return .table(t) }
            throw fail("expected `,` or `}` in an inline table")
        }
    }

    // MARK: tree building

    static func insertInline(_ v: Value, key: [String], into t: inout Table, parser: TOMLParser) throws {
        if key.count == 1 {
            guard t[key[0]] == nil else { throw parser.fail("key `\(key[0])` is defined twice") }
            t[key[0]] = v
            return
        }
        var sub = t[key[0]]?.table ?? Table(line: parser.line)
        if let existing = t[key[0]], existing.table == nil { throw parser.fail("key `\(key[0])` is not a table") }
        try insertInline(v, key: Array(key.dropFirst()), into: &sub, parser: parser)
        t[key[0]] = .table(sub)
    }

    /// Inserts `key = v` into the table at `path` (the last element of an
    /// array of tables when a path component names one).
    static func insert(_ v: Value, at path: [String], key: [String], into root: inout Table, parser: TOMLParser) throws {
        try mutate(&root, path: path, parser: parser) { t in try insertInline(v, key: key, into: &t, parser: parser) }
    }

    static func mutate(_ t: inout Table, path: [String], parser: TOMLParser, _ body: (inout Table) throws -> Void) throws {
        guard let head = path.first else { try body(&t); return }
        let rest = Array(path.dropFirst())
        switch t[head] {
        case .table(var sub)?:
            try mutate(&sub, path: rest, parser: parser, body)
            t[head] = .table(sub)
        case .array(var arr)?:
            guard var last = arr.last?.table else { throw parser.fail("`\(head)` is an array, not a table") }
            try mutate(&last, path: rest, parser: parser, body)
            arr[arr.count - 1] = .table(last)
            t[head] = .array(arr)
        case nil:
            var sub = Table(line: parser.line)
            try mutate(&sub, path: rest, parser: parser, body)
            t[head] = .table(sub)
        default:
            throw parser.fail("`\(head)` is a value, not a table")
        }
    }

    static func ensureTable(in root: inout Table, path: [String], line: Int, parser: TOMLParser) throws {
        let parent = Array(path.dropLast())
        let name = path.last!
        try mutate(&root, path: parent, parser: parser) { t in
            switch t[name] {
            case nil: t[name] = .table(Table(line: line))
            case .table?: break
            default: throw parser.fail("`\(name)` is already a value")
            }
        }
    }

    static func appendTable(to root: inout Table, path: [String], line: Int, parser: TOMLParser) throws {
        let parent = Array(path.dropLast())
        let name = path.last!
        try mutate(&root, path: parent, parser: parser) { t in
            switch t[name] {
            case nil: t[name] = .array([.table(Table(line: line))])
            case .array(var arr)?:
                guard arr.allSatisfy({ $0.table != nil }) else { throw parser.fail("`\(name)` is an array of values, not of tables") }
                arr.append(.table(Table(line: line)))
                t[name] = .array(arr)
            default: throw parser.fail("`\(name)` is already defined as a table or value")
            }
        }
    }
}
