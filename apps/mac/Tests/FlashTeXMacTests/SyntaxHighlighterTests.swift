import AppKit
import XCTest
@testable import FlashTeXMac

/// Lexer runs on tricky inputs and the incremental invariant (lane mac-syntax-highlight):
/// after any sequence of edits, `runs(in:)` over the whole text equals a
/// fresh full lex, and the line table equals the text's line starts.
@MainActor
final class SyntaxHighlighterTests: XCTestCase {
    typealias Kind = SyntaxHighlighter.Kind

    private func runs(_ s: String) -> [SyntaxHighlighter.Run] { SyntaxHighlighter.runs(of: s as NSString) }

    /// `(substring, kind)` pairs for readable assertions.
    private func spans(_ s: String) -> [(String, Kind)] {
        let ns = s as NSString
        return runs(s).map { (ns.substring(with: $0.range), $0.kind) }
    }

    private func assertSpans(_ s: String, _ expected: [(String, Kind)], file: StaticString = #filePath, line: UInt = #line) {
        let got = spans(s)
        XCTAssertEqual(got.map { "\($0.1):\($0.0)" }, expected.map { "\($0.1):\($0.0)" }, file: file, line: line)
    }

    // MARK: control sequences, environments, arguments

    func testControlSequencesAndEnvironmentNames() {
        assertSpans("\\section{Intro}", [("\\section", .command), ("{", .brace), ("}", .brace)])
        assertSpans("\\begin{itemize}\\item x\\end{itemize}", [
            ("\\begin", .command), ("{", .brace), ("itemize", .environment), ("}", .brace),
            ("\\item", .command),
            ("\\end", .command), ("{", .brace), ("itemize", .environment), ("}", .brace),
        ])
        assertSpans("a\\\\b \\, \\{x\\}", [("\\\\", .command), ("\\,", .command), ("\\{", .command), ("\\}", .command)])
    }

    func testReferencesFilesAndDefinitions() {
        assertSpans("\\label{eq:1} \\ref{eq:1} \\cite[p.~3]{knuth84}", [
            ("\\label", .command), ("{", .brace), ("eq:1", .reference), ("}", .brace),
            ("\\ref", .command), ("{", .brace), ("eq:1", .reference), ("}", .brace),
            ("\\cite", .command), ("[", .bracket), ("]", .bracket), ("{", .brace), ("knuth84", .reference), ("}", .brace),
        ])
        assertSpans("\\input{ch/one} \\usepackage[utf8]{inputenc}", [
            ("\\input", .command), ("{", .brace), ("ch/one", .file), ("}", .brace),
            ("\\usepackage", .command), ("[", .bracket), ("]", .bracket), ("{", .brace), ("inputenc", .file), ("}", .brace),
        ])
        assertSpans("\\newcommand{\\R}{\\mathbb{R}} \\def\\foo{1}", [
            ("\\newcommand", .command), ("{", .brace), ("\\R", .definition), ("}", .brace),
            ("{", .brace), ("\\mathbb", .command), ("{", .brace), ("}", .brace), ("}", .brace), // R outside math is plain
            ("\\def", .command), ("\\foo", .definition), ("{", .brace), ("}", .brace),
        ])
    }

    func testNestedBracesInReferenceArgument() {
        // The argument is coloured up to its matching brace, not the first one.
        assertSpans("\\ref{a{b}c}!", [("\\ref", .command), ("{", .brace), ("a{b}c", .reference), ("}", .brace)])
        // Unterminated argument on the line: only the command is coloured.
        assertSpans("\\ref{a\nb}", [("\\ref", .command), ("{", .brace), ("}", .brace)])
    }

    // MARK: math

    func testInlineAndDisplayMath() {
        assertSpans("x $a^2+b$ y", [("$", .mathDelimiter), ("a^", .math), ("2", .number), ("+b", .math), ("$", .mathDelimiter)])
        assertSpans("$\\alpha_1 = 3.5$", [
            ("$", .mathDelimiter), ("\\alpha", .mathCommand), ("_", .math), ("1", .number), (" = ", .math), ("3.5", .number), ("$", .mathDelimiter),
        ])
        assertSpans("\\[ x \\]", [("\\[", .mathDelimiter), (" x ", .math), ("\\]", .mathDelimiter)])
        assertSpans("\\( x \\)", [("\\(", .mathDelimiter), (" x ", .math), ("\\)", .mathDelimiter)])
        assertSpans("$$ x $$", [("$$", .mathDelimiter), (" x ", .math), ("$$", .mathDelimiter)])
        assertSpans("\\begin{align}\na &= b \\\\\n\\end{align}", [
            ("\\begin", .command), ("{", .brace), ("align", .environment), ("}", .brace),
            ("a &= b ", .math), ("\\\\", .mathCommand),
            ("\\end", .command), ("{", .brace), ("align", .environment), ("}", .brace),
        ])
    }

    func testEscapedDollarAndCommentInMath() {
        assertSpans("cost \\$5 and $x$", [("\\$", .command), ("$", .mathDelimiter), ("x", .math), ("$", .mathDelimiter)])
        assertSpans("$x % not $ math\ny$", [
            ("$", .mathDelimiter), ("x ", .math), ("% not $ math", .comment), ("y", .math), ("$", .mathDelimiter),
        ])
        assertSpans("\\% $y$", [("\\%", .command), ("$", .mathDelimiter), ("y", .math), ("$", .mathDelimiter)])
    }

    func testUnbalancedDollarStopsAtBlankLine() {
        let s = "a $b\nc\n\nd \\emph{e}"
        assertSpans(s, [("$", .mathDelimiter), ("b", .math), ("c", .math), ("\\emph", .command), ("{", .brace), ("}", .brace)])
        // A single `$` on the last line colours nothing after it.
        assertSpans("x\n$", [("$", .mathDelimiter)])
    }

    func testMathEnvironmentEndsOnlyAtItsEnd() {
        let s = "\\begin{equation}\n\\begin{cases} 1 & x \\\\ 2 \\end{cases}\n\\end{equation} text"
        let got = spans(s)
        XCTAssertTrue(got.contains { $0 == ("1", .number) })
        XCTAssertTrue(got.contains { $0 == ("\\\\", .mathCommand) })
        XCTAssertEqual(got.last?.0, "}") // "text" is plain
        XCTAssertFalse(got.contains { $0.0.contains("text") })
    }

    /// Math environments nest — `cases` in `align`, `split` or `array` in
    /// `equation` — so the mode counts how deep it is and only the outermost
    /// `\end` leaves math.
    ///
    /// The flat flag it replaced left math at the *inner* `\end{cases}`: every
    /// symbol between there and the outer `\end{align}` was lexed in text mode,
    /// so `\alpha` came out `.command` (purple) instead of `.mathCommand`, and
    /// digits and letters lost their math colouring entirely. This is the case
    /// `testMathEnvironmentEndsOnlyAtItsEnd` above could not see: it puts no
    /// math between the inner and the outer `\end`, so the wrong mode has
    /// nothing to colour wrongly.
    func testNestedMathEnvironmentsStayInMathUntilTheOutermostEnd() {
        let s = "\\begin{align}\nf &= \\begin{cases} 1 \\end{cases} \\\\\ng &= \\alpha + 2\n\\end{align}\n\\beta 3"
        let got = spans(s)
        // After the inner `\end{cases}`, the line is still math.
        XCTAssertTrue(got.contains { $0 == ("\\alpha", .mathCommand) },
                      "\\alpha after \\end{cases} is still a math command: \(got)")
        XCTAssertTrue(got.contains { $0 == ("2", .number) })
        XCTAssertTrue(got.contains { $0.1 == .math && $0.0.contains("g") },
                      "letters after the inner \\end are still math: \(got)")
        XCTAssertFalse(got.contains { $0 == ("\\alpha", .command) }, "never the text-mode colour")
        // The outermost `\end{align}` does leave math.
        XCTAssertTrue(got.contains { $0 == ("\\beta", .command) }, "after \\end{align} the document is text again")
        XCTAssertFalse(got.contains { $0 == ("3", .number) }, "a digit in text mode is not a math number")
        // Three deep, and the depth unwinds one `\end` at a time.
        let deep = "\\begin{equation}\\begin{split}\\begin{array}{r}1\\end{array}a\\end{split}b\\end{equation}c"
        let deepSpans = spans(deep)
        XCTAssertTrue(deepSpans.contains { $0.1 == .math && $0.0.contains("a") }, "inside split after \\end{array}")
        XCTAssertTrue(deepSpans.contains { $0.1 == .math && $0.0.contains("b") }, "inside equation after \\end{split}")
        XCTAssertFalse(deepSpans.contains { $0.1 == .math && $0.0.contains("c") }, "after \\end{equation} it is text")
        // A math environment opened inside verbatim or a comment is still text
        // in the lexer's eyes: the body is never entered.
        let verb = "\\begin{verbatim}\\begin{align}\\end{verbatim}\\gamma"
        XCTAssertTrue(spans(verb).contains { $0 == ("\\gamma", .command) })
    }

    // MARK: verbatim and comments

    func testVerbatimEnvironmentAndInlineVerb() {
        let s = "\\begin{verbatim}\n$x$ \\foo % hi\n\\end{verbatim}\n\\bar"
        assertSpans(s, [
            ("\\begin", .command), ("{", .brace), ("verbatim", .environment), ("}", .brace),
            ("\n$x$ \\foo % hi\n", .verbatim),
            ("\\end", .command), ("{", .brace), ("verbatim", .environment), ("}", .brace),
            ("\\bar", .command),
        ])
        assertSpans("\\verb|a$b| $c$", [("\\verb", .command), ("|a$b|", .verbatim), ("$", .mathDelimiter), ("c", .math), ("$", .mathDelimiter)])
        assertSpans("\\begin{lstlisting}[language=C]\nint x; // $\n\\end{lstlisting}", [
            ("\\begin", .command), ("{", .brace), ("lstlisting", .environment), ("}", .brace),
            ("[language=C]\nint x; // $\n", .verbatim),
            ("\\end", .command), ("{", .brace), ("lstlisting", .environment), ("}", .brace),
        ])
    }

    func testCommentEnvironmentAndLineComments() {
        assertSpans("a % b \\c $d$\ne", [("% b \\c $d$", .comment)])
        assertSpans("\\begin{comment}\nx $y$\n\\end{comment}z", [
            ("\\begin", .command), ("{", .brace), ("comment", .environment), ("}", .brace),
            ("\nx $y$\n", .comment),
            ("\\end", .command), ("{", .brace), ("comment", .environment), ("}", .brace),
        ])
    }

    // MARK: encodings

    func testCRLFLinesAndModes() {
        let s = "a $b\r\nc$ d\r\n\r\n\\x"
        assertSpans(s, [("$", .mathDelimiter), ("b\r", .math), ("c", .math), ("$", .mathDelimiter), ("\\x", .command)])
        var h = SyntaxHighlighter()
        h.reset(s as NSString)
        XCTAssertEqual(h.lineStarts, [0, 6, 12, 14])
        XCTAssertEqual(h.modes, [.text, .inlineMath, .text, .text])
    }

    func testSurrogatePairsAndCombiningMarksNeverSplit() {
        let s = "\\😀 $α̈ + 😀² = 1$ ẍ\\ref{ẍ}"
        let ns = s as NSString
        for run in runs(s) {
            let start = run.range.location, end = NSMaxRange(run.range)
            if start > 0 { XCTAssertFalse(UTF16.isTrailSurrogate(ns.character(at: start)), "run starts inside a pair at \(start)") }
            if end < ns.length { XCTAssertFalse(UTF16.isTrailSurrogate(ns.character(at: end)), "run ends inside a pair at \(end)") }
        }
        XCTAssertEqual(spans(s).first?.0, "\\😀")
        XCTAssertTrue(spans(s).contains { $0 == ("ẍ", .reference) })
    }

    // MARK: incremental line model

    private static func expectedLineStarts(_ ns: NSString) -> [Int] {
        var starts = [0]
        for i in 0..<ns.length where ns.character(at: i) == 0x0A { starts.append(i + 1) }
        return starts
    }

    func testResetLineTable() {
        var h = SyntaxHighlighter()
        h.reset("" as NSString)
        XCTAssertEqual(h.lineStarts, [0]); XCTAssertEqual(h.modes, [.text])
        h.reset("a\nb\n" as NSString)
        XCTAssertEqual(h.lineStarts, [0, 2, 4]); XCTAssertEqual(h.lineCount, 3)
        XCTAssertEqual(h.line(at: 0), 0); XCTAssertEqual(h.line(at: 1), 0); XCTAssertEqual(h.line(at: 2), 1); XCTAssertEqual(h.line(at: 4), 2); XCTAssertEqual(h.line(at: 99), 2)
        XCTAssertEqual(h.lineRange(1), NSRange(location: 2, length: 2))
    }

    /// Applies `edit` to both the model and a plain string; checks the model
    /// against a fresh full lex.
    private func applyAndCheck(_ h: inout SyntaxHighlighter, _ text: inout String, range: NSRange, replacement: String,
                               file: StaticString = #filePath, line: UInt = #line) {
        let ns = (text as NSString).replacingCharacters(in: range, with: replacement) as NSString
        text = ns as String
        let dirty = h.edit(range: range, replacementLength: (replacement as NSString).length, text: ns)
        XCTAssertEqual(h.lineStarts, Self.expectedLineStarts(ns), "line starts after \(range) <- \(replacement.debugDescription)", file: file, line: line)
        var fresh = SyntaxHighlighter()
        fresh.reset(ns)
        XCTAssertEqual(h.modes, fresh.modes, "line modes after \(range) <- \(replacement.debugDescription)", file: file, line: line)
        let whole = NSRange(location: 0, length: ns.length)
        XCTAssertEqual(h.runs(in: whole, text: ns), fresh.runs(in: whole, text: ns), "runs after \(range) <- \(replacement.debugDescription)", file: file, line: line)
        XCTAssertTrue(dirty.location <= range.location && NSMaxRange(dirty) >= min(ns.length, range.location + (replacement as NSString).length),
                      "dirty \(dirty) must cover the replacement", file: file, line: line)
        XCTAssertEqual(dirty.location, h.lineStarts[h.line(at: range.location)], "dirty starts at a line start", file: file, line: line)
    }

    func testIncrementalEditsMatchFullLex() {
        var text = "\\documentclass{article}\n\\begin{document}\nHello $x$ world.\n\n\\begin{align}\na &= b\n\\end{align}\n% c\n\\end{document}\n"
        var h = SyntaxHighlighter()
        h.reset(text as NSString)
        applyAndCheck(&h, &text, range: NSRange(location: 47, length: 0), replacement: "y")         // typing in text
        applyAndCheck(&h, &text, range: NSRange(location: 48, length: 1), replacement: "")          // backspace
        applyAndCheck(&h, &text, range: NSRange(location: 43, length: 0), replacement: "$")         // unbalance the inline math
        applyAndCheck(&h, &text, range: NSRange(location: 43, length: 1), replacement: "")          // restore
        applyAndCheck(&h, &text, range: NSRange(location: 0, length: 0), replacement: "\\[\n")      // open display math at the top
        applyAndCheck(&h, &text, range: NSRange(location: 0, length: 3), replacement: "")
        applyAndCheck(&h, &text, range: NSRange(location: 60, length: 0), replacement: "\n\n\n")    // insert lines
        applyAndCheck(&h, &text, range: NSRange(location: 55, length: 10), replacement: "Q")        // delete across lines
        applyAndCheck(&h, &text, range: NSRange(location: 20, length: 0), replacement: "\\begin{verbatim}") // verbatim to the end
        applyAndCheck(&h, &text, range: NSRange(location: 20, length: 16), replacement: "")
        applyAndCheck(&h, &text, range: NSRange(location: 0, length: (text as NSString).length), replacement: "")  // clear
        applyAndCheck(&h, &text, range: NSRange(location: 0, length: 0), replacement: "$a\n\nb$")
        applyAndCheck(&h, &text, range: NSRange(location: 2, length: 2), replacement: "")           // join the paragraph: b$ closes
        applyAndCheck(&h, &text, range: NSRange(location: 3, length: 0), replacement: "\n")         // trailing newline
        applyAndCheck(&h, &text, range: NSRange(location: 4, length: 0), replacement: "\n")
        applyAndCheck(&h, &text, range: NSRange(location: 3, length: 2), replacement: "")
    }

    func testRandomEditsMatchFullLex() {
        var state: UInt64 = 0x9E3779B97F4A7C15
        func next() -> UInt64 { state = state &* 6364136223846793005 &+ 1442695040888963407; return state >> 11 }
        let atoms = ["a", "\n", "\n\n", "$", "$$", "\\[", "\\]", "\\(", "\\)", "\\begin{align}", "\\end{align}", "\\begin{verbatim}",
                     "\\end{verbatim}", "%", "\\%", "\\$", "{", "}", "\\ref{x}", "\\verb|q|", " ", "1.5", "\\alpha", "\r\n", "é", "😀", "\\😀",
                     // Nested math environments: the mode carries a depth, so the
                     // incremental lexer has to converge on the depth too.
                     "\\begin{cases}", "\\end{cases}"]
        var text = ""
        var h = SyntaxHighlighter()
        h.reset("" as NSString)
        for _ in 0..<400 {
            let ns = text as NSString
            let insert = next() % 3 != 0 || ns.length == 0
            if insert {
                let loc = Int(next() % UInt64(ns.length + 1))
                // Never split a surrogate pair.
                var at = loc
                if at > 0, at < ns.length, UTF16.isTrailSurrogate(ns.character(at: at)) { at -= 1 }
                applyAndCheck(&h, &text, range: NSRange(location: at, length: 0), replacement: atoms[Int(next() % UInt64(atoms.count))])
            } else {
                var loc = Int(next() % UInt64(ns.length))
                var len = min(ns.length - loc, Int(next() % 6) + 1)
                if UTF16.isTrailSurrogate(ns.character(at: loc)) { loc -= 1; len += 1 }
                let endUnit = loc + len
                if endUnit < ns.length, UTF16.isTrailSurrogate(ns.character(at: endUnit)) { len += 1 }
                applyAndCheck(&h, &text, range: NSRange(location: loc, length: len), replacement: "")
            }
        }
    }

    func testRunsInRangeAreClippedAndLineAligned() {
        let s = "\\section{A}\n$x+1$\n"
        var h = SyntaxHighlighter()
        h.reset(s as NSString)
        let got = h.runs(in: NSRange(location: 2, length: 3), text: s as NSString)
        XCTAssertEqual(got, [SyntaxHighlighter.Run(2, 3, .command)])
        XCTAssertEqual(h.kind(at: 0, text: s as NSString), .command)
        XCTAssertEqual(h.kind(at: 9, text: s as NSString), nil) // "A" is plain
        XCTAssertEqual(h.kind(at: 13, text: s as NSString), .math)
        XCTAssertEqual(h.kind(at: 15, text: s as NSString), .number)
    }

    // MARK: performance (load-gated)

    /// A keystroke in a 560 KB / ~10 000-line document re-lexes one line and
    /// shifts the line table: ≤ 2 ms CPU (budget from the lane assignment).
    func testKeystrokeRelexOn560KBDocument() throws {
        let uptime = IMEHarness.uptime()
        let load = IMEHarness.loadAverage1() ?? 0
        let forced = ProcessInfo.processInfo.environment["FLASHTEX_BENCH_FORCE"] != nil
        print("syntax-highlight bench: \(uptime)\(forced ? " (FLASHTEX_BENCH_FORCE set)" : "")")
        if load > 20, !forced { throw XCTSkip("1-minute load \(load) > 20; timing is not meaningful (\(uptime))") }
        let text = LargeDocumentEditorTests.proseDocument(bytes: 560_000)
        var ns = text as NSString
        var h = SyntaxHighlighter()
        let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        h.reset(ns)
        let resetMs = Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6
        XCTAssertGreaterThan(h.lineCount, 9_000)
        var cpu: [Double] = []
        var lexed: [Int] = []
        let positions = [100, ns.length / 3, ns.length / 2, ns.length - 200]
        for (i, p) in positions.enumerated() {
            let replacement = i % 2 == 0 ? "x" : "$"
            ns = ns.replacingCharacters(in: NSRange(location: p, length: 0), with: replacement) as NSString
            let t0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            _ = h.edit(range: NSRange(location: p, length: 0), replacementLength: 1, text: ns)
            cpu.append(Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - t0) / 1e6)
            lexed.append(h.lastEditLinesLexed)
            // Undo it so the document stays balanced for the next probe.
            ns = ns.replacingCharacters(in: NSRange(location: p, length: 1), with: "") as NSString
            _ = h.edit(range: NSRange(location: p, length: 1), replacementLength: 0, text: ns)
        }
        let whole = NSRange(location: 0, length: ns.length)
        XCTAssertEqual(h.runs(in: whole, text: ns), SyntaxHighlighter.runs(of: ns))
        print(String(format: "syntax-highlight bench: 560 KB reset %.2f ms cpu; keystroke edits %@ ms cpu (lines re-lexed %@); %@",
                     resetMs, cpu.map { String(format: "%.3f", $0) }.joined(separator: ", "), lexed.description, uptime))
        XCTAssertLessThanOrEqual(cpu.max() ?? 0, 2.0, "keystroke re-lex over budget (\(uptime))")
        XCTAssertLessThanOrEqual(lexed.max() ?? 0, 8)
    }
}
