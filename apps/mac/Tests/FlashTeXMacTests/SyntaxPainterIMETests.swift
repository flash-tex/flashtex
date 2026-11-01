import AppKit
import XCTest
@testable import FlashTeXMac

/// GH#780: `SyntaxPainter.flush()` must not re-queue itself while marked
/// text exists, and the kept dirty range must still be painted after every
/// way a composition ends (commit via `insertText`, cancel via
/// `setMarkedText("")` + `unmarkText`, teardown via a bare `unmarkText` that
/// keeps the text).
///
/// Drives the real `NSTextInputClient` path on a bare `NSTextView` +
/// `SyntaxPainter` — no helper, no window (the painter's whole-text window
/// covers a windowless view, and temporary attributes need no drawing).
@MainActor
final class SyntaxPainterIMETests: XCTestCase {
    static let original = "\\begin{document}\nHello world.\n\\end{document}\n"
    /// UTF-16 offset right after "Hello " (all ASCII up to there).
    static let insertAt = ("\\begin{document}\nHello " as NSString).length
    static let noReplacement = NSRange(location: NSNotFound, length: 0)

    private func makePair() -> (NSTextView, SyntaxPainter) {
        let tv = NSTextView(frame: NSRect(x: 0, y: 0, width: 400, height: 400))
        tv.string = Self.original
        let painter = SyntaxPainter()
        painter.attach(tv)
        return (tv, painter)
    }

    /// One composition step, like `IMEHarness.compose`.
    private func compose(_ tv: NSTextView, _ text: String) {
        tv.setMarkedText(text as NSString,
                         selectedRange: NSRange(location: (text as NSString).length, length: 0),
                         replacementRange: Self.noReplacement)
    }

    /// Lets the painter's queued `DispatchQueue.main.async` flushes run.
    private func drain(_ seconds: TimeInterval = 0.3) {
        RunLoop.main.run(until: Date(timeIntervalSinceNow: seconds))
    }

    private func text(of tv: NSTextView) -> NSString {
        tv.textStorage?.string as NSString? ?? ""
    }

    /// Temporary colour the painter applied at `index`, if any.
    private func paintedColor(at index: Int, in tv: NSTextView) -> NSColor? {
        guard let lm = tv.layoutManager else { return nil }
        return lm.temporaryAttribute(SyntaxPainter.key, atCharacterIndex: index, effectiveRange: nil) as? NSColor
    }

    // MARK: no polling while marked text exists

    func testFlushDoesNotRescheduleWhileMarkedTextExists() {
        let (tv, painter) = makePair()
        let paintsBefore = painter.paints
        XCTAssertGreaterThan(paintsBefore, 0, "attach paints the window, establishing the baseline")

        tv.setSelectedRange(NSRange(location: Self.insertAt, length: 0))
        compose(tv, "にほん")
        XCTAssertTrue(tv.hasMarkedText())
        drain()
        XCTAssertGreaterThan(painter.deferredFlushes, 0, "the scheduled flush ran and deferred on marked text")
        XCTAssertNotNil(painter.pendingDirty, "the dirty range is kept while marked text exists")
        XCTAssertEqual(painter.paints, paintsBefore, "nothing is painted while marked text exists")

        // Several idle main-queue turns with no new edits: with the old
        // self-re-scheduling flush this count grew once per turn, forever.
        let deferred = painter.deferredFlushes
        drain(0.5)
        XCTAssertEqual(painter.deferredFlushes, deferred, "no new flush ran while the composition sat idle")
        XCTAssertEqual(painter.paints, paintsBefore, "still nothing painted while marked text exists")
        XCTAssertTrue(tv.hasMarkedText(), "the composition itself is untouched by the idle turns")
    }

    // MARK: the paint still lands however the composition ends

    func testCommitPaintsTheCommittedText() {
        let (tv, painter) = makePair()
        tv.setSelectedRange(NSRange(location: Self.insertAt, length: 0))
        compose(tv, "よみ")
        drain()
        XCTAssertNotNil(painter.pendingDirty)

        // Commit fresh command text (nothing like it existed at baseline, so
        // its colour can only come from a post-commit paint).
        tv.insertText("\\cite{key}", replacementRange: Self.noReplacement)
        XCTAssertFalse(tv.hasMarkedText())
        drain()
        XCTAssertNil(painter.pendingDirty, "the commit flushed the kept dirty range")
        XCTAssertTrue(painter.inSync(with: text(of: tv)), "the lexer describes the committed text")
        XCTAssertNotNil(paintedColor(at: Self.insertAt + 1, in: tv),
                        "the committed command is highlighted after the commit")
    }

    func testCancelPaintsAndResyncsTheLexer() {
        let (tv, painter) = makePair()
        tv.setSelectedRange(NSRange(location: Self.insertAt, length: 0))
        compose(tv, "漢")
        drain()
        XCTAssertTrue(tv.hasMarkedText())

        // Cancel exactly like `IMEHarness.cancel`: the empty `setMarkedText`
        // removes the marked characters through a character storage edit
        // (editedCharacters, delta -1), then `unmarkText` is a no-op.
        compose(tv, "")
        tv.unmarkText()
        XCTAssertFalse(tv.hasMarkedText())
        XCTAssertEqual(tv.string, Self.original, "the cancel reverted the text")
        drain()
        XCTAssertNil(painter.pendingDirty, "the cancel flushed the kept dirty range")
        XCTAssertTrue(painter.inSync(with: text(of: tv)),
                      "the lexer picked up the cancel's length-changing edit instead of going stale")
        XCTAssertNotNil(paintedColor(at: 1, in: tv), "the reverted commands are highlighted after the cancel")
    }

    func testBareUnmarkPaintsAndResyncsTheLexer() {
        let (tv, painter) = makePair()
        tv.setSelectedRange(NSRange(location: Self.insertAt, length: 0))
        compose(tv, "にほん")
        drain()
        XCTAssertTrue(tv.hasMarkedText())

        // The composition ends with the text kept but unmarked and no further
        // character-typed edit: `unmarkText` alone strips the marked state
        // through storage edits the painter must already observe.
        tv.unmarkText()
        XCTAssertFalse(tv.hasMarkedText())
        XCTAssertEqual(tv.string, (Self.original as NSString).replacingCharacters(
            in: NSRange(location: Self.insertAt, length: 0), with: "にほん"))
        drain()
        XCTAssertNil(painter.pendingDirty, "the unmark flushed the kept dirty range")
        XCTAssertTrue(painter.inSync(with: text(of: tv)), "the lexer describes the unmarked text")
        XCTAssertNotNil(paintedColor(at: 1, in: tv), "the commands are highlighted after the unmark")
    }
}
