import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// Bounded re-lexing (lane REVIEW-APP): typing the `}` of `\begin{verbatim}`
/// or `\begin{comment}` changes the mode of every line below it, and the
/// highlighter re-lexed them all inside the keystroke (55 191 lines at 3 MB).
/// An edit now stops after `SyntaxPainter.relexLineBudget` lines, recording
/// `staleFrom`; readers stay exact past it, and idle steps catch up.
@MainActor
final class SyntaxLazyRelexTests: XCTestCase {
    /// Random edits under a tiny line budget: after each, every reader
    /// (`runs`, `state`, `lineState`) answers what a fresh full lex answers,
    /// and catching up leaves the stored states equal to it.
    func testBoundedEditsStayExactForEveryReader() {
        let pieces = ["plain words\n", "$x\n", "y$\n", "\\begin{verbatim}\n", "\\end{verbatim}\n", "\\begin{comment}\n",
                      "\\end{comment}\n", "% c\n", "\\[\n", "\\]\n", "\\makeatletter\n", "\\makeatother\n", "\\verb|$|\n", "\n"]
        var rng = SystemRandomNumberGenerator()
        var text = (0..<60).map { pieces[($0 * 5 + $0 / 4) % pieces.count] }.joined()
        var h = SyntaxHighlighter()
        h.reset(text as NSString)
        var sawStale = 0
        for step in 0..<300 {
            let ns = text as NSString
            let at = Int.random(in: 0...ns.length, using: &rng)
            let len = Int.random(in: 0...min(12, ns.length - at), using: &rng)
            let replacement = Bool.random(using: &rng) ? pieces.randomElement(using: &rng)! : ["", "$", "x", "}", "\\begin{comment}"].randomElement(using: &rng)!
            let new = ns.replacingCharacters(in: NSRange(location: at, length: len), with: replacement) as NSString
            text = new as String
            _ = h.edit(range: NSRange(location: at, length: len), replacementLength: (replacement as NSString).length, text: new, lineBudget: 2)
            if h.staleFrom != nil { sawStale += 1 }
            var fresh = SyntaxHighlighter()
            fresh.reset(new)
            let whole = NSRange(location: 0, length: new.length)
            XCTAssertEqual(h.lineStarts, fresh.lineStarts, "step \(step)")
            XCTAssertEqual(h.runs(in: whole, text: new), fresh.runs(in: whole, text: new), "runs, step \(step)")
            for l in stride(from: 0, to: h.lineCount, by: 3) {
                XCTAssertTrue(h.lineState(l, text: new) == fresh.lineState(l, text: new), "line \(l), step \(step)")
            }
            if new.length > 0 {
                let p = Int.random(in: 0..<new.length, using: &rng)
                XCTAssertTrue(h.state(at: p, text: new) == fresh.state(at: p, text: new), "state at \(p), step \(step)")
            }
            if step % 7 == 0 { // sometimes catch up fully, sometimes leave it stale for the next edit
                while h.staleFrom != nil { h.catchUp(lines: 3, text: new) }
                XCTAssertEqual(h.modes, fresh.modes, "caught up, step \(step)")
                XCTAssertEqual(h.atLetters, fresh.atLetters, "caught up, step \(step)")
            }
        }
        XCTAssertGreaterThan(sawStale, 10, "the budget did bind")
    }

    /// The same, seeded (reproducible), catching up after every edit from a
    /// copy: the stored states always end equal to a fresh full lex.
    func testSeededBoundedEditsCatchUpToTheFullLex() {
        let pieces = ["plain words\n", "$x\n", "y$\n", "\\begin{verbatim}\n", "\\end{verbatim}\n", "\\begin{comment}\n",
                      "\\end{comment}\n", "% c\n", "\\[\n", "\\]\n", "\\makeatletter\n", "\\makeatother\n", "\\verb|$|\n", "\n"]
        var seed: UInt64 = 12345
        func r(_ n: Int) -> Int { seed = seed &* 6364136223846793005 &+ 1442695040888963407; return Int((seed >> 33) % UInt64(max(1, n))) }
        var text = (0..<60).map { pieces[($0 * 5 + $0 / 4) % pieces.count] }.joined()
        var h = SyntaxHighlighter()
        h.reset(text as NSString)
        for step in 0..<400 {
            let ns = text as NSString
            let at = r(ns.length + 1)
            let len = r(min(12, ns.length - at) + 1)
            let extra = ["", "$", "x", "}", "\\begin{comment}"]
            let replacement = r(2) == 0 ? pieces[r(pieces.count)] : extra[r(extra.count)]
            let staleBefore = h.staleFrom
            let new = ns.replacingCharacters(in: NSRange(location: at, length: len), with: replacement) as NSString
            text = new as String
            _ = h.edit(range: NSRange(location: at, length: len), replacementLength: (replacement as NSString).length, text: new, lineBudget: 2)
            var fresh = SyntaxHighlighter(); fresh.reset(new)
            var probe = h
            while probe.staleFrom != nil { probe.catchUp(lines: 3, text: new) }
            if probe.modes != fresh.modes {
                let bad = zip(probe.modes, fresh.modes).enumerated().first { $0.element.0 != $0.element.1 }!.offset
                let first = h.line(at: at)
                print("DIVERGE step \(step): edit at \(at) len \(len) <- \(replacement.debugDescription), line \(first), staleBefore \(String(describing: staleBefore)) staleAfter \(String(describing: h.staleFrom)), first bad line \(bad): got \(probe.modes[bad]) want \(fresh.modes[bad]); h(before catch) \(h.modes[bad]) lexed \(h.lastEditLinesLexed)")
                XCTFail("diverged"); return
            }
        }
    }

    private final class Probe: ObservableObject {
        @Published var text: String
        init(text: String) { self.text = text }
    }

    private struct Host: View {
        @ObservedObject var probe: Probe
        var body: some View { SourceEditorView(text: $probe.text) }
    }

    private func waitFor(_ seconds: TimeInterval, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline, !done() { try await Task.sleep(nanoseconds: 10_000_000) }
    }

    func testOpeningACommentInABookRelexesABoundedSliceAndCatchesUp() async throws {
        var s = "\\documentclass{book}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < 3_000_000 {
            if n % 8 == 0 { s += "\\section{Section \(n / 8 + 1)}\n\n" }
            s += "The quick brown fox jumps over the lazy dog near the river bank.\n"
            s += "Numbers such as $x^2 + y^2 = z^2$ appear inside the prose too.\n\n"
            n += 1
        }
        s += "\\end{document}\n"
        let ns = s as NSString
        let probe = Probe(text: s)
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

        let at = ns.range(of: "The quick", range: NSRange(location: ns.length / 2, length: ns.length / 2)).location
        tv.setSelectedRange(NSRange(location: at, length: 0))
        tv.scrollRangeToVisible(tv.selectedRange())
        try await Task.sleep(nanoseconds: 100_000_000)
        var lexed: [Int] = [], costs: [Double] = []
        for ch in "\\begin{comment}\n" {
            let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            tv.insertText(String(ch), replacementRange: tv.selectedRange())
            costs.append(Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6)
            lexed.append(co.syntax.highlighter.lastEditLinesLexed)
        }
        print("costs \(costs.map { String(format: "%.1f", $0) })")
        print("lines lexed per keystroke of \\begin{comment}: \(lexed); worst keystroke \(String(format: "%.2f", costs.max() ?? 0)) ms main-thread CPU")
        XCTAssertLessThanOrEqual(lexed.max() ?? 0, SyntaxPainter.relexLineBudget, "no keystroke re-lexes the rest of the book")

        // Scrolling far below the opener before the catch-up is done shows the comment's colour.
        let storage = try XCTUnwrap(tv.textStorage)
        let far = (storage.string as NSString).range(of: "The quick", options: .backwards).location
        tv.scrollRangeToVisible(NSRange(location: far, length: 0))
        try await Task.sleep(nanoseconds: 50_000_000)
        let text = storage.string as NSString
        XCTAssertEqual(co.syntax.highlighter.mode(at: far + 3, text: text), .commentEnvironment, "the tail reads as inside the comment")

        // Idle steps finish the job: stored states equal a fresh lex.
        try await waitFor(30) { co.syntax.highlighter.staleFrom == nil }
        XCTAssertNil(co.syntax.highlighter.staleFrom)
        XCTAssertGreaterThan(co.syntax.catchUpSteps, 0)
        var fresh = SyntaxHighlighter()
        fresh.reset(storage.string as NSString)
        XCTAssertEqual(co.syntax.highlighter.modes, fresh.modes)
    }
}
