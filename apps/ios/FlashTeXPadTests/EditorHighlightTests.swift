import FlashTeXEditorCore
import UIKit
import XCTest
@testable import FlashTeXPad

/// Syntax colouring on the iPad: the storage's colours follow the shared
/// `SyntaxHighlighter` token model, an edit recolours only the lines it
/// touched (and the result equals a fresh full lex — the incremental
/// invariant), and a keystroke on a 200 KB document stays under the 4 ms
/// budget on the main actor.
@MainActor
final class EditorHighlightTests: XCTestCase {
    let theme = EditorTheme.standard

    private func storage(_ text: String) -> EditorTextStorage {
        let s = EditorTextStorage(theme: theme)
        s.replaceCharacters(in: NSRange(location: 0, length: 0), with: text)
        return s
    }

    private func color(_ s: EditorTextStorage, _ at: Int) -> UIColor? {
        s.foregroundColor(at: at)
    }

    func testTokenKindsGetTheirColours() {
        let text = "\\section{Intro} % note\n$x^2$ \\[ \\alpha \\] \\begin{verbatim}\\foo\\end{verbatim} \\ref{eq:1}"
        let s = storage(text)
        let ns = text as NSString
        func at(_ needle: String) -> Int { ns.range(of: needle).location }
        XCTAssertEqual(color(s, at("\\section")), theme.colors[.command])
        XCTAssertEqual(color(s, at("{Intro}")), theme.colors[.brace])
        XCTAssertEqual(color(s, at("% note")), theme.colors[.comment])
        XCTAssertEqual(color(s, at("$x")), theme.colors[.mathDelimiter])
        XCTAssertEqual(color(s, at("x^2") + 1), theme.colors[.math])
        XCTAssertEqual(color(s, at("\\alpha")), theme.colors[.mathCommand])
        XCTAssertEqual(color(s, at("verbatim}")), theme.colors[.environment])
        XCTAssertEqual(color(s, at("\\foo")), theme.text, "verbatim bodies stay plain")
        XCTAssertEqual(color(s, at("eq:1")), theme.colors[.reference])
        XCTAssertEqual(color(s, at("Intro")), theme.text, "plain text keeps the text colour")
    }

    /// Every run the storage coloured matches the model's runs for the same text.
    private func assertMatchesFullLex(_ s: EditorTextStorage, file: StaticString = #filePath, line: UInt = #line) {
        let text = s.string as NSString
        let expected = SyntaxHighlighter.runs(of: text)
        var byPosition: [Int: SyntaxHighlighter.Kind] = [:]
        for run in expected { for i in run.range.location..<NSMaxRange(run.range) { byPosition[i] = run.kind } }
        for i in 0..<text.length {
            let want = byPosition[i].flatMap(theme.color(for:)) ?? theme.text
            let got = color(s, i) ?? theme.text
            if want != got {
                XCTFail("unit \(i) (\(text.substring(with: NSRange(location: i, length: 1)))): expected \(byPosition[i].map { "\($0)" } ?? "text")", file: file, line: line)
                return
            }
        }
        XCTAssertTrue(s.isInSync, file: file, line: line)
    }

    func testIncrementalEditsEqualAFreshLex() {
        let s = storage("\\begin{document}\nHello $a+b$ world\n\\begin{align}\nx &= 1\n\\end{align}\nbye % c\n\\end{document}\n")
        assertMatchesFullLex(s)
        let ns = s.string as NSString
        // Open math on line 2 without closing: the lines after must recolour.
        s.replaceCharacters(in: NSRange(location: ns.range(of: "$a").location, length: 0), with: "$")
        assertMatchesFullLex(s)
        // Type a comment marker that swallows the rest of its line.
        s.replaceCharacters(in: NSRange(location: (s.string as NSString).range(of: "world").location, length: 0), with: "%")
        assertMatchesFullLex(s)
        // Delete `\begin{align}` (the environment's math mode ends).
        let begin = (s.string as NSString).range(of: "\\begin{align}\n")
        s.replaceCharacters(in: begin, with: "")
        assertMatchesFullLex(s)
        // Paste a verbatim block across lines.
        s.replaceCharacters(in: NSRange(location: 0, length: 0), with: "\\begin{verbatim}\n\\x $ {\n\\end{verbatim}\n")
        assertMatchesFullLex(s)
        // Undo-like whole replacement.
        s.replaceCharacters(in: NSRange(location: 0, length: s.length), with: "plain\n")
        assertMatchesFullLex(s)
    }

    func testAKeystrokeRecoloursOnlyItsLine() {
        let s = storage(String(repeating: "\\textbf{x} $y$ % c\n", count: 200))
        let line = 100
        let at = (s.string as NSString).range(of: "\\textbf", options: [], range: NSRange(location: line * 19, length: 40)).location + 3
        s.replaceCharacters(in: NSRange(location: at, length: 0), with: "z")
        XCTAssertEqual(s.lastLinesLexed, 1)
        XCTAssertEqual(s.lastHighlightedRange, NSRange(location: line * 19, length: 20))
        assertMatchesFullLex(s)
    }

    static func largeDocument(targetBytes: Int = 200_000) -> String {
        var out = "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n"
        var i = 0
        while out.utf8.count < targetBytes {
            out += "\\section{Section \(i)} % heading \(i)\nText with $x_\(i)^2 + \\alpha$ and \\textbf{bold} \\ref{sec:\(i)}.\n"
            out += "\\begin{itemize}\n    \\item one \\cite{k\(i)}\n    \\item two\n\\end{itemize}\n"
            out += "\\begin{align}\n    a &= b_\(i) \\\\\n    c &= \\frac{1}{2}\n\\end{align}\n\n"
            i += 1
        }
        return out + "\\end{document}\n"
    }

    /// A keystroke in the middle of a 200 KB document: the storage's work
    /// (line table update, re-lex of the touched lines, recolouring) must
    /// stay under 4 ms on the main actor. Measured without a layout manager
    /// so the number is the highlighter's, not TextKit's.
    ///
    /// Sampling (issue #1015): the 20 keystrokes land just after "Text " on
    /// evenly spread body lines — ordinary typing in plain-text context. This
    /// is deliberate, not cherry-picking: evenly spaced fractional offsets
    /// (`length * k / 51`) deterministically land inside `\begin`/`\end{align}`
    /// tokens 6 times out of 50, and destroying a math-environment boundary
    /// made `SyntaxHighlighter.edit` re-lex the whole tail of the document
    /// (3,000–8,000 lines here; 100–700 ms on an iPad simulator, median still
    /// ~0.8 ms — the bimodal signature of the two failed real runs). Since
    /// #1017 that re-lex stops at the paragraph's blank line; it has its own
    /// test below. This budget test gates the steady-state keystroke path,
    /// where one insertion provably re-lexes exactly one line.
    func testKeystrokeOn200KBDocumentStaysUnderBudget() {
        let text = Self.largeDocument()
        XCTAssertGreaterThan(text.utf8.count, 200_000)
        let s = storage(text)
        let ns = s.string as NSString
        // Anchor each keystroke in plain-text context, spread over the doc.
        var anchors: [Int] = []
        var from = 0
        while from < ns.length {
            let r = ns.range(of: "Text with ", options: [], range: NSRange(location: from, length: ns.length - from))
            guard r.location != NSNotFound else { break }
            anchors.append(r.location + 5) // just after "Text "
            from = r.location + 1
        }
        XCTAssertGreaterThanOrEqual(anchors.count, 20, "largeDocument must contain body lines to sample")
        var samples: [Double] = []
        for j in 0..<20 {
            let at = anchors[j * anchors.count / 20] + j // +j: each earlier insert shifted the text by one
            let start = DispatchTime.now().uptimeNanoseconds
            s.replaceCharacters(in: NSRange(location: at, length: 0), with: "x")
            let end = DispatchTime.now().uptimeNanoseconds
            samples.append(Double(end - start) / 1_000_000)
        }
        samples.sort()
        let median = samples[samples.count / 2]
        print("editor.highlight.keystroke.200KB: median \(median) ms, max \(samples.last!) ms, lines lexed last \(s.lastLinesLexed)")
        XCTAssertLessThan(median, 4, "median keystroke highlight cost (ms) on a 200 KB document")
        XCTAssertLessThan(samples[samples.count * 9 / 10], 4, "p90 keystroke highlight cost (ms)")
    }

    /// Issue #1017 (was the #1015 worst case): a keystroke that destroys a
    /// math-environment boundary (here, typing inside the first `\end{align}`)
    /// used to shift every later line-start mode, so the incremental re-lex
    /// walked the whole tail (~9,400 of 9,443 lines, 100-700 ms simulator
    /// stalls). No math mode crosses a blank line now, so the damage ends at
    /// the paragraph and the re-lex converges right after it, and the result
    /// still equals a fresh full lex.
    func testDestroyingMathBoundaryRelexIsBoundedByTheParagraph() {
        let s = storage(Self.largeDocument())
        let ns = s.string as NSString
        let end = ns.range(of: "\\end{align}")
        guard end.location != NSNotFound else { return XCTFail("largeDocument must contain \\end{align}") }
        let start = DispatchTime.now().uptimeNanoseconds
        s.replaceCharacters(in: NSRange(location: end.location + 2, length: 0), with: "x")
        let ms = Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000
        print("editor.highlight.boundary-destruction.200KB: lines lexed \(s.lastLinesLexed), \(ms) ms")
        XCTAssertLessThanOrEqual(s.lastLinesLexed, 4, "destroying \\end{align} re-lexes the paragraph, not the document tail")
        XCTAssertLessThan(s.lastHighlightedRange.length, 200)
        assertMatchesFullLex(s)
    }

    func testControllerKeystrokeOnLargeDocumentReportsWithinBudget() {
        let editor = EditorController()
        editor.load(text: Self.largeDocument(), caret: 0, revision: 1)
        editor.select(NSRange(location: editor.length / 2, length: 0))
        var samples: [Double] = []
        for _ in 0..<10 {
            let start = DispatchTime.now().uptimeNanoseconds
            editor.type("x")
            samples.append(Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000)
        }
        samples.sort()
        print("editor.controller.keystroke.200KB (storage + text view, no window): median \(samples[samples.count / 2]) ms, max \(samples.last!) ms")
        XCTAssertLessThan(samples[samples.count / 2], 40, "the whole keystroke path, including UITextView bookkeeping, stays interactive")
    }

    func testThemeHasLightAndDarkColours() {
        let light = UITraitCollection(userInterfaceStyle: .light)
        let dark = UITraitCollection(userInterfaceStyle: .dark)
        let command = theme.colors[.command]!
        XCTAssertNotEqual(command.resolvedColor(with: light), command.resolvedColor(with: dark))
        for kind in SyntaxHighlighter.Kind.allCases where kind != .verbatim {
            XCTAssertNotNil(theme.color(for: kind), "\(kind) has no colour")
        }
        XCTAssertNil(theme.color(for: .verbatim))
    }
}
