import Foundation

extension TeXpand {
    /// What the parser needs to know from the registry (PLAN §4.3).
    public struct Oracle: Sendable {
        /// Rule 3: a leaf consumes the rest of the abbreviation as params.
        public var isLeaf: @Sendable (String) -> Bool
        /// Rule 7: false when `name` definitely has no `<<children>>` slot;
        /// nil when the name is unknown (resolution reports it later).
        public var acceptsChildren: @Sendable (String) -> Bool?
        /// Rule 5: `<…>` overlays only inside a beamer frame.
        public var allowsOverlay: Bool

        public init(isLeaf: @escaping @Sendable (String) -> Bool = { _ in false },
                    acceptsChildren: @escaping @Sendable (String) -> Bool? = { _ in nil },
                    allowsOverlay: Bool = false) {
            self.isLeaf = isLeaf; self.acceptsChildren = acceptsChildren; self.allowsOverlay = allowsOverlay
        }

        /// No registry: nothing is a leaf, everything may have children.
        public static let permissive = Oracle(allowsOverlay: true)
    }

    /// Abbreviation parser: grammar PLAN §4.2 with rules §4.3. Pure and
    /// total — every input returns (the fuzz test holds it to that).
    public struct Parser {
        public static func parse(_ input: String, oracle: Oracle = .permissive) -> Result<Abbreviation, ParseError> {
            var p = Parser(Array(input.utf16), oracle)
            do {
                let items = try p.sequence()
                if let c = p.cur {
                    if c == u(")") { throw ParseError(.invalid, at: p.i, "unmatched `)`") }
                    throw ParseError(.invalid, at: p.i, "unexpected `\(p.char(p.i))`")
                }
                return .success(Abbreviation(items: items))
            } catch let e as ParseError {
                return .failure(e)
            } catch {
                return .failure(ParseError(.invalid, at: 0, "\(error)"))
            }
        }

        /// Characters that end a non-leaf param (rule 4), plus `:` and
        /// whitespace: a param containing any needs braces to round-trip.
        static func needsBraces(_ p: String) -> Bool {
            p.utf16.contains { terminators.contains($0) || $0 == u(":") || isSpace($0) || $0 == u("}") || $0 == u("]") }
        }

        /// A leaf param written raw would parse differently.
        static func leafNeedsBraces(_ p: String) -> Bool {
            p.utf16.contains(where: isSpace) || TeXpand.topLevel(p, find: ":") != nil || unwrapBraces(p) != p
        }

        // MARK: state

        let s: [UInt16]
        let oracle: Oracle
        var i = 0
        /// Open `(` groups: a leaf's raw params stop at an unmatched `)` only inside one.
        var groupDepth = 0

        init(_ s: [UInt16], _ oracle: Oracle) { self.s = s; self.oracle = oracle }

        static func u(_ c: Unicode.Scalar) -> UInt16 { UInt16(c.value) }
        static let terminators: Set<UInt16> = Set(">+*()[{#.<".utf16)
        static func isSpace(_ c: UInt16) -> Bool { c == 0x20 || c == 0x09 || c == 0x0A || c == 0x0D }
        static func isLetter(_ c: UInt16) -> Bool { (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) }
        static func isDigit(_ c: UInt16) -> Bool { c >= 0x30 && c <= 0x39 }
        static func isLabelChar(_ c: UInt16) -> Bool { isLetter(c) || isDigit(c) || c == u("_") || c == u("-") || c == u("@") }

        var cur: UInt16? { i < s.count ? s[i] : nil }
        func u(_ c: Unicode.Scalar) -> UInt16 { Self.u(c) }
        func char(_ k: Int) -> String { k < s.count ? String(decoding: [s[k]], as: UTF16.self) : "end" }
        func text(_ a: Int, _ b: Int) -> String { String(decoding: s[a..<b], as: UTF16.self) }

        func incomplete(_ m: String) -> ParseError { ParseError(.incomplete, at: i, m) }
        func invalid(_ m: String, at k: Int? = nil) -> ParseError { ParseError(.invalid, at: k ?? i, m) }

        /// Whitespace is invalid everywhere a bracket does not protect it (rule 6).
        func unexpected() -> ParseError {
            guard let c = cur else { return incomplete("unexpected end") }
            if Self.isSpace(c) { return invalid("whitespace ends the abbreviation") }
            return invalid("unexpected `\(char(i))`")
        }

        // MARK: grammar

        /// seq = item { "+" item } [ ">" seq ]
        mutating func sequence() throws -> [Item] {
            var items = [try item()]
            while let c = cur {
                if c == u("+") {
                    i += 1
                    items.append(try item())
                } else if c == u(">") {
                    let at = i
                    i += 1
                    var last = items.removeLast()
                    switch last.body {
                    case .group:
                        throw invalid("a group cannot have children; put `>` inside the parentheses", at: at)
                    case .element(let e):
                        if e.leafParams || oracle.isLeaf(e.name) { throw invalid("`\(e.name)` cannot have children", at: at) }
                        if oracle.acceptsChildren(e.name) == false { throw invalid("`\(e.name)` cannot have children", at: at) }
                    }
                    last.children = try sequence()
                    items.append(last)
                    return items
                } else {
                    break
                }
            }
            return items
        }

        /// item = ( group | element ) [ repeat ], suffixes allowed after the repeat.
        mutating func item() throws -> Item {
            let start = i
            guard let c = cur else { throw incomplete("expected an abbreviation") }
            var body: Item.Body
            if c == u("(") {
                i += 1
                groupDepth += 1
                let inner = try sequence()
                guard let close = cur else { throw incomplete("expected `)`") }
                guard close == u(")") else { throw unexpected() }
                i += 1
                groupDepth -= 1
                body = .group(inner)
            } else if Self.isLetter(c) {
                body = .element(try element())
            } else if c == u(")") {
                throw invalid(groupDepth > 0 ? "empty group or missing item before `)`" : "unmatched `)`")
            } else {
                throw unexpected()
            }
            var rep: Repeat?
            if cur == u("*") {
                i += 1
                if let d = cur, Self.isDigit(d) {
                    let at = i
                    let n = digits()
                    guard let n, n >= 1 else { throw invalid("a repeat count is at least 1", at: at) }
                    guard n <= 1000 else { throw invalid("a repeat count is at most 1000", at: at) }
                    rep = .count(n)
                } else {
                    rep = .perLine
                }
                // `item*3<+->`: an element's suffixes may follow its repeat.
                if case .element(var e) = body, !e.leafParams {
                    try suffixes(&e)
                    body = .element(e)
                }
            }
            return Item(body: body, repeatCount: rep, offset: start)
        }

        /// Digits as a number, nil if absurdly long.
        mutating func digits() -> Int? {
            let a = i
            while let d = cur, Self.isDigit(d) { i += 1 }
            return i - a > 6 ? nil : Int(text(a, i))
        }

        /// element = name [shape] ["!"] [params] {suffix}
        mutating func element() throws -> Element {
            let start = i
            while let c = cur, Self.isLetter(c) { i += 1 }
            var e = Element(name: text(start, i), offset: start)
            if let c = cur, Self.isDigit(c) { e.shape = try shape() }
            if cur == u("!") {
                i += 1
                e.star = true
                // `align!3` as well as `align3!`.
                if e.shape == nil, let c = cur, Self.isDigit(c) { e.shape = try shape() }
            }
            if cur == u(":") {
                if oracle.isLeaf(e.name) {
                    e.params = try leafParams()
                    e.leafParams = true
                    return e
                }
                e.params = try params()
            }
            try suffixes(&e)
            return e
        }

        /// shape = digits [ "x" digits ]
        mutating func shape() throws -> Shape {
            let at = i
            guard let rows = digits(), rows >= 1, rows <= 1000 else { throw invalid("a size is between 1 and 1000", at: at) }
            guard cur == u("x") else { return Shape(rows: rows) }
            i += 1
            guard let d = cur else { throw incomplete("expected the second dimension after `x`") }
            guard Self.isDigit(d) else { throw invalid("expected a digit after `x`") }
            let at2 = i
            guard let cols = digits(), cols >= 1, cols <= 1000 else { throw invalid("a size is between 1 and 1000", at: at2) }
            return Shape(rows: rows, cols: cols)
        }

        /// Non-leaf params (rule 4): each ends at `:` or the first of
        /// `> + * ( ) [ { # . <` at depth 0 — `.` only before a letter — and
        /// a leading `{` protects the whole param.
        mutating func params() throws -> [String] {
            var out: [String] = []
            while cur == u(":") {
                i += 1
                guard let c = cur else { throw incomplete("expected a parameter after `:`") }
                if c == u("{") {
                    let body = try balanced(open: u("{"), close: u("}"))
                    out.append(body)
                    continue
                }
                let a = i
                // A `.` starts a modifier only before a letter (modifier names
                // are letters) and never as the second dot of `..`, so
                // `col:0.3` and `for:i=1..n` stay one param.
                func ends(_ c: UInt16) -> Bool {
                    guard Self.terminators.contains(c) else { return false }
                    if c == u(".") {
                        let beforeLetter = i + 1 < s.count && Self.isLetter(s[i + 1])
                        let afterDot = i > a && s[i - 1] == u(".")
                        if !beforeLetter || afterDot { return false }
                    }
                    return true
                }
                while let c = cur, c != u(":"), !ends(c) {
                    if Self.isSpace(c) { throw invalid("whitespace ends the abbreviation; wrap the parameter in braces") }
                    if c == u("]") || c == u("}") { throw invalid("unmatched `\(char(i))`") }
                    i += 1
                }
                out.append(text(a, i))
            }
            return out
        }

        /// Leaf params (rule 3): the rest of the abbreviation — or of the
        /// enclosing group — split on `:` at bracket depth 0.
        mutating func leafParams() throws -> [String] {
            i += 1 // the first ':'
            var out: [String] = []
            var stack: [UInt16] = []
            var a = i
            var braced = false // the current param is wrapped in braces so far
            while let c = cur {
                if c == u("\\"), i + 1 < s.count { i += 2; continue }
                if stack.isEmpty {
                    if c == u(")") && groupDepth > 0 { break }
                    if c == u(":") { out.append(text(a, i)); i += 1; a = i; continue }
                }
                switch c {
                case u("("): stack.append(u(")"))
                case u("["): stack.append(u("]"))
                case u("{"): stack.append(u("}"))
                case u(")"), u("]"), u("}"):
                    guard stack.last == c else {
                        throw invalid(stack.isEmpty ? "unmatched `\(char(i))`" : "`\(char(i))` closes the wrong bracket")
                    }
                    stack.removeLast()
                default:
                    if Self.isSpace(c) {
                        // Whitespace only inside a brace-wrapped param (rule 6).
                        braced = s[a] == u("{") && stack.first == u("}")
                        if !braced { throw invalid("whitespace ends the abbreviation; wrap the parameter in braces") }
                    }
                }
                i += 1
            }
            if !stack.isEmpty { throw incomplete("expected `\(String(decoding: [stack.last!], as: UTF16.self))`") }
            if i == a && cur == nil { throw incomplete("expected a parameter after `:`") }
            out.append(text(a, i))
            return out.map(Self.unwrapBraces)
        }

        /// `{…}` whose closing brace is the last character loses its braces.
        static func unwrapBraces(_ p: String) -> String {
            let u16 = Array(p.utf16)
            guard u16.count >= 2, u16.first == u("{"), u16.last == u("}") else { return p }
            var depth = 0
            var k = 0
            while k < u16.count {
                let c = u16[k]
                if c == u("\\") { k += 2; continue }
                if c == u("{") { depth += 1 }
                if c == u("}") { depth -= 1; if depth == 0 && k != u16.count - 1 { return p } }
                k += 1
            }
            return String(decoding: u16[1..<(u16.count - 1)], as: UTF16.self)
        }

        /// suffix = opt | arg | overlay | label | modifier
        mutating func suffixes(_ e: inout Element) throws {
            while let c = cur {
                switch c {
                case u("["):
                    e.opts.append(try balanced(open: u("["), close: u("]")))
                case u("{"):
                    e.args.append(try balanced(open: u("{"), close: u("}")))
                case u("<"):
                    guard oracle.allowsOverlay else { throw invalid("overlays `<…>` only work inside a beamer frame") }
                    guard e.overlay == nil else { throw invalid("two overlays on `\(e.name)`") }
                    i += 1
                    let a = i
                    while let d = cur, d != u(">") {
                        if Self.isSpace(d) { throw invalid("whitespace ends the abbreviation") }
                        i += 1
                    }
                    guard cur != nil else { throw incomplete("expected `>` closing the overlay") }
                    e.overlay = text(a, i)
                    i += 1
                case u("#"):
                    guard e.label == nil else { throw invalid("two labels on `\(e.name)`") }
                    i += 1
                    let a = i
                    while let d = cur, Self.isLabelChar(d) { i += 1 }
                    e.label = i > a ? .named(text(a, i)) : .auto
                case u("."):
                    i += 1
                    let a = i
                    while let d = cur, Self.isLetter(d) { i += 1 }
                    if i == a {
                        if cur == nil { throw incomplete("expected a modifier name after `.`") }
                        throw invalid("a modifier name is letters only")
                    }
                    e.modifiers.append(text(a, i))
                case u(":"):
                    throw invalid("parameters go before `[…]`, `{…}`, labels and modifiers")
                default:
                    return
                }
            }
        }

        /// `open … close` with [] {} () nesting and `\x` escapes; returns
        /// the inside. Whitespace is allowed here.
        mutating func balanced(open: UInt16, close: UInt16) throws -> String {
            i += 1
            let a = i
            var stack: [UInt16] = [close]
            while let c = cur {
                if c == u("\\") { i += min(2, s.count - i); continue }
                switch c {
                case u("["): stack.append(u("]"))
                case u("{"): stack.append(u("}"))
                case u("]"), u("}"):
                    guard stack.last == c else { throw invalid("`\(char(i))` closes the wrong bracket") }
                    stack.removeLast()
                    if stack.isEmpty {
                        let body = text(a, i)
                        i += 1
                        return body
                    }
                default: break
                }
                i += 1
            }
            throw incomplete("expected `\(String(decoding: [stack.last!], as: UTF16.self))`")
        }
    }
}
