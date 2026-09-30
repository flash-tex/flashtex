import XCTest
import FlashTeXEditorCore
@testable import FlashTeXMac

/// APP-PERF-AUDIT (docs/evidence/app-perf-2026-09-30): the per-keystroke
/// editor fixes must not change behaviour. Each fast path is checked against
/// the construction it replaces.
@MainActor
final class AppPerfAuditTests: XCTestCase {
    typealias EN = EditorNavigation

    /// The pairs as they were built before: from every `uses(in:)` result.
    static func referencePairs(in text: NSString) -> [EN.EnvironmentPair] {
        var pairs: [EN.EnvironmentPair] = []
        var open: [String: [Int]] = [:]
        for u in EN.uses(in: text) {
            guard let arg = u.arg, !arg.isEmpty else { continue }
            if u.name == "begin" {
                open[arg, default: []].append(pairs.count)
                pairs.append(EN.EnvironmentPair(name: arg, begin: u.range, end: nil))
            } else if u.name == "end", let i = open[arg]?.popLast() {
                pairs[i].end = u.range
            }
        }
        return pairs
    }

    /// Hand-picked lexical corners: verbatim/comment blocks, `\verb`,
    /// starred names, spaces before the brace, escapes inside arguments,
    /// arguments broken by a newline, arguments that never close, nested
    /// braces, non-ASCII and surrogate-pair names, a CRLF document.
    func testEnvironmentPairsMatchTheUsesConstruction() {
        let cases = [
            "",
            "\\begin{a}x\\end{a}",
            "\\begin {a}x\\end  {a}",
            "\\begin*{a}x\\end{a}\\end*{a}",
            "\\begin{verbatim}\\begin{a}\\end{verbatim}\\end{a}",
            "\\begin{comment}\\begin{a}\\end{comment}\\begin{b}\\end{b}",
            "\\begin{verbatim}never closed \\begin{a}\\end{a}",
            "% \\begin{a}\n\\begin{b}% \\end{b}\n\\end{b}",
            "\\verb|\\begin{a}|\\begin{b}\\verb+x\\end{b}\n\\end{b}",
            "\\verb",
            "\\verb*|\\end{x}|\\begin{x}\\end{x}",
            "\\\\begin{a}\\%\\begin{b}\\end{b}",
            "\\begin{a\nb}\\end{a\nb}\\begin{c}\\end{c}",
            "\\label{x\ny}\\begin{d}\\end{d}",
            "\\section{a {b} \\} c}\\begin{e}\\end{e}",
            "\\newcommand{\\foo}{\\begin{f}}\\end{f}",
            "\\begin{}\\end{}\\begin{g}\\end{g}",
            "\\begin{é}x\\end{é}\\begin{e\u{301}}\\end{é}",
            "\\begin{😀}x\\end{😀}\\begin{a}naïve café — ok\\end{a}",
            "\\begin{a}\r\n\\begin{a}\r\n\\end{a}\r\n\\end{a}\r\n",
            "\\begin{lstlisting}\\end{a}\\end{lstlisting}\\begin{a}\\end{a}",
            "\\begin{a}\\begin{b}\\end{a}\\end{b}",
            "\\end{a}\\begin{a}",
            "\\begin{a}",
            "{\\begin{a}}{\\end{a}",
            "\\x\\begin{y}\\end{y}\\",
        ]
        for c in cases {
            let s = c as NSString
            XCTAssertEqual(EN.environmentPairs(in: s), Self.referencePairs(in: s), "case: \(c.debugDescription)")
        }
    }

    /// Random documents over the characters the lexer treats specially.
    func testEnvironmentPairsMatchTheUsesConstructionOnRandomText() {
        var rng = SystemRandomNumberGenerator()
        let atoms = ["\\begin{a}", "\\end{a}", "\\begin{b}", "\\end{b}", "\\begin {a}", "\\end{verbatim}", "\\begin{verbatim}",
                     "\\begin{comment}", "\\end{comment}", "\\verb|", "\\verb", "|", "%", "\n", "\r\n", "{", "}", "\\", "\\\\",
                     "*", " ", "x", "é", "😀", "\\label{", "\\section{", "begin", "end", "\\begin{", "\\end{"]
        for _ in 0..<3_000 {
            let count = Int.random(in: 0..<40, using: &rng)
            let text = (0..<count).map { _ in atoms.randomElement(using: &rng)! }.joined() as NSString
            XCTAssertEqual(EN.environmentPairs(in: text), Self.referencePairs(in: text), "text: \((text as String).debugDescription)")
        }
    }

    /// `mayClose` is false exactly when `closer(afterTyping:)` must be nil
    /// whatever the text: a letter never pays for the buffer copy, and every
    /// opener that can be closed still reaches `closer`.
    func testAutoCloseMayCloseIsExact() {
        let texts = ["", "x", "\\left", "$x", "\\", "a{", "\\left(", "\\(", "\\[", "% c", "\\verb|"]
        let pairSets: [Set<Character>] = [AutoClose.conventionalPairs, [], ["{"], ["("], ["$", "["]]
        let openers: [Character] = ["a", "Z", "1", " ", "(", "[", "{", "|", ".", "$", ")", "]", "}", "\\", "é", "\n"]
        for pairs in pairSets {
            for opener in openers {
                for t in texts {
                    let text = t + String(opener)
                    let caret = (text as NSString).length
                    let closer = AutoClose.closer(afterTyping: opener, in: text, caretUTF16: caret, mathMode: true, pairs: pairs)
                    if closer != nil {
                        XCTAssertTrue(AutoClose.mayClose(afterTyping: opener, pairs: pairs), "\(opener) in \(t.debugDescription) pairs \(pairs)")
                    }
                }
                if !AutoClose.mayClose(afterTyping: opener, pairs: pairs) {
                    for t in texts {
                        let text = t + String(opener)
                        XCTAssertNil(AutoClose.closer(afterTyping: opener, in: text, caretUTF16: (text as NSString).length, mathMode: true, pairs: pairs))
                    }
                }
            }
        }
        XCTAssertFalse(AutoClose.mayClose(afterTyping: "a"))
        XCTAssertTrue(AutoClose.mayClose(afterTyping: "{"))
        XCTAssertTrue(AutoClose.mayClose(afterTyping: "."), "`\\left.` pairs with `\\right.`")
    }
}
