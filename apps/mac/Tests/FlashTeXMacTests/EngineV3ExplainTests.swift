import Foundation
import XCTest
import FlashTeXProtocol
@testable import FlashTeXMac

/// Lane ERROR-RECOVERY gap 5: TeX's terse messages gain a plain-language
/// explanation, and a fix only where it is mechanical and the row's range
/// is exactly what the fix replaces (the texts below are the corpus's,
/// docs/evidence/error-recovery-2026-10-04, as diag-v1 places them).
final class EngineV3ExplainTests: XCTestCase {
    func row(_ code: String, _ message: String, in text: String, at marked: String, after: String = "") -> RuntimeV1.Diagnostic {
        let from = after.isEmpty ? text.startIndex : text.range(of: after)!.upperBound
        let r = text.range(of: marked, range: from ..< text.endIndex)!
        let start = text.utf8.distance(from: text.startIndex, to: r.lowerBound)
        return .init(severity: .warning, message: message, source: .init(path: "main.tex", startByte: start, endByte: start + marked.utf8.count),
                     recovery: nil, code: code, help: .init(message: "TeX's help."))
    }

    func apply(_ r: RuntimeV1.Diagnostic, _ message: String, _ text: String) -> RuntimeV1.Diagnostic {
        EngineV3Explain.apply(r, code: r.code!, message: message, texts: ["main.tex": text])
    }

    func fixed(_ r: RuntimeV1.Diagnostic, _ text: String) -> String? {
        guard let f = r.help?.replacement else { return nil }
        var b = Array(text.utf8)
        b.replaceSubrange(f.startByte ..< f.endByte, with: Array(f.text.utf8))
        return String(decoding: b, as: UTF8.self)
    }

    func testEveryCorpusCodeIsExplainedAndTeXsHelpKept() {
        for code in ["tex/undefined-control-sequence", "tex/missing-dollar", "tex/too-many-right-braces", "tex/misplaced-alignment-tab",
                     "tex/file-ended-while-scanning", "tex/paragraph-ended-before-argument-complete", "tex/display-math-should-end-with-dollars",
                     "tex/emergency-stop", "tex/fatal-error-no-output", "tex/capacity-exceeded", "latex/environment-mismatch",
                     "latex/environment-undefined", "latex/file-not-found", "latex/lonely-item", "latex/there-no-line-here-to-end"] {
            XCTAssertNotNil(EngineV3Explain.explanation(code: code, message: ""), code)
        }
        XCTAssertNil(EngineV3Explain.explanation(code: "tex/overfull-hbox", message: ""))
        XCTAssertEqual(EngineV3Explain.explanation(code: "tex/file-ended-while-scanning", message: "File ended while scanning use of \\textbf ."),
                       "The argument of \\textbf opens with { but is never closed, so TeX read to the end of the file and stopped. Close the brace.")
        XCTAssertTrue(EngineV3Explain.explanation(code: "tex/file-ended-while-scanning", message: "File ended while scanning use of \\@xdblarg.")!
            .hasPrefix("The argument of a command opens"), "LaTeX's internal names are not shown")
        let text = "Some text } more text.\n"
        let r = apply(row("tex/too-many-right-braces", "Too many }'s.", in: text, at: "}"), "Too many }'s.", text)
        XCTAssertEqual(r.notes?.first, "This } closes no group: there is no { before it that is still open.")
        XCTAssertEqual(r.notes?.last, "TeX's help.", "TeX's help moves to the notes when a fix takes its place")
    }

    func testMechanicalFixes() {
        var text = "Some text } more text.\n"
        XCTAssertEqual(fixed(apply(row("tex/too-many-right-braces", "Too many }'s.", in: text, at: "}"), "Too many }'s.", text), text),
                       "Some text  more text.\n")
        text = "Salt & pepper.\n"
        XCTAssertEqual(fixed(apply(row("tex/misplaced-alignment-tab", "Misplaced alignment tab character &.", in: text, at: "&"),
                                   "Misplaced alignment tab character &.", text), text), "Salt \\& pepper.\n")
        // A mismatched \end, on the line LaTeX names.
        text = "x\n\\begin{itemize}\n\\item One\n\\end{enumerate}\n"
        var m = "LaTeX Error: \\begin{itemize} on input line 2 ended by \\end{enumerate}."
        XCTAssertEqual(fixed(apply(row("latex/environment-mismatch", m, in: text, at: "\\end{enumerate}"), m, text), text),
                       "x\n\\begin{itemize}\n\\item One\n\\end{itemize}\n")
        // A list never closed: \end{itemize} goes before \end{document}.
        text = "\\begin{itemize}\n\\item One\n\\end{document}\n"
        m = "LaTeX Error: \\begin{itemize} on input line 1 ended by \\end{document}."
        XCTAssertEqual(fixed(apply(row("latex/environment-mismatch", m, in: text, at: "\\end{document}"), m, text), text),
                       "\\begin{itemize}\n\\item One\n\\end{itemize}\n\\end{document}\n")
    }

    func testNoFixWhereItWouldGuess() {
        // \begin{foo} unknown: LaTeX closes `document` with \end{foo}; never "fix" that.
        var text = "\\begin{foo}x\\end{foo}\n"
        var m = "LaTeX Error: \\begin{document} ended by \\end{foo}."
        XCTAssertNil(apply(row("latex/environment-mismatch", m, in: text, at: "\\end{foo}"), m, text).help?.replacement)
        // `\[` is amsmath's equation*: the text never says \begin{equation*}.
        text = "\\[ x = y\n\nMore text.\n\\end{document}\n"
        m = "LaTeX Error: \\begin{equation*} on input line 1 ended by \\end{document}."
        XCTAssertNil(apply(row("latex/environment-mismatch", m, in: text, at: "\\end{document}"), m, text).help?.replacement)
        // The range is not the character the fix replaces.
        text = "Some text } more.\n"
        XCTAssertNil(apply(row("tex/too-many-right-braces", "Too many }'s.", in: text, at: "more"), "Too many }'s.", text).help?.replacement)
    }

    /// `\usepackage{amsmth}`: LaTeX reports it after looking ahead into the
    /// next line (TeX's place is the next `\usepackage`); the row moves onto
    /// the name and offers the one close package.
    func testAMistypedPackageIsPlacedOnItsNameAndFixed() {
        let text = "\\documentclass{article}\n\\usepackage{amsmth}\n\\usepackage{graphicx}\n"
        let m = "LaTeX Error: File `amsmth.sty' not found."
        let r = apply(row("latex/file-not-found", m, in: text, at: "\\usepackage", after: "{amsmth}"), m, text)
        let src = try! XCTUnwrap(r.source)
        XCTAssertEqual(text.utf8Slice(src.startByte, src.endByte), "amsmth")
        XCTAssertEqual(r.help?.message, "did you mean amsmath?")
        XCTAssertEqual(fixed(r, text), "\\documentclass{article}\n\\usepackage{amsmath}\n\\usepackage{graphicx}\n")
        // A list entry; and a name nothing is close to (placed, not fixed).
        let list = "\\usepackage{xcolor,amsmth}\n\\begin{document}\n"
        XCTAssertEqual(fixed(apply(row("latex/file-not-found", m, in: list, at: "\\begin"), m, list), list), "\\usepackage{xcolor,amsmath}\n\\begin{document}\n")
        let far = "\\usepackage{qqqqqq}\n\\begin{document}\n"
        let mf = "LaTeX Error: File `qqqqqq.sty' not found."
        let rf = apply(row("latex/file-not-found", mf, in: far, at: "\\begin"), mf, far)
        XCTAssertEqual(rf.source.flatMap { far.utf8Slice($0.startByte, $0.endByte) }, "qqqqqq")
        XCTAssertNil(rf.help?.replacement)
    }
}
