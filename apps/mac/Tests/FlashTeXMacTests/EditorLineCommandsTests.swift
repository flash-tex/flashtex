import AppKit
import XCTest
@testable import FlashTeXMac

/// Pure line-command plans plus the hosted CompletingTextView undo/selection
/// path (lane editor-line-commands, EditorLineCommands.swift).
final class EditorLineCommandsTests: XCTestCase {
    typealias ELC = EditorLineCommands

    func apply(_ plan: ELC.Plan, to text: String) -> String {
        (text as NSString).replacingCharacters(in: plan.range, with: plan.replacement)
    }

    func caret(_ text: String, at needle: String) -> NSRange {
        let r = (text as NSString).range(of: needle)
        return NSRange(location: r.location, length: 0)
    }

    func span(_ text: String, of needle: String) -> NSRange {
        (text as NSString).range(of: needle)
    }

    // MARK: empty buffer

    func testEmptyBufferIsANoOpForEveryCommand() {
        let sel = NSRange(location: 0, length: 0)
        XCTAssertNil(ELC.duplicate(in: "", selection: sel))
        XCTAssertNil(ELC.move(in: "", selection: sel, down: true))
        XCTAssertNil(ELC.move(in: "", selection: sel, down: false))
        XCTAssertNil(ELC.deleteLines(in: "", selection: sel))
        XCTAssertNil(ELC.joinLines(in: "", selection: sel))
        XCTAssertNil(ELC.sortLines(in: "", selection: sel, descending: false))
        XCTAssertNil(ELC.trimTrailingWhitespace(in: "", selection: sel))
    }

    // MARK: duplicate

    func testDuplicateLineInsertsACopyBelowAndSelectsTheCopy() {
        let text = "aa\nbb\ncc\n"
        let plan = ELC.duplicate(in: text, selection: caret(text, at: "bb"))!
        let out = apply(plan, to: text)
        XCTAssertEqual(out, "aa\nbb\nbb\ncc\n")
        XCTAssertEqual((out as NSString).substring(with: NSRange(location: plan.selection.location, length: 2)), "bb")
    }

    func testDuplicateMultiLineSelectionDuplicatesEveryTouchedLine() {
        let text = "aa\nbb\ncc\n"
        let plan = ELC.duplicate(in: text, selection: span(text, of: "bb\ncc"))!
        XCTAssertEqual(apply(plan, to: text), "aa\nbb\ncc\nbb\ncc\n")
    }

    func testDuplicateLastLineWithoutTrailingNewlineInsertsASeparator() {
        let text = "aa\nbb"
        let plan = ELC.duplicate(in: text, selection: caret(text, at: "bb"))!
        XCTAssertEqual(apply(plan, to: text), "aa\nbb\nbb")
        XCTAssertEqual((apply(plan, to: text) as NSString).substring(with: NSRange(location: plan.selection.location, length: 2)), "bb")
    }

    func testDuplicatePreservesCRLF() {
        let text = "aa\r\nbb\r\n"
        let plan = ELC.duplicate(in: text, selection: caret(text, at: "aa"))!
        let out = apply(plan, to: text)
        XCTAssertEqual(out, "aa\r\naa\r\nbb\r\n")
        XCTAssertTrue(out.contains("\r\n"))
        XCTAssertFalse(out.contains("\n\n"))
    }

    // MARK: move

    func testMoveLineDownSwapsWithTheNextLineAndKeepsTheSelection() {
        let text = "aa\nbb\ncc\n"
        let plan = ELC.move(in: text, selection: caret(text, at: "aa"), down: true)!
        let out = apply(plan, to: text)
        XCTAssertEqual(out, "bb\naa\ncc\n")
        XCTAssertEqual((out as NSString).substring(with: NSRange(location: plan.selection.location, length: 2)), "aa")
    }

    func testMoveLineUpSwapsWithThePreviousLine() {
        let text = "aa\nbb\ncc\n"
        let plan = ELC.move(in: text, selection: caret(text, at: "cc"), down: false)!
        XCTAssertEqual(apply(plan, to: text), "aa\ncc\nbb\n")
    }

    func testMoveMultiLineBlockMovesEveryTouchedLineTogether() {
        let text = "aa\nbb\ncc\ndd\n"
        let plan = ELC.move(in: text, selection: span(text, of: "bb\ncc"), down: true)!
        XCTAssertEqual(apply(plan, to: text), "aa\ndd\nbb\ncc\n")
    }

    func testMoveIsNoOpAtBufferEdges() {
        let text = "aa\nbb\ncc\n"
        XCTAssertNil(ELC.move(in: text, selection: caret(text, at: "aa"), down: false))
        XCTAssertNil(ELC.move(in: text, selection: caret(text, at: "cc"), down: true))
        let last = "aa\nbb"
        XCTAssertNil(ELC.move(in: last, selection: caret(last, at: "bb"), down: true))
    }

    func testMoveNeverSplitsABeginOrEndLine() {
        let text = "x\n\\begin{itemize} hello\n\\end{itemize}\n"
        let plan = ELC.move(in: text, selection: caret(text, at: "begin"), down: true)!
        let out = apply(plan, to: text)
        XCTAssertEqual(out, "x\n\\end{itemize}\n\\begin{itemize} hello\n")
        XCTAssertTrue(out.contains("\\begin{itemize} hello"))
        XCTAssertEqual(out.components(separatedBy: "\\begin{itemize}").count, 2)
    }

    func testMoveLastLineWithoutTrailingNewline() {
        let text = "aa\nbb"
        let plan = ELC.move(in: text, selection: caret(text, at: "bb"), down: false)!
        XCTAssertEqual(apply(plan, to: text), "bb\naa")
    }

    func testMovePreservesCRLF() {
        let text = "aa\r\nbb\r\ncc\r\n"
        let plan = ELC.move(in: text, selection: caret(text, at: "aa"), down: true)!
        XCTAssertEqual(apply(plan, to: text), "bb\r\naa\r\ncc\r\n")
    }

    // MARK: delete

    func testDeleteLineRemovesTheTouchedLine() {
        let text = "aa\nbb\ncc\n"
        let plan = ELC.deleteLines(in: text, selection: caret(text, at: "bb"))!
        XCTAssertEqual(apply(plan, to: text), "aa\ncc\n")
        XCTAssertEqual(plan.selection.length, 0)
    }

    func testDeleteMultiLineSelection() {
        let text = "aa\nbb\ncc\ndd\n"
        let plan = ELC.deleteLines(in: text, selection: span(text, of: "bb\ncc"))!
        XCTAssertEqual(apply(plan, to: text), "aa\ndd\n")
    }

    func testDeleteLastLineWithoutTrailingNewline() {
        let text = "aa\nbb"
        let plan = ELC.deleteLines(in: text, selection: caret(text, at: "bb"))!
        XCTAssertEqual(apply(plan, to: text), "aa")
    }

    func testDeleteWholeBuffer() {
        let text = "only\n"
        let plan = ELC.deleteLines(in: text, selection: caret(text, at: "only"))!
        XCTAssertEqual(apply(plan, to: text), "")
    }

    func testDeletePreservesCRLF() {
        let text = "aa\r\nbb\r\ncc\r\n"
        let plan = ELC.deleteLines(in: text, selection: caret(text, at: "bb"))!
        XCTAssertEqual(apply(plan, to: text), "aa\r\ncc\r\n")
    }

    // MARK: join

    func testJoinLinesInsertsOneSpaceAndStripsNextLeadingWhitespace() {
        let text = "foo\n   bar\n"
        let plan = ELC.joinLines(in: text, selection: caret(text, at: "foo"))!
        XCTAssertEqual(apply(plan, to: text), "foo bar\n")
    }

    func testJoinStripsATrailingPercentCommentMarkerOnlyWhenItEndsTheLine() {
        XCTAssertEqual(ELC.stripTrailingPercentIfLineComment("foo %"), "foo ")
        XCTAssertEqual(ELC.stripTrailingPercentIfLineComment("foo%"), "foo")
        XCTAssertEqual(ELC.stripTrailingPercentIfLineComment("foo %  "), "foo ")
        XCTAssertEqual(ELC.stripTrailingPercentIfLineComment("foo % comment"), "foo % comment")
        XCTAssertEqual(ELC.stripTrailingPercentIfLineComment("foo\\%"), "foo\\%")
        let text = "foo %\n  bar\n"
        let plan = ELC.joinLines(in: text, selection: caret(text, at: "foo"))!
        XCTAssertEqual(apply(plan, to: text), "foo bar\n")
        let kept = "foo % still\n  bar\n"
        XCTAssertEqual(apply(ELC.joinLines(in: kept, selection: caret(kept, at: "foo"))!, to: kept), "foo % still bar\n")
    }

    func testJoinMultiLineSelectionJoinsEveryTouchedLine() {
        let text = "a\n  b\n\tc\n"
        let plan = ELC.joinLines(in: text, selection: NSRange(location: 0, length: (text as NSString).length))!
        XCTAssertEqual(apply(plan, to: text), "a b c\n")
    }

    func testJoinOnLastLineIsANoOp() {
        XCTAssertNil(ELC.joinLines(in: "aa\nbb", selection: caret("aa\nbb", at: "bb")))
        XCTAssertNil(ELC.joinLines(in: "only", selection: NSRange(location: 0, length: 0)))
    }

    func testJoinLastLineWithoutTrailingNewline() {
        let text = "aa\nbb"
        let plan = ELC.joinLines(in: text, selection: caret(text, at: "aa"))!
        XCTAssertEqual(apply(plan, to: text), "aa bb")
    }

    func testJoinPreservesCRLF() {
        let text = "aa\r\nbb\r\n"
        let plan = ELC.joinLines(in: text, selection: caret(text, at: "aa"))!
        XCTAssertEqual(apply(plan, to: text), "aa bb\r\n")
    }

    // MARK: sort

    func testSortLinesAscendingIsStableAndLocaleAware() {
        let text = "zoo\näpple\napple\nzoo\n"
        let plan = ELC.sortLines(in: text, selection: NSRange(location: 0, length: (text as NSString).length), descending: false)!
        let out = apply(plan, to: text)
        let expected = ["zoo", "äpple", "apple", "zoo"].enumerated().sorted { a, b in
            let c = a.element.localizedStandardCompare(b.element)
            if c == .orderedSame { return a.offset < b.offset }
            return c == .orderedAscending
        }.map(\.element)
        XCTAssertEqual(out.split(separator: "\n", omittingEmptySubsequences: false).dropLast().map(String.init), expected)
        // The two identical "zoo" rows survive; enumerated offsets keep the sort stable.
        XCTAssertEqual(out.components(separatedBy: "zoo").count - 1, 2)
    }

    func testSortLinesDescending() {
        let text = "b\na\nc\n"
        let plan = ELC.sortLines(in: text, selection: NSRange(location: 0, length: (text as NSString).length), descending: true)!
        XCTAssertEqual(apply(plan, to: text), "c\nb\na\n")
    }

    func testSortTouchedLinesOnlyLeavesTheRestAlone() {
        let text = "keep\ncc\naa\nkeep2\n"
        let plan = ELC.sortLines(in: text, selection: span(text, of: "cc\naa"), descending: false)!
        XCTAssertEqual(apply(plan, to: text), "keep\naa\ncc\nkeep2\n")
    }

    func testSortSingleLineIsANoOp() {
        XCTAssertNil(ELC.sortLines(in: "aa\nbb\n", selection: caret("aa\nbb\n", at: "aa"), descending: false))
    }

    func testSortLastLineWithoutTrailingNewline() {
        let text = "c\na\nb"
        let plan = ELC.sortLines(in: text, selection: NSRange(location: 0, length: (text as NSString).length), descending: false)!
        XCTAssertEqual(apply(plan, to: text), "a\nb\nc")
    }

    func testSortPreservesCRLF() {
        let text = "c\r\na\r\nb\r\n"
        let plan = ELC.sortLines(in: text, selection: NSRange(location: 0, length: (text as NSString).length), descending: false)!
        XCTAssertEqual(apply(plan, to: text), "a\r\nb\r\nc\r\n")
    }

    // MARK: trim

    func testTrimTrailingWhitespaceOfTheWholeDocument() {
        let text = "aa  \nbb\t\ncc\n"
        let plan = ELC.trimTrailingWhitespace(in: text, selection: NSRange(location: 0, length: 0))!
        XCTAssertEqual(apply(plan, to: text), "aa\nbb\ncc\n")
    }

    func testTrimPreservesVerbatimBodies() {
        let text = "keep  \n\\begin{verbatim}\n  spaced  \n\\end{verbatim}\ntrail  \n"
        let plan = ELC.trimTrailingWhitespace(in: text, selection: NSRange(location: 0, length: 0))!
        let out = apply(plan, to: text)
        XCTAssertTrue(out.contains("\\begin{verbatim}\n  spaced  \n\\end{verbatim}"))
        XCTAssertTrue(out.hasPrefix("keep\n"))
        XCTAssertTrue(out.contains("\ntrail\n"))
    }

    func testTrimPreservesStarredAndListingsVerbatimNames() {
        for env in ["verbatim*", "Verbatim", "lstlisting", "minted"] {
            let text = "\\begin{\(env)}\n  keep  \n\\end{\(env)}\n"
            XCTAssertNil(ELC.trimTrailingWhitespace(in: text, selection: NSRange(location: 0, length: 0)), env)
        }
    }

    func testTrimPreservesALineThatIsOnlyBackslashBackslashPlusSpaces() {
        let text = "aa  \n\\\\   \nbb\t\n"
        let plan = ELC.trimTrailingWhitespace(in: text, selection: NSRange(location: 0, length: 0))!
        XCTAssertEqual(apply(plan, to: text), "aa\n\\\\   \nbb\n")
    }

    func testTrimAlreadyCleanIsANoOp() {
        XCTAssertNil(ELC.trimTrailingWhitespace(in: "aa\nbb\n", selection: NSRange(location: 0, length: 0)))
    }

    func testTrimLastLineWithoutTrailingNewline() {
        let text = "aa  \nbb  "
        let plan = ELC.trimTrailingWhitespace(in: text, selection: NSRange(location: 0, length: 0))!
        XCTAssertEqual(apply(plan, to: text), "aa\nbb")
    }

    func testTrimPreservesCRLF() {
        let text = "aa  \r\nbb\t\r\n"
        let plan = ELC.trimTrailingWhitespace(in: text, selection: NSRange(location: 0, length: 0))!
        XCTAssertEqual(apply(plan, to: text), "aa\r\nbb\r\n")
    }

    // MARK: replacement span

    func testTrimmedReplacementDropsCommonUTF16Affixes() {
        let old = "aaaXbbb"
        let new = "aaaYbbb"
        let trimmed = ELC.trimmedReplacement(old: old, new: new,
                                             range: NSRange(location: 10, length: (old as NSString).length))!
        XCTAssertEqual(trimmed.range, NSRange(location: 13, length: 1))
        XCTAssertEqual(trimmed.replacement, "Y")
    }

    func testDuplicateReplacesOnlyTheChangedSpan() {
        let text = "aa\nbb\ncc\n"
        let plan = ELC.duplicate(in: text, selection: caret(text, at: "bb"))!
        XCTAssertGreaterThan(plan.range.location, 0, "common prefix 'aa\\n' is not rewritten")
        XCTAssertLessThan(NSMaxRange(plan.range), (text as NSString).length, "common suffix is not rewritten")
    }
}

@MainActor
final class EditorLineCommandsHostTests: XCTestCase {
    func host(_ text: String) throws -> (NSWindow, CompletingTextView) {
        HostedWindowSupport.prepare()
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
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

    func testDuplicateIsOneUndoStepAndKeepsTheSelectionOnTheCopy() throws {
        let (window, tv) = try host("aa\nbb\ncc\n")
        defer { window.orderOut(nil) }
        let b = (tv.string as NSString).range(of: "bb")
        tv.setSelectedRange(NSRange(location: b.location, length: 0))
        tv.undoManager?.removeAllActions()
        tv.duplicateLines(nil)
        XCTAssertEqual(tv.string, "aa\nbb\nbb\ncc\n")
        XCTAssertEqual((tv.string as NSString).substring(with: NSRange(location: tv.selectedRange().location, length: 2)), "bb")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Duplicate Line")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "aa\nbb\ncc\n")
        tv.undoManager?.redo()
        XCTAssertEqual(tv.string, "aa\nbb\nbb\ncc\n")
    }

    func testMoveJoinDeleteSortTrimAreEachOneUndoStep() throws {
        let (window, tv) = try host("cc  \naa\nbb\n")
        defer { window.orderOut(nil) }

        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.undoManager?.removeAllActions()
        tv.moveLinesDown(nil)
        XCTAssertEqual(tv.string, "aa\ncc  \nbb\n")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Move Line Down")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "cc  \naa\nbb\n")

        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.undoManager?.removeAllActions()
        tv.joinSelectedLines(nil)
        XCTAssertEqual(tv.string, "cc aa\nbb\n")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Join Lines")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "cc  \naa\nbb\n")

        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.undoManager?.removeAllActions()
        tv.deleteLines(nil)
        XCTAssertEqual(tv.string, "aa\nbb\n")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Delete Line")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "cc  \naa\nbb\n")

        tv.setSelectedRange(NSRange(location: 0, length: (tv.string as NSString).length))
        tv.undoManager?.removeAllActions()
        tv.sortLinesAscending(nil)
        XCTAssertEqual(tv.string, "aa\nbb\ncc  \n")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Sort Lines")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "cc  \naa\nbb\n")

        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.undoManager?.removeAllActions()
        tv.trimTrailingWhitespace(nil)
        XCTAssertEqual(tv.string, "cc\naa\nbb\n")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Trim Trailing Whitespace")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "cc  \naa\nbb\n")
    }

    func testEmptyBufferHostedCommandsAreNoOpsWithoutUndo() throws {
        let (window, tv) = try host("")
        defer { window.orderOut(nil) }
        tv.undoManager?.removeAllActions()
        tv.duplicateLines(nil)
        tv.moveLinesUp(nil)
        tv.deleteLines(nil)
        tv.joinSelectedLines(nil)
        tv.sortLinesAscending(nil)
        tv.trimTrailingWhitespace(nil)
        XCTAssertEqual(tv.string, "")
        XCTAssertEqual(tv.undoManager?.canUndo, false)
    }

    func testMoveUpAtTopIsNoOpWithoutUndo() throws {
        let (window, tv) = try host("aa\nbb\n")
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.undoManager?.removeAllActions()
        tv.moveLinesUp(nil)
        XCTAssertEqual(tv.string, "aa\nbb\n")
        XCTAssertEqual(tv.undoManager?.canUndo, false)
    }
}
