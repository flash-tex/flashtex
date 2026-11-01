import Foundation

/// TeXpand's configuration values: definition files, `texpand.toml` and the
/// built-in catalog, as an ordered tree with each table's 1-based line.
/// Parsing is TOMLDecoder's (dduan/TOMLDecoder, MIT, pure Swift, full TOML
/// 1.1 — evaluated against TOMLKit and swift-toml in docs/texpand/HOST.md);
/// this file is only the shape the registry reads. Dates and times arrive as
/// their TOML text.
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
        /// 1-based; nil when the parser did not say.
        public var line: Int?
        public var message: String
        public var description: String { line.map { "line \($0): \(message)" } ?? message }
    }

    /// Parses a whole TOML document (TOML 1.1, a superset of 1.0) into its
    /// root table, via TOMLDecoder (TeXpandTOMLLibrary.swift).
    public static func parseTOML(_ text: String) throws -> TOMLTable {
        try TOMLLibrary.parse(text)
    }
}
