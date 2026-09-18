import AppKit
import HostedWindows
import XCTest
@testable import FlashTeXMac

/// Math-mode ranking in the completion list: inside `$…$` the compiler's math
/// commands come before the text ones, which the vocabulary's table order
/// otherwise buries. The mode itself is decided by `SyntaxHighlighter`'s lexer
/// (`Completion.isMathMode`), so there is no second set of delimiter rules.
final class CompletionMathModeTests: XCTestCase {
    private func labels(_ s: [Completion.Suggestion]) -> [String] { s.map(\.insertText) }

    // MARK: where the caret is

    func testMathModeAtTheCaret() {
        func math(_ text: String, _ caret: Int) -> Bool { Completion.isMathMode(in: text as NSString, caretUTF16: caret) }
        // `$…$`: inside yes, on either side no. The mode is read *at* the
        // caret, not at the start of its line.
        XCTAssertFalse(math("a $x$ b", 2))
        XCTAssertTrue(math("a $x$ b", 3))
        XCTAssertTrue(math("a $x$ b", 4), "the caret before the closing delimiter is still inside")
        XCTAssertFalse(math("a $x$ b", 6))
        // The other three delimiter pairs.
        XCTAssertTrue(math("\\(x\\)", 2))
        XCTAssertFalse(math("\\(x\\) y", 6))
        XCTAssertTrue(math("\\[x\\]", 2))
        XCTAssertTrue(math("$$x$$", 3))
        // Math environments, including a nested one.
        XCTAssertTrue(math("\\begin{align}\n x\n\\end{align}\n", 15))
        XCTAssertTrue(math("\\begin{align}\n\\begin{cases}\n x\n\\end{cases}\n\\end{align}\n", 29))
        XCTAssertFalse(math("\\begin{align}\n x\n\\end{align}\ny", 30))
        XCTAssertFalse(math("\\begin{itemize}\n\\item x\n\\end{itemize}\n", 22))
        // Verbatim wins: a `$` in a listing is a dollar sign.
        XCTAssertFalse(math("\\begin{verbatim}\n$x$\n\\end{verbatim}\n", 19))
        // Degenerate input never throws or guesses.
        XCTAssertFalse(math("", 0))
        XCTAssertFalse(math("$x$", -5))
        XCTAssertFalse(math("$x$", 999))
    }

    func testAnOutOfSyncHighlighterIsRefusedRatherThanTrusted() {
        var stale = SyntaxHighlighter()
        stale.reset("plain text with no maths" as NSString)
        // The model describes different text: the answer is no, not a guess
        // from a line table that does not match.
        XCTAssertFalse(Completion.isMathMode(in: "$\\alpha$" as NSString, caretUTF16: 3, highlighter: stale))
        // With no model at all the text is lexed whole and the answer is right.
        XCTAssertTrue(Completion.isMathMode(in: "$\\alpha$" as NSString, caretUTF16: 3))
    }

    // MARK: the ranking rule

    /// `section` and `subsection` are text commands; `sum`, `sigma` and `sqrt`
    /// are math ones. Injected through `supported:` so the rule is tested
    /// independently of the compiler's growing inventory.
    private let vocabulary = ["section", "sum", "sigma", "subsection", "sqrt"]

    func testMathModeFloatsMathCommandsAboveTextOnes() {
        let text = "x \\s"
        let caret = (text as NSString).length
        let plain = Completion.suggestions(in: text, caretUTF16: caret, metadata: nil, supported: vocabulary)
        XCTAssertEqual(labels(plain), ["\\section", "\\sum", "\\sigma", "\\subsection", "\\sqrt"],
                       "an unknown mode keeps table order and hides nothing")
        // A known mode is a filter (mac/intellisense-gaps-2): inside math the
        // text-only commands are gone, in text the math-only ones are; what
        // survives keeps the table order (math first only matters when a
        // both-modes command shares the list).
        let math = Completion.suggestions(in: text, caretUTF16: caret, metadata: nil, supported: vocabulary, mathMode: true)
        XCTAssertEqual(labels(math), ["\\sum", "\\sigma", "\\sqrt"], "math mode: \\section and \\subsection are text-only")
        let prose = Completion.suggestions(in: text, caretUTF16: caret, metadata: nil, supported: vocabulary, mathMode: false)
        XCTAssertEqual(labels(prose), ["\\section", "\\subsection"], "text mode: the math-only commands are hidden")
        // A both-modes command (`\textbf` has a math description) ranks after
        // the math ones inside math and stays in text.
        let both = ["textbf", "sum", "sigma", "section"]
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\", caretUTF16: 3, metadata: nil, supported: both, mathMode: true)),
                       ["\\sum", "\\sigma", "\\textbf"])
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\", caretUTF16: 3, metadata: nil, supported: both, mathMode: false)),
                       ["\\textbf", "\\section"])
    }

    func testAFilterThatWouldEmptyTheListFallsBackToWhatItHid() {
        // `\ite` inside `$…$`: only text-only commands start with it, so the
        // author still sees them rather than nothing.
        let math = Completion.suggestions(in: "x \\s", caretUTF16: 4, metadata: nil, supported: ["section", "subsection"], mathMode: true)
        XCTAssertEqual(labels(math), ["\\section", "\\subsection"])
        let text = Completion.suggestions(in: "x \\s", caretUTF16: 4, metadata: nil, supported: ["sum", "sqrt"], mathMode: false)
        XCTAssertEqual(labels(text), ["\\sum", "\\sqrt"])
        // The fuzzy fallback filters the same way, with the same escape hatch:
        // `sbs` is a subsequence of \subsection (text) and \substack (math).
        let fuzzy = ["subsection", "substack"]
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\sbs", caretUTF16: 6, metadata: nil, supported: fuzzy, mathMode: true)), ["\\substack"])
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\sbs", caretUTF16: 6, metadata: nil, supported: fuzzy, mathMode: false)), ["\\subsection"])
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\sbs", caretUTF16: 6, metadata: nil, supported: ["subsection"], mathMode: true)), ["\\subsection"])
        // What stays: the label/reference family and `\\` are text-mode in
        // the inventory but belong inside equation/align.
        for name in Completion.mathAllowedTextCommands {
            guard let entry = Completion.Vocabulary.byName[name] else { XCTFail("\\\(name) is not in the inventory"); continue }
            XCTAssertEqual(entry.mode, .text, name)
            XCTAssertTrue(Completion.allows(entry, mathMode: true), name)
        }
        XCTAssertEqual(Completion.suggestions(in: "$\\lab", caretUTF16: 5, metadata: nil, mathMode: true).map(\.insertText), ["\\label"])
        // Names outside the inventory (a project's `supported` list) have no mode and are never hidden.
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\my", caretUTF16: 4, metadata: nil, supported: ["mymacro"], mathMode: true)), ["\\mymacro"])
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\my", caretUTF16: 4, metadata: nil, supported: ["mymacro"], mathMode: false)), ["\\mymacro"])
    }

    func testTheRealVocabularyIsFilteredByMode() throws {
        // Inside `$…$`: \frac is offered, \section is not; in text the reverse.
        let mathFra = Completion.suggestions(in: "$\\fra", caretUTF16: 5, metadata: nil, mathMode: true)
        XCTAssertEqual(mathFra.first?.label, "\\frac{num}{den}")
        XCTAssertTrue(mathFra.allSatisfy { $0.detail.hasPrefix("math · ") }, "\(labels(mathFra))")
        let mathSec = labels(Completion.suggestions(in: "$\\sec", caretUTF16: 5, metadata: nil, mathMode: true))
        XCTAssertTrue(mathSec.contains("\\sec"), "\(mathSec)")
        XCTAssertFalse(mathSec.contains("\\section"), "\(mathSec)")
        let textSec = labels(Completion.suggestions(in: "x \\sec", caretUTF16: 6, metadata: nil, mathMode: false))
        XCTAssertTrue(textSec.contains("\\section"), "\(textSec)")
        XCTAssertFalse(textSec.contains("\\sec"), "\(textSec)")
        // Text mode, `\frac` spelled out: only the math command matches, so the fallback still shows it.
        XCTAssertEqual(labels(Completion.suggestions(in: "x \\frac", caretUTF16: 7, metadata: nil, mathMode: false)), ["\\frac"])
        // Every row the real vocabulary offers in a known mode passes the filter, unless the fallback fired.
        for prefix in ["a", "b", "s", "t", "m"] {
            for mode in [true, false] {
                let rows = Completion.suggestions(in: "x \\" + prefix, caretUTF16: 3 + prefix.count, metadata: nil, mathMode: mode)
                let entries = rows.compactMap { Completion.Vocabulary.byName[String($0.insertText.dropFirst())] }
                XCTAssertEqual(entries.count, rows.count, "\\\(prefix) in \(mode ? "math" : "text")")
                XCTAssertTrue(entries.allSatisfy { Completion.allows($0, mathMode: mode) }, "\\\(prefix) in \(mode ? "math" : "text"): \(rows.map(\.label))")
            }
        }
    }

    func testTheExactlyTypedSpellingStillOutranksEverything() {
        // `\sec` is a math operator, `\section` a text command: in text mode
        // the exact spelling must not be displaced, and in math mode the math
        // commands must not displace it either.
        let exactText = Completion.suggestions(in: "x \\section", caretUTF16: 10, metadata: nil, supported: vocabulary)
        XCTAssertEqual(labels(exactText).first, "\\section")
        let exactMath = Completion.suggestions(in: "$\\section", caretUTF16: 9, metadata: nil, supported: vocabulary, mathMode: true)
        XCTAssertEqual(labels(exactMath).first, "\\section", "the exact spelling stays first even in math mode")
        // And a math command typed exactly is first for the same reason.
        XCTAssertEqual(labels(Completion.suggestions(in: "$\\sum", caretUTF16: 5, metadata: nil,
                                                     supported: vocabulary, mathMode: true)).first, "\\sum")
    }

    func testProjectDeclarationsOutrankMathSymbols() throws {
        // The user's own `\sigmoid` is more relevant than `\sigma`, in math
        // mode as anywhere else.
        let metadata = Completion.Metadata(origin: .compileResult(projectId: "p"), revision: 3, commands: [
            .init(name: "sigmoid", definitions: 1, occurrences: 2, locationsTruncated: false, definedIn: "main.tex"),
        ])
        let math = Completion.suggestions(in: "$\\s", caretUTF16: 3, metadata: metadata,
                                          supported: vocabulary + ["sigmoid"], mathMode: true)
        XCTAssertEqual(labels(math).first, "\\sigmoid")
        XCTAssertTrue(try XCTUnwrap(math.first?.detail).contains("declared in main.tex"))
        XCTAssertEqual(labels(math).dropFirst().prefix(3).map { $0 }, ["\\sum", "\\sigma", "\\sqrt"])
    }

    // MARK: against the real vocabulary

    func testTheRealVocabularyReordersInsideMath() throws {
        let text = "$x \\se"
        let caret = (text as NSString).length
        let plain = Completion.suggestions(in: text, caretUTF16: caret, metadata: nil)
        let math = Completion.suggestions(in: text, caretUTF16: caret, metadata: nil, mathMode: true)
        XCTAssertFalse(plain.isEmpty)
        XCTAssertFalse(try XCTUnwrap(plain.first?.detail).hasPrefix("math · "), "text mode leads with a text command")
        XCTAssertTrue(try XCTUnwrap(math.first?.detail).hasPrefix("math · "), "math mode leads with a math command")
        // `\setminus` and `\sec` are the math `se` commands; in text mode they
        // are at the very back of the table.
        XCTAssertTrue(math.prefix(2).allSatisfy { $0.detail.hasPrefix("math · ") })
    }

    func testWordAndArgumentCompletionsAreUnaffected() {
        // Only the command list is ranked; a `\ref{` key list or a prose word
        // has no mode to speak of and must come back identical.
        let refs = "\\label{eq:one}\n$\\ref{"
        XCTAssertEqual(Completion.suggestions(in: refs, caretUTF16: (refs as NSString).length, metadata: nil, mathMode: true).map(\.label),
                       Completion.suggestions(in: refs, caretUTF16: (refs as NSString).length, metadata: nil).map(\.label))
        let env = "$\\begin{ali"
        XCTAssertEqual(Completion.suggestions(in: env, caretUTF16: (env as NSString).length, metadata: nil, mathMode: true).map(\.label),
                       Completion.suggestions(in: env, caretUTF16: (env as NSString).length, metadata: nil).map(\.label))
    }

    // MARK: the editor wires it up

    @MainActor
    func testTheEditorAnswersMathModeFromItsOwnSyntaxModel() throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        defer { window.orderOut(nil) }
        // Unwired (a bare text view): unknown, so the list keeps its plain order and hides nothing.
        XCTAssertNil(tv.mathModeAtCaret(0))
        // Wired the way SourceEditorView wires it.
        tv.string = "text $x + y$ text"
        tv.mathModeAtCaret = { [weak tv] index in
            guard let text = tv?.string as NSString? else { return nil }
            return Completion.isMathMode(in: text, caretUTF16: index)
        }
        XCTAssertEqual(tv.mathModeAtCaret(8), true)
        XCTAssertEqual(tv.mathModeAtCaret(2), false)
        XCTAssertEqual(tv.mathModeAtCaret(15), false)
    }
}
