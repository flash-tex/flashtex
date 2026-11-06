import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// Syntax colours are temporary attributes painted for the visible window
/// only (SyntaxPainter). `attach` runs from `makeNSView`, before the text view
/// has a window, and `window(for:)` then answered "the whole buffer": a 3 MB
/// book was coloured end to end at open, ~490 K temporary-attribute runs that
/// every later keystroke paid for in AppKit (lane REVIEW-APP: 7.2 ms against
/// 3.7 ms per math keystroke, the same text repainted to the window).
@MainActor
final class SyntaxPaintWindowTests: XCTestCase {
    private final class Probe: ObservableObject {
        @Published var text: String
        init(text: String) { self.text = text }
    }

    private struct Host: View {
        @ObservedObject var probe: Probe
        var body: some View { SourceEditorView(text: $probe.text) }
    }

    static func book(bytes: Int) -> String {
        var s = "\\documentclass{book}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < bytes {
            if n % 8 == 0 { s += "\\section{Section \(n / 8 + 1)}\n\n" }
            s += "The quick brown fox jumps over the lazy dog near the river bank.\n"
            s += "Numbers such as $x^2 + y^2 = z^2$ appear inside the prose too.\n\n"
            n += 1
        }
        return s + "\\end{document}\n"
    }

    private func waitFor(_ seconds: TimeInterval, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline, !done() { try await Task.sleep(nanoseconds: 10_000_000) }
    }

    private func paintedUnits(_ co: SourceEditorView.Coordinator) -> Int { co.syntax.painted.reduce(0) { $0 + $1.length } }

    private func covers(_ co: SourceEditorView.Coordinator, _ r: NSRange) -> Bool {
        co.syntax.painted.contains { $0.location <= r.location && NSMaxRange(r) <= NSMaxRange($0) }
    }

    private func visibleCharacters(_ tv: NSTextView) -> NSRange {
        let lm = tv.layoutManager!
        let glyphs = lm.glyphRange(forBoundingRect: tv.visibleRect, in: tv.textContainer!)
        return lm.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
    }

    func testABookIsPaintedOnlyAroundWhatIsVisible() async throws {
        let text = Self.book(bytes: 3_000_000)
        let ns = text as NSString
        let probe = Probe(text: text)
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled])
        var hosting: NSHostingView<Host>? = NSHostingView(rootView: Host(probe: probe))
        window.contentView = hosting
        window.orderFrontRegardless() // never makeKey
        defer { window.orderOut(nil); window.contentView = nil; hosting = nil }
        var found: NSTextView?
        try await waitFor(10) { found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil }
        let tv = try XCTUnwrap(found)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        try await waitFor(10) { tv.textStorage?.length == ns.length && co.syntax.highlighter.length == ns.length }

        // The first screen is coloured once the view is on screen.
        try await waitFor(5) { SyntaxPainter.isOnScreen(tv) && self.covers(co, self.visibleCharacters(tv)) }
        XCTAssertTrue(covers(co, visibleCharacters(tv)), "the visible text is painted")
        print("painted at open: \(paintedUnits(co)) of \(ns.length) UTF-16 units")
        XCTAssertLessThan(paintedUnits(co), 100_000, "only the neighbourhood of what is visible, not the whole book")

        // Scrolling to the middle paints the middle (and nothing like the whole book).
        let middle = ns.range(of: "The quick", range: NSRange(location: ns.length / 2, length: ns.length / 2))
        tv.scrollRangeToVisible(middle)
        try await waitFor(5) { self.covers(co, self.visibleCharacters(tv)) }
        XCTAssertTrue(covers(co, visibleCharacters(tv)), "a scroll paints the newly visible text")
        let lm = try XCTUnwrap(tv.layoutManager)
        let keyword = ns.range(of: "\\section", range: NSRange(location: middle.location, length: 20_000))
        if keyword.location != NSNotFound, covers(co, keyword) {
            XCTAssertNotNil(lm.temporaryAttribute(SyntaxPainter.key, atCharacterIndex: keyword.location, effectiveRange: nil), "a command is coloured")
        }
        XCTAssertLessThan(paintedUnits(co), 200_000)
    }
}
