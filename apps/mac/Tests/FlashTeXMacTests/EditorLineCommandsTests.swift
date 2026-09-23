import AppKit
import HostedWindows
import XCTest
@testable import FlashTeXMac

/// Pure line-command plans plus the hosted CompletingTextView undo/selection
/// path (lane editor-line-commands). Duplicate uses main's
/// `EditorKeyHandling.duplicateLinesEdit` / `duplicateLines(below:)`.
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
        XCTAssertNil(ELC.move(in: "", selection: sel, down: true))
        XCTAssertNil(ELC.move(in: "", selection: sel, down: false))
        XCTAssertNil(ELC.deleteLines(in: "", selection: sel))
        XCTAssertNil(ELC.joinLines(in: "", selection: sel))
        XCTAssertNil(ELC.sortLines(in: "", selection: sel, descending: false))
        XCTAssertNil(ELC.trimTrailingWhitespace(in: "", selection: sel))
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
        let sel = span(text, of: "bb\ncc")
        let plan = ELC.move(in: text, selection: sel, down: true)!
        let out = apply(plan, to: text)
        XCTAssertEqual(out, "aa\ndd\nbb\ncc\n")
        XCTAssertEqual(plan.selection, NSRange(location: 6, length: sel.length))
        XCTAssertEqual((out as NSString).substring(with: plan.selection), "bb\ncc")
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

    func testJoinMultiLineTrimsTrailingWhitespaceOnEveryNonLastLine() {
        let text = "a\nb  \nc\n"
        let plan = ELC.joinLines(in: text, selection: NSRange(location: 0, length: (text as NSString).length))!
        XCTAssertEqual(apply(plan, to: text), "a b c\n")
    }

    func testJoinStripsATrailingPercentOnMiddleLinesTheSameAsTheFirst() {
        // Same rule as the left-hand line of a two-line join: a `%` that ends
        // the line (optional trailing space, not escaped) is dropped, because
        // keeping it would comment out every subsequent survivor. A `%` that
        // does not end the line is kept, even though the rest of the join
        // then sits in that comment.
        let stripped = "a\nb %\nc\n"
        XCTAssertEqual(apply(ELC.joinLines(in: stripped, selection: NSRange(location: 0, length: (stripped as NSString).length))!, to: stripped), "a b c\n")
        let escaped = "a\nb \\%\nc\n"
        XCTAssertEqual(apply(ELC.joinLines(in: escaped, selection: NSRange(location: 0, length: (escaped as NSString).length))!, to: escaped), "a b \\% c\n")
        let kept = "a\nb % still\nc\n"
        XCTAssertEqual(apply(ELC.joinLines(in: kept, selection: NSRange(location: 0, length: (kept as NSString).length))!, to: kept), "a b % still c\n")
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

    func testTrimMapsSelectionBySubtractingRemovalsThatEndAtOrBeforeIt() {
        // "aa  \nbb  \n" — removals [2,4) and [7,9). After trim: "aa\nbb\n".
        let text = "aa  \nbb  \n"
        func loc(_ n: Int) -> Int {
            ELC.trimTrailingWhitespace(in: text, selection: NSRange(location: n, length: 0))!.selection.location
        }
        XCTAssertEqual(loc(0), 0)
        XCTAssertEqual(loc(5), 3, "caret at start of bb (offset 5) must shift by the two spaces removed on line 1")
        XCTAssertEqual(loc(3), 2, "inside the first trailing-whitespace run clamps to that run's start")
        XCTAssertEqual(loc(6), 4)
        XCTAssertEqual(loc(8), 5, "inside the second trailing-whitespace run clamps to 7, then subtracts the first removal")
        XCTAssertEqual(loc(9), 5)
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
}

@MainActor
final class EditorLineCommandsHostTests: XCTestCase {
    func host(_ text: String) throws -> (NSWindow, CompletingTextView) {
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
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
        tv.duplicateLines(below: true)
        XCTAssertEqual(tv.string, "aa\nbb\nbb\ncc\n")
        XCTAssertEqual((tv.string as NSString).substring(with: NSRange(location: tv.selectedRange().location, length: 2)), "bb")
        XCTAssertEqual(tv.undoManager?.undoActionName, "Duplicate Line")
        tv.undoManager?.undo()
        XCTAssertEqual(tv.string, "aa\nbb\ncc\n")
        tv.undoManager?.redo()
        XCTAssertEqual(tv.string, "aa\nbb\nbb\ncc\n")
    }

    func testDuplicateLastUnterminatedLineSelectsTheCopy() throws {
        let (window, tv) = try host("aa\nbb")
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 5, length: 0))
        tv.duplicateLines(below: true)
        XCTAssertEqual(tv.string, "aa\nbb\nbb")
        let sel = tv.selectedRange()
        let copy = NSRange(location: 6, length: 2)
        XCTAssertGreaterThanOrEqual(sel.location, copy.location, "on the copy, not the original line")
        XCTAssertLessThanOrEqual(NSMaxRange(sel), NSMaxRange(copy), "not past the copy")
        let line = (tv.string as NSString).lineRange(for: NSRange(location: min(sel.location, (tv.string as NSString).length), length: 0))
        XCTAssertEqual((tv.string as NSString).substring(with: line), "bb")
    }

    func testDuplicateMultiLineSelectionIsKeptOnTheCopy() throws {
        let (window, tv) = try host("aa\nbb\ncc\n")
        defer { window.orderOut(nil) }
        let sel = (tv.string as NSString).range(of: "bb\ncc")
        tv.setSelectedRange(sel)
        tv.duplicateLines(below: true)
        XCTAssertEqual(tv.string, "aa\nbb\ncc\nbb\ncc\n")
        XCTAssertEqual((tv.string as NSString).substring(with: tv.selectedRange()), "bb\ncc")
    }

    /// One ⌥⇧↓ must not apply twice if both the menu key-equivalent path
    /// (`performKeyEquivalent`) and `keyDown` see the event.
    func testOptionShiftDownProducesExactlyOneCopy() throws {
        let (window, tv) = try host("aa\n")
        defer { window.orderOut(nil) }
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        let event = try XCTUnwrap(NSEvent.keyEvent(
            with: .keyDown, location: .zero, modifierFlags: [.option, .shift],
            timestamp: ProcessInfo.processInfo.systemUptime,
            windowNumber: window.windowNumber, context: nil,
            characters: "\u{F701}", charactersIgnoringModifiers: "\u{F701}",
            isARepeat: false, keyCode: 125))
        _ = tv.performKeyEquivalent(with: event)
        tv.keyDown(with: event)
        XCTAssertEqual(tv.string, "aa\naa\n", "one keystroke, one copy")
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

    /// Making the line commands runnable puts Duplicate Line in the palette,
    /// and it out-ranked Go to Line for the query "go to line": its title
    /// supplies "line", and its description supplies "go" and "to" only
    /// because it ends "⇧⌘D remains Go to Matching". `rank` scored a row by
    /// its BEST-placed term, so that incidental description hit cost nothing
    /// and the row tied Go to Line -- whose title holds all three terms --
    /// then won on declaration order. Ranking by the worst-placed term is
    /// what makes a row that matches everything in its title win.
    func testPaletteRanksByTheWorstPlacedTermSoFullTitleMatchesWin() {
        XCTAssertTrue(CommandPaletteModel.isRunnable(.duplicateLine), "the line commands are runnable now")
        let rows = CommandPaletteModel.rows(matching: "go to line")
        XCTAssertEqual(rows.first?.id, .goToLine, "matched in full by the title: \(rows.prefix(3).map(\.id))")
        XCTAssertTrue(rows.contains { $0.id == .duplicateLine }, "still a match, just ranked below")
        XCTAssertLessThan(
            rows.firstIndex(where: { $0.id == .goToLine }) ?? .max,
            rows.firstIndex(where: { $0.id == .duplicateLine }) ?? .max
        )
        // A single term still ranks by where it lands, so the plain query is
        // unaffected: Duplicate Line keeps its title match.
        XCTAssertTrue(CommandPaletteModel.rows(matching: "duplicate").contains { $0.id == .duplicateLine })
    }
}
