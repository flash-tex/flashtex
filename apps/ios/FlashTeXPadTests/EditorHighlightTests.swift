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
    func testKeystrokeOn200KBDocumentStaysUnderBudget() {
        let text = Self.largeDocument()
        XCTAssertGreaterThan(text.utf8.count, 200_000)
        let s = storage(text)
        let ns = s.string as NSString
        var samples: [Double] = []
        for k in 1...20 {
            let at = ns.length * k / 21
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
