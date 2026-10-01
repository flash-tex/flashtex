import XCTest
@testable import FlashTeXEditorCore

/// PLAN M2 expander: holes, Snippet IR, renumbering, indentation, repeat
/// counters, labels, variants; typed params (§4.4); the grid and scope
/// models the structure editor (M10b) builds on.
final class ExpanderTests: XCTestCase {
    typealias T = TeXpand

    /// §6.1's own examples, as a user layer.
    static let specExamples = T.Layer(name: "spec", source: #"""
    [[abbr]]
    name = "sum"
    scope = ["math"]
    leaf = true
    params = [{ name = "r", type = "range", default = "i=1..n" }]
    body = '\sum_{<<p.r.var>>=<<p.r.lo>>}^{<<p.r.hi>>} <<0>>'

    [[abbr]]
    name = "dd"
    scope = ["math"]
    leaf = true
    params = [{ name = "q", type = "ratio", default = "y/x" }]
    body = '\frac{<<profile.diff_d>><<p.q.num>>}{<<profile.diff_d>><<p.q.dens.0>>}'
    [[abbr.variant]]
    when = { package = "physics" }
    body = '\dv{<<p.q.num>>}{<<p.q.dens.0>>}'

    [[abbr]]
    name = "holes"
    body = '<<1:a>> <<=1>> <<=1|upper>> <<2|x,y>> \<<lit>> <<star>><<i>>/<<i0>> <<selection>>'

    [[abbr]]
    name = "grid"
    shape = "param"
    params = [{ name = "fill", type = "raw", default = "0" }, { name = "shape", type = "shape", default = "2x2" }]
    body = '<<p.shape.rows>> by <<p.shape.cols>> of <<p.fill.value>>'

    [[abbr]]
    name = "twice"
    params = [{ name = "v", type = "raw", default = "x" }]
    body = '<<p.v.value>> = <<p.v.value>>'
    """#)

    let engine = T.Engine(layers: [specExamples])
    let math = T.Context(scope: .math)

    func testMissingParamsBecomeTabstopsWithDefaults() {
        XCTAssertEqual(engine.expandToString("sum", in: math), "\\sum_{${1:i}=${2:1}}^{${3:n}} $0", "rule 8: `;sum` alone is useful")
        XCTAssertEqual(engine.expandToString("sum:k=0..m", in: math), "\\sum_{k=0}^{m} $0")
        XCTAssertEqual(engine.expandToString("sum:0..m", in: math), "\\sum_{${1:i}=0}^{m} $0", "a field the param leaves out comes from the default")
        XCTAssertEqual(engine.expandToString("sum:k", in: math), "error: `sum`, param `r`: `k` is not a range (`lo..hi` or `i=lo..hi`)")
    }

    func testProfilesAndPackageVariants() {
        XCTAssertEqual(engine.expandToString("dd:y/x", in: math), "\\frac{dy}{dx}")
        var upright = T.Settings()
        upright.profile = "upright"
        XCTAssertEqual(T.Engine(settings: upright, layers: [Self.specExamples]).expandToString("dd:y/x", in: math), "\\frac{\\mathrm{d}y}{\\mathrm{d}x}")
        var physics = math
        physics.packages = ["physics"]
        XCTAssertEqual(engine.expandToString("dd:y/x", in: physics), "\\dv{y}{x}", "the first matching variant wins")
        XCTAssertEqual(engine.expandToString("dd", in: math), "\\frac{d${1:y}}{d${2:x}}")
    }

    func testShapeModes() {
        XCTAssertEqual(engine.expandToString("grid3x4"), "3 by 4 of ${1:0}", "a size fills the `shape` param")
        XCTAssertEqual(engine.expandToString("grid3:I"), "3 by 3 of I")
        XCTAssertEqual(engine.expandToString("grid"), "${1:2} by ${2:2} of ${3:0}")
        XCTAssertEqual(engine.expandToString("sec3"), "error: `sec` takes no size")
        XCTAssertEqual(engine.expandToString("sec3>items"), "error: `sec` takes no size", "checked before the children")
        let bad = T.Layer(name: "user", source: "[[abbr]]\nname = \"m\"\nshape = \"param\"\nbody = 'x'\n")
        XCTAssertTrue(T.Engine(layers: [bad]).diagnostics.contains { $0.definition == "m" && $0.message.contains("param named `shape`") })
    }

    func testHoleForms() {
        XCTAssertEqual(engine.expandToString("holes"), "${1:a} $1 ${1/upper} ${2|x,y|} <<lit>> 1/0 $3")
        XCTAssertEqual(engine.expandToString("holes!*2"),
                       "${1:a} $1 ${1/upper} ${2|x,y|} <<lit>> *1/0 $3\n${4:a} $4 ${4/upper} ${5|x,y|} <<lit>> *2/1 $6")
        var wrap = T.Context()
        wrap.selection = "chosen"
        XCTAssertEqual(engine.expandToString("holes", in: wrap), "${1:a} $1 ${1/upper} ${2|x,y|} <<lit>> 1/0 chosen")
        XCTAssertEqual(engine.expandToString("twice"), "${1:x} = $1", "a defaulted param used twice mirrors itself")
    }

    func testTemplateParsing() throws {
        let t = try T.Template("a\n  <<children>>\n\t<<label>>\n   b\n")
        XCTAssertEqual(t.lines.map(\.level), [0, 1, 1, 1])
        XCTAssertEqual(t.lines[3].parts, [.text(" b")], "an odd space is kept")
        XCTAssertThrowsError(try T.Template("<<nope>>"))
        XCTAssertThrowsError(try T.Template("<<1"))
        XCTAssertThrowsError(try T.Template("<<=0>>"))
        XCTAssertThrowsError(try T.Template("<<=1|shout>>"))
        XCTAssertThrowsError(try T.Template("<<arg.0>>"))
        XCTAssertEqual(try T.Template("<<p.q.dens.0>>").holes, [.param(name: "q", field: "dens.0")])
        XCTAssertEqual(try T.Template("<<profile.label_prefix.fig>>").holes, [.profile("label_prefix.fig")])
    }

    func testRenumberingIsGlobalAndContiguous() throws {
        // Every successful catalog expansion numbers its stops 1…N in
        // document order, and expanding twice gives the same snippet.
        let e = T.Engine()
        let abbreviations = ["enum3", "fig", "subfig2x2", "thm{A}#a+pf", "btab:lrr:3.float#t", "alg>fn>st*3+ret", "align!4",
                             "frame>(cols:3,3,3)+block>items2", "doc:hw", "sec#", "tab", "desc3"]
        for a in abbreviations {
            let ctx = T.Context(documentClass: "beamer")
            guard case .success(let x) = e.expand(a, in: ctx) else { XCTFail(a); continue }
            let stops = x.snippet.tabstopIndices
            XCTAssertEqual(stops, Array(1...max(1, stops.count)).prefix(stops.count).map { $0 }, a)
            XCTAssertEqual(try? e.expand(a, in: ctx).get(), x, "\(a) is deterministic")
        }
    }

    func testRequirementsAreReportedOnceAndOnlyWhenMissing() throws {
        let e = T.Engine()
        let x = try e.expand("subfig2+fig+btab").get()
        XCTAssertEqual(x.requires.map(\.name), ["subcaption", "graphicx", "booktabs"])
        let loaded = try e.expand("subfig2+fig+btab", in: T.Context(packages: ["graphicx"])).get()
        XCTAssertEqual(loaded.requires.map(\.name), ["subcaption", "booktabs"])
        XCTAssertEqual(T.PackageRequirement("cleveref", options: ["capitalize"]).description, "\\usepackage[capitalize]{cleveref}")
    }

    func testBaseIndentAppliesToEveryLine() {
        let ctx = T.Context(indentUnit: "    ", baseIndent: "\t")
        XCTAssertEqual(T.Engine().expandToString("items2", in: ctx), "\\begin{itemize}\n\t    \\item $1\n\t    \\item $2\n\t\\end{itemize}")
    }

    func testLimits() {
        let e = T.Engine()
        XCTAssertEqual(e.expandToString("enum>item*1000+item"), "\\begin{enumerate}\n" + (1...1001).map { "  \\item $\($0)" }.joined(separator: "\n") + "\n\\end{enumerate}")
        XCTAssertEqual(e.expandToString("(enum>item*1000)*3"), "error: the expansion is too large (over 2000 elements)")
        let loop = T.Layer(name: "user", source: "[[abbr]]\nname = \"loop\"\ndefault_child = \"loop\"\nbody = '<<children>>'\n")
        XCTAssertEqual(T.Engine(layers: [loop]).expandToString("loop"), "error: `loop` nests too deeply (a default child that contains itself?)")
    }

    func testFlattenedAndLaTeXSnippet() throws {
        let x = try T.Engine().expand("sec#").get()
        let flat = x.snippet.flattened()
        XCTAssertEqual(flat.text, "\\section{}\\label{sec:}")
        XCTAssertEqual(flat.fields, [.init(index: 1, range: NSRange(location: 9, length: 0))])
        XCTAssertEqual(flat.latexSnippet, LaTeXSnippet(text: "\\section{}\\label{sec:}", caretUTF16: 9, stops: [22]))

        let sum = try engine.expand("sum", in: math).get().snippet.flattened()
        XCTAssertEqual(sum.text, "\\sum_{i=1}^{n} ")
        XCTAssertEqual(sum.fields.map(\.range.location), [6, 8, 12])
        XCTAssertEqual(sum.latexSnippet.caretUTF16, 6)
        XCTAssertEqual(sum.latexSnippet.stops, [8, 12, 15], "`$0` is the last stop")

        let items = try T.Engine().expand("items2").get().snippet.flattened(baseIndent: "  ", indentUnit: "\t")
        XCTAssertEqual(items.text, "\\begin{itemize}\n  \t\\item \n  \t\\item \n  \\end{itemize}")
    }

    // MARK: typed params (§4.4)

    func testTypedParams() {
        func f(_ text: String, _ type: T.ParamType, _ field: String) -> String? {
            (try? T.parseParam(text, as: type).get())?.field(field)
        }
        XCTAssertEqual(f("x", .raw, "value"), "x")
        XCTAssertEqual(f("12", .int, "value"), "12")
        XCTAssertNil(f("1a", .int, "value"))
        XCTAssertEqual(f("i=1..n", .range, "var"), "i")
        XCTAssertEqual(f("i=1..n", .range, "lo"), "1")
        XCTAssertEqual(f("i=1..n", .range, "hi"), "n")
        XCTAssertNil(f("0..\\infty", .range, "var"))
        XCTAssertEqual(f("0..\\infty", .range, "hi"), "\\infty")
        XCTAssertEqual(f("-\\pi..\\pi", .range, "lo"), "-\\pi")
        XCTAssertEqual(f("f(a..b)..c", .range, "lo"), "f(a..b)", "`..` inside brackets is not the separator")
        XCTAssertEqual(f("y/x", .ratio, "num"), "y")
        XCTAssertEqual(f("f/x,y", .ratio, "dens.1"), "y")
        XCTAssertEqual(f("f/x,y", .ratio, "dens.count"), "2")
        XCTAssertNil(f("f/x,", .ratio, "num"), "an empty denominator is an error")
        XCTAssertEqual(f("a,{b,c},d", .list, "items.1"), "{b,c}")
        XCTAssertEqual(f("a,{b,c},d", .list, "items.count"), "3")
        XCTAssertEqual(f("x->0", .arrow, "kind"), "to")
        XCTAssertEqual(f("x|->x^2", .arrow, "kind"), "mapsto")
        XCTAssertEqual(f("x|->x^2", .arrow, "rhs"), "x^2")
        XCTAssertEqual(f("x|x>0", .pred, "rhs"), "x>0")
        XCTAssertEqual(f("3", .shape, "cols"), "3")
        XCTAssertEqual(f("2x4", .shape, "cols"), "4")
        XCTAssertEqual(f("mxn", .shape, "symbolic"), "true")
        XCTAssertNil(f("3x", .shape, "rows"))
        XCTAssertEqual(f("l|c|r", .colspec, "ncols"), "3")
        XCTAssertEqual(f("@{}p{2cm}>{\\bfseries}c@{}", .colspec, "ncols"), "2")
        XCTAssertEqual(f("*{4}{c}|l", .colspec, "ncols"), "5")
        XCTAssertEqual(f("XS", .colspec, "ncols"), "2")
        XCTAssertNil(f("p", .colspec, "ncols"))
        XCTAssertNil(f("|", .colspec, "ncols"))
    }

    func testSlug() {
        XCTAssertEqual(T.slug("The $\\alpha$ case"), "the-alpha-case")
        XCTAssertEqual(T.slug("  Hello, World!  "), "hello-world")
        XCTAssertEqual(T.slug("Épée 2"), "p-e-2")
    }

    // MARK: scope and grid models (M3 / M10 / M10b)

    func testScopeFlags() {
        XCTAssertEqual(T.ScopeStack.text().flags, ["text"])
        XCTAssertEqual(T.ScopeStack.math.flags, ["math", "math:inline"])
        XCTAssertEqual(T.ScopeStack.preamble.flags, ["preamble"])
        XCTAssertEqual(T.ScopeStack.text("frame", "itemize").flags, ["text", "beamer", "list", "env:frame", "env:itemize"])
        XCTAssertEqual(T.ScopeStack.text("align*").flags, ["math", "math:display", "structure", "env:align"])
        XCTAssertEqual(T.ScopeStack.text("equation", "pmatrix").flags, ["math", "math:display", "structure", "env:equation", "env:pmatrix"])
        XCTAssertEqual(T.ScopeStack.text("lstlisting").flags, ["verbatim", "env:lstlisting"])
        XCTAssertEqual(T.ScopeStack.text("figure").flags, ["text", "float", "env:figure"])
        let s = T.ScopeStack([.init(.document), .init(.environment("equation"), start: 10, bodyStart: 26),
                              .init(.environment("bmatrix"), start: 30, bodyStart: 45, bodyEnd: 80)])
        XCTAssertEqual(s.innermost { T.ScopeStack.structureEnvironments.contains($0) }?.bodyStart, 45)
    }

    func testGridRoundTrip() {
        let g = T.Grid.parse("a & b \\\\ c & {d & e} \\\\[2pt] \\hline f & \\& g")
        XCTAssertEqual(g.rows, [.cells(["a", "b"]), .cells(["c", "{d & e}"]), .rule("\\hline"), .cells(["f", "\\& g"])])
        XCTAssertEqual(g.columnCount, 2)
        XCTAssertEqual(g.lines(), ["a & b \\\\", "c & {d & e} \\\\", "\\hline", "f & \\& g"])
        XCTAssertEqual(T.Grid.parse(g.lines().joined(separator: "\n")), g)
        let booktabs = T.Grid.parse("\\toprule A & B \\\\ \\midrule 1 & 2 \\\\ \\bottomrule")
        XCTAssertEqual(booktabs.rows, [.rule("\\toprule"), .cells(["A", "B"]), .rule("\\midrule"), .cells(["1", "2"]), .rule("\\bottomrule")])
        XCTAssertEqual(booktabs.lines().last, "\\bottomrule")
        XCTAssertEqual(booktabs.lines()[3], "1 & 2 \\\\", "a row before a closing rule keeps its \\\\")
    }
}
