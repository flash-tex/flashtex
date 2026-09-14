import AppKit
import XCTest
import HostedWindows
@testable import FlashTeXMac

/// Pure reindent rules plus the hosted CompletingTextView undo/selection
/// path (lane editor-reindent, EditorIndentation.swift).
final class EditorIndentationTests: XCTestCase {
    typealias EI = EditorIndentation

    let unit = "  "

    func reindent(_ text: String, unit: String? = nil) -> [String] {
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false)
        return EI.reindent(lines, baseDepth: 0, unit: unit ?? self.unit)
    }

    func join(_ lines: [String]) -> String { lines.joined(separator: "\n") }

    func apply(_ plan: EI.Plan, to text: String) -> String {
        (text as NSString).replacingCharacters(in: plan.range, with: plan.replacement)
    }

    func bodyLines(_ text: String, env: String) -> [String] {
        let lines = text.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        let open = "\\begin{\(env)}"
        let close = "\\end{\(env)}"
        guard let b = lines.firstIndex(where: { $0.trimmingCharacters(in: .whitespaces).hasPrefix(open) }),
              let e = lines.lastIndex(where: { $0.trimmingCharacters(in: .whitespaces).hasPrefix(close) }),
              e > b else { return [] }
        return Array(lines[(b + 1)..<e])
    }

    // MARK: nesting

    func testNestingIncreasesAfterBeginAndDecreasesOnEnd() {
        let src = """
        \\begin{itemize}
        \\item a
        \\begin{enumerate}
        \\item b
        \\end{enumerate}
        \\end{itemize}
        """
        XCTAssertEqual(join(reindent(src)), """
        \\begin{itemize}
          \\item a
          \\begin{enumerate}
            \\item b
          \\end{enumerate}
        \\end{itemize}
        """)
    }

    func testSameLineBeginEndDoesNotShiftLeft() {
        XCTAssertEqual(reindent("  \\begin{center} x \\end{center}"), ["\\begin{center} x \\end{center}"])
    }

    // MARK: document exception

    func testDocumentDoesNotIndentItsBody() {
        XCTAssertEqual(EI.flatEnvironments, ["document"])
        let src = """
        \\documentclass{article}
        \\begin{document}
        Hello
        \\begin{itemize}
        \\item a
        \\end{itemize}
        \\end{document}
        """
        XCTAssertEqual(join(reindent(src)), """
        \\documentclass{article}
        \\begin{document}
        Hello
        \\begin{itemize}
          \\item a
        \\end{itemize}
        \\end{document}
        """)
    }

    // MARK: verbatim preservation

    func testPreservedBodiesStayByteIdentical() {
        XCTAssertEqual(EI.preservedBodyEnvironments, SyntaxHighlighter.verbatimEnvironments.union(["comment"]))
        XCTAssertTrue(EI.preservedBodyEnvironments.contains("verbatim*"))
        XCTAssertTrue(EI.preservedBodyEnvironments.contains("Verbatim"))
        XCTAssertTrue(EI.preservedBodyEnvironments.contains("BVerbatim"))
        for env in ["verbatim", "verbatim*", "Verbatim", "BVerbatim", "lstlisting", "minted", "comment"] {
            let src = """
            \\begin{itemize}
            \\begin{\(env)}
              keep  tabs\tand spaces
            % \\end{\(env)} is text here
            \\end{\(env)}
            \\item after
            \\end{itemize}
            """
            let out = join(reindent(src))
            XCTAssertEqual(bodyLines(out, env: env), bodyLines(src, env: env), env)
            XCTAssertTrue(out.contains("\n  keep  tabs\tand spaces\n"), env)
            XCTAssertTrue(out.contains("% \\end{\(env)} is text here"), env)
            XCTAssertTrue(out.contains("\n  \\item after\n"), env)
        }
    }

    // MARK: comments and escapes

    func testCommentsAndEscapesAreIgnoredWhenCounting() {
        let src = """
        % \\begin{itemize}
        \\{ not a group
        still outer
        \\}
        foo % \\begin{center}
        """
        XCTAssertEqual(join(reindent(src)), """
        % \\begin{itemize}
        \\{ not a group
        still outer
        \\}
        foo % \\begin{center}
        """)
    }

    func testPercentAfterEscapedBackslashIsAComment() {
        // `\\` is a control symbol; the following `%` comments out `\begin`.
        XCTAssertEqual(reindent("\\\\% \\begin{itemize}\n\\item x"), ["\\\\% \\begin{itemize}", "\\item x"])
    }

    // MARK: braces

    func testBraceGroupSpanningLinesAddsAUnit() {
        let src = """
        \\textbf{
        inner
        }
        after
        """
        XCTAssertEqual(join(reindent(src)), """
        \\textbf{
          inner
        }
        after
        """)
    }

    // MARK: item continuation (implemented)

    func testItemLinesKeepDepthAndContinuationGetsOneExtraUnit() {
        let src = """
        \\begin{itemize}
        \\item first
        continuation
        \\item second
        \\end{itemize}
        """
        XCTAssertEqual(join(reindent(src)), """
        \\begin{itemize}
          \\item first
            continuation
          \\item second
        \\end{itemize}
        """)
    }

    // MARK: blank lines, no join/split

    func testBlankLinesBecomeEmptyAndLineCountIsStable() {
        let src = "\\begin{itemize}\n  \n\\item a\n\n\\end{itemize}"
        let out = reindent(src)
        XCTAssertEqual(out.count, 5)
        XCTAssertEqual(out[1], "")
        XCTAssertEqual(out[3], "")
        XCTAssertEqual(out[0], "\\begin{itemize}")
        XCTAssertEqual(out[2], "  \\item a")
        XCTAssertEqual(out[4], "\\end{itemize}")
    }

    // MARK: unbalanced

    func testUnbalancedEndNeverGoesLeftOfZeroAndDoesNotCrash() {
        let src = "\\end{itemize}\n\\end{enumerate}\n}\n}\\end{foo}"
        let out = reindent(src)
        XCTAssertEqual(out.count, 4)
        for line in out {
            XCTAssertFalse(line.hasPrefix(" "), "never negative indent: \(line)")
        }
        XCTAssertEqual(out[0], "\\end{itemize}")
        XCTAssertEqual(out[3], "}\\end{foo}")
    }

    func testUnbalancedBeginDoesNotCrash() {
        let src = "\\begin{a}\n\\begin{b}\nnever closed"
        XCTAssertEqual(join(reindent(src)), "\\begin{a}\n  \\begin{b}\n    never closed")
    }

    // MARK: tabs vs spaces

    func testTabUnitAndSpaceUnit() {
        XCTAssertEqual(join(reindent("\\begin{a}\nx\n\\end{a}", unit: "\t")), "\\begin{a}\n\tx\n\\end{a}")
        XCTAssertEqual(join(reindent("\\begin{a}\nx\n\\end{a}", unit: "    ")), "\\begin{a}\n    x\n\\end{a}")
        XCTAssertEqual(join(reindent("\\begin{a}\n\tx\n\\end{a}", unit: "  ")), "\\begin{a}\n  x\n\\end{a}")
    }

    // MARK: CRLF

    func testCRLFTerminatorsArePreserved() {
        let text = "\\begin{itemize}\r\n\\item a\r\n\\end{itemize}\r\n"
        let plan = EI.plan(in: text, selection: NSRange(location: 0, length: (text as NSString).length),
                           unit: "  ", tabWidth: 4, wholeDocument: true)!
        let out = apply(plan, to: text)
        XCTAssertEqual(out, "\\begin{itemize}\r\n  \\item a\r\n\\end{itemize}\r\n")
        XCTAssertTrue(out.contains("\r\n"))
        XCTAssertFalse(out.contains("\n\n"))
    }

    // MARK: partial selection with context

    func testPartialSelectionMatchesTheLineAbove() {
        // Neighbour `\item a` sits at indent 0; the selected inner line must
        // match that context (plus one continuation unit), not the structural
        // depth of the unselected `\begin`.
        let text = "\\begin{itemize}\n\\item a\n        inner\n\\end{itemize}\n"
        let inner = (text as NSString).range(of: "        inner")
        let plan = EI.plan(in: text, selection: inner, unit: "  ", tabWidth: 4, wholeDocument: false)!
        let line = (text as NSString).lineRange(for: inner)
        XCTAssertTrue(NSLocationInRange(plan.range.location, line))
        XCTAssertLessThanOrEqual(NSMaxRange(plan.range), NSMaxRange(line))
        XCTAssertEqual(apply(plan, to: text), "\\begin{itemize}\n\\item a\n  inner\n\\end{itemize}\n")
    }

    func testPartialSelectionInsideVerbatimLeavesTheBodyAlone() {
        let text = "\\begin{verbatim}\n  keep\n\\end{verbatim}\n"
        let body = (text as NSString).range(of: "  keep")
        XCTAssertNil(EI.plan(in: text, selection: body, unit: "  ", tabWidth: 4, wholeDocument: false),
                     "already identical: no edit")
        let whole = EI.plan(in: text, selection: NSRange(location: 0, length: 0), unit: "  ", tabWidth: 4, wholeDocument: true)
        XCTAssertNil(whole, "verbatim body plus flush begin/end is already indented")
    }

    // MARK: selection mapped to non-whitespace

    func testPlanMapsCaretOntoNonWhitespaceContent() {
        let text = "\\begin{itemize}\n    \\item foo\n\\end{itemize}\n"
        let f = (text as NSString).range(of: "foo")
        let plan = EI.plan(in: text, selection: NSRange(location: f.location, length: 0),
                           unit: "  ", tabWidth: 4, wholeDocument: false)!
        let new = apply(plan, to: text) as NSString
        XCTAssertEqual(new.substring(with: NSRange(location: plan.selection.location, length: 3)), "foo")
        XCTAssertEqual(plan.selection.length, 0)
    }

    func testEmptyUnitFallsBackToTwoSpaces() {
        XCTAssertEqual(EI.reindent(["\\begin{a}", "x", "\\end{a}"].map { $0[...] }, baseDepth: 0, unit: ""),
                       ["\\begin{a}", "  x", "\\end{a}"])
    }

    // MARK: minimal replacement

    func testAlreadyIndentedDocumentProducesNoPlan() {
        let text = "\\begin{itemize}\n  \\item a\n\\end{itemize}\n"
        XCTAssertNil(EI.plan(in: text, selection: NSRange(location: 0, length: 0),
                             unit: "  ", tabWidth: 4, wholeDocument: true))
        XCTAssertNil(EI.trimmedReplacement(old: text, new: text,
                                           range: NSRange(location: 0, length: (text as NSString).length)))
    }

    func testOneLineChangeReplacesOnlyThatLineSpan() {
        let text = "\\begin{itemize}\n  \\item a\n\\item b\n\\end{itemize}\n"
        let plan = EI.plan(in: text, selection: NSRange(location: 0, length: 0),
                           unit: "  ", tabWidth: 4, wholeDocument: true)!
        let itemB = (text as NSString).range(of: "\\item b")
        let line = (text as NSString).lineRange(for: itemB)
        XCTAssertTrue(NSLocationInRange(plan.range.location, line),
                      "range \(plan.range) must sit on the \\item b line \(line)")
        XCTAssertLessThanOrEqual(NSMaxRange(plan.range), NSMaxRange(line))
        XCTAssertEqual(apply(plan, to: text), "\\begin{itemize}\n  \\item a\n  \\item b\n\\end{itemize}\n")
    }

    func testTrimmedReplacementDropsCommonUTF16Affixes() {
        let old = "aaaXbbb"
        let new = "aaaYbbb"
        let trimmed = EI.trimmedReplacement(old: old, new: new,
                                            range: NSRange(location: 10, length: (old as NSString).length))!
        XCTAssertEqual(trimmed.range, NSRange(location: 13, length: 1))
        XCTAssertEqual(trimmed.replacement, "Y")
    }
}

@MainActor
final class EditorIndentationHostTests: XCTestCase {
    func host(_ text: String) throws -> (NSWindow, CompletingTextView) {
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled])
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        let tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        window.orderFrontRegardless()
        window.makeFirstResponder(tv)
        tv.allowsUndo = true
        tv.string = text
        return (window, tv)
    }

    func testReindentLinesIsOneUndoStepAndPreservesSelection() throws {
        let (window, tv) = try host("\\begin{itemize}\n\\item foo\n\\end{itemize}\n")
        defer { window.orderOut(nil) }
        let unit = EditorPreferences.shared.indentString
        let f = (tv.string as NSString).range(of: "foo")
        tv.setSelectedRange(NSRange(location: f.location, length: 3))
        tv.undoManager?.removeAllActions()
        tv.reindentSelectedLines(nil)
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\(unit)\\item foo\n\\end{itemize}\n")
        XCTAssertEqual((tv.string as NSString).substring(with: tv.selectedRange()), "foo")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Re-indent Lines")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\\item foo\n\\end{itemize}\n")
        tv.undoManager?.redo()
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\(unit)\\item foo\n\\end{itemize}\n")
    }

    func testReindentDocumentIsOneUndoStep() throws {
        let (window, tv) = try host("\\begin{document}\n\\begin{itemize}\n\\item x\n\\end{itemize}\n\\end{document}\n")
        defer { window.orderOut(nil) }
        let unit = EditorPreferences.shared.indentString
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.undoManager?.removeAllActions()
        tv.reindentWholeDocument(nil)
        XCTAssertEqual(tv.string, "\\begin{document}\n\\begin{itemize}\n\(unit)\\item x\n\\end{itemize}\n\\end{document}\n")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Re-indent Document")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "\\begin{document}\n\\begin{itemize}\n\\item x\n\\end{itemize}\n\\end{document}\n")
    }

    func testCaretLineIsReindentedWhenNothingIsSelected() throws {
        let (window, tv) = try host("\\begin{a}\nx\n\\end{a}\n")
        defer { window.orderOut(nil) }
        let unit = EditorPreferences.shared.indentString
        let x = (tv.string as NSString).range(of: "x")
        tv.setSelectedRange(NSRange(location: x.location, length: 0))
        tv.reindentSelectedLines(nil)
        XCTAssertEqual(tv.string, "\\begin{a}\n\(unit)x\n\\end{a}\n")
        XCTAssertEqual(tv.selectedRange().length, 0)
        XCTAssertEqual((tv.string as NSString).substring(with: NSRange(location: tv.selectedRange().location, length: 1)), "x")
    }

    func testAlreadyIndentedDocumentIsNoTextChangeAndNoUndo() throws {
        let unit = EditorPreferences.shared.indentString
        let text = "\\begin{itemize}\n\(unit)\\item foo\n\\end{itemize}\n"
        let (window, tv) = try host(text)
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.undoManager?.removeAllActions()
        XCTAssertEqual(tv.undoManager?.canUndo, false)
        tv.reindentWholeDocument(nil)
        XCTAssertEqual(tv.string, text)
        XCTAssertEqual(tv.undoManager?.canUndo, false)
    }

    func testReindentDoesNotWrapThroughCaretContextWhenCaretIsInMath() throws {
        let (window, tv) = try host("\\begin{itemize}\n\\item $a + b$\n\\end{itemize}\n")
        defer { window.orderOut(nil) }
        let plus = (tv.string as NSString).range(of: "+")
        tv.setSelectedRange(NSRange(location: plus.location, length: 0))
        // ASCII document: UTF-16 offset == UTF-8 byte offset.
        let caret = CaretContext.derive(tv.string, caretByte: plus.location)
        XCTAssertEqual(caret.wrap, .alreadyMath)
        let unit = EditorPreferences.shared.indentString
        let plan = try XCTUnwrap(EditorIndentation.plan(in: tv.string, selection: tv.selectedRange(),
                                                        unit: unit, tabWidth: EditorPreferences.shared.tabWidth,
                                                        wholeDocument: false))
        let wrapped = caret.normalize(plan.replacement)
        XCTAssertTrue(wrapped.text != plan.replacement,
                      "caret-context wrapping would alter this replacement; reindent must insert it raw")
        tv.undoManager?.removeAllActions()
        tv.reindentSelectedLines(nil)
        XCTAssertEqual(tv.string, "\\begin{itemize}\n\(unit)\\item $a + b$\n\\end{itemize}\n")
        XCTAssertTrue(tv.string.contains("$a + b$"))
        XCTAssertFalse(tv.string.contains("\\text{"))
        XCTAssertFalse(tv.string.contains("\\mbox{"))
    }
}

@MainActor
final class EditorIndentationPerfTests: XCTestCase {
    func testReindentOf560KBDocumentIsUnder50ms() throws {
        let text = LargeDocumentEditorTests.proseDocument(bytes: 560_000)
        XCTAssertGreaterThanOrEqual(text.utf8.count, 560_000)
        // The LargeDocument fixture is already flush-left inside `\begin{document}`,
        // so plan() is a no-op; measure a rewrite of every line (the worst case).
        let split = EditorIndentation.splitLines(text as NSString)
        let messy = split.map { "        " + $0.content }
        var samples: [Double] = []
        var last: [String] = []
        for _ in 0..<8 {
            let t0 = DispatchTime.now().uptimeNanoseconds
            last = EditorIndentation.reindent(messy.map { $0[...] }, baseDepth: 0, unit: "  ")
            let t1 = DispatchTime.now().uptimeNanoseconds
            samples.append(Double(t1 - t0) / 1e6)
        }
        XCTAssertEqual(last.count, messy.count)
        let sorted = samples.sorted()
        let best = sorted[0], median = sorted[sorted.count / 2]
        print(String(format: "EditorIndentation.reindent 560KB: best %.3f ms, median %.3f ms", best, median))
        XCTAssertLessThan(best, 50.0)
    }
}
