import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// Large-document editor operations (lane mac-editor-a11y-2): selection
/// changes (select-all, shift-arrow over a long line), input-method
/// composition inside a long line, keyboard page-down through ~10k lines,
/// 200 diagnostic marks, and a keystroke near the START of the buffer, on a
/// 60 KB and a 560 KB prose document hosted like `ContentView` hosts the
/// editor (real `NSTextView`, real `ShellModel`, never key).
///
/// Every figure is wall and thread-CPU milliseconds; budgets are asserted on
/// CPU (this machine runs other agents' builds). The bench skips when the
/// 1-minute load average is above 20 unless `FLASHTEX_BENCH_FORCE` is set
/// (then the figures are printed, labelled, and the budgets still apply).
@MainActor
final class LargeDocumentEditorTests: XCTestCase {

    // MARK: fixtures

    /// Prose: ~56-byte lines in 6-line paragraphs separated by a blank line,
    /// a `\section` every 30 paragraphs, and one 6 KB line at a third of the
    /// document (the "long line" cases). 560 KB ≈ 10 000 lines.
    static func proseDocument(bytes: Int) -> String {
        var s = "\\documentclass{article}\n\\begin{document}\n"
        let lines = [
            "The quick brown fox jumps over the lazy dog near the river.\n",
            "Naïve café patrons — résumé in hand — watched the tide turn.\n",
            "Numbers such as $x^2 + y^2 = z^2$ appear inside the prose too.\n",
            "A \\textbf{bold} claim and an \\emph{emphasised} rebuttal follow.\n",
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit sed.\n",
            "Finally the paragraph ends with a short concluding sentence.\n",
        ]
        var paragraph = 0
        var longLineInserted = false
        while s.utf8.count < bytes {
            if paragraph % 30 == 0 { s += "\\section{Section \(paragraph / 30 + 1)}\n\n" }
            if !longLineInserted, s.utf8.count > bytes / 3 {
                s += String(repeating: "This sentence repeats on one very long line without a break, naïve as it is. ", count: 80) + "\n\n"
                longLineInserted = true
            }
            for line in lines { s += line }
            s += "\n"
            paragraph += 1
        }
        s += "\\end{document}\n"
        return s
    }

    final class Probe {
        var coordinator: SourceEditorView.Coordinator?
        var bindingSets = 0
        var bodyEvaluations = 0
    }

    struct Host: View {
        var model: ShellModel
        var probe: Probe
        var marks: [EditorDiagnostics.Mark]
        var body: some View {
            probe.bodyEvaluations += 1
            return SourceEditorView(
                text: Binding(get: { model.activeText }, set: { new in
                    model.updateActiveText(new); probe.bindingSets += 1
                }),
                selection: model.selection,
                pendingEdit: model.pendingEdit,
                marks: marks.isEmpty ? model.editorMarks : marks,
                result: model.result,
                onCaretChange: { model.caretUTF16 = $0 },
                onSelectionChange: { model.caretLengthUTF16 = $0.length },
                onEditApplied: { model.editApplied($0, newText: $1) },
                onEditRefused: { model.editRefused($0, reason: $1) }
            )
        }
    }

    func host(_ model: ShellModel, probe: Probe, marks: [EditorDiagnostics.Mark] = []) async throws -> (NSWindow, NSTextView) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, probe: probe, marks: marks))
        window.orderFrontRegardless() // never makeKey
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        let tv = try XCTUnwrap(found)
        probe.coordinator = tv.delegate as? SourceEditorView.Coordinator
        XCTAssertNotNil(probe.coordinator)
        XCTAssertTrue(window.makeFirstResponder(tv))
        return (window, tv)
    }

    func waitUntil(_ what: String, timeout: TimeInterval = 20, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    func turn() async throws { try await Task.sleep(nanoseconds: 30_000_000) }

    static func threadCpuNs() -> UInt64 { clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) }

    struct Sample: CustomStringConvertible {
        var wall: Double, cpu: Double
        var description: String { String(format: "%.3f ms wall / %.3f ms cpu", wall, cpu) }
    }

    /// (wall ms, cpu ms) of `body` followed by one drained run-loop turn (the
    /// coalesced announcement, SwiftUI's observation-driven view update).
    static func timedWithTurn(_ body: () -> Void) -> Sample {
        let w0 = MonotonicClock.nowNs(), c0 = threadCpuNs()
        body()
        RunLoop.main.run(mode: .default, before: Date())
        return Sample(wall: Double(MonotonicClock.nowNs() - w0) / 1e6, cpu: Double(threadCpuNs() - c0) / 1e6)
    }

    static func timed(_ body: () -> Void) -> Sample {
        let w0 = MonotonicClock.nowNs(), c0 = threadCpuNs()
        body()
        return Sample(wall: Double(MonotonicClock.nowNs() - w0) / 1e6, cpu: Double(threadCpuNs() - c0) / 1e6)
    }

    static func stats(_ samples: [Sample]) -> String {
        let cpu = LatencyStats(samples.map(\.cpu)), wall = LatencyStats(samples.map(\.wall))
        return String(format: "n=%d cpu p50 %.3f p95 %.3f max %.3f ms; wall p50 %.3f max %.3f ms",
                      cpu.count, cpu.p50Ms ?? 0, cpu.p95Ms ?? 0, cpu.maxMs ?? 0, wall.p50Ms ?? 0, wall.maxMs ?? 0)
    }

    /// Records `uptime` and skips on a busy machine (1-minute load > 20).
    private func requireCalmMachine() throws -> String {
        let uptime = IMEHarness.uptime()
        let load = IMEHarness.loadAverage1() ?? 0
        let forced = ProcessInfo.processInfo.environment["FLASHTEX_BENCH_FORCE"] != nil
        print("large-document bench: \(uptime)\(forced ? " (FLASHTEX_BENCH_FORCE set)" : "")")
        if load > 20, !forced { throw XCTSkip("1-minute load \(load) > 20; timing is not meaningful (\(uptime))") }
        return uptime
    }

    // MARK: the bench

    func testSixtyKilobyteOperations() async throws {
        try await measure(bytes: 60_000, label: "60 KB")
    }

    func testFiveHundredSixtyKilobyteOperations() async throws {
        try await measure(bytes: 560_000, label: "560 KB")
    }

    private func measure(bytes: Int, label: String) async throws {
        _ = try requireCalmMachine()
        let text = Self.proseDocument(bytes: bytes)
        let ns = text as NSString
        let lineCount = text.utf8.reduce(0) { $0 + ($1 == 0x0A ? 1 : 0) }
        XCTAssertGreaterThanOrEqual(text.utf8.count, bytes)
        let model = ShellModel()
        model.replaceProject(entryText: text)
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let lm = try XCTUnwrap(tv.layoutManager)
        var announced: [String] = []
        co.announce = { announced.append($0) }
        var report: [String] = ["\(label): \(text.utf8.count) bytes, \(ns.length) UTF-16, \(lineCount) lines"]

        // Full layout once (the app's background layout does this after a load).
        let layout = Self.timed { lm.ensureLayout(forCharacterRange: NSRange(location: 0, length: ns.length)) }
        report.append("full layout: \(layout)")
        try await turn()

        // 1. Select all (one selection change + coalesced announcement + view update).
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        try await turn()
        announced = []
        let selectAll = Self.timedWithTurn { tv.selectAll(nil) }
        report.append("select all: \(selectAll); announced \(announced.first?.prefix(60) ?? "nothing")")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 0, length: ns.length))
        XCTAssertEqual(model.caretLengthUTF16, ns.length)

        // 2. Shift-right ×40 over the long line, then shift-down ×40 over prose lines.
        let longLine = ns.range(of: "This sentence repeats")
        XCTAssertNotEqual(longLine.location, NSNotFound)
        tv.setSelectedRange(NSRange(location: longLine.location + 2_000, length: 0))
        tv.scrollRangeToVisible(tv.selectedRange())
        try await turn()
        var shiftRight: [Sample] = []
        for _ in 0..<40 { shiftRight.append(Self.timedWithTurn { tv.moveRightAndModifySelection(nil) }) }
        report.append("shift-right on the 6 KB line: \(Self.stats(shiftRight))")
        XCTAssertEqual(tv.selectedRange().length, 40)
        tv.setSelectedRange(NSRange(location: ns.length / 2, length: 0))
        tv.scrollRangeToVisible(tv.selectedRange())
        try await turn()
        var shiftDown: [Sample] = []
        for _ in 0..<40 { shiftDown.append(Self.timedWithTurn { tv.moveDownAndModifySelection(nil) }) }
        report.append("shift-down over prose lines: \(Self.stats(shiftDown))")
        XCTAssertGreaterThan(tv.selectedRange().length, 40 * 20)
        // Growing selection over the whole tail: the announcement counts the selection.
        tv.setSelectedRange(NSRange(location: 0, length: ns.length - 10))
        try await turn()
        var shiftRightHuge: [Sample] = []
        for _ in 0..<5 { shiftRightHuge.append(Self.timedWithTurn { tv.moveRightAndModifySelection(nil) }) }
        report.append("shift-right extending a whole-document selection: \(Self.stats(shiftRightHuge))")

        // 3. Page down from the top through the whole document, then arrow-down ×100.
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.scrollRangeToVisible(NSRange(location: 0, length: 0))
        try await turn()
        var pageDowns: [Sample] = []
        var pages = 0
        while tv.selectedRange().location < ns.length - 1, pages < 2_000 {
            let before = tv.selectedRange().location
            pageDowns.append(Self.timedWithTurn { tv.pageDown(nil) })
            pages += 1
            if tv.selectedRange().location == before { break }
        }
        report.append("page down ×\(pages) through \(lineCount) lines: \(Self.stats(pageDowns))")
        XCTAssertGreaterThan(pages, 10)
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.scrollRangeToVisible(NSRange(location: 0, length: 0))
        try await turn()
        var arrowDowns: [Sample] = []
        for _ in 0..<100 { arrowDowns.append(Self.timedWithTurn { tv.moveDown(nil) }) }
        report.append("arrow down ×100: \(Self.stats(arrowDowns))")

        // 4. Input-method composition inside the long line (10 steps + commit).
        let composeAt = longLine.location + 3_000
        tv.setSelectedRange(NSRange(location: composeAt, length: 0))
        tv.scrollRangeToVisible(tv.selectedRange())
        try await turn()
        let revisionBefore = model.editorRevision
        let bindingSetsBefore = probe.bindingSets
        var composeSteps: [Sample] = []
        let kana = ["か", "かん", "かんじ", "かんじへ", "かんじへん", "漢字変", "漢字変換", "漢字変換で", "漢字変換です", "漢字変換です。"]
        for step in kana {
            composeSteps.append(Self.timedWithTurn {
                tv.setMarkedText(step, selectedRange: NSRange(location: (step as NSString).length, length: 0),
                                 replacementRange: NSRange(location: NSNotFound, length: 0))
            })
        }
        XCTAssertTrue(tv.hasMarkedText())
        XCTAssertEqual(model.editorRevision, revisionBefore, "no revision per composition step")
        XCTAssertEqual(probe.bindingSets, bindingSetsBefore, "nothing reached the binding during the composition")
        let commit = Self.timedWithTurn { tv.insertText("漢字変換です。", replacementRange: NSRange(location: NSNotFound, length: 0)) }
        report.append("IME composition in the 6 KB line: steps \(Self.stats(composeSteps)); commit \(commit)")
        XCTAssertFalse(tv.hasMarkedText())
        XCTAssertEqual(model.editorRevision, revisionBefore + 1, "the commit is one revision")
        XCTAssertTrue(model.activeText.contains("漢字変換です。"))

        // 5. 200 marks: first paint, all shifted by one keystroke, unchanged.
        let marks = SourceEditorViewTests.marks(count: 200, in: model.activeText)
        let firstPaint = Self.timed { co.marks.update(marks, in: tv, reset: false) }
        let shifted = marks.map { m in
            SourceEditorViewTests.mark(NSRange(location: m.nsRange.location + 1, length: m.nsRange.length), m.severity, m.message,
                                       index: m.identity.index)
        }
        let allShifted = Self.timed { co.marks.update(shifted, in: tv, reset: false) }
        let unchanged = Self.timed { co.marks.update(shifted, in: tv, reset: false) }
        report.append("200 marks: first paint \(firstPaint); all shifted \(allShifted); unchanged \(unchanged); painted ranges \(co.marks.painted.count)")
        XCTAssertLessThan(unchanged.cpu, 0.5)

        // 6. Keystrokes near the START of the buffer (everything after the caret
        // is behind the edit) vs near the end, whole keystroke incl. the turn.
        let startAt = ns.range(of: "The quick brown fox").location
        tv.setSelectedRange(NSRange(location: startAt, length: 0))
        tv.scrollRangeToVisible(tv.selectedRange())
        try await turn()
        var startKeys: [Sample] = []
        for _ in 0..<20 { startKeys.append(Self.timedWithTurn { tv.insertText("z", replacementRange: tv.selectedRange()) }) }
        report.append("keystrokes at the start of the buffer: \(Self.stats(startKeys))")
        let endAt = (model.activeText as NSString).range(of: "\\end{document}", options: .backwards).location
        tv.setSelectedRange(NSRange(location: endAt, length: 0))
        tv.scrollRangeToVisible(tv.selectedRange())
        try await turn()
        var endKeys: [Sample] = []
        for _ in 0..<20 { endKeys.append(Self.timedWithTurn { tv.insertText("z", replacementRange: tv.selectedRange()) }) }
        report.append("keystrokes at the end of the buffer: \(Self.stats(endKeys))")
        XCTAssertEqual(model.activeText.utf8.count, text.utf8.count + 40 + "漢字変換です。".utf8.count)
        XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
        report.append("SwiftUI body evaluations: \(probe.bodyEvaluations); binding sets: \(probe.bindingSets)")

        print("large-document bench " + report.joined(separator: "\n  "))
    }

    // MARK: delimiter pre-check (pure)

    func testDelimiterAdjacentIsAnExactPrefilterOfTheMatcher() {
        let text = "a{b}c [d] $e$ \\{f\\} % {g}\nnaïve 👩‍💻{h}"
        let storage = NSTextStorage(string: text)
        let ns = text as NSString
        let units = Set("{}[]$".utf16)
        for caret in 0...ns.length {
            let adjacent = SourceEditorView.BraceMatcher.delimiterAdjacent(in: storage, caretUTF16: caret)
            let match = SourceEditorView.BraceMatcher.match(in: text, caretUTF16: caret)
            if match != nil { XCTAssertTrue(adjacent, "caret \(caret) has a match, so the pre-check must say yes") }
            let before: unichar = caret > 0 ? ns.character(at: caret - 1) : 0
            let at: unichar = caret < ns.length ? ns.character(at: caret) : 0
            XCTAssertEqual(adjacent, units.contains(before) || units.contains(at), "caret \(caret)")
        }
        XCTAssertFalse(SourceEditorView.BraceMatcher.delimiterAdjacent(in: storage, caretUTF16: -1))
        XCTAssertFalse(SourceEditorView.BraceMatcher.delimiterAdjacent(in: storage, caretUTF16: ns.length + 1))
        XCTAssertFalse(SourceEditorView.BraceMatcher.delimiterAdjacent(in: nil, caretUTF16: 0))
    }

    /// The highlight still appears and moves when typing next to a delimiter
    /// on a large buffer, and typing prose leaves it untouched and silent.
    func testHighlightStillFollowsDelimitersOnALargeBuffer() async throws {
        let text = Self.proseDocument(bytes: 60_000)
        let model = ShellModel()
        model.replaceProject(entryText: text)
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let ns = text as NSString
        let bold = ns.range(of: "\\textbf{bold}")
        XCTAssertNotEqual(bold.location, NSNotFound)
        // Caret after `{`: the pair is highlighted.
        tv.setSelectedRange(NSRange(location: bold.location + 8, length: 0))
        try await turn()
        XCTAssertEqual(co.braceHighlight?.open, NSRange(location: bold.location + 7, length: 1))
        XCTAssertEqual(co.braceHighlight?.close, NSRange(location: bold.location + 12, length: 1))
        // Typing inside the braces moves the caret off the opener (no pair at
        // the caret, as before); stepping back onto it shows the shifted pair.
        tv.insertText("x", replacementRange: tv.selectedRange())
        try await turn()
        XCTAssertNil(co.braceHighlight)
        tv.moveLeft(nil)
        try await turn()
        XCTAssertEqual(co.braceHighlight?.open, NSRange(location: bold.location + 7, length: 1))
        XCTAssertEqual(co.braceHighlight?.close, NSRange(location: bold.location + 13, length: 1))
        XCTAssertTrue(co.announcements.last?.hasSuffix(", matches line \(SourceEditorView.lineColumn(text: model.activeText, utf16: bold.location + 13)!.line) column \(SourceEditorView.lineColumn(text: model.activeText, utf16: bold.location + 13)!.column)") ?? false, "\(co.announcements.last ?? "nothing")")
        // Caret in the middle of a word: no highlight, and typing there stays silent.
        let fox = ns.range(of: "quick brown").location
        tv.setSelectedRange(NSRange(location: fox + 3, length: 0))
        try await turn()
        XCTAssertNil(co.braceHighlight)
        let announcedBefore = co.announcements.count
        tv.insertText("y", replacementRange: tv.selectedRange())
        try await turn()
        XCTAssertNil(co.braceHighlight)
        XCTAssertEqual(co.announcements.count, announcedBefore, "typing is not announced")
        XCTAssertTrue(model.activeText.contains("\\textbf{xbold}"))
        XCTAssertTrue(model.activeText.contains("quiyck brown"))
        XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
    }
}

/// Durable edits on the large buffer through the REAL preview controller and
/// compiler (skips without `FLASHTEX_PREVIEW_CONTROLLER`/`FLASHTEX_COMPILER`):
/// a 500 KB paste into a 60 KB document, a whole-document selection, a
/// shift-arrow selection and typing at the start of the 560 KB buffer all
/// end durable, byte-exact, with one revision per edit.
extension LargeDocumentEditorTests {
    func testLargePasteSelectionAndTypingEndDurableThroughTheRealHelper() async throws {
        let base = Self.proseDocument(bytes: 60_000)
        let h = try await IMEHarness.attached("large-doc", text: base)
        defer { h.close() }
        let model = h.model, tv = h.textView!
        let d1 = try await h.helperDocument()
        XCTAssertEqual(d1.revision, 1)
        let revBefore = model.editorRevision

        // Paste (one insertText, as `paste:` does) 500 KB before \end{document}.
        let block = Self.proseDocument(bytes: 500_000)
            .replacingOccurrences(of: "\\documentclass{article}\n\\begin{document}\n", with: "")
            .replacingOccurrences(of: "\\end{document}\n", with: "")
        let at = (base as NSString).range(of: "\\end{document}", options: .backwards).location
        tv.setSelectedRange(NSRange(location: at, length: 0))
        let paste = Self.timed { tv.insertText(block, replacementRange: NSRange(location: at, length: 0)) }
        let afterPaste = model.editorRevision
        XCTAssertEqual(afterPaste, revBefore + 1, "the paste is one revision")
        let t0 = MonotonicClock.nowNs()
        // The helper numbers its own revisions (r1 = open): the paste is r2.
        try await h.waitUntil("paste durable", timeout: 120) {
            model.controllerState.durable["main.tex"]?.revision == d1.revision + 1
        }
        XCTAssertEqual(model.controllerState.editorRevisionByDurable["main.tex"]?[d1.revision + 1], afterPaste)
        let pasteDurableMs = Double(MonotonicClock.nowNs() - t0) / 1e6
        let d2 = try await h.helperDocument()
        XCTAssertEqual(d2.revision, d1.revision + 1)
        XCTAssertTrue(d2.text.sameBytes(as: model.activeText))
        XCTAssertGreaterThanOrEqual(model.activeText.utf8.count, 560_000)

        // Whole-document selection, then a shift-arrow: no revision, nothing sent.
        tv.selectAll(nil)
        tv.moveLeftAndModifySelection(nil)
        try await h.turn()
        XCTAssertEqual(model.editorRevision, afterPaste, "selection changes are not edits")
        XCTAssertEqual(model.caretLengthUTF16, tv.selectedRange().length)

        // Type at the start of the buffer: one revision per keystroke, all durable.
        let start = (model.activeText as NSString).range(of: "The quick brown fox").location
        tv.setSelectedRange(NSRange(location: start, length: 0))
        tv.scrollRangeToVisible(tv.selectedRange())
        try await h.turn()
        var keys: [Sample] = []
        for ch in ["Z", "é", "→"] {
            keys.append(Self.timed { tv.insertText(ch, replacementRange: tv.selectedRange()) })
        }
        let final = model.editorRevision
        XCTAssertEqual(final, afterPaste + 3)
        let t1 = MonotonicClock.nowNs()
        // Keystrokes typed while an edit is in flight are grouped into the next
        // submission: the durable document reaches `final` in 1–3 helper revisions.
        try await h.waitUntil("typing durable", timeout: 120) {
            model.controllerState.editorRevisionByDurable["main.tex"]?.values.contains(final) == true
        }
        let typingDurableMs = Double(MonotonicClock.nowNs() - t1) / 1e6
        let d5 = try await h.helperDocument()
        XCTAssertTrue((d1.revision + 2...d1.revision + 4).contains(d5.revision), "helper revision \(d5.revision)")
        XCTAssertTrue(d5.text.sameBytes(as: model.activeText), "the helper holds the typed buffer byte for byte")
        XCTAssertTrue(model.activeText.sameBytes(as: SourceEditorView.nativeText(of: tv)))
        XCTAssertTrue(d5.text.contains("Zé→The quick brown fox"))
        XCTAssertEqual(d5.sha256, SourceDigest.sha256Hex(model.activeText))
        print("large-document durable (real helper, \(model.activeText.utf8.count) bytes): paste \(paste), durable after \(Int(pasteDurableMs)) ms; keystrokes at the start \(Self.stats(keys)), durable after \(Int(typingDurableMs)) ms; \(IMEHarness.uptime())")
    }

    // MARK: bounded selection announcement (lane mac-editor-a11y-3)

    /// Below the limit the bounded form is the unbounded one; above it the
    /// line-span form, exact at both ends, including a selection that ends at
    /// column 1 (that line is not counted) and an invalid range (nil).
    func testBoundedSelectionAnnouncementSwitchesToLineSpanAboveTheLimit() {
        let limit = SourceEditorView.largeSelectionAnnouncementLimit
        let small = "ab\ncdé\nf"
        for range in [NSRange(location: 0, length: 0), NSRange(location: 1, length: 4), NSRange(location: 0, length: 8)] {
            XCTAssertEqual(SourceEditorView.boundedSelectionAnnouncement(text: small, range: range),
                           SourceEditorView.selectionAnnouncement(text: small, range: range), "\(range)")
        }
        // 1 000 lines of 70 UTF-16 units (some non-ASCII), 70 000 > limit.
        let line = "Naïve café patrons — résumé in hand — watched the tide turn twice ok.\n"
        XCTAssertEqual(line.utf16.count, 70)
        let big = String(repeating: line, count: 1_000)
        let all = NSRange(location: 0, length: big.utf16.count)
        XCTAssertGreaterThan(all.length, limit)
        XCTAssertEqual(SourceEditorView.boundedSelectionAnnouncement(text: big, range: all),
                       "Selected 1000 lines, line 1 column 1 to line 1001 column 1")
        // 65 537 units from offset 3 end at unit 65 540 = line 937 (65540 / 70 + 1), column 21.
        XCTAssertEqual(SourceEditorView.boundedSelectionAnnouncement(text: big, range: NSRange(location: 3, length: limit + 1)),
                       "Selected 937 lines, line 1 column 4 to line 937 column 21")
        // Exactly at the limit: still the exact grapheme count (65 536 units end at line 937 column 17).
        let atLimit = NSRange(location: 0, length: limit)
        let unbounded = SourceEditorView.selectionAnnouncement(text: big, range: atLimit)
        XCTAssertEqual(SourceEditorView.boundedSelectionAnnouncement(text: big, range: atLimit), unbounded)
        let graphemes = String(big.utf16.prefix(limit))?.count ?? -1
        XCTAssertEqual(graphemes, limit, "precomposed BMP prose: one grapheme per unit")
        XCTAssertEqual(unbounded, "Selected \(graphemes) characters, line 1 column 1 to line 937 column 17")
        XCTAssertNil(SourceEditorView.boundedSelectionAnnouncement(text: big, range: NSRange(location: 0, length: all.length + 1)))
        XCTAssertNil(SourceEditorView.boundedSelectionAnnouncement(text: big, range: NSRange(location: -1, length: limit + 2)))
    }

    /// Whole-document selection on the 560 KB buffer, once more: the select-all
    /// through the hosted editor as it stands, then the two announcement forms
    /// timed directly on the same text. Figures are printed with `uptime`;
    /// the only assertion is that the bounded form stays under 5 ms CPU.
    func testWholeDocumentSelectionCostAndBoundedAnnouncement() async throws {
        let uptime = try requireCalmMachine()
        let text = Self.proseDocument(bytes: 560_000)
        let ns = text as NSString
        let model = ShellModel()
        model.replaceProject(entryText: text)
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let lm = try XCTUnwrap(tv.layoutManager)
        var announced: [String] = []
        co.announce = { announced.append($0) }
        lm.ensureLayout(forCharacterRange: NSRange(location: 0, length: ns.length))
        try await turn()
        var selectAlls: [Sample] = []
        for _ in 0..<3 {
            tv.setSelectedRange(NSRange(location: 0, length: 0))
            try await turn()
            announced = []
            selectAlls.append(Self.timedWithTurn { tv.selectAll(nil) })
            try await turn()
        }
        let all = NSRange(location: 0, length: ns.length)
        let native = co.currentText(of: tv)
        _ = SourceEditorView.lineColumn(text: native, utf16: ns.length) // breadcrumbs warm, as in the coordinator
        var unbounded: [Sample] = [], bounded: [Sample] = []
        var messages: (String?, String?) = (nil, nil)
        for _ in 0..<5 {
            unbounded.append(Self.timed { messages.0 = SourceEditorView.selectionAnnouncement(text: native, range: all) })
            bounded.append(Self.timed { messages.1 = SourceEditorView.boundedSelectionAnnouncement(text: native, range: all) })
        }
        let boundedCpu = LatencyStats(bounded.map(\.cpu))
        print("""
            whole-document selection (560 KB, \(ns.length) UTF-16): select all ×3 \(Self.stats(selectAlls)); \
            announced through the coordinator: \(announced.first?.prefix(70) ?? "nothing"); \
            announcement alone, unbounded \(Self.stats(unbounded)) → "\(messages.0 ?? "nil")"; \
            bounded \(Self.stats(bounded)) → "\(messages.1 ?? "nil")"; \(uptime)
            """)
        XCTAssertLessThan(boundedCpu.p50Ms ?? .infinity, 5, "bounded announcement stays cheap")
        XCTAssertEqual(messages.1?.hasPrefix("Selected ") ?? false, true)
        XCTAssertTrue(messages.1?.contains(" lines, line 1 column 1 to line ") ?? false, messages.1 ?? "nil")
    }
}
