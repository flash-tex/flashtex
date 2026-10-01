import XCTest
@testable import FlashTeXEditorCore

/// PLAN M1: golden rows for every operator in §4.1 and every rule in §4.3,
/// `Incomplete`/`Invalid` with offsets, and a fuzz property.
final class ParserTests: XCTestCase {
    typealias T = TeXpand

    /// The math leaves of §14 (M4 ships them; the parser only needs the oracle).
    static let leaves: Set<String> = ["sum", "prod", "lim", "set", "seq", "int", "dd", "pd", "map", "ref", "cite", "plot", "tree", "pmat"]
    static let oracle = T.Oracle(isLeaf: { leaves.contains($0) },
                                 acceptsChildren: { ["img", "cap", "sum"].contains($0) ? false : nil },
                                 allowsOverlay: false)
    static let beamer = T.Oracle(isLeaf: { leaves.contains($0) }, acceptsChildren: { _ in nil }, allowsOverlay: true)

    func parse(_ s: String, _ o: T.Oracle = oracle) -> T.Abbreviation? {
        try? T.Parser.parse(s, oracle: o).get()
    }

    func error(_ s: String, _ o: T.Oracle = oracle) -> T.ParseError? {
        if case .failure(let e) = T.Parser.parse(s, oracle: o) { return e }
        return nil
    }

    /// A compact structural dump: `name` with `<shape>`, `!`, `:param`,
    /// `[opt]`, `{arg}`, `<ov>`, `#label`, `.mod`, `*N`, `>(children)`.
    func tree(_ s: String, _ o: T.Oracle = oracle, file: StaticString = #filePath, line: UInt = #line) -> String {
        switch T.Parser.parse(s, oracle: o) {
        case .success(let a): return a.items.map(dump).joined(separator: " + ")
        case .failure(let e): XCTFail("\(s): \(e)", file: file, line: line); return ""
        }
    }

    func dump(_ item: T.Item) -> String {
        var out: String
        switch item.body {
        case .group(let items): out = "(" + items.map(dump).joined(separator: " + ") + ")"
        case .element(let e):
            out = e.name
            if let s = e.shape { out += "<\(s)>" }
            if e.star { out += "!" }
            for p in e.params { out += (e.leafParams ? "::" : ":") + "'\(p)'" }
            for o in e.opts { out += "[\(o)]" }
            for a in e.args { out += "{\(a)}" }
            if let ov = e.overlay { out += "<ov \(ov)>" }
            switch e.label {
            case .named(let n)?: out += "#\(n)"
            case .auto?: out += "#auto"
            case nil: break
            }
            for m in e.modifiers { out += ".\(m)" }
        }
        switch item.repeatCount {
        case .count(let n)?: out += "*\(n)"
        case .perLine?: out += "*lines"
        case nil: break
        }
        if !item.children.isEmpty { out += " >(" + item.children.map(dump).joined(separator: " + ") + ")" }
        return out
    }

    // MARK: §4.1 operators, one row each

    func testOperatorRows() {
        let rows: [(String, String, T.Oracle)] = [
            ("enum>item*3", "enum >(item*3)", Self.oracle),                             // >
            ("fig>img+cap", "fig >(img + cap)", Self.oracle),                           // +
            ("item*4", "item*4", Self.oracle),                                          // *N
            ("enum>item*", "enum >(item*lines)", Self.oracle),                          // bare *
            ("frame>(cols:6,4>items3)+note", "frame >((cols:'6,4' >(items<3>)) + note)", Self.oracle), // ( )
            ("pmat3x3", "pmat<3x3>", Self.oracle),                                      // NxM
            ("align3", "align<3>", Self.oracle),                                        // N
            ("enum4", "enum<4>", Self.oracle),
            ("align!", "align!", Self.oracle),                                          // !
            ("sec!", "sec!", Self.oracle),
            ("sum:i=1..n", "sum::'i=1..n'", Self.oracle),                              // :param (leaf)
            ("tab:lcr:4", "tab:'lcr':'4'", Self.oracle),                                // :param (non-leaf)
            ("img[width=.8\\linewidth]", "img[width=.8\\linewidth]", Self.oracle),      // [..]
            ("sec{Introduction}", "sec{Introduction}", Self.oracle),                    // {..}
            ("btab:lrr:5{Name,Score,Time}", "btab:'lrr':'5'{Name,Score,Time}", Self.oracle),
            ("item*3<+->", "item<ov +->*3", Self.beamer),                               // <..>
            ("fig#arch", "fig#arch", Self.oracle),                                      // #name
            ("sec{Intro}#", "sec{Intro}#auto", Self.oracle),                            // #
            ("tab:lcr:4.float", "tab:'lcr':'4'.float", Self.oracle),                    // .mod
            ("item{Step @}*3", "item{Step @}*3", Self.oracle),                          // @ (substituted at expansion)
        ]
        for (input, expected, oracle) in rows {
            XCTAssertEqual(tree(input, oracle), expected, input)
        }
    }

    // MARK: §4.3 rules

    /// 1. Names are letters only; digits after them are the shape.
    func testRule1NamesAreLetters() {
        XCTAssertEqual(tree("pmat3x3"), "pmat<3x3>")
        XCTAssertEqual(tree("dd2:y/x"), "dd<2>::'y/x'")
        XCTAssertEqual(tree("align!3"), "align<3>!", "`!` before the size also works")
        XCTAssertEqual(error("pmat3xa")?.kind, .invalid)
        XCTAssertEqual(error("pmat3x")?.kind, .incomplete)
    }

    /// 2. `*` is repetition only; the star variant is `!`.
    func testRule2StarIsRepetition() {
        XCTAssertEqual(tree("sec*"), "sec*lines")
        XCTAssertEqual(tree("sec!*2"), "sec!*2")
        XCTAssertEqual(error("sec*0")?.kind, .invalid)
    }

    /// 3. A leaf consumes the rest as params, split on `:` at depth 0.
    func testRule3LeafRawParams() {
        XCTAssertEqual(tree("lim:x->0"), "lim::'x->0'")
        XCTAssertEqual(tree("set:x|x>0"), "set::'x|x>0'")
        XCTAssertEqual(tree("seq:x_1+..+n"), "seq::'x_1+..+n'")
        XCTAssertEqual(tree("map:f:x|->x^2"), "map::'f'::'x|->x^2'")
        XCTAssertEqual(tree("plot:sin(x):-pi..pi"), "plot::'sin(x)'::'-pi..pi'", "`:` inside brackets does not split")
        XCTAssertEqual(tree("sum:i=1..n+x"), "sum::'i=1..n+x'", "no siblings after a leaf's params")
        XCTAssertEqual(tree("(sum:i=1..n)+(prod:j=1..m)"), "(sum::'i=1..n') + (prod::'j=1..m')")
        XCTAssertEqual(tree("(sum:i=1..n)*2"), "(sum::'i=1..n')*2")
        XCTAssertEqual(tree("ref:fig:arch"), "ref::'fig'::'arch'")
        XCTAssertEqual(tree("sum"), "sum", "a leaf without params is an ordinary element")
    }

    /// 4. Non-leaf params end at `> + * ( ) [ { # . <`; braces protect.
    func testRule4NonLeafParams() {
        XCTAssertEqual(tree("thm:{a>b}"), "thm:'a>b'")
        XCTAssertEqual(tree("cols:6,4>items3"), "cols:'6,4' >(items<3>)")
        XCTAssertEqual(tree("x:a+y"), "x:'a' + y")
        XCTAssertEqual(tree("x:a*2"), "x:'a'*2")
        XCTAssertEqual(tree("x:a[o]{g}#l.m"), "x:'a'[o]{g}#l.m")
        XCTAssertEqual(tree("x::b"), "x:'':'b'", "an empty param is an omitted one")
        // Adaptation: `.` ends a param only before a letter (a modifier).
        XCTAssertEqual(tree("col:0.3"), "col:'0.3'")
        XCTAssertEqual(tree("for:i=1..n>st"), "for:'i=1..n' >(st)")
        XCTAssertEqual(tree("tab:lcr:4.float"), "tab:'lcr':'4'.float")
        XCTAssertEqual(tree("x:1."), "x:'1.'")
        XCTAssertEqual(error("x:a[o]:b")?.kind, .invalid, "params come before suffixes")
    }

    /// 5. Overlays only inside a beamer frame.
    func testRule5Overlay() {
        let e = error("item<+->")
        XCTAssertEqual(e?.kind, .invalid)
        XCTAssertEqual(e?.offset, 4)
        XCTAssertEqual(tree("item<+->", Self.beamer), "item<ov +->")
        XCTAssertEqual(error("item<+-", Self.beamer)?.kind, .incomplete)
    }

    /// 6. Whitespace only inside `[]`, `{}` and brace-wrapped leaf params.
    func testRule6Whitespace() {
        XCTAssertEqual(tree("sec{A long title}"), "sec{A long title}")
        XCTAssertEqual(tree("img[width = 3cm]"), "img[width = 3cm]")
        XCTAssertEqual(tree("set:{x | x > 0}"), "set::'x | x > 0'")
        XCTAssertEqual(error("enum 3")?.kind, .invalid)
        XCTAssertEqual(error("enum 3")?.offset, 4)
        XCTAssertEqual(error("set:x | x")?.kind, .invalid)
        XCTAssertEqual(error("x:a b")?.kind, .invalid)
        XCTAssertEqual(error("a+ b")?.kind, .invalid)
    }

    /// 7. Children need a `<<children>>` slot.
    func testRule7ChildrenWithoutSlot() {
        let e = error("img>cap")
        XCTAssertEqual(e?.kind, .invalid)
        XCTAssertEqual(e?.offset, 3)
        XCTAssertEqual(e?.message, "`img` cannot have children")
        XCTAssertEqual(error("sum>x")?.message, "`sum` cannot have children", "a leaf has no children")
        XCTAssertEqual(error("(a+b)>c")?.kind, .invalid, "a group cannot have children")
        XCTAssertNotNil(parse("unknown>x"), "unknown names are the resolver's business")
    }

    /// 8. Missing params and args become tabstops — the expander's rule;
    /// the parser keeps the element with nothing filled in.
    func testRule8MissingParamsParse() {
        XCTAssertEqual(tree("sum"), "sum")
        XCTAssertEqual(tree("sec"), "sec")
    }

    // MARK: incomplete vs invalid

    func testIncompleteInputs() {
        let rows: [(String, Int)] = [
            ("", 0), ("enum>", 5), ("fig>img+", 8), ("(a+b", 4), ("sec{Intro", 9), ("img[width", 9),
            ("tab:", 4), ("tab:lcr:", 8), ("x.", 2), ("pmat3x", 6), ("sum:", 4), ("sum:{x", 6), ("((a)", 4),
        ]
        for (input, offset) in rows {
            let e = error(input)
            XCTAssertEqual(e?.kind, .incomplete, "\(input): \(String(describing: e))")
            XCTAssertEqual(e?.offset, offset, input)
        }
    }

    func testInvalidInputs() {
        let rows: [(String, Int)] = [
            (")", 0), ("a)", 1), ("a]", 1), ("3a", 0), ("a..b", 2), ("a:b]", 3), ("sec{a]}", 5), ("a#x#y", 3),
            ("a*0", 2), ("sec{A}}", 6), ("sum:x)", 5), ("sum:(x]", 6), ("a<b>", 1), ("+a", 0), ("a++b", 2),
        ]
        for (input, offset) in rows {
            let e = error(input)
            XCTAssertEqual(e?.kind, .invalid, "\(input): \(String(describing: e))")
            XCTAssertEqual(e?.offset, offset, input)
        }
    }

    func testOffsetsAreUTF16() {
        // `λ` is one UTF-16 unit, `𝛌` two: offsets count units.
        XCTAssertEqual(error("x:{λ𝛌} y")?.offset, 7)
        XCTAssertEqual(tree("sec{λ}"), "sec{λ}")
    }

    func testEscapesInBrackets() {
        XCTAssertEqual(tree("sec{a\\}b}"), "sec{a\\}b}")
        XCTAssertEqual(tree("img[a\\]b]"), "img[a\\]b]")
    }

    // MARK: properties

    /// Ok parses re-serialise to an equal AST.
    func testRoundTripOnExamples() {
        let inputs = [
            "enum>item*3", "fig>img+cap", "frame>(cols:6,4>items3)+note", "align!3", "sum:i=1..n", "tab:lcr:4.float",
            "thm:{a>b}", "set:{x | x > 0}", "(sum:i=1..n)+(prod:j=1..m)", "desc>item[Term]*3", "sec{Intro}#",
            "item{Step @}*3", "x::b", "ref:{a:b}", "map:f:A->B", "a>(b>c)+d", "x:{{a}b}",
        ]
        for input in inputs {
            guard let ast = parse(input) else { XCTFail("\(input) should parse"); continue }
            XCTAssertEqual(parse(ast.description), ast, "\(input) → \(ast.description)")
        }
    }

    /// The parser terminates without crashing on arbitrary input, and every
    /// Ok parse round-trips. Deterministic (seeded) so a failure reproduces.
    func testFuzzTerminatesAndRoundTrips() {
        var rng = SplitMix64(seed: 0x7E_4E_A1D)
        let alphabet = Array("abcsumlixyz0129 >+*()[]{}<>#.:!@|-=/\\,_λ\n")
        var ok = 0
        for _ in 0..<12_000 {
            let n = Int(rng.next() % 24)
            let s = String((0..<n).map { _ in alphabet[Int(rng.next() % UInt64(alphabet.count))] })
            let oracle = rng.next() % 2 == 0 ? Self.oracle : Self.beamer
            switch T.Parser.parse(s, oracle: oracle) {
            case .success(let ast):
                ok += 1
                let again = T.Parser.parse(ast.description, oracle: oracle)
                XCTAssertEqual(try? again.get(), ast, "round trip of \(s.debugDescription) via \(ast.description.debugDescription)")
            case .failure(let e):
                XCTAssert(e.offset >= 0 && e.offset <= s.utf16.count, "offset \(e.offset) out of range for \(s.debugDescription)")
            }
        }
        XCTAssertGreaterThan(ok, 200, "the fuzz corpus should include valid abbreviations")
    }

    /// The expander never crashes on what the parser accepts.
    func testFuzzExpanderIsTotal() {
        let engine = T.Engine()
        var rng = SplitMix64(seed: 42)
        let words = ["enum", "item", "fig", "img", "cap", "sec", "tab", "btab", "thm", "pf", "eq", "align", "alg", "st", "frame", "cols", "ref", "doc", "x"]
        let glue = [">", "+", "*2", "*", "!", "3", ":lcr", "{T}", "[o]", "#l", "#", ".float", "(", ")", "@"]
        for _ in 0..<1_500 {
            var s = ""
            for _ in 0..<(1 + Int(rng.next() % 5)) {
                s += words[Int(rng.next() % UInt64(words.count))]
                if rng.next() % 2 == 0 { s += glue[Int(rng.next() % UInt64(glue.count))] }
            }
            for scope in [T.ScopeStack.text(), .math, .preamble, .text("frame")] {
                _ = engine.expand(s, in: T.Context(scope: scope, documentClass: "beamer"))
            }
        }
    }
}

/// Deterministic PRNG for the fuzz tests.
struct SplitMix64 {
    var state: UInt64
    init(seed: UInt64) { state = seed }
    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}
