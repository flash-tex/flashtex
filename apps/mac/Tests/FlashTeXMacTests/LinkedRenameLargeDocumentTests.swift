import AppKit
import SwiftUI
import XCTest
import HostedWindows
@testable import FlashTeXMac
@testable import FlashTeXEditorCore

/// Linked `\begin{name}`/`\end{name}` rename on a book-sized buffer (lane
/// REVIEW-APP): every keystroke inside an environment name used to copy the
/// buffer to a native String and lex all of it (`LaTeXScan.uses`) to find the
/// pair again. The session now carries from one keystroke to the next; only
/// the first keystroke of a rename scans, over the storage's own UTF-16.
///
/// Main-thread CPU (`CLOCK_THREAD_CPUTIME_ID`) of each keystroke, so the
/// figure does not move with the machine's load; the 16 ms main-turn budget
/// (#1900) is reported through `TimingBudget`.
@MainActor
final class LinkedRenameLargeDocumentTests: XCTestCase {
    private final class Probe: ObservableObject {
        @Published var text: String
        init(text: String) { self.text = text }
    }

    private struct Host: View {
        @ObservedObject var probe: Probe
        var body: some View { SourceEditorView(text: $probe.text) }
    }

    static func book(bytes: Int) -> String {
        var s = "\\documentclass{book}\n\\usepackage{amsmath}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < bytes {
            if n % 8 == 0 { s += "\\section{Section \(n / 8 + 1)}\n\n" }
            s += "The quick brown fox jumps over the lazy dog near the river bank.\n"
            s += "Numbers such as $x^2 + y^2 = z^2$ appear inside the prose too.\n\n"
            s += "\\begin{theorem}\nEvery bounded sequence has a convergent subsequence.\n\\end{theorem}\n"
            s += "\\begin{proof}\nBy compactness.\n\\begin{equation}\n  a_{n_k} \\to a\n\\end{equation}\n\\end{proof}\n\n"
            n += 1
        }
        return s + "\\end{document}\n"
    }

    private func waitFor(_ seconds: TimeInterval, _ done: () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline, !done() { try await Task.sleep(nanoseconds: 10_000_000) }
    }

    func testRenamingAnEnvironmentOnABookSizedBufferScansOncePerRename() async throws {
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

        // The end of the `theorem` name of a pair in the middle of the book.
        let begin = ns.range(of: "\\begin{theorem}", range: NSRange(location: ns.length / 2, length: ns.length / 2))
        tv.setSelectedRange(NSRange(location: NSMaxRange(begin) - 1, length: 0))
        let scans = co.linkedPairScans
        var costs: [Double] = []
        for ch in "lemma" {
            let c0 = clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID)
            tv.insertText(String(ch), replacementRange: tv.selectedRange())
            costs.append(Double(clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) - c0) / 1e6)
            try await Task.sleep(nanoseconds: 1_000_000)
        }
        let storage = try XCTUnwrap(tv.textStorage?.mutableString)
        XCTAssertEqual(storage.substring(with: NSRange(location: begin.location, length: 20)), "\\begin{theoremlemma}")
        let end = storage.range(of: "\\end{", range: NSRange(location: begin.location, length: storage.length - begin.location))
        XCTAssertEqual(storage.substring(with: NSRange(location: end.location, length: 18)), "\\end{theoremlemma}", "the partner follows every keystroke")
        XCTAssertEqual(co.linkedPairScans - scans, 1, "one pair scan for the whole rename")

        let first = costs.first ?? 0, rest = costs.dropFirst().max() ?? 0
        print(String(format: "linked rename on %d K UTF-16 (main-thread CPU): first keystroke %.2f ms, worst later keystroke %.2f ms",
                     ns.length / 1000, first, rest))
        TimingBudget.assertWithin(first, 16, "first keystroke of a linked rename, 3 MB buffer (main-thread CPU)")
        TimingBudget.assertWithin(rest, 16, "later keystroke of a linked rename, 3 MB buffer (main-thread CPU)")
        XCTAssertLessThan(rest, 30, "a keystroke inside a linked name must not rescan the whole buffer")
    }

    /// An edit elsewhere ends the carried session: the next rename keystroke
    /// scans again and still finds the right partner.
    func testAnEditOutsideTheNameRescans() async throws {
        let text = "\\begin{a}\nx\n\\end{a}\n\\begin{b}\ny\n\\end{b}\n"
        let probe = Probe(text: text)
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled])
        var hosting: NSHostingView<Host>? = NSHostingView(rootView: Host(probe: probe))
        window.contentView = hosting
        window.orderFrontRegardless()
        defer { window.orderOut(nil); window.contentView = nil; hosting = nil }
        var found: NSTextView?
        try await waitFor(10) { found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil }
        let tv = try XCTUnwrap(found)
        let co = try XCTUnwrap(tv.delegate as? SourceEditorView.Coordinator)
        func type(_ s: String, at p: Int) {
            tv.setSelectedRange(NSRange(location: p, length: 0))
            for ch in s { tv.insertText(String(ch), replacementRange: tv.selectedRange()) }
        }
        let scans = co.linkedPairScans
        type("1", at: 8) // \begin{a|}
        XCTAssertEqual(tv.string, "\\begin{a1}\nx\n\\end{a1}\n\\begin{b}\ny\n\\end{b}\n")
        type("z", at: 11) // the body: ends the session
        type("2", at: 9) // back in the name
        XCTAssertEqual(tv.string, "\\begin{a12}\nzx\n\\end{a12}\n\\begin{b}\ny\n\\end{b}\n")
        XCTAssertEqual(co.linkedPairScans - scans, 2, "the body edit dropped the carried session")
        let b = (tv.string as NSString).range(of: "\\begin{b}")
        type("3", at: NSMaxRange(b) - 1)
        XCTAssertEqual(tv.string, "\\begin{a12}\nzx\n\\end{a12}\n\\begin{b3}\ny\n\\end{b3}\n", "another pair: its own scan")
    }
}
