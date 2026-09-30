import XCTest
@testable import FlashTeXEditorCore

/// PLAN M4 (instant atoms) and M5 (ligatures, postfix, fractions) through
/// the capture controller, with the simulated editor of `CaptureTests`.
final class InlineTests: XCTestCase {
    typealias T = TeXpand
    typealias Editor = CaptureTests.Editor

    static func settings(_ edit: (inout T.Settings) -> Void = { _ in }) -> T.Settings {
        var s = T.Settings()
        s.enabled = true
        s.ligatures = true
        edit(&s)
        return s
    }

    /// An editor inside inline math: `$` + … with the caret before the closing `$`.
    func math(_ settings: T.Settings = InlineTests.settings(), before: String = "") -> Editor {
        let e = Editor("$" + before + "$", settings: settings)
        e.caret = e.text.length - 1
        return e
    }

    // MARK: instant atoms (§9.2)

    func testInstantAtomsCommitWithoutTab() {
        let e = math()
        e.type(";a^2")
        XCTAssertEqual(e.string, "$\\alpha^2$", "`;a^2` is `\\alpha^2` without Tab")
        XCTAssertEqual(e.caret, 9)
        let f = math()
        f.type(";a+;b ")
        XCTAssertEqual(f.string, "$\\alpha+\\beta $", "`;a+;b` is `\\alpha+\\beta`")
        let g = math()
        g.type(";th_1")
        XCTAssertEqual(g.string, "$\\theta_1$")
        let h = math()
        h.type(";inf)")
        XCTAssertEqual(h.string, "$\\infty)$")
    }

    func testInstantAtomWaitsWhileALetterFollows() {
        let e = math()
        e.type(";p")
        XCTAssertEqual(e.state, .capturing(leader: 1, end: 3))
        e.type("mat2")
        XCTAssertEqual(e.string, "$;pmat2$", "`;p` then letters heads for `pmat`: no commit")
        e.tab()
        XCTAssertTrue(e.string.hasPrefix("$\\begin{pmatrix}"))
        let t = math()
        t.type(";a")
        t.tab()
        XCTAssertEqual(t.string, "$\\alpha$", "Tab commits an instant atom too")
    }

    func testInstantAtomsOnlyInMathAndOnlyWhenOn() {
        let text = Editor("Text ", settings: InlineTests.settings())
        text.type(";a ")
        XCTAssertEqual(text.string, "Text ;a ", "Greek atoms are math-scoped")
        let off = math(InlineTests.settings { $0.instantAtoms = false })
        off.type(";a^")
        XCTAssertEqual(off.string, "$;a^$", "instant atoms switched off")
    }

    func testUndoingAnInstantAtomMarksIt() {
        let e = math()
        e.type(";a^")
        XCTAssertEqual(e.string, "$\\alpha^$")
        e.undo()
        XCTAssertEqual(e.string, "$;a^$", "one undo restores the literal")
        e.move(to: 3)
        e.tab()
        XCTAssertEqual(e.string, "$;a\t^$", "the undone atom is marked")
    }

    // MARK: ligatures (§9.3)

    func testLigaturesFireInMath() {
        let e = math()
        e.type("a->b")
        XCTAssertEqual(e.string, "$a\\to b$")
        let f = math()
        f.type("x!=y")
        XCTAssertEqual(f.string, "$x\\neq y$")
        let g = math()
        g.type("1...n")
        XCTAssertEqual(g.string, "$1\\dots n$")
    }

    func testDeferredCommitResolvesLeqVersusIff() {
        let e = math()
        e.type("a <=")
        XCTAssertEqual(e.string, "$a <=$", "`<=` waits: it may become `<=>`")
        e.type(">")
        XCTAssertEqual(e.string, "$a \\iff $", "`<=>` wins")
        let f = math()
        f.type("a <= b")
        XCTAssertEqual(f.string, "$a \\leq  b$", "the next character settles `<=`, then is typed normally")
        let g = math()
        g.type("|-")
        XCTAssertEqual(g.string, "$|-$")
        g.type(">")
        XCTAssertEqual(g.string, "$\\mapsto $", "`|->` beats `|-`")
        let h = math()
        h.type("a <=")
        h.tab()
        XCTAssertEqual(h.string, "$a \\leq $", "Tab settles a waiting ligature")
        let i = math()
        i.type("a <=")
        _ = i.esc()
        i.type("b")
        XCTAssertEqual(i.string, "$a <=b$", "Esc leaves it literal")
    }

    func testGuardsAndRegexLigatures() {
        let e = math()
        e.type("a xx b")
        XCTAssertEqual(e.string, "$a \\times  b$")
        let max = math(before: "\\ma")
        max.caret = 4
        max.type("xx")
        XCTAssertEqual(max.string, "$\\maxx$", "the guard: not inside a control word")
        let sub = math()
        sub.type("x1+x12")
        XCTAssertEqual(sub.string, "$x_1+x_{12}$", "digit subscripts")
        let alpha = math(before: "\\alpha")
        alpha.caret = 7
        alpha.type("1")
        XCTAssertEqual(alpha.string, "$\\alpha1$", "no subscript after a control word")
    }

    func testLigaturesStayOutOfTextVerbatimAndCaptures() {
        let text = Editor("Text ", settings: InlineTests.settings())
        text.type("a->b")
        XCTAssertEqual(text.string, "Text a->b", "math only by default")
        let verb = Editor("\\begin{verbatim}\n", settings: InlineTests.settings())
        verb.type("->")
        XCTAssertEqual(verb.string, "\\begin{verbatim}\n->")
        let capture = math()
        capture.type(";lim:x->0")
        XCTAssertEqual(capture.string, "$;lim:x->0$", "not inside a capture region")
        capture.tab()
        XCTAssertEqual(capture.string, "$\\lim_{x \\to 0} $")
        let off = math(InlineTests.settings { $0.ligatures = false })
        off.type("a->b")
        XCTAssertEqual(off.string, "$a->b$", "ligatures off")
        XCTAssertFalse(T.Settings().ligatures, "and off by default")
    }

    func testUndoingALigatureRestoresTheTrigger() {
        let e = math()
        e.type("a->")
        XCTAssertEqual(e.string, "$a\\to $")
        e.undo()
        XCTAssertEqual(e.string, "$a->$")
        e.type("b")
        XCTAssertEqual(e.string, "$a->b$", "no re-firing")
    }

    func testTrailingSpaceFollowsTheProfile() {
        let project = T.Layer(name: "project", source: "[profile]\nligature_trailing_space = \"false\"\n")
        let e = Editor("$$", settings: InlineTests.settings())
        e.controller.engine = T.Engine(settings: InlineTests.settings(), layers: [project])
        e.caret = 1
        e.type("a->b")
        XCTAssertEqual(e.string, "$a\\tob$", "no trailing space: the profile's choice, even when it runs together")
    }

    // MARK: postfix (§9.4)

    func testPostfixModifiers() {
        let e = math()
        e.type("\\alpha_i.hat")
        e.tab()
        XCTAssertEqual(e.string, "$\\hat{\\alpha_i}$", "`\\alpha_i.hat` → `\\hat{\\alpha_i}`")
        let p = math()
        p.type("(x+1).sqrt")
        p.tab()
        XCTAssertEqual(p.string, "$\\sqrt{x+1}$", "strip_parens")
        let t = math()
        t.type("(AB).T")
        t.tab()
        XCTAssertEqual(t.string, "$(AB)^\\top$", "the transpose keeps its parentheses")
        let bb = math()
        bb.type("x+R.bb")
        let out = bb.tab()
        XCTAssertEqual(bb.string, "$x+\\mathbb{R}$", "one letter is the atom")
        XCTAssertEqual(out.commit?.requires.map(\.name), ["amssymb"])
        let ub = math()
        ub.type("{a+b}.ub")
        ub.tab()
        XCTAssertEqual(ub.string, "$\\underbrace{a+b}_{}$")
        XCTAssertEqual(ub.caret, 19, "the caret in the underbrace's label")
        let frac = math()
        frac.type("\\frac{a}{b}.inv")
        frac.tab()
        XCTAssertEqual(frac.string, "$\\frac{a}{b}^{-1}$", "a command with its groups is one atom")
        let lr = math()
        lr.type("\\left(a\\right)^2.bar")
        lr.tab()
        XCTAssertEqual(lr.string, "$\\bar{\\left(a\\right)^2}$", "a \\left…\\right pair with a script")
    }

    func testPostfixNeverOnNumbersOrOutsideMath() {
        let e = math()
        e.type("3.14")
        e.tab()
        XCTAssertEqual(e.string, "$3.14\t$", "`3.14` never triggers postfix")
        let unknown = math()
        unknown.type("x.foo")
        unknown.tab()
        XCTAssertEqual(unknown.string, "$x.foo\t$")
        let text = Editor("Text x.hat", settings: InlineTests.settings())
        text.tab()
        XCTAssertEqual(text.string, "Text x.hat\t")
        let off = math(InlineTests.settings { $0.postfix = false })
        off.type("x.hat")
        off.tab()
        XCTAssertEqual(off.string, "$x.hat\t$")
    }

    // MARK: fractions (owner decision: `//`)

    func testFractionOperator() {
        let e = math()
        e.type("(x+1)//2")
        e.tab()
        XCTAssertEqual(e.string, "$\\frac{x+1}{2}$", "`(x+1)//2` Tab → `\\frac{x+1}{2}`")
        let open = math()
        open.type("a//")
        open.tab()
        XCTAssertEqual(open.string, "$\\frac{a}{}$", "`a//` gets a tabstop denominator")
        XCTAssertEqual(open.caret, 10)
        let single = math()
        single.type("a/b")
        single.tab()
        XCTAssertEqual(single.string, "$a/b\t$", "a single `/` never expands")
        let text = Editor("see a//b", settings: InlineTests.settings())
        text.tab()
        XCTAssertEqual(text.string, "see a//b\t", "`//` in text does not trigger")
        let comment = Editor("$x$ % a//b", settings: InlineTests.settings())
        comment.tab()
        XCTAssertEqual(comment.string, "$x$ % a//b\t", "nor in a comment")
    }

    func testFractionSettings() {
        let slash = math(InlineTests.settings { $0.fractionOperator = "/" })
        slash.type("a/b")
        slash.tab()
        XCTAssertEqual(slash.string, "$\\frac{a}{b}$", "`/` restores the single-slash form")
        let off = math(InlineTests.settings { $0.fractionTrigger = .off })
        off.type("a//b")
        off.tab()
        XCTAssertEqual(off.string, "$a//b\t$")
        let auto = math(InlineTests.settings { $0.fractionTrigger = .auto })
        auto.type("a/")
        XCTAssertEqual(auto.string, "$a/$", "one slash: nothing")
        auto.type("/")
        XCTAssertEqual(auto.string, "$\\frac{a}{}$", "auto: the second slash expands")
        auto.type("b")
        XCTAssertEqual(auto.string, "$\\frac{a}{b}$", "the caret is in the denominator")
        auto.undo(); auto.undo()
        XCTAssertEqual(auto.string, "$a//$", "undo-to-literal gives a literal `//`")
        let dfrac = T.Layer(name: "user", source: "[profile]\nfrac = '\\dfrac'\n")
        let d = math()
        d.controller.engine = T.Engine(settings: InlineTests.settings(), layers: [dfrac])
        d.type("x//y")
        d.tab()
        XCTAssertEqual(d.string, "$\\dfrac{x}{y}$", "profile.frac")
    }

    // MARK: the atom parser

    func testAtomParser() {
        func atom(_ s: String) -> String? {
            let ns = s as NSString
            return T.atomStart(in: ns, end: ns.length).map { ns.substring(from: $0) }
        }
        XCTAssertEqual(atom("x"), "x")
        XCTAssertEqual(atom("ab"), "b")
        XCTAssertEqual(atom("a+123"), "123")
        XCTAssertEqual(atom("\\alpha_i"), "\\alpha_i")
        XCTAssertEqual(atom("x^{2}_1"), "x^{2}_1")
        XCTAssertEqual(atom("a+(b+c)"), "(b+c)")
        XCTAssertEqual(atom("\\sqrt[3]{x}"), "\\sqrt[3]{x}")
        XCTAssertEqual(atom("\\mathbf{v}_i^2"), "\\mathbf{v}_i^2")
        XCTAssertEqual(atom("1+\\left[ a \\right]"), "\\left[ a \\right]")
        XCTAssertEqual(atom("f\\left\\langle a \\right\\rangle"), "\\left\\langle a \\right\\rangle")
        XCTAssertNil(atom("a+"))
        XCTAssertNil(atom("a)"))
        XCTAssertNil(atom(""))
        XCTAssertEqual(T.stripParens("(a)+(b)"), "(a)+(b)", "only a matching outer pair is stripped")
        XCTAssertEqual(T.stripParens("((a))"), "(a)")
    }
}
