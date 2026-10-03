import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

/// P5-KEYSTROKE-MAIN (EditorEditTail.swift): a keystroke inside a paragraph
/// redraws that paragraph, not every line below it, unless something below it
/// changed; long line fragments draw only near the clip rect, with the same
/// pixels.
@MainActor
final class EditorEditTailTests: XCTestCase {
    private var window: NSWindow?

    override func tearDown() async throws {
        EditTailState.longLineClipEnabled = true
        window?.orderOut(nil)
        window = nil
    }

    /// Six paragraphs; with wrapping off each is one long line. The last is
    /// the longest, so typing into another never widens the text view.
    static let text: String = (1...6).map { n in
        "Paragraph \(n) " + String(repeating: "lorem ipsum dolor sit amet ", count: n == 6 ? 80 : 60)
    }.joined(separator: "\n") + "\n"

    private func hosted(_ text: String, wrap: Bool, width: CGFloat = 320, height: CGFloat = 240) -> CompletingTextView {
        HostedWindowSupport.prepare() // non-activating
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: width, height: height),
                                                styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let scroll = CompletingTextView.scrollable()
        scroll.frame = NSRect(x: 0, y: 0, width: width, height: height)
        window.contentView = scroll
        let tv = scroll.documentView as! CompletingTextView
        tv.isRichText = false
        tv.textContainerInset = NSSize(width: 8, height: 8) // as SourceEditorView.makeNSView sets it
        tv.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        EditorPreferences.setLineWrapping(wrap, on: tv)
        tv.string = text
        window.orderFrontRegardless() // never makeKey
        scroll.layoutSubtreeIfNeeded()
        tv.layoutManager?.ensureLayout(for: tv.textContainer!)
        window.displayIfNeeded()
        self.window = window
        return tv
    }

    /// The view rect of the paragraph holding `location`.
    private func paragraphRect(_ tv: NSTextView, at location: Int) throws -> NSRect {
        let lm = try XCTUnwrap(tv.layoutManager)
        let para = (tv.string as NSString).paragraphRange(for: NSRange(location: location, length: 0))
        let glyphs = lm.glyphRange(forCharacterRange: para, actualCharacterRange: nil)
        var r = NSRect.null
        lm.enumerateLineFragments(forGlyphRange: glyphs) { rect, _, _, _, _ in r = r.union(rect) }
        return r.offsetBy(dx: tv.textContainerOrigin.x, dy: tv.textContainerOrigin.y)
    }

    /// Runs `edit` and returns the rects the text view was asked to redraw
    /// (after the edit-tail filter), through the end of the edit.
    private func drawn(_ tv: CompletingTextView, _ edit: () -> Void) -> [NSRect] {
        window?.displayIfNeeded()
        var rects: [NSRect] = []
        tv.onInvalidate = { rects.append($0) }
        edit()
        tv.onInvalidate = nil
        XCTAssertNil(tv.editTail, "the edit ended")
        return rects.filter { !$0.isEmpty }
    }

    private func location(of needle: String, in tv: NSTextView) -> Int {
        let r = (tv.string as NSString).range(of: needle)
        XCTAssertNotEqual(r.location, NSNotFound, needle)
        return r.location
    }

    // MARK: the tail

    func testTypingInsideALongLineRedrawsThatLineOnly() throws {
        let tv = hosted(Self.text, wrap: false)
        let at = location(of: "Paragraph 2 ", in: tv) + 12
        tv.setSelectedRange(NSRange(location: at, length: 0))
        window?.displayIfNeeded()
        let line = try paragraphRect(tv, at: at)
        let dropped = tv.editTailDroppedCount

        let typed = drawn(tv) { tv.insertText("x", replacementRange: tv.selectedRange()) }
        XCTAssertFalse(typed.isEmpty, "the edited line is redrawn")
        XCTAssertTrue(typed.contains { $0.intersects(line) }, "the edited line is redrawn: \(typed)")
        XCTAssertTrue(typed.allSatisfy { $0.maxY <= line.maxY + 0.5 }, "nothing below the edited line is redrawn: \(typed), line \(line)")
        XCTAssertEqual(tv.editTailDroppedCount, dropped + 1)

        let deleted = drawn(tv) { tv.deleteBackward(nil) }
        XCTAssertTrue(deleted.allSatisfy { $0.maxY <= line.maxY + 0.5 }, "a deletion inside the line: \(deleted)")
        XCTAssertEqual(tv.string, Self.text, "the edits themselves are unchanged")
    }

    func testALineBreakRedrawsTheLinesBelow() throws {
        let tv = hosted(Self.text, wrap: false)
        let at = location(of: "Paragraph 2 ", in: tv) + 12
        tv.setSelectedRange(NSRange(location: at, length: 0))
        let line = try paragraphRect(tv, at: at)
        let dropped = tv.editTailDroppedCount
        let rects = drawn(tv) { tv.insertText("\n", replacementRange: tv.selectedRange()) }
        XCTAssertTrue(rects.contains { $0.maxY > line.maxY + 1 }, "every later line moved down: \(rects)")
        XCTAssertEqual(tv.editTailDroppedCount, dropped)
        _ = drawn(tv) { tv.deleteBackward(nil) } // deleting the break: nothing deferred either
        XCTAssertEqual(tv.editTailDroppedCount, dropped)
        XCTAssertEqual(tv.string, Self.text)
    }

    func testAChangeBelowTheParagraphDuringTheEditKeepsTheRedraw() throws {
        let tv = hosted(Self.text, wrap: false)
        let at = location(of: "Paragraph 2 ", in: tv) + 12
        let below = location(of: "Paragraph 4", in: tv)
        tv.setSelectedRange(NSRange(location: at, length: 0))
        let lineBelow = try paragraphRect(tv, at: below)
        // What the brace/environment-pair highlight does from the text view's
        // delegate: a temporary attribute far below the edit, during it.
        let lm = try XCTUnwrap(tv.layoutManager)
        let token = NotificationCenter.default.addObserver(forName: NSText.didChangeNotification, object: tv, queue: nil) { _ in
            MainActor.assumeIsolated {
                lm.addTemporaryAttribute(.backgroundColor, value: NSColor.yellow, forCharacterRange: NSRange(location: below, length: 9))
            }
        }
        defer { NotificationCenter.default.removeObserver(token) }
        let dropped = tv.editTailDroppedCount
        let rects = drawn(tv) { tv.insertText("x", replacementRange: tv.selectedRange()) }
        XCTAssertTrue(rects.contains { $0.intersects(lineBelow) }, "the highlighted line below is redrawn: \(rects)")
        XCTAssertEqual(tv.editTailDroppedCount, dropped)
    }

    func testAWrappedParagraphThatGainsALineRedrawsBelowAndOneThatDoesNotDoesNot() throws {
        let tv = hosted(Self.text, wrap: true)
        let at = location(of: "Paragraph 2 ", in: tv) + 12
        tv.setSelectedRange(NSRange(location: at, length: 0))
        let para = try paragraphRect(tv, at: at)

        let one = drawn(tv) { tv.insertText("x", replacementRange: tv.selectedRange()) }
        let after = try paragraphRect(tv, at: at)
        if after.maxY == para.maxY {
            XCTAssertTrue(one.allSatisfy { $0.maxY <= para.maxY + 0.5 }, "the paragraph kept its height: \(one)")
        }
        // A whole new row of text: the paragraph grows, every later line moves.
        let grown = drawn(tv) { tv.insertText(String(repeating: "word ", count: 30), replacementRange: tv.selectedRange()) }
        XCTAssertGreaterThan(try paragraphRect(tv, at: at).maxY, para.maxY)
        XCTAssertTrue(grown.contains { $0.maxY > para.maxY + 1 }, "the lines below moved and are redrawn: \(grown)")
    }

    func testAnEditThatNeverFinishesRedrawsAtTheNextDisplay() throws {
        let tv = hosted(Self.text, wrap: false)
        let at = location(of: "Paragraph 2 ", in: tv) + 12
        // shouldChangeText without the edit or didChangeText: nothing may stay
        // deferred past the next display.
        XCTAssertTrue(tv.shouldChangeText(in: NSRange(location: at, length: 0), replacementString: "x"))
        XCTAssertNotNil(tv.editTail)
        let line = try paragraphRect(tv, at: at)
        var rects: [NSRect] = []
        tv.onInvalidate = { rects.append($0) }
        tv.setNeedsDisplay(NSRect(x: 0, y: line.minY, width: 320, height: 400), avoidAdditionalLayout: false)
        XCTAssertTrue(rects.allSatisfy { $0.maxY <= line.maxY + 0.5 }, "deferred while the edit is open: \(rects)")
        tv.viewWillDraw() // what AppKit calls before the next display
        tv.onInvalidate = nil
        XCTAssertNil(tv.editTail, "the next display ends the deferral")
        XCTAssertTrue(rects.contains { $0.maxY > line.maxY + 1 }, "and redraws what was deferred: \(rects)")
    }

    // MARK: the gutter

    struct Host: View {
        var model: ShellModel
        var body: some View {
            SourceEditorView(text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                             selection: model.selection, pendingEdit: model.pendingEdit, showLineNumbers: true)
        }
    }

    /// The line-number gutter is redrawn after an edit only when lines moved.
    func testTheGutterIsRedrawnOnlyWhenAnEditMovesLines() async throws {
        let model = ShellModel()
        model.updateActiveText("\\documentclass{article}\n\\begin{document}\nHello there\nsecond line\n\\end{document}\n")
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 300), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model))
        window.orderFrontRegardless()
        self.window = window
        var found: NSTextView?
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline, found == nil {
            found = TypingBenchDriver.findTextView(in: [window.contentView!])
            if found == nil { try await Task.sleep(nanoseconds: 10_000_000) }
        }
        let tv = try XCTUnwrap(found as? CompletingTextView)
        try await Task.sleep(nanoseconds: 200_000_000)
        let gutter = try XCTUnwrap(tv.enclosingScrollView?.verticalRulerView as? LineNumberGutter)
        tv.layoutManager?.ensureLayout(for: tv.textContainer!)
        tv.setSelectedRange(NSRange(location: location(of: "there", in: tv), length: 0))

        let before = gutter.editRedraws
        tv.insertText("x", replacementRange: tv.selectedRange())
        XCTAssertEqual(gutter.editRedraws, before, "a character inside a line moves no line number")
        tv.insertText("\n", replacementRange: tv.selectedRange())
        XCTAssertEqual(gutter.editRedraws, before + 1, "a line break moves every number below it")
        XCTAssertTrue(model.activeText.contains("Hello x\nthere"), model.activeText)
    }

    // MARK: long lines

    private func render(_ tv: NSTextView, _ rect: NSRect) throws -> Data {
        let rep = try XCTUnwrap(tv.bitmapImageRepForCachingDisplay(in: rect))
        tv.cacheDisplay(in: rect, to: rep)
        return try XCTUnwrap(rep.tiffRepresentation)
    }

    func testALongLineDrawnNearTheClipRectHasTheSamePixels() throws {
        let tv = hosted(Self.text, wrap: false)
        // Scrolled into the middle of the lines, then to their start.
        let lm = try XCTUnwrap(tv.layoutManager as? EditTailLayoutManager)
        for x in [CGFloat(1200), 0] {
            tv.scroll(NSPoint(x: x, y: 0))
            let rect = tv.visibleRect
            EditTailState.longLineClipEnabled = false
            let whole = try render(tv, rect)
            EditTailState.longLineClipEnabled = true
            let clipped0 = lm.clippedLineDraws
            let near = try render(tv, rect)
            XCTAssertGreaterThan(lm.clippedLineDraws, clipped0, "the long lines were drawn near the clip rect only (x \(x))")
            XCTAssertEqual(near, whole, "same pixels at x \(x)")
        }
    }

    func testARightToLeftLineIsDrawnWhole() throws {
        let hebrew = "Paragraph " + String(repeating: "שלום עולם lorem ipsum ", count: 60) + "\n"
        let tv = hosted(hebrew + Self.text, wrap: false)
        let ns = tv.string as NSString
        XCTAssertTrue(EditTailLayoutManager.hasBidi(ns, ns.paragraphRange(for: NSRange(location: 0, length: 0))))
        XCTAssertFalse(EditTailLayoutManager.hasBidi(ns, ns.paragraphRange(for: NSRange(location: (hebrew as NSString).length + 1, length: 0))))
        tv.scroll(NSPoint(x: 600, y: 0))
        let rect = tv.visibleRect
        EditTailState.longLineClipEnabled = false
        let whole = try render(tv, rect)
        EditTailState.longLineClipEnabled = true
        XCTAssertEqual(try render(tv, rect), whole, "the bidi line drawn whole, the others near the clip rect: same pixels")
    }
}
