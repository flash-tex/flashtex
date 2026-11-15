import AppKit
import Darwin
import HostedWindows
import XCTest
@testable import FlashTeXMac

/// Vim line motions on a book-sized buffer (lane REVIEW-APP): `j`/`k`/`+`/`-`,
/// counts, `ip` and visual-line `J` must cost the same on a 3 MB document as
/// on a short one. They used to walk the whole buffer from offset 0 per
/// motion (`lineNumber(of:)`, `lineCount`, `lineStart(ofLine:)`), re-bridging
/// `textView.string` (a full copy) for every character read.
///
/// Measured on the main thread's CPU time (`thread_info`), the key handler
/// alone (the run loop turn after each key is outside the measurement), so
/// the figure does not move with the machine's load. The 16 ms main-turn
/// budget (#1900) is reported through `TimingBudget`; a motion that costs
/// more than 100 ms of CPU is broken on any hardware and fails everywhere.
@MainActor
final class VimLargeDocumentTests: XCTestCase {
    private var window: NSWindow!
    private var tv: CompletingTextView!

    override func setUp() async throws {
        HostedWindowSupport.prepare()
        window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled], backing: .buffered, defer: false)
        let scroll = CompletingTextView.scrollable()
        scroll.frame = window.contentView!.bounds
        window.contentView!.addSubview(scroll)
        tv = try XCTUnwrap(scroll.documentView as? CompletingTextView)
        tv.layoutManager?.allowsNonContiguousLayout = true // as the app's editor: layout near the caret only
        window.orderFrontRegardless() // never makeKey
        window.makeFirstResponder(tv)
        tv.vimEnabledOverride = true
    }

    override func tearDown() async throws {
        tv?.vimEnabledOverride = false
        window.orderOut(nil)
    }

    /// LaTeX shaped like a long book: chapters and sections, theorem and
    /// proof environments, displayed equations on their own
    /// `\begin{equation}`/`\end{equation}` lines, itemize lists, and prose
    /// paragraphs separated by blank lines.
    static func book(bytes: Int) -> String {
        var s = "\\documentclass{book}\n\\usepackage{amsmath}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < bytes {
            if n % 40 == 0 { s += "\\chapter{Chapter \(n / 40 + 1)}\n\n" }
            if n % 8 == 0 { s += "\\section{Section \(n / 8 + 1)}\n\\label{sec:\(n)}\n\n" }
            s += "The quick brown fox jumps over the lazy dog near the river bank.\n"
            s += "A \\textbf{bold} claim and an \\emph{emphasised} rebuttal follow it.\n"
            s += "Numbers such as $x^2 + y^2 = z^2$ appear inside the prose too.\n\n"
            s += "\\begin{theorem}\\label{thm:\(n)}\nEvery bounded sequence has a convergent subsequence.\n\\end{theorem}\n"
            s += "\\begin{proof}\nBy compactness of the closed interval.\n\\begin{equation}\n  a_{n_k} \\to a\n\\end{equation}\n\\end{proof}\n\n"
            if n % 3 == 0 { s += "\\begin{itemize}\n  \\item first\n  \\item second\n\\end{itemize}\n\n" }
            n += 1
        }
        return s + "\\end{document}\n"
    }

    /// The calling thread's CPU time, µs.
    private func cpuMicros() -> UInt64 {
        let port = mach_thread_self()
        defer { mach_port_deallocate(mach_task_self_, port) }
        var info = thread_basic_info()
        var count = mach_msg_type_number_t(MemoryLayout<thread_basic_info_data_t>.size / MemoryLayout<integer_t>.size)
        let kr = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) { thread_info(port, thread_flavor_t(THREAD_BASIC_INFO), $0, &count) }
        }
        guard kr == KERN_SUCCESS else { return 0 }
        return UInt64(info.user_time.seconds + info.system_time.seconds) * 1_000_000
            + UInt64(info.user_time.microseconds + info.system_time.microseconds)
    }

    /// One key through the editor's `keyDown`; returns the handler's CPU ms.
    @discardableResult
    private func key(_ chars: String, code: UInt16 = 0) -> Double {
        let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: [], timestamp: ProcessInfo.processInfo.systemUptime,
                                 windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: chars,
                                 charactersIgnoringModifiers: chars, isARepeat: false, keyCode: code)!
        let before = cpuMicros()
        tv.keyDown(with: e)
        let ms = Double(cpuMicros() &- before) / 1000
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.001))
        return ms
    }

    private func keys(_ s: String) -> [Double] {
        s.map { $0 == "\u{1B}" ? key("\u{1B}", code: 53) : key(String($0)) }
    }

    private var caret: Int { tv.selectedRange().location }

    func testLineMotionsOnABookSizedBufferStayInsideTheMainTurnBudget() {
        let text = Self.book(bytes: 3_000_000)
        let ns = text as NSString
        tv.string = text
        // A line in the middle of the book, its column remembered by `j`.
        let middle = ns.range(of: "The quick", range: NSRange(location: ns.length / 2, length: ns.length / 2)).location
        tv.setSelectedRange(NSRange(location: middle, length: 0))
        tv.scrollRangeToVisible(NSRange(location: middle, length: 0))
        RunLoop.main.run(until: Date(timeIntervalSinceNow: 0.05))
        key("l"); key("l") // warm the layout around the caret; column 2

        let tableBuilds = tv.vim.lineTableBuilds
        var costs: [String: Double] = [:]
        func run(_ name: String, _ s: String) {
            let c = keys(s)
            costs[name] = max(costs[name] ?? 0, c.max() ?? 0)
        }
        let startLine = ns.lineRange(for: NSRange(location: caret, length: 0))
        run("j", "j")
        let next = ns.lineRange(for: NSRange(location: NSMaxRange(startLine), length: 0))
        let nextChars = next.length - (ns.character(at: NSMaxRange(next) - 1) == 0x0A ? 1 : 0)
        XCTAssertEqual(caret, next.location + min(2, max(0, nextChars - 1)), "j keeps the column on the next line")
        run("k", "k")
        XCTAssertEqual(caret, startLine.location + 2, "k comes back")
        run("j", String(repeating: "j", count: 10))
        run("k", String(repeating: "k", count: 10))
        run("5j", "5j")
        run("+", "+")
        run("-", "-")
        run("vip", "vip\u{1B}")
        run("Vjj", "Vjj\u{1B}")
        XCTAssertEqual(tv.vim.mode, .normal)
        XCTAssertEqual(tv.vim.lineTableBuilds, tableBuilds, "relative line motions never number the whole buffer")

        let worst = costs.values.max() ?? 0
        let summary = costs.sorted { $0.key < $1.key }.map { String(format: "%@ %.2f", $0.key, $0.value) }.joined(separator: ", ")
        print("vim motions on \(ns.length / 1000) K UTF-16 (CPU ms, worst per key): \(summary)")
        TimingBudget.assertWithin(worst, 16, "worst vim line motion on a 3 MB buffer (main-thread CPU)")
        XCTAssertLessThan(worst, 100, "a line motion must not cost a whole-buffer walk: \(summary)")
    }

    /// Visual-line `J` joins the selected lines; `:N` and `G` still land on
    /// the right absolute line after an edit far above them.
    func testAbsoluteLinesStayCorrectAfterAnEdit() {
        let lines = (0..<2000).map { "line \($0)" }
        tv.string = lines.joined(separator: "\n")
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        keys("1500G")
        XCTAssertEqual(caret, (tv.string as NSString).range(of: "line 1499").location)
        keys("ggx") // edit at the top: every line start below shifts by one
        let builds = tv.vim.lineTableBuilds
        keys("1500G")
        XCTAssertEqual(tv.vim.lineTableBuilds, builds + 1, "an edit drops the line table; the next absolute jump rebuilds it once")
        XCTAssertEqual(caret, (tv.string as NSString).range(of: "line 1499").location)
        keys("VjJ")
        XCTAssertTrue(tv.string.contains("line 1499 line 1500\nline 1501"), "visual-line J joins the two selected lines")
        keys("G")
        XCTAssertEqual(caret, (tv.string as NSString).range(of: "line 1999").location)
    }
}
