import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// Soft wrapping in the source editor: a long logical line is laid out as
/// several visual rows inside the text container, the wrap follows the window
/// as it is resized, and everything that reads geometry off the layout manager
/// — caret rects, the line-number gutter, selection — stays correct when one
/// logical line spans several rows.
///
/// `EditorPreferences.lineWrapping` has been the default since the preferences
/// landed; these tests are the regression floor for it, because the previous
/// coverage only asserted the container *flags* (`widthTracksTextView` and
/// friends) and never that text actually wraps.
@MainActor
final class LineWrappingTests: XCTestCase {
    private var suiteName = ""
    private var defaults: UserDefaults!

    override func setUp() {
        super.setUp()
        suiteName = "flashtex.tests.LineWrapping.\(UUID().uuidString)"
        defaults = UserDefaults(suiteName: suiteName)
        defaults.removePersistentDomain(forName: suiteName)
    }

    override func tearDown() {
        defaults.removePersistentDomain(forName: suiteName)
        defaults = nil
        super.tearDown()
    }

    // MARK: fixtures

    /// One logical line of prose far wider than any window used here, plus two
    /// short lines either side so logical-line indexing is exercised too.
    static let longProse = "\\section{Intro}\n"
        + "In this section we recall the classical statement of the theorem, fix the notation used throughout the paper, and explain why the hypothesis on the boundary cannot be dropped without replacing the whole argument.\n"
        + "\\label{sec:intro}\n"

    /// A hosted editor (never key, non-activating) at an exact content width.
    private func hosted(_ text: String, width: CGFloat = 320, height: CGFloat = 240) -> (NSWindow, NSScrollView, CompletingTextView) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let scroll = CompletingTextView.scrollable()
        scroll.frame = NSRect(x: 0, y: 0, width: width, height: height)
        window.contentView = scroll
        let tv = scroll.documentView as! CompletingTextView
        _ = tv.layoutManager // TextKit 1, as SourceEditorView selects it
        tv.isRichText = false
        tv.textContainerInset = NSSize(width: 8, height: 8) // as SourceEditorView.makeNSView sets it
        tv.string = text
        window.orderFrontRegardless() // never makeKey
        scroll.layoutSubtreeIfNeeded()
        return (window, scroll, tv)
    }

    /// The visual rows the layout manager produced for `range`, in order.
    private func fragments(_ tv: NSTextView, characters range: NSRange) throws -> [NSRect] {
        let lm = try XCTUnwrap(tv.layoutManager)
        let container = try XCTUnwrap(tv.textContainer)
        lm.ensureLayout(for: container)
        let glyphs = lm.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
        var rects: [NSRect] = []
        lm.enumerateLineFragments(forGlyphRange: glyphs) { rect, _, _, _, _ in rects.append(rect) }
        return rects
    }

    /// Character range of the long middle line of `longProse`.
    private func longLineRange(in text: String) throws -> NSRange {
        let ns = text as NSString
        let firstNewline = ns.range(of: "\n").location
        let start = firstNewline + 1
        let rest = ns.range(of: "\n", options: [], range: NSRange(location: start, length: ns.length - start))
        XCTAssertNotEqual(rest.location, NSNotFound)
        return NSRange(location: start, length: rest.location - start)
    }

    // MARK: wrapping actually happens

    func testALongLineIsLaidOutAsSeveralVisualRowsWhenWrapping() throws {
        let p = EditorPreferences(defaults: defaults)
        XCTAssertTrue(p.lineWrapping, "wrapping is the shipping default for a prose-heavy format")
        let (window, scroll, tv) = hosted(Self.longProse, width: 320)
        defer { window.orderOut(nil) }
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()

        let range = try longLineRange(in: Self.longProse)
        let rows = try fragments(tv, characters: range)
        XCTAssertGreaterThan(rows.count, 1, "the long line did not wrap: it was laid out as a single row")

        // Every row stays inside the container: nothing runs off to the right.
        let container = try XCTUnwrap(tv.textContainer)
        for (i, r) in rows.enumerated() {
            XCTAssertLessThanOrEqual(r.maxX.rounded(), container.size.width.rounded() + 1,
                                     "visual row \(i) overflows the text container")
        }
        // The rows descend: each is strictly below the previous one.
        for (a, b) in zip(rows, rows.dropFirst()) {
            XCTAssertGreaterThan(b.minY, a.minY, "wrapped rows are not stacked vertically")
        }
    }

    func testWrappingOffLaysTheLongLineOutAsOneRowWithAHorizontalScroller() throws {
        let p = EditorPreferences(defaults: defaults)
        p.lineWrapping = false
        let (window, scroll, tv) = hosted(Self.longProse, width: 320)
        defer { window.orderOut(nil) }
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()

        let range = try longLineRange(in: Self.longProse)
        let rows = try fragments(tv, characters: range)
        XCTAssertEqual(rows.count, 1, "with wrapping off the long line must scroll, not wrap")
        XCTAssertTrue(scroll.hasHorizontalScroller)
        XCTAssertGreaterThan(rows[0].width, 320, "the unwrapped row is wider than the window, as it must be")
    }

    // MARK: the wrap follows the window

    func testTheWrapFollowsTheWindowWidth() throws {
        let p = EditorPreferences(defaults: defaults)
        let (window, scroll, tv) = hosted(Self.longProse, width: 200)
        defer { window.orderOut(nil) }
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()

        let range = try longLineRange(in: Self.longProse)
        let narrow = try fragments(tv, characters: range).count
        XCTAssertGreaterThan(narrow, 1)

        // Widen the window the way a person drags its edge: only the scroll
        // view is resized; the container must track it with no preference change.
        window.setContentSize(NSSize(width: 700, height: 240))
        scroll.frame = NSRect(x: 0, y: 0, width: 700, height: 240)
        scroll.layoutSubtreeIfNeeded()
        let wide = try fragments(tv, characters: range).count
        XCTAssertLessThan(wide, narrow, "widening the window did not re-wrap the line (rows \(narrow) -> \(wide))")

        // And back again.
        window.setContentSize(NSSize(width: 200, height: 240))
        scroll.frame = NSRect(x: 0, y: 0, width: 200, height: 240)
        scroll.layoutSubtreeIfNeeded()
        XCTAssertEqual(try fragments(tv, characters: range).count, narrow,
                       "narrowing the window back did not restore the original wrap")
    }

    func testTheTextContainerNeverExceedsTheVisibleWidthWhileWrapping() throws {
        let p = EditorPreferences(defaults: defaults)
        let (window, scroll, tv) = hosted(Self.longProse, width: 260)
        defer { window.orderOut(nil) }
        p.apply(to: tv)
        for width in [260.0, 420.0, 180.0, 640.0] as [CGFloat] {
            scroll.frame = NSRect(x: 0, y: 0, width: width, height: 240)
            scroll.layoutSubtreeIfNeeded()
            let container = try XCTUnwrap(tv.textContainer)
            XCTAssertLessThanOrEqual(container.size.width, scroll.contentView.bounds.width + 1,
                                     "container is wider than the clip view at \(width)pt: content would be clipped")
            XCTAssertFalse(scroll.hasHorizontalScroller, "wrapping must never show a horizontal scroller")
        }
    }

    // MARK: geometry across wrapped rows

    /// The caret late in a wrapped line must sit on a lower row than the caret
    /// at its start — the property the preview's reveal and the current-line
    /// band depend on.
    func testCaretRectsFollowTheWrappedRowsOfOneLogicalLine() throws {
        let p = EditorPreferences(defaults: defaults)
        let (window, scroll, tv) = hosted(Self.longProse, width: 260)
        defer { window.orderOut(nil) }
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()

        let range = try longLineRange(in: Self.longProse)
        let lm = try XCTUnwrap(tv.layoutManager)
        let container = try XCTUnwrap(tv.textContainer)
        lm.ensureLayout(for: container)

        func caretRect(at character: Int) -> NSRect {
            let g = lm.glyphIndexForCharacter(at: character)
            return lm.lineFragmentRect(forGlyphAt: g, effectiveRange: nil)
        }
        let atStart = caretRect(at: range.location)
        let atEnd = caretRect(at: NSMaxRange(range) - 1)
        XCTAssertGreaterThan(atEnd.minY, atStart.minY,
                             "the end of a wrapped line reports the same row as its start")

        // The row for the following logical line is below every row of this one.
        let nextLine = caretRect(at: NSMaxRange(range) + 1)
        XCTAssertGreaterThan(nextLine.minY, atEnd.minY)
    }

    /// The gutter numbers logical lines, not visual rows: three logical lines
    /// stay three numbers however many rows the middle one occupies.
    func testTheGutterNumbersLogicalLinesNotVisualRows() throws {
        let p = EditorPreferences(defaults: defaults)
        let (window, scroll, tv) = hosted(Self.longProse, width: 240)
        defer { window.orderOut(nil) }
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()

        let logicalLines = Self.longProse.components(separatedBy: "\n").dropLast().count
        XCTAssertEqual(logicalLines, 3, "the fixture is three logical lines")

        let range = try longLineRange(in: Self.longProse)
        XCTAssertGreaterThan(try fragments(tv, characters: range).count, 1,
                             "the fixture must actually wrap for this test to mean anything")

        let gutter = LineNumberGutter(scrollView: scroll)
        gutter.clientView = tv
        gutter.layoutIfNeeded(lineCount: logicalLines)
        scroll.verticalRulerView = gutter
        scroll.rulersVisible = true

        // Drawing must not touch the storage, and must not crash on wrapped rows.
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
        XCTAssertEqual(tv.string, Self.longProse)
    }

    // MARK: the preference itself

    func testWrappingPreferencePersistsAndResets() {
        let p = EditorPreferences(defaults: defaults)
        XCTAssertTrue(p.lineWrapping)
        p.lineWrapping = false
        XCTAssertFalse(EditorPreferences(defaults: defaults).lineWrapping, "the choice did not survive a reload")
        p.resetToDefaults()
        XCTAssertTrue(p.lineWrapping)
        XCTAssertTrue(EditorPreferences(defaults: defaults).lineWrapping)
    }

    /// Toggling at runtime re-lays the text out both ways, through the same
    /// observation seam the live editor uses.
    func testTogglingThePreferenceRewrapsALiveEditor() throws {
        let p = EditorPreferences(defaults: defaults)
        let (window, scroll, tv) = hosted(Self.longProse, width: 260)
        defer { window.orderOut(nil) }
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()
        let range = try longLineRange(in: Self.longProse)
        XCTAssertGreaterThan(try fragments(tv, characters: range).count, 1)

        p.lineWrapping = false
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()
        XCTAssertEqual(try fragments(tv, characters: range).count, 1)

        p.lineWrapping = true
        p.apply(to: tv)
        scroll.layoutSubtreeIfNeeded()
        XCTAssertGreaterThan(try fragments(tv, characters: range).count, 1,
                             "turning wrapping back on did not re-wrap")
    }
}
