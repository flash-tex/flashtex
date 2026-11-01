import Foundation

/// TeXpand: Emmet-style LaTeX abbreviations (docs/texpand/PLAN.md).
///
/// The core — parser, registry, template engine, expander — is pure Swift
/// with Foundation only, shared by the Mac and iPad editors through the
/// `FlashTeXEditorCore` symlink, and runs headlessly in tests. Everything
/// editor-specific (keystrokes, Tab precedence, undo, preview) belongs to
/// the host adapters (docs/texpand/HOST.md), which arrive in M3.
public enum TeXpand {}

// MARK: - AST

extension TeXpand {
    /// A parsed abbreviation: top-level siblings (`a+b+c`).
    public struct Abbreviation: Equatable, Sendable, CustomStringConvertible {
        public var items: [Item]
        public init(items: [Item]) { self.items = items }
        /// Canonical re-serialisation; parsing it yields an equal AST.
        public var description: String { Self.serialize(items) }

        static func serialize(_ items: [Item]) -> String {
            items.map(\.description).joined(separator: "+")
        }
    }

    /// One item of a sequence: an element or a group, repeated, with the
    /// children `>` attached to it.
    public struct Item: Equatable, Sendable, CustomStringConvertible {
        public enum Body: Equatable, Sendable {
            case element(Element)
            case group([Item])
        }
        public var body: Body
        public var repeatCount: Repeat?
        public var children: [Item]
        /// UTF-16 offset of the item in the abbreviation.
        public var offset: Int

        public init(body: Body, repeatCount: Repeat? = nil, children: [Item] = [], offset: Int = 0) {
            self.body = body; self.repeatCount = repeatCount; self.children = children; self.offset = offset
        }

        public var element: Element? { if case .element(let e) = body { return e }; return nil }

        public var description: String {
            var out: String
            var trailing = ""
            switch body {
            case .element(let e):
                out = e.headDescription
                trailing = e.suffixDescription
            case .group(let items):
                out = "(" + Abbreviation.serialize(items) + ")"
            }
            // Suffixes print before the repeat: `item[x]*3` and `item*3[x]`
            // parse to the same element.
            out += trailing
            if let r = repeatCount { out += r.description }
            if !children.isEmpty {
                // A child sequence runs to the end of its enclosing sequence,
                // so an item with children must be the last one; a group keeps
                // later siblings from being swallowed.
                out += ">" + Abbreviation.serialize(children)
            }
            return out
        }

        /// Equality ignores offsets (a re-serialised abbreviation has different ones).
        public static func == (a: Item, b: Item) -> Bool {
            a.body == b.body && a.repeatCount == b.repeatCount && a.children == b.children
        }
    }

    public enum Repeat: Equatable, Sendable, CustomStringConvertible {
        /// `*N`
        case count(Int)
        /// Bare `*`: once per selected line (wrap mode, M7).
        case perLine
        public var description: String {
            switch self {
            case .count(let n): return "*\(n)"
            case .perLine: return "*"
            }
        }
    }

    /// `N` or `NxM` after a name (`enum4`, `pmat3x3`).
    public struct Shape: Equatable, Sendable, CustomStringConvertible {
        public var rows: Int
        public var cols: Int?
        public init(rows: Int, cols: Int? = nil) { self.rows = rows; self.cols = cols }
        public var count: Int { rows * (cols ?? 1) }
        public var description: String { cols.map { "\(rows)x\($0)" } ?? "\(rows)" }
    }

    public enum Label: Equatable, Sendable {
        /// `#name`
        case named(String)
        /// Bare `#`: slugged from the title or caption.
        case auto
    }

    /// One abbreviation element: `name[shape][!][:params]{suffix}`.
    public struct Element: Equatable, Sendable {
        public var name: String
        public var shape: Shape?
        public var star: Bool = false
        /// Params in order, split on `:` at depth 0; a fully braced param
        /// (`{a>b}`) is unwrapped. Empty strings are omitted params.
        public var params: [String] = []
        /// Whether the params were consumed raw (a leaf, rule 4.3.3).
        public var leafParams: Bool = false
        public var opts: [String] = []
        public var args: [String] = []
        public var overlay: String?
        public var label: Label?
        public var modifiers: [String] = []
        /// UTF-16 offset of the name in the abbreviation.
        public var offset: Int = 0

        public init(name: String, offset: Int = 0) { self.name = name; self.offset = offset }

        var headDescription: String {
            var out = name
            if let shape { out += shape.description }
            if star { out += "!" }
            for p in params {
                // Braces protect grammar characters in non-leaf params; a
                // leaf's raw params only need them around whitespace, a `:`
                // or text that is itself brace-wrapped.
                let braces = leafParams ? Parser.leafNeedsBraces(p) : Parser.needsBraces(p)
                out += braces ? ":{" + p + "}" : ":" + p
            }
            return out
        }

        var suffixDescription: String {
            var out = ""
            for o in opts { out += "[" + o + "]" }
            for a in args { out += "{" + a + "}" }
            if let overlay { out += "<" + overlay + ">" }
            switch label {
            case .named(let n)?: out += "#" + n
            case .auto?: out += "#"
            case nil: break
            }
            for m in modifiers { out += "." + m }
            return out
        }

        public static func == (a: Element, b: Element) -> Bool {
            a.name == b.name && a.shape == b.shape && a.star == b.star && a.params == b.params
                && a.opts == b.opts && a.args == b.args && a.overlay == b.overlay
                && a.label == b.label && a.modifiers == b.modifiers
        }
    }

    /// A parse failure. `incomplete`: the input could still become valid by
    /// typing more (the capture keeps going); `invalid`: it cannot (the
    /// capture cancels). `offset` is a UTF-16 offset into the abbreviation.
    public struct ParseError: Error, Equatable, Sendable, CustomStringConvertible {
        public enum Kind: Equatable, Sendable { case incomplete, invalid }
        public var kind: Kind
        public var offset: Int
        public var message: String
        public init(_ kind: Kind, at offset: Int, _ message: String) {
            self.kind = kind; self.offset = offset; self.message = message
        }
        public var description: String { "\(kind == .incomplete ? "incomplete" : "invalid") at \(offset): \(message)" }
    }
}
