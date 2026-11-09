import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// The `\begin{X}` ↔ `\end{X}` highlight on a book-sized buffer (lane
/// REVIEW-APP). With the caret on a line holding `\begin{` or `\end{`, every
/// caret move and every keystroke lexed the whole buffer on the main thread
/// (`EditorNavigation.environmentPair(at:in:)` → `environmentPairs`), and read
/// the caret's line through `storage.string`, a copy of the whole buffer.
///
/// Main-thread CPU (`CLOCK_THREAD_CPUTIME_ID`) of each caret move or
/// keystroke, so the figure does not move with the machine's load; the 16 ms
/// main-turn budget (#1900) is reported through `TimingBudget`.
@MainActor
final class EnvironmentHighlightLargeDocumentTests: XCTestCase {
    private final class Probe: ObservableObject {
        @Published var text: String
        init(text: String) { self.text = text }
    }

    private struct Host: View {
        @ObservedObject var probe: Probe
        var body: some View { SourceEditorView(text: $probe.text) }
    }

    /// Chapters, sections, theorem/proof pairs and displayed equations on
    /// their own `\begin{equation}`/`\end{equation}` lines.
    static func book(bytes: Int) -> String {
        var s = "\\documentclass{book}\n\\usepackage{amsmath}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < bytes {
            if n % 40 == 0 { s += "\\chapter{Chapter \(n / 40 + 1)}\n\n" }
            if n % 8 == 0 { s += "\\section{Section \(n / 8 + 1)}\n\n" }
            s += "The quick brown fox jumps over the lazy dog near the river bank.\n"
            s += "Numbers such as $x^2 + y^2 = z^2$ appear inside the prose too.\n\n"
            s += "\\begin{theorem}\nEvery bounded sequence has a convergent subsequence.\n\\end{theorem}\n"
            s += "\\begin{proof}\nBy compactness.\n\\begin{equation}\n  a_{n_k} \\to a\n\\end{equation}\n\\end{proof}\n\n"
            n += 1
        }
        return s + "\\end{document}\n"
    }

    private func cpuMs(_ body: () -> Void) -> Double {
        let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
        body()
        return Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6
    }

    private func waitFor(_ seconds: TimeInterval, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline, !done() { try await Task.sleep(nanoseconds: 10_000_000) }
    }

    /// Cached pairs mapped across the edits since their scan: an edit before
    /// a pair shifts it, one between its spans moves only the `\end`, one
    /// touching either span hides it until the next scan.
    func testCachedPairsMapAcrossEditsSinceTheScan() {
        let text = "x\n\\begin{a}\nbody\n\\end{a}\n" as NSString
        typealias Cache = SourceEditorView.Coordinator.EnvironmentPairCache
        let pairs = EditorNavigation.environmentPairs(in: text)
        let begin = text.range(of: "\\begin{a}"), end = text.range(of: "\\end{a}")
        var cache = Cache(pairs: pairs)
        XCTAssertEqual(cache.pair(at: begin.location)?.end, end)

        cache.edits = [(NSRange(location: 0, length: 0), 3)] // "abc" typed at the top
        XCTAssertEqual(cache.pair(at: begin.location + 3)?.begin, NSRange(location: begin.location + 3, length: begin.length))
        XCTAssertEqual(cache.pair(at: begin.location + 3)?.end, NSRange(location: end.location + 3, length: end.length))
        XCTAssertNil(cache.pair(at: 1), "inside the typed text: no pair")

        cache.edits = [(NSRange(location: text.range(of: "body").location, length: 4), 1)] // body → one character
        XCTAssertEqual(cache.pair(at: begin.location)?.end, NSRange(location: end.location - 3, length: end.length))
        XCTAssertEqual(cache.pair(at: end.location - 3)?.begin, begin, "from the \\end side too")

        cache.edits = [(NSRange(location: begin.location + 7, length: 1), 1)] // the name itself retyped
        XCTAssertNil(cache.pair(at: begin.location), "an edit inside the \\begin hides the pair until a rescan")
    }

    func testEnvironmentHighlightOnABookSizedBufferStaysOffTheMainThread() async throws {
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
        XCTAssertEqual(co.syntax.highlighter.length, ns.length)

        // Twenty `\begin{equation}` lines through the middle of the book.
        var begins: [Int] = []
        var from = ns.length / 3
        while begins.count < 20 {
            let r = ns.range(of: "\\begin{equation}", range: NSRange(location: from, length: ns.length - from))
            begins.append(r.location + 3)
            from = NSMaxRange(r)
        }
        func expectedEnd(_ caret: Int) -> NSRange {
            ns.range(of: "\\end{equation}", range: NSRange(location: caret, length: ns.length - caret))
        }

        // First visit: the pair is found (now, or once a scan lands).
        tv.setSelectedRange(NSRange(location: begins[0], length: 0))
        try await waitFor(15) { co.braceHighlight?.close == expectedEnd(begins[0]) }
        XCTAssertEqual(co.braceHighlight?.close, expectedEnd(begins[0]), "the \\end{equation} partner is highlighted")

        var moves: [Double] = []
        for caret in begins.dropFirst() {
            moves.append(cpuMs { tv.setSelectedRange(NSRange(location: caret, length: 0)) })
            XCTAssertEqual(co.braceHighlight?.close, expectedEnd(caret), "no edit since the scan: the partner shows at once")
            try await Task.sleep(nanoseconds: 1_000_000)
        }

        // Indenting a `\begin` line: each keystroke changes the buffer, and
        // the caret stays on the `\begin` it pushes along.
        let line = ns.lineRange(for: NSRange(location: begins[10], length: 0))
        tv.setSelectedRange(NSRange(location: line.location, length: 0))
        let scansBeforeTyping = co.environmentPairScansOnMain
        var keys: [Double] = []
        for ch in "        " {
            keys.append(cpuMs { tv.insertText(String(ch), replacementRange: tv.selectedRange()) })
            try await Task.sleep(nanoseconds: 1_000_000)
        }
        let typedCaret = tv.selectedRange().location
        let storage = try XCTUnwrap(tv.textStorage?.mutableString)
        let begin = storage.range(of: "\\begin{equation}", range: NSRange(location: typedCaret, length: storage.length - typedCaret))
        XCTAssertEqual(begin.location, typedCaret)
        let end = storage.range(of: "\\end{equation}", range: NSRange(location: typedCaret, length: storage.length - typedCaret))
        try await waitFor(15) { co.braceHighlight?.close == end }
        XCTAssertEqual(co.braceHighlight?.open, begin, "after typing, the highlight catches up with the new buffer")
        XCTAssertEqual(co.braceHighlight?.close, end)
        XCTAssertEqual(co.environmentPairScansOnMain, scansBeforeTyping, "above the off-main threshold the pair scan never runs on the main thread")
        XCTAssertEqual(co.environmentPairScansOnMain, 0)

        let worstMove = moves.max() ?? 0, worstKey = keys.max() ?? 0
        print(String(format: "environment highlight on %d K UTF-16 (main-thread CPU): worst caret move %.2f ms, worst keystroke on a \\begin line %.2f ms",
                     ns.length / 1000, worstMove, worstKey))
        TimingBudget.assertWithin(worstMove, 16, "worst caret move onto a \\begin line, 3 MB buffer (main-thread CPU)")
        TimingBudget.assertWithin(worstKey, 16, "worst keystroke on a \\begin line, 3 MB buffer (main-thread CPU)")
        // A whole-buffer lex per move cost 70+ ms of CPU here; nothing a
        // caret move does on its own line may come close.
        XCTAssertLessThan(worstMove, 30, "a caret move onto a \\begin line must not lex the whole buffer")
    }
}
