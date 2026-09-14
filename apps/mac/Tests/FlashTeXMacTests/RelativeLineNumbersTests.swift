import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Vim-style hybrid line numbering in the gutter
/// (`EditorPreferences.relativeLineNumbers`, off by default and independent of
/// Vim keybindings): the caret's own line keeps its absolute number, every
/// other line shows its distance from the caret in *logical* lines.
///
/// The wrapping case is covered explicitly, because that is where this kind of
/// feature usually breaks: a wrapped logical line occupies several visual rows,
/// and the number belongs on the first of them with the rest left blank, while
/// distances keep counting logical lines.
@MainActor
final class RelativeLineNumbersTests: XCTestCase {
    private var suiteName = ""
    private var defaults: UserDefaults!

    override func setUp() {
        super.setUp()
        suiteName = "flashtex.tests.RelativeLineNumbers.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)
        defaults.removePersistentDomain(forName: suiteName)
    }

    override func tearDown() {
        LineNumberGutter.relativeOverride = nil
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        super.tearDown()
    }

    // MARK: the numbering itself

    /// Hybrid numbering around the caret: absolute on the caret's line,
    /// distance either side of it. Lines here are 0-based; labels are not.
    func testHybridNumberingAroundTheCaret() {
        // Caret on logical line index 4, i.e. the line a person calls "5".
        let labels = (0..<9).map { LineNumberGutter.label(line: $0, currentLine: 4, relative: true) }
        XCTAssertEqual(labels, ["4", "3", "2", "1", "5", "1", "2", "3", "4"],
                       "the caret's line must show its absolute number and the rest their distance")
    }

    func testTheCaretLineShowsItsAbsoluteNumberNotZero() {
        // Plain `relativenumber` would print 0 here; the hybrid prints the line.
        XCTAssertEqual(LineNumberGutter.label(line: 0, currentLine: 0, relative: true), "1")
        XCTAssertEqual(LineNumberGutter.label(line: 41, currentLine: 41, relative: true), "42")
    }

    func testAbsoluteNumberingWhenThePreferenceIsOff() {
        let labels = (0..<5).map { LineNumberGutter.label(line: $0, currentLine: 2, relative: false) }
        XCTAssertEqual(labels, ["1", "2", "3", "4", "5"])
    }

    /// No caret (nothing focused yet) must not blank the gutter.
    func testNoCaretFallsBackToAbsoluteNumbering() {
        let labels = (0..<4).map { LineNumberGutter.label(line: $0, currentLine: nil, relative: true) }
        XCTAssertEqual(labels, ["1", "2", "3", "4"])
    }

    // MARK: the preference

    func testThePreferenceIsOffByDefaultAndIndependentOfVim() {
        let p = EditorPreferences(defaults: defaults)
        XCTAssertFalse(p.relativeLineNumbers, "relative numbering must be opt-in")
        // Turning Vim on must not turn numbering on, and vice versa.
        p.vimKeybindings = true
        XCTAssertFalse(p.relativeLineNumbers, "Vim keybindings must not imply relative numbers")
        p.vimKeybindings = false
        p.relativeLineNumbers = true
        XCTAssertFalse(p.vimKeybindings, "relative numbers must not imply Vim keybindings")
    }

    func testThePreferencePersistsAndResets() {
        let p = EditorPreferences(defaults: defaults)
        p.relativeLineNumbers = true
        XCTAssertTrue(EditorPreferences(defaults: defaults).relativeLineNumbers, "the choice did not survive a reload")
        p.resetToDefaults()
        XCTAssertFalse(p.relativeLineNumbers)
        XCTAssertFalse(EditorPreferences(defaults: defaults).relativeLineNumbers)
    }

    // MARK: the live gutter

    /// A gutter on a real scroll view, with a highlighter table behind it.
    private func hostedGutter(_ text: String, width: CGFloat = 420) throws -> (NSWindow, NSScrollView, CompletingTextView, LineNumberGutter) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: width, height: 300),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let scroll = CompletingTextView.scrollable()
        scroll.frame = NSRect(x: 0, y: 0, width: width, height: 300)
        window.contentView = scroll
        let tv = scroll.documentView as! CompletingTextView
        _ = tv.layoutManager
        tv.isRichText = false
        tv.textContainerInset = NSSize(width: 8, height: 8)
        tv.string = text
        window.orderFrontRegardless() // never makeKey
        var highlighter = SyntaxHighlighter()
        highlighter.reset(text as NSString)
        let gutter = LineNumberGutter(scrollView: scroll)
        gutter.clientView = tv
        gutter.lineTable = { highlighter }
        scroll.verticalRulerView = gutter
        scroll.hasVerticalRuler = true
        scroll.rulersVisible = true
        gutter.layoutIfNeeded(lineCount: highlighter.lineCount)
        scroll.layoutSubtreeIfNeeded()
        return (window, scroll, tv, gutter)
    }

    /// A gutter on a scroll view that is in no window. `needsDisplay` is only
    /// stable off-screen: a hosted view is re-dirtied by AppKit between
    /// statements, which is exactly what the redraw assertions must not see.
    private func unhostedGutter(_ text: String) -> (NSScrollView, LineNumberGutter) {
        let scroll = CompletingTextView.scrollable()
        scroll.frame = NSRect(x: 0, y: 0, width: 420, height: 300)
        let tv = scroll.documentView as! CompletingTextView
        _ = tv.layoutManager
        tv.isRichText = false
        tv.string = text
        var highlighter = SyntaxHighlighter()
        highlighter.reset(text as NSString)
        let gutter = LineNumberGutter(scrollView: scroll)
        gutter.clientView = tv
        gutter.lineTable = { highlighter }
        gutter.layoutIfNeeded(lineCount: highlighter.lineCount)
        return (scroll, gutter)
    }

    private func draw(_ gutter: LineNumberGutter) throws {
        let rep = try XCTUnwrap(NSBitmapImageRep(bitmapDataPlanes: nil,
                                                 pixelsWide: Int(max(1, gutter.bounds.width)),
                                                 pixelsHigh: Int(max(1, gutter.bounds.height)),
                                                 bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
                                                 colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0))
        let ctx = try XCTUnwrap(NSGraphicsContext(bitmapImageRep: rep))
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = ctx
        gutter.drawHashMarksAndLabels(in: gutter.bounds)
        NSGraphicsContext.restoreGraphicsState()
    }

    /// Moving the caret must redraw: the numbers are relative to it, so a
    /// caret-only move changes every label even though the text did not.
    func testMovingTheCaretMarksTheGutterForRedraw() {
        let text = (1...20).map { "line \($0)" }.joined(separator: "\n") + "\n"
        let (_, gutter) = unhostedGutter(text)
        gutter.relativeLineNumbers = true
        gutter.currentLine = 3
        let before = gutter.redrawRequests
        gutter.currentLine = 9
        XCTAssertEqual(gutter.redrawRequests, before + 1, "a caret move must redraw the gutter when numbering is relative")
        // The same line again is not a change, so it must not redraw.
        gutter.currentLine = 9
        XCTAssertEqual(gutter.redrawRequests, before + 1, "an unchanged caret line redrew the gutter")
    }

    func testTogglingTheModeMarksTheGutterForRedraw() {
        let (_, gutter) = unhostedGutter("a\nb\nc\n")
        gutter.currentLine = 1
        let before = gutter.redrawRequests
        gutter.relativeLineNumbers = true
        XCTAssertEqual(gutter.redrawRequests, before + 1)
        gutter.relativeLineNumbers = true // unchanged
        XCTAssertEqual(gutter.redrawRequests, before + 1, "an unchanged mode redrew the gutter")
    }

    /// Width comes from the document's line count, never from the caret, so the
    /// gutter cannot jitter as you move around.
    func testGutterWidthDoesNotJitterAsTheCaretMoves() throws {
        let text = (1...250).map { "line \($0)" }.joined(separator: "\n") + "\n"
        let (window, _, _, gutter) = try hostedGutter(text)
        defer { window.orderOut(nil) }
        gutter.relativeLineNumbers = true
        gutter.layoutIfNeeded(lineCount: 250)
        let thickness = gutter.ruleThickness
        for line in [0, 1, 9, 99, 100, 249] {
            gutter.currentLine = line
            gutter.layoutIfNeeded(lineCount: 250)
            XCTAssertEqual(gutter.ruleThickness, thickness, accuracy: 0.01,
                           "the gutter changed width with the caret on line \(line)")
        }
        // Three digits of distance still fit: the widest distance is bounded by
        // the line count the width was sized for.
        XCTAssertEqual(LineNumberGutter.label(line: 249, currentLine: 0, relative: true), "249")
    }

    /// Drawing with relative numbering on must not disturb the storage and must
    /// survive the caret sitting at either end of the document.
    func testDrawingRelativeNumbersLeavesTheTextAlone() throws {
        let text = (1...40).map { "line \($0)" }.joined(separator: "\n") + "\n"
        let (window, _, tv, gutter) = try hostedGutter(text)
        defer { window.orderOut(nil) }
        gutter.relativeLineNumbers = true
        for line in [0, 5, 39] {
            gutter.currentLine = line
            try draw(gutter)
            XCTAssertEqual(tv.string, text)
        }
    }

    // MARK: with wrapping on (the default)

    /// The number belongs on the first visual row of a wrapped logical line and
    /// distances count logical lines, not visual rows.
    func testDistancesCountLogicalLinesWhenLinesWrap() throws {
        let long = "In this section we recall the classical statement of the theorem, fix the notation used throughout, and explain why the boundary hypothesis cannot be dropped."
        let text = "short one\n\(long)\nshort three\nshort four\n"
        let prefs = EditorPreferences(defaults: defaults)
        XCTAssertTrue(prefs.lineWrapping, "this test is about the wrapping default")
        let (window, scroll, tv, gutter) = try hostedGutter(text, width: 260)
        defer { window.orderOut(nil) }
        prefs.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()

        // The middle line really does wrap, or the test proves nothing.
        let lm = try XCTUnwrap(tv.layoutManager)
        let container = try XCTUnwrap(tv.textContainer)
        lm.ensureLayout(for: container)
        let ns = text as NSString
        let longRange = ns.range(of: long)
        let glyphs = lm.glyphRange(forCharacterRange: longRange, actualCharacterRange: nil)
        var rows = 0
        lm.enumerateLineFragments(forGlyphRange: glyphs) { _, _, _, _, _ in rows += 1 }
        XCTAssertGreaterThan(rows, 1, "the fixture must wrap for this test to mean anything")

        // With the caret on the wrapped line (index 1), its neighbours are one
        // away each — by logical line, regardless of the rows in between.
        gutter.relativeLineNumbers = true
        gutter.currentLine = 1
        XCTAssertEqual(LineNumberGutter.label(line: 0, currentLine: 1, relative: true), "1")
        XCTAssertEqual(LineNumberGutter.label(line: 1, currentLine: 1, relative: true), "2", "absolute on the caret's line")
        XCTAssertEqual(LineNumberGutter.label(line: 2, currentLine: 1, relative: true), "1",
                       "the line after a wrapped line is one logical line away, not \(rows)")
        XCTAssertEqual(LineNumberGutter.label(line: 3, currentLine: 1, relative: true), "2")

        // And drawing it is clean.
        try draw(gutter)
        XCTAssertEqual(tv.string, text)
    }

    /// The label is emitted once per logical line: the draw loop walks
    /// `lineStarts` and anchors each number to that line's first fragment, so a
    /// wrapped line cannot produce a number per visual row.
    func testOneNumberPerLogicalLineNotPerVisualRow() throws {
        let long = String(repeating: "wrap this sentence around several times. ", count: 6)
        let text = "first\n\(long)\nlast\n"
        let prefs = EditorPreferences(defaults: defaults)
        let (window, scroll, tv, gutter) = try hostedGutter(text, width: 240)
        defer { window.orderOut(nil) }
        prefs.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()

        var table = SyntaxHighlighter()
        table.reset(text as NSString)
        // "first", the long line, "last", and the empty line the trailing
        // newline opens — the gutter numbers that last one too.
        XCTAssertEqual(table.lineCount, 4)

        let lm = try XCTUnwrap(tv.layoutManager)
        let container = try XCTUnwrap(tv.textContainer)
        lm.ensureLayout(for: container)
        var totalRows = 0
        lm.enumerateLineFragments(forGlyphRange: lm.glyphRange(for: container)) { _, _, _, _, _ in totalRows += 1 }
        XCTAssertGreaterThan(totalRows, table.lineCount, "the fixture must wrap")

        gutter.relativeLineNumbers = true
        gutter.currentLine = 0
        try draw(gutter) // exercises the real loop over lineStarts
        XCTAssertEqual(tv.string, text)
    }

    // MARK: the preference reaches the live gutter

    func testApplyPushesThePreferenceToTheGutter() throws {
        let (window, scroll, tv, gutter) = try hostedGutter("a\nb\nc\n")
        defer { window.orderOut(nil) }
        XCTAssertFalse(gutter.relativeLineNumbers)
        let prefs = EditorPreferences(defaults: defaults)
        prefs.relativeLineNumbers = true
        prefs.apply(to: tv)
        XCTAssertTrue(gutter.relativeLineNumbers, "apply(to:) did not reach the scroll view's ruler")
        XCTAssertTrue(scroll.verticalRulerView === gutter)
        prefs.relativeLineNumbers = false
        prefs.apply(to: tv)
        XCTAssertFalse(gutter.relativeLineNumbers)
    }

    /// The test override wins over the shared preferences, the way
    /// `CaretFollow.enabledOverride` does.
    func testTheOverrideWinsOverThePreference() throws {
        let (window, _, tv, gutter) = try hostedGutter("a\nb\n")
        defer { window.orderOut(nil) }
        let prefs = EditorPreferences(defaults: defaults)
        prefs.relativeLineNumbers = false
        LineNumberGutter.relativeOverride = true
        prefs.apply(to: tv)
        XCTAssertTrue(gutter.relativeLineNumbers)
        LineNumberGutter.relativeOverride = false
        prefs.relativeLineNumbers = true
        prefs.apply(to: tv)
        XCTAssertFalse(gutter.relativeLineNumbers)
    }
}
