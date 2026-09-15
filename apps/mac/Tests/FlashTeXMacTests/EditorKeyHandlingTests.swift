import AppKit
import XCTest
@testable import FlashTeXMac

/// Pure decision logic for GH74: Tab/Shift-Tab indentation and the
/// completion-snippet overtype hook. No NSTextView involved — these are the
/// same functions the Coordinator (SourceEditorView.swift) and Completion.swift
/// call through their small hooks.
final class EditorKeyHandlingTests: XCTestCase {
    typealias EKH = EditorKeyHandling

    // MARK: line starts / multi-line

    func testLineStartsTouchesEveryLineACaretOrSelectionSpans() {
        let text = "aaa\nbbb\nccc\nddd"
        XCTAssertEqual(EKH.lineStarts(in: text, range: NSRange(location: 1, length: 0)), [0], "caret: just its own line")
        XCTAssertEqual(EKH.lineStarts(in: text, range: NSRange(location: 1, length: 6)), [0, 4], "selection crossing one newline touches two lines")
        XCTAssertEqual(EKH.lineStarts(in: text, range: NSRange(location: 0, length: 4)), [0],
                       "ends exactly at the start of line 2: line 2 is not touched")
        XCTAssertEqual(EKH.lineStarts(in: text, range: NSRange(location: 0, length: 15)), [0, 4, 8, 12], "whole buffer")
        XCTAssertFalse(EKH.isMultiLine(text, range: NSRange(location: 1, length: 0)))
        XCTAssertFalse(EKH.isMultiLine(text, range: NSRange(location: 0, length: 2)), "same line")
        XCTAssertTrue(EKH.isMultiLine(text, range: NSRange(location: 1, length: 6)))
    }

    // MARK: indent

    func testIndentPrefixesEveryTouchedLineAndGrowsTheSelection() {
        let text = "aa\nbb\ncc"
        let (edits, selection) = EKH.indentEdits(in: text, range: NSRange(location: 1, length: 5), unit: "  ")!
        XCTAssertEqual(edits, [
            EKH.LineEdit(range: NSRange(location: 0, length: 0), replacement: "  "),
            EKH.LineEdit(range: NSRange(location: 3, length: 0), replacement: "  "),
        ])
        // Apply in the order the Coordinator does (last line first) to check the result text.
        var s = text as NSString
        for e in edits.sorted(by: { $0.range.location > $1.range.location }) {
            s = s.replacingCharacters(in: e.range, with: e.replacement) as NSString
        }
        XCTAssertEqual(s as String, "  aa\n  bb\ncc")
        XCTAssertEqual(selection, NSRange(location: 0, length: 10), "selects both full (now indented) lines, through their shared boundary")
    }

    func testIndentAtCaretIndentsOnlyItsOwnLineAndKeepsTheCaretACaret() {
        let (edits, selection) = EKH.indentEdits(in: "aa\nbb", range: NSRange(location: 4, length: 0), unit: "\t")!
        XCTAssertEqual(edits, [EKH.LineEdit(range: NSRange(location: 3, length: 0), replacement: "\t")])
        XCTAssertEqual(selection, NSRange(location: 5, length: 0), "shifts with the inserted tab, stays a caret")
    }

    // MARK: outdent

    func testOutdentRemovesUpToOneUnitOfMatchingLeadingWhitespace() {
        let text = "    aa\n  bb\ncc\n\tdd"
        let (edits, _) = EKH.outdentEdits(in: text, range: NSRange(location: 0, length: text.utf16.count), unit: "    ")!
        // Line 1: 4 leading spaces, all removed. Line 2: only 2 (< 4), all removed.
        // Line 3: none, no edit. Line 4: a tab, not a space, no edit (unit is spaces).
        XCTAssertEqual(edits.count, 2)
        XCTAssertTrue(edits.contains(EKH.LineEdit(range: NSRange(location: 0, length: 4), replacement: "")))
        XCTAssertTrue(edits.contains(EKH.LineEdit(range: NSRange(location: 7, length: 2), replacement: "")))
    }

    func testOutdentWithTabUnitRemovesOneLeadingTabOnly() {
        let (edits, _) = EKH.outdentEdits(in: "\t\taa", range: NSRange(location: 0, length: 0), unit: "\t")!
        XCTAssertEqual(edits, [EKH.LineEdit(range: NSRange(location: 0, length: 1), replacement: "")])
    }

    func testOutdentWithNoLeadingWhitespaceIsANoOp() {
        let (edits, selection) = EKH.outdentEdits(in: "aa", range: NSRange(location: 1, length: 0), unit: "    ")!
        XCTAssertEqual(edits, [])
        XCTAssertEqual(selection, NSRange(location: 1, length: 0), "nothing to remove: the caret does not move")
    }

    // MARK: completion-snippet closer (Completion.swift's hook)

    func testProgrammaticCloserFindsTheCloserRightAfterTheSnippetCaret() {
        // `\section{}` with the caret placed right after `{` (Completion.swift's `Vocabulary.Entry.snippet`).
        XCTAssertEqual(EKH.programmaticCloser(in: "\\section{}", insertedAt: 5, caretUTF16: 5 + 9), 14)
        XCTAssertNil(EKH.programmaticCloser(in: "\\section{}", insertedAt: 5, caretUTF16: 5 + 3), "not before a closer")
        XCTAssertNil(EKH.programmaticCloser(in: "word", insertedAt: 0, caretUTF16: 4), "caret at the very end: nothing follows")
    }

    /// GH#2: which replacements carry the closer for a delimiter opened before
    /// them, and so must swallow the editor's auto-inserted one.
    func testSupersedesTrackedCloserRecognisesAnUnmatchedClosingBracket() {
        // Environment templates continue the `{` of the `\begin{` the user typed.
        XCTAssertTrue(EKH.supersedesTrackedCloser("proof}\n\n\\end{proof}", closer: "}"))
        XCTAssertTrue(EKH.supersedesTrackedCloser(Completion.environmentSnippet("itemize", indent: "").text, closer: "}"))
        XCTAssertTrue(EKH.supersedesTrackedCloser(Completion.environmentSnippet("figure", indent: "  ").text, closer: "}"),
                      "the bracketed `[width=…]` and the later braces are all balanced; the leading `}` is not")
        XCTAssertTrue(EKH.supersedesTrackedCloser("itemize}", closer: "}"), "the plain `\\end{` insertion")
        // Balanced snippets close only what they opened.
        XCTAssertFalse(EKH.supersedesTrackedCloser("\\frac{}{}", closer: "}"))
        XCTAssertFalse(EKH.supersedesTrackedCloser("\\section{}", closer: "}"))
        XCTAssertFalse(EKH.supersedesTrackedCloser("\\left( \\right)", closer: ")"))
        XCTAssertFalse(EKH.supersedesTrackedCloser("alpha", closer: "}"), "no delimiter at all")
        // The unmatched closer has to be the one actually sitting there.
        XCTAssertFalse(EKH.supersedesTrackedCloser("proof}", closer: "]"))
        XCTAssertTrue(EKH.supersedesTrackedCloser("opt]", closer: "]"))
        // An escaped brace is a literal, not a delimiter.
        XCTAssertFalse(EKH.supersedesTrackedCloser("\\}", closer: "}"))
        XCTAssertTrue(EKH.supersedesTrackedCloser("\\{\\}}", closer: "}"), "the escapes are literals; the last `}` is unmatched")
        // Self-partnered delimiters cannot be counted, so they are never eaten.
        XCTAssertFalse(EKH.supersedesTrackedCloser("x$", closer: "$"))
        XCTAssertFalse(EKH.supersedesTrackedCloser("x}", closer: "\\"))
        // Crossed brackets are not a shape this understands: change nothing.
        XCTAssertFalse(EKH.supersedesTrackedCloser("{a]", closer: "]"))
    }
    // MARK: duplicate line (⌥⇧↓ / ⌥⇧↑)

    /// Applies the edit the way the text view does, so the test asserts on
    /// resulting text rather than on an offset arithmetic restatement.
    private func duplicated(_ text: String, _ range: NSRange, below: Bool) -> (text: String, selection: NSRange)? {
        guard let (edit, selection) = EKH.duplicateLinesEdit(in: text, range: range, below: below) else { return nil }
        let out = (text as NSString).replacingCharacters(in: edit.range, with: edit.replacement)
        return (out, selection)
    }

    func testDuplicateLineDownCopiesTheCaretsLineAndFollowsTheCopy() {
        let text = "\\alpha\n\\beta\n\\gamma\n"
        // caret inside `\beta`, at the `e`
        let r = duplicated(text, NSRange(location: 9, length: 0), below: true)
        XCTAssertEqual(r?.text, "\\alpha\n\\beta\n\\beta\n\\gamma\n")
        XCTAssertEqual(r?.selection, NSRange(location: 15, length: 0), "the caret keeps its column on the copy")
    }

    func testDuplicateLineUpLeavesTheCaretOnTheUpperCopy() {
        let text = "\\alpha\n\\beta\n\\gamma\n"
        let r = duplicated(text, NSRange(location: 9, length: 0), below: false)
        XCTAssertEqual(r?.text, "\\alpha\n\\beta\n\\beta\n\\gamma\n",
                       "same text either way; only which copy the caret is on differs")
        XCTAssertEqual(r?.selection, NSRange(location: 9, length: 0),
                       "inserting above leaves the original offsets describing the copy")
    }

    func testRepeatedDuplicateDownStacksCopies() {
        var text = "x = 1\n"
        var sel = NSRange(location: 2, length: 0)
        for _ in 0..<3 {
            guard let r = duplicated(text, sel, below: true) else { return XCTFail("no edit") }
            text = r.text
            sel = r.selection
        }
        XCTAssertEqual(text, "x = 1\nx = 1\nx = 1\nx = 1\n")
        XCTAssertEqual(sel, NSRange(location: 20, length: 0), "still the same column, three lines down")
    }

    func testDuplicateCopiesEveryLineASelectionTouches() {
        let text = "a\nb\nc\n"
        // selection from the start of `a` through the start of `c`: touches a and b
        let r = duplicated(text, NSRange(location: 0, length: 4), below: true)
        XCTAssertEqual(r?.text, "a\nb\na\nb\nc\n")
        XCTAssertEqual(r?.selection, NSRange(location: 4, length: 4), "the selection moves onto the copy")
    }

    func testDuplicateTheLastLineSuppliesTheNewlineItHasNot() {
        let down = duplicated("a\nb", NSRange(location: 3, length: 0), below: true)
        XCTAssertEqual(down?.text, "a\nb\nb")
        XCTAssertEqual(down?.selection, NSRange(location: 5, length: 0))
        let up = duplicated("a\nb", NSRange(location: 3, length: 0), below: false)
        XCTAssertEqual(up?.text, "a\nb\nb")
        XCTAssertEqual(up?.selection, NSRange(location: 3, length: 0))
    }

    func testDuplicateAnEmptyLineAndAnEmptyDocument() {
        XCTAssertEqual(duplicated("a\n\nb\n", NSRange(location: 2, length: 0), below: true)?.text, "a\n\n\nb\n")
        XCTAssertEqual(duplicated("", NSRange(location: 0, length: 0), below: true)?.text, "\n")
    }

    func testDuplicateRejectsARangeOutsideTheText() {
        XCTAssertNil(EKH.duplicateLinesEdit(in: "ab", range: NSRange(location: 3, length: 0), below: true))
        XCTAssertNil(EKH.duplicateLinesEdit(in: "ab", range: NSRange(location: 0, length: 5), below: true))
    }

    /// Review-found: a caret at EOF on an unterminated last line must land on
    /// the copy (the same column, which is the copy's end), not past it.
    func testDuplicateUnterminatedLastLineSelectsTheCopy() {
        let text = "aa\nbb"
        let r = duplicated(text, NSRange(location: 5, length: 0), below: true)
        XCTAssertEqual(r?.text, "aa\nbb\nbb")
        let copy = NSRange(location: 6, length: 2)
        let sel = r?.selection
        XCTAssertNotNil(sel)
        XCTAssertGreaterThanOrEqual(sel?.location ?? -1, copy.location)
        XCTAssertLessThanOrEqual(sel.map { NSMaxRange($0) } ?? -1, NSMaxRange(copy))
        XCTAssertEqual(sel?.length, 0)
    }

    /// Review-found: a multi-line selection stays on the copied block, not
    /// the original.
    func testDuplicateMultiLineSelectionIsKeptOnTheCopy() {
        let text = "aa\nbb\ncc\n"
        let sel = (text as NSString).range(of: "bb\ncc")
        guard let r = duplicated(text, sel, below: true) else { return XCTFail("no edit") }
        XCTAssertEqual(r.text, "aa\nbb\ncc\nbb\ncc\n")
        XCTAssertEqual(r.selection.length, sel.length)
        XCTAssertEqual((r.text as NSString).substring(with: r.selection), "bb\ncc")
    }

}
