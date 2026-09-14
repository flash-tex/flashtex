import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// `SourceEditorView`: VoiceOver label/value/selection and line-column
/// announcements, windowed diagnostic marks on a large buffer, navigation
/// selections that never fight typing, one-step capture undo through the
/// model, and the large-document keystroke round trip with correct UTF-8
/// caret offsets. Hosted tests put the real `NSTextView` in an `NSWindow`
/// through `NSHostingView` (never key: the tests must not steal focus).
@MainActor
final class SourceEditorViewTests: XCTestCase {

    // MARK: fixtures

    /// ASCII plus multi-byte text on every line, `bytes` or slightly more UTF-8 bytes.
    static func largeDocument(bytes: Int) -> String {
        var s = "\\documentclass{article}\n\\begin{document}\n"
        var n = 0
        while s.utf8.count < bytes {
            s += "Paragraph \(n): the quick brown fox — naïve café \\textbf{bold} $x^2 + y^2 = z^2$ jumps over the lazy dog.\n"
            n += 1
        }
        s += "\\end{document}\n"
        return s
    }

    /// A mark with a synthetic identity (the painter only uses range, severity and tooltip).
    static func mark(_ range: NSRange, _ severity: RuntimeV1.Severity, _ message: String, recovery: String? = nil,
                     index: Int = 0) -> EditorDiagnostics.Mark {
        EditorDiagnostics.Mark(identity: .init(resultID: "r", index: index,
                                               source: .init(path: "main.tex", startByte: range.location, endByte: NSMaxRange(range))),
                               nsRange: range, severity: severity, message: message, recovery: recovery, resultStatus: .ok)
    }

    static func marks(count: Int, in text: String) -> [EditorDiagnostics.Mark] {
        let len = (text as NSString).length
        return (0..<count).map { i in
            mark(NSRange(location: i * (len / count), length: 5), i % 3 == 0 ? .error : .warning, "mark \(i)", index: i)
        }
    }

    /// Hosts the real editor bound to a `ShellModel`, like `ContentView` does.
    private final class Probe {
        var editApplied: [(ShellModel.PendingEdit, String)] = []
        var editRefused: [(ShellModel.PendingEdit, String)] = []
        var bindingSetNs: UInt64 = 0
        var bindingSetCpuNs: UInt64 = 0
        var coordinator: SourceEditorView.Coordinator?
    }

    private struct Host: View {
        var model: ShellModel
        var probe: Probe
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { new in
                    model.updateActiveText(new)
                    probe.bindingSetNs = MonotonicClock.nowNs()
                    probe.bindingSetCpuNs = SourceEditorViewTests.threadCpuNs()
                }),
                selection: model.selection,
                pendingEdit: model.pendingEdit,
                marks: model.editorMarks,
                result: model.result,
                onCaretChange: { model.caretUTF16 = $0 },
                onSelectionChange: { model.caretLengthUTF16 = $0.length },
                onEditApplied: { edit, text in
                    probe.editApplied.append((edit, text))
                    model.editApplied(edit, newText: text)
                },
                onEditRefused: { edit, reason in
                    probe.editRefused.append((edit, reason))
                    model.editRefused(edit, reason: reason)
                }
            )
        }
    }

    private func host(_ model: ShellModel, probe: Probe) async throws -> (NSWindow, NSTextView) {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, probe: probe))
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

    private func waitUntil(_ what: String, timeout: TimeInterval = 10, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
    }

    /// Lets the current run-loop turn end (coalesced announcements, async edit delivery).
    private func turn() async throws { try await Task.sleep(nanoseconds: 30_000_000) }

    /// CPU time of the current thread. Budget assertions use it because this
    /// machine runs several agents' builds concurrently: wall-clock outliers
    /// there are preemption, not editor work (wall figures are still printed).
    static func threadCpuNs() -> UInt64 { clock_gettime_nsec_np(CLOCK_THREAD_CPUTIME_ID) }

    /// (wall ms, cpu ms) of `body`.
    static func timed(_ body: () -> Void) -> (wall: Double, cpu: Double) {
        let w0 = MonotonicClock.nowNs(), c0 = threadCpuNs()
        body()
        return (Double(MonotonicClock.nowNs() - w0) / 1e6, Double(threadCpuNs() - c0) / 1e6)
    }

    // MARK: line / column (pure)

    func testSelectionAnnouncementCountsLinesAndUserPerceivedColumns() {
        let text = "ab\nnaïve 👩‍💻x\r\nlast\rline"
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: 0, length: 0)), "Line 1, column 1")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: 2, length: 0)), "Line 1, column 3")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: 3, length: 0)), "Line 2, column 1")
        // "naïve " is 6 user characters; the emoji is 5 UTF-16 units (ZWJ sequence).
        let emoji = (text as NSString).range(of: "👩‍💻")
        XCTAssertEqual(emoji.length, 5)
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: emoji.location, length: 0)), "Line 2, column 7")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: NSMaxRange(emoji), length: 0)), "Line 2, column 8")
        XCTAssertNil(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: emoji.location + 1, length: 0)),
                     "inside a surrogate pair is not a caret position")
        // CRLF is one line break; a bare CR is a line break too.
        let last = (text as NSString).range(of: "last").location
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: last, length: 0)), "Line 3, column 1")
        let line = (text as NSString).range(of: "line").location
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: line, length: 2)),
                       "Selected 2 characters, line 4 column 1 to 3")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: 1, length: 3)),
                       "Selected 3 characters, line 1 column 2 to line 2 column 2")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: emoji.location, length: 5)),
                       "Selected 1 character, line 2 column 7 to 8")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: (text as NSString).length, length: 0)), "Line 4, column 5")
        XCTAssertNil(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: (text as NSString).length + 1, length: 0)))
        XCTAssertNil(SourceEditorView.selectionAnnouncement(text: text, range: NSRange(location: 1, length: 100)))
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: "", range: NSRange(location: 0, length: 0)), "Line 1, column 1")
        XCTAssertEqual(SourceEditorView.caretByte(text: text, utf16: NSMaxRange(emoji)), "ab\nnaïve 👩‍💻".utf8.count)
        XCTAssertNil(SourceEditorView.caretByte(text: text, utf16: emoji.location + 1))
    }

    func testLineColumnOnLargeBufferIsSubMillisecond() {
        let text = Self.largeDocument(bytes: 60_000)
        let end = (text as NSString).length
        var lc: (line: Int, column: Int)?
        let t = Self.timed { lc = SourceEditorView.lineColumn(text: text, utf16: end) }
        XCTAssertEqual(lc?.line, text.split(separator: "\n", omittingEmptySubsequences: false).count)
        XCTAssertEqual(lc?.column, 1)
        XCTAssertLessThan(t.cpu, 1.0, "line/column at the end of a 60 KB buffer took \(t.cpu) ms CPU (\(t.wall) ms wall)")
        // Column counting is per line, so a caret inside the last line is exact too.
        let lastLine = "\\end{document}\n"
        XCTAssertEqual(SourceEditorView.lineColumn(text: text, utf16: end - 1)?.column, (lastLine as NSString).length)
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: "a\r\nb", range: NSRange(location: 3, length: 0)), "Line 2, column 1")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: "a\rb\nc", range: NSRange(location: 4, length: 0)), "Line 3, column 1")
        XCTAssertEqual(SourceEditorView.selectionAnnouncement(text: "a\rb\nc", range: NSRange(location: 3, length: 0)), "Line 2, column 2")
    }

    func testNativeTextIsByteEqualToTheStorageAndCheapToCompare() {
        let tv = NSTextView(frame: .zero)
        XCTAssertEqual(SourceEditorView.nativeText(of: tv), "")
        let text = Self.largeDocument(bytes: 60_000) + "👩‍💻 end"
        tv.string = text
        var native = ""
        let convert = Self.timed { native = SourceEditorView.nativeText(of: tv) }
        XCTAssertTrue(native.sameBytes(as: text))
        XCTAssertTrue(native.isContiguousUTF8)
        var same = false
        let compare = Self.timed { same = native.sameBytes(as: text) }
        XCTAssertTrue(same)
        XCTAssertLessThan(convert.cpu, 1.0, "native conversion took \(convert.cpu) ms CPU (\(convert.wall) ms wall)")
        XCTAssertLessThan(compare.cpu, 0.2, "byte comparison of the native copy took \(compare.cpu) ms CPU (\(compare.wall) ms wall)")
        // An unpaired surrogate cannot be encoded: the bridge's replacement is used.
        tv.string = "a" + String(utf16CodeUnits: [0xD800], count: 1) + "b"
        XCTAssertEqual(SourceEditorView.nativeText(of: tv), tv.string)
        XCTAssertEqual(SourceEditorView.nativeText(of: tv).unicodeScalars.count, 3)
    }

    // MARK: VoiceOver

    func testEditorExposesLabelValueAndSelectedTextRangeAndAnnouncesCaretMoves() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "first line\nsecond naïve line\n")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        var spoken: [String] = []
        co.announce = { spoken.append($0) }

        XCTAssertEqual(tv.accessibilityLabel(), "LaTeX source")
        XCTAssertEqual(tv.accessibilityRole(), .textArea)
        XCTAssertEqual(tv.accessibilityValue() as? String, model.activeText)
        XCTAssertEqual(tv.accessibilityNumberOfCharacters(), (model.activeText as NSString).length)
        XCTAssertNotNil(tv.accessibilityHelp())
        XCTAssertNil(tv.textLayoutManager, "TextKit 1 was selected up front (temporary attributes)")

        // A caret move (arrow key / click) announces once per turn, after the turn.
        tv.setSelectedRange(NSRange(location: 11, length: 0))
        tv.setSelectedRange(NSRange(location: 18, length: 0))
        XCTAssertEqual(spoken, [])
        try await turn()
        XCTAssertEqual(spoken, ["Line 2, column 8"])
        XCTAssertEqual(tv.accessibilitySelectedTextRange(), NSRange(location: 18, length: 0))
        XCTAssertEqual(model.caretUTF16, 18)

        // Selecting announces the extent.
        tv.setSelectedRange(NSRange(location: 0, length: 5))
        try await turn()
        XCTAssertEqual(spoken.last, "Selected 5 characters, line 1 column 1 to 6")
        XCTAssertEqual(tv.accessibilitySelectedTextRange(), NSRange(location: 0, length: 5))
        XCTAssertEqual(model.caretLengthUTF16, 5)

        // Typing is not announced (VoiceOver reads the typed text itself), and
        // the binding carried the edit.
        spoken = []
        tv.setSelectedRange(NSRange(location: 5, length: 0))
        try await turn()
        spoken = []
        tv.insertText("!", replacementRange: NSRange(location: 5, length: 0))
        tv.insertText("?", replacementRange: NSRange(location: 6, length: 0))
        try await turn()
        XCTAssertEqual(spoken, [], "typing steps are not announced")
        XCTAssertEqual(model.activeText, "first!? line\nsecond naïve line\n")
        XCTAssertEqual(tv.accessibilityValue() as? String, model.activeText)
        XCTAssertEqual(model.caretUTF16, 7)

        // A navigation selection announces immediately and is exposed to AX.
        model.selection = .init(path: "main.tex", nsRange: NSRange(location: 13, length: 6), token: 1)
        try await waitUntil("navigation applied") { tv.selectedRange() == NSRange(location: 13, length: 6) }
        XCTAssertEqual(spoken, ["Selected 6 characters, line 2 column 1 to 7"])
        XCTAssertEqual(tv.accessibilitySelectedTextRange(), NSRange(location: 13, length: 6))
        try await turn()
        XCTAssertEqual(spoken.count, 1, "the programmatic selection change is not announced twice")
    }

    // MARK: marks

    func testMarksArePaintedOnlyAroundTheVisibleWindowAndFast() throws {
        let text = Self.largeDocument(bytes: 60_000)
        let scroll = CompletingTextView.scrollable()
        let tv = scroll.documentView as! NSTextView
        _ = tv.layoutManager
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = scroll
        window.orderFrontRegardless()
        defer { window.orderOut(nil) }
        tv.string = text
        let lm = try XCTUnwrap(tv.layoutManager)
        // Steady state: the document is laid out before diagnostics arrive
        // (TextKit 1 lays the whole text out in the background after display).
        lm.ensureLayout(for: try XCTUnwrap(tv.textContainer))
        let marks = Self.marks(count: 200, in: text)
        XCTAssertEqual(marks.count, 200)
        let painter = SourceEditorView.MarkPainter()

        let first = Self.timed { painter.update(marks, in: tv, reset: false) }
        XCTAssertEqual(painter.paints, 1)
        let window0 = SourceEditorView.MarkPainter.window(for: tv)
        XCTAssertEqual(window0.location, 0)
        XCTAssertLessThan(window0.length, (text as NSString).length / 2, "only a window around the visible text is painted")
        XCTAssertEqual(painter.painted, [window0])

        func underlined(_ mark: EditorDiagnostics.Mark) -> Bool {
            lm.temporaryAttribute(.underlineStyle, atCharacterIndex: mark.nsRange.location, effectiveRange: nil) != nil
        }
        let inside = marks.filter { NSMaxRange($0.nsRange) <= NSMaxRange(window0) }
        let outside = marks.filter { $0.nsRange.location >= NSMaxRange(window0) }
        XCTAssertGreaterThan(inside.count, 0); XCTAssertGreaterThan(outside.count, 100)
        XCTAssertTrue(inside.allSatisfy(underlined))
        XCTAssertFalse(outside.contains(where: underlined))
        XCTAssertEqual(lm.temporaryAttribute(.underlineColor, atCharacterIndex: marks[0].nsRange.location, effectiveRange: nil) as? NSColor, .systemRed)
        XCTAssertEqual(lm.temporaryAttribute(.toolTip, atCharacterIndex: marks[1].nsRange.location, effectiveRange: nil) as? String, "mark 1")

        // Unchanged marks cost nothing.
        let same = Self.timed { painter.update(marks, in: tv, reset: false) }
        XCTAssertEqual(painter.paints, 1)

        // Every mark moved (typing before them rebases all 200): clear + repaint the window.
        let shifted = marks.map { Self.mark(NSRange(location: $0.nsRange.location + 1, length: 5), $0.severity, $0.message,
                                            index: $0.diagnosticIndex) }
        let moved = Self.timed { painter.update(shifted, in: tv, reset: false) }
        XCTAssertEqual(painter.paints, 2)
        XCTAssertNil(lm.temporaryAttribute(.underlineStyle, atCharacterIndex: marks[1].nsRange.location, effectiveRange: nil))
        XCTAssertNotNil(lm.temporaryAttribute(.underlineStyle, atCharacterIndex: shifted[1].nsRange.location, effectiveRange: nil))

        // Scrolling to the end paints the newly visible window only.
        tv.scrollRangeToVisible(NSRange(location: (text as NSString).length, length: 0))
        let scrolled = Self.timed { painter.scrolled(tv) }
        XCTAssertEqual(painter.paints, 3)
        XCTAssertTrue(underlined(shifted[199]), "the last mark is painted once its window is visible")
        XCTAssertEqual(painter.painted.count, 2, "two disjoint painted windows; the text between them is untouched")
        XCTAssertFalse(underlined(shifted[100]), "marks between the two windows are still unpainted")
        painter.scrolled(tv)
        XCTAssertEqual(painter.paints, 3, "a scroll inside the painted range repaints nothing")

        let passes = [("first", first), ("unchanged", same), ("all-shifted", moved), ("scroll-to-end", scrolled)]
        print("marks (60 KB, 200 marks, debug build): " + passes.map { "\($0.0) \($0.1.cpu) ms CPU / \($0.1.wall) ms wall" }.joined(separator: ", "))
        for (name, t) in passes {
            XCTAssertLessThan(t.cpu, 2.0, "\(name) mark pass took \(t.cpu) ms CPU (\(t.wall) ms wall)")
        }

        // A text reset drops every temporary attribute; the painter starts over.
        tv.string = text
        painter.update(shifted, in: tv, reset: true)
        XCTAssertEqual(painter.paints, 4)
        XCTAssertEqual(painter.painted, [SourceEditorView.MarkPainter.window(for: tv)])
        painter.update([], in: tv, reset: false)
        XCTAssertEqual(painter.paints, 4)
        XCTAssertFalse(shifted.contains(where: underlined), "clearing marks removes every painted underline")
    }

    func testPainterTracksGapsAndEdits() {
        let painter = SourceEditorView.MarkPainter()
        XCTAssertEqual(painter.gaps(in: NSRange(location: 10, length: 20)), [NSRange(location: 10, length: 20)])
        XCTAssertEqual(SourceEditorView.MarkPainter.merged([NSRange(location: 30, length: 10), NSRange(location: 0, length: 10),
                                                            NSRange(location: 10, length: 5), NSRange(location: 12, length: 0)]),
                       [NSRange(location: 0, length: 15), NSRange(location: 30, length: 10)])
        // Gaps around and between painted ranges.
        let tv = NSTextView(frame: .zero)
        tv.string = String(repeating: "x", count: 100)
        let mark = Self.mark(NSRange(location: 50, length: 2), .warning, "m")
        painter.update([mark], in: tv, reset: false) // offscreen: the whole text is the window
        XCTAssertEqual(painter.painted, [NSRange(location: 0, length: 100)])
        XCTAssertEqual(painter.gaps(in: NSRange(location: 20, length: 30)), [])
        painter.update([], in: tv, reset: false)
        XCTAssertEqual(painter.painted, [NSRange(location: 0, length: 100)])
        // An insertion before a painted range grows it to keep covering the shifted attributes.
        painter.noteEdit(range: NSRange(location: 5, length: 0), replacementLength: 7)
        XCTAssertEqual(painter.painted, [NSRange(location: 0, length: 107)])
        painter.noteEdit(range: NSRange(location: 500, length: 0), replacementLength: 3)
        XCTAssertEqual(painter.painted, [NSRange(location: 0, length: 107)], "an edit after every painted range changes nothing")
        painter.noteEdit(range: NSRange(location: 10, length: 50), replacementLength: 1)
        XCTAssertEqual(painter.painted, [NSRange(location: 0, length: 107)], "a deletion never shrinks the estimate")
    }

    func testWholeDocumentApplyMarksStillPaintsEverything() throws {
        let text = Self.largeDocument(bytes: 60_000)
        let tv = NSTextView(frame: NSRect(x: 0, y: 0, width: 300, height: 100))
        tv.string = text
        let marks = Self.marks(count: 200, in: text)
        let whole = Self.timed { SourceEditorView.applyMarks(marks, to: tv) }
        let lm = try XCTUnwrap(tv.layoutManager)
        XCTAssertTrue(marks.allSatisfy { lm.temporaryAttribute(.underlineStyle, atCharacterIndex: $0.nsRange.location, effectiveRange: nil) != nil })
        // Errors win over warnings where they overlap.
        let a = Self.mark(NSRange(location: 10, length: 10), .warning, "w")
        let b = Self.mark(NSRange(location: 15, length: 10), .error, "e", recovery: "fix", index: 1)
        SourceEditorView.applyMarks([b, a], to: tv)
        XCTAssertEqual(lm.temporaryAttribute(.underlineColor, atCharacterIndex: 12, effectiveRange: nil) as? NSColor, .systemOrange)
        XCTAssertEqual(lm.temporaryAttribute(.underlineColor, atCharacterIndex: 17, effectiveRange: nil) as? NSColor, .systemRed)
        XCTAssertEqual(lm.temporaryAttribute(.toolTip, atCharacterIndex: 17, effectiveRange: nil) as? String, b.toolTip)
        XCTAssertNil(lm.temporaryAttribute(.underlineStyle, atCharacterIndex: marks[3].nsRange.location, effectiveRange: nil))
        print("whole-document applyMarks: \(whole.cpu) ms CPU / \(whole.wall) ms wall (60 KB, 200 marks, offscreen view, includes layout)")
    }

    func testHostedEditorRepaintsMarksWhenScrolled() async throws {
        let model = ShellModel()
        let text = Self.largeDocument(bytes: 60_000)
        model.replaceProject(entryText: text)
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let lm = try XCTUnwrap(tv.layoutManager)
        // Diagnostics through the model: a result whose diagnostics point at the buffer.
        let last = text.utf8.count - 20
        let diagnostics = (0..<200).map { i in
            RuntimeV1.Diagnostic(severity: .warning, message: "d\(i)", source: .init(path: "main.tex", startByte: i == 199 ? last : i * 200, endByte: (i == 199 ? last : i * 200) + 4), recovery: nil)
        }
        model.result = RuntimeV1.CompileResult(projectId: "p", revision: model.editorRevision, status: .ok, pages: [],
                                               diagnostics: diagnostics, pdfPath: nil)
        try await waitUntil("marks painted") { co.marks.paints >= 1 }
        // Byte ranges that land inside a multi-byte scalar are not marks.
        XCTAssertEqual(co.marks.marks, model.editorMarks)
        XCTAssertGreaterThan(co.marks.marks.count, 150)
        let lastMark = try XCTUnwrap(co.marks.marks.last)
        XCTAssertNil(lm.temporaryAttribute(.underlineStyle, atCharacterIndex: lastMark.nsRange.location, effectiveRange: nil),
                     "the last mark is far below the visible window")
        tv.scrollRangeToVisible(NSRange(location: (text as NSString).length, length: 0))
        try await waitUntil("scroll repaint") {
            lm.temporaryAttribute(.underlineStyle, atCharacterIndex: lastMark.nsRange.location, effectiveRange: nil) != nil
        }
        XCTAssertGreaterThanOrEqual(co.marks.paints, 2)
    }

    // MARK: navigation vs typing

    func testNavigationSelectionWaitsForATypingPauseInsteadOfMovingTheCaretBack() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "alpha\nbeta\n")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let end = (model.activeText as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        tv.insertText("x", replacementRange: NSRange(location: end, length: 0))
        XCTAssertEqual(model.activeText, "alpha\nbeta\nx")
        XCTAssertNotEqual(co.lastUserEditNs, 0)

        // Navigation to an earlier range while typing: deferred, caret stays.
        model.selection = .init(path: "main.tex", nsRange: NSRange(location: 0, length: 5), token: 1)
        try await turn()
        XCTAssertEqual(tv.selectedRange(), NSRange(location: end + 1, length: 0), "the caret did not move backwards while typing")
        XCTAssertEqual(co.deferredSelection?.token, 1)
        XCTAssertEqual(co.appliedToken, 1)
        // Still typing: still deferred.
        tv.insertText("y", replacementRange: NSRange(location: end + 1, length: 0))
        try await turn()
        XCTAssertEqual(tv.selectedRange(), NSRange(location: end + 2, length: 0))
        XCTAssertEqual(model.activeText, "alpha\nbeta\nxy")
        // Typing pauses: the newest navigation applies.
        try await waitUntil("deferred navigation", timeout: 3) { tv.selectedRange() == NSRange(location: 0, length: 5) }
        XCTAssertNil(co.deferredSelection)
        XCTAssertEqual(co.announcements.last, "Selected 5 characters, line 1 column 1 to 6")

        // A navigation forward of the caret applies at once, even mid-typing.
        tv.setSelectedRange(NSRange(location: 0, length: 0))
        tv.insertText("z", replacementRange: NSRange(location: 0, length: 0))
        model.selection = .init(path: "main.tex", nsRange: NSRange(location: 7, length: 4), token: 2)
        try await waitUntil("forward navigation") { tv.selectedRange() == NSRange(location: 7, length: 4) }
        XCTAssertNil(co.deferredSelection)
        XCTAssertEqual((tv.string as NSString).substring(with: tv.selectedRange()), "beta")

        // A superseded deferred token is dropped: only the newest applies.
        tv.setSelectedRange(NSRange(location: 12, length: 0))
        tv.insertText("w", replacementRange: NSRange(location: 12, length: 0))
        model.selection = .init(path: "main.tex", nsRange: NSRange(location: 1, length: 1), token: 3)
        try await turn()
        XCTAssertEqual(co.deferredSelection?.token, 3)
        model.selection = .init(path: "main.tex", nsRange: NSRange(location: 2, length: 2), token: 4)
        try await waitUntil("newest navigation", timeout: 3) { tv.selectedRange() == NSRange(location: 2, length: 2) }
        XCTAssertNil(co.deferredSelection)
        XCTAssertEqual(co.appliedToken, 4)
    }

    func testRecreatedCoordinatorDoesNotReplayAnOldNavigation() {
        let view = SourceEditorView(text: .constant("abc"), selection: .init(path: "main.tex", nsRange: NSRange(location: 0, length: 1), token: 7))
        let co = view.makeCoordinator()
        XCTAssertEqual(co.appliedToken, 7, "an old navigation token is treated as already applied")
        XCTAssertEqual(co.appliedEditToken, 0, "a pending edit the model still waits on is applied")
        XCTAssertEqual(SourceEditorView(text: .constant("")).makeCoordinator().appliedToken, 0)
    }

    // MARK: capture insertion undo

    func testCaptureInsertionIsOneUndoStepDeliveredToTheModelOnce() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "\\begin{document}\n\\end{document}\n")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let undo = try XCTUnwrap(tv.undoManager)
        let base = model.editorRevision

        // The user types (one coalesced typing step), then a capture is approved.
        let at = ("\\begin{document}\n" as NSString).length
        tv.setSelectedRange(NSRange(location: at, length: 0))
        for (i, ch) in ["a", "b", "c"].enumerated() { tv.insertText(ch, replacementRange: NSRange(location: at + i, length: 0)) }
        XCTAssertEqual(model.activeText, "\\begin{document}\nabc\\end{document}\n")
        XCTAssertEqual(model.editorRevision, base + 3)
        try await turn() // the keystrokes' event (and its by-event undo group) ends before the capture is approved
        let insert = "\n\\[ E = mc^2 \\]\n"
        let edit = ShellModel.PendingEdit(path: "main.tex", nsRange: NSRange(location: at + 3, length: 0), text: insert, token: 1)
        model.pendingEdit = edit
        try await waitUntil("edit applied in the view") { tv.string.contains(insert) }
        let afterInsert = "\\begin{document}\nabc" + insert + "\\end{document}\n"
        XCTAssertEqual(tv.string, afterInsert)
        XCTAssertEqual(tv.selectedRange(), NSRange(location: at + 3, length: (insert as NSString).length), "the insertion is selected")
        try await waitUntil("model told once") { probe.editApplied.count == 1 }
        XCTAssertEqual(probe.editApplied[0].0, edit)
        XCTAssertEqual(probe.editApplied[0].1, afterInsert)
        XCTAssertEqual(model.activeText, afterInsert, "the model adopted the edited text through onEditApplied")
        XCTAssertEqual(model.editorRevision, base + 4, "the insertion bumped the revision exactly once")
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(undo.undoActionName, "Insert Capture")
        XCTAssertEqual(probe.coordinator?.announcements.last, "Inserted capture. Selected 16 characters, line 2 column 4 to line 4 column 1")

        // More typing after the capture is its own step.
        let tail = at + 3 + (insert as NSString).length
        tv.setSelectedRange(NSRange(location: tail, length: 0))
        tv.insertText("d", replacementRange: NSRange(location: tail, length: 0))
        tv.insertText("e", replacementRange: NSRange(location: tail + 1, length: 0))
        let afterTail = "\\begin{document}\nabc" + insert + "de\\end{document}\n"
        XCTAssertEqual(model.activeText, afterTail)
        try await turn()

        // Undo peels the steps in order; each reaches the model through the binding.
        XCTAssertTrue(undo.canUndo)
        undo.undo()
        XCTAssertEqual(tv.string, afterInsert, "undo 1: the trailing typing")
        XCTAssertEqual(model.activeText, afterInsert)
        undo.undo()
        XCTAssertEqual(tv.string, "\\begin{document}\nabc\\end{document}\n", "undo 2: the capture insertion as one step")
        XCTAssertEqual(model.activeText, "\\begin{document}\nabc\\end{document}\n")
        undo.undo()
        XCTAssertEqual(tv.string, "\\begin{document}\n\\end{document}\n", "undo 3: the leading typing")
        XCTAssertEqual(model.activeText, tv.string)
        XCTAssertEqual(probe.editApplied.count, 1, "undo/redo never re-deliver the pending edit")

        undo.redo()
        XCTAssertEqual(tv.string, "\\begin{document}\nabc\\end{document}\n")
        undo.redo()
        XCTAssertEqual(tv.string, afterInsert, "redo restores the capture as one step")
        XCTAssertEqual(model.activeText, afterInsert)
        undo.redo()
        XCTAssertEqual(tv.string, afterTail)
        XCTAssertEqual(model.activeText, afterTail)
        XCTAssertFalse(undo.canRedo)
        XCTAssertEqual(probe.editApplied.count, 1)
        try await turn()
        XCTAssertEqual(model.activeText, afterTail, "no stale binding write after the turn")
    }

    /// An edit whose range no longer fits is refused explicitly (never
    /// reported as applied: a capture reported applied but not inserted was
    /// lost for good, as `appliedCaptureIDs` refused its retry as a duplicate).
    func testPendingEditOutsideTheBufferIsRefusedWithoutChangingText() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "short\n")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let edit = ShellModel.PendingEdit(path: "main.tex", nsRange: NSRange(location: 50, length: 0), text: "x", token: 1)
        model.pendingEdit = edit
        try await waitUntil("model told") { probe.editRefused.count == 1 }
        XCTAssertTrue(probe.editApplied.isEmpty, "never reported as applied")
        XCTAssertTrue(probe.editRefused.first?.1.contains("outside the buffer") == true, probe.editRefused.first?.1 ?? "-")
        XCTAssertEqual(tv.string, "short\n")
        XCTAssertNil(model.pendingEdit)
        XCTAssertEqual(model.navigationNote?.hasPrefix("Edit not applied"), true)
        XCTAssertFalse(tv.undoManager?.canUndo ?? true)
    }

    // MARK: undo through the durable core

    /// The reviewed insertion flow of `ShellModelBridgeTests`, but with the real
    /// editor in a hosted window doing the adoption and the undo/redo: the
    /// capture is one undo step, the model hears about it once (receipt →
    /// confirmed, no document_edit), ⌘Z / ⇧⌘Z reach the durable document as
    /// ordinary edits in order, and the tombstone survives the undo.
    func testCaptureUndoRedoReachesTheDurableDocumentInOrder() async throws {
        let model = ShellModel()
        model.autoCompile = false
        XCTAssertEqual(model.activeText, "Hello FlashTeX.\n")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let store = try await ShellModelBridgeTests.attach(model)
        let bridge = try XCTUnwrap(model.bridge)
        XCTAssertTrue(bridge.ledgerUsable, bridge.ledgerStatus)
        let undo = try XCTUnwrap(tv.undoManager)

        // Typing in the editor streams to the bridge and the durable document.
        tv.setSelectedRange(NSRange(location: 5, length: 0))
        tv.insertText(" naïve", replacementRange: NSRange(location: 5, length: 0))
        let before = "Hello naïve FlashTeX.\n"
        XCTAssertEqual(model.activeText, before)
        let editedRevision = model.editorRevision
        try await waitUntil("durable typing") { bridge.durable?.revision == editedRevision && bridge.durable?.text == before }
        var ledgerText = try await bridge.ledger!.status().document?.text
        XCTAssertEqual(ledgerText, before)

        // Pin the caret after "naïve " through the editor's own caret report (UTF-16 12 → byte 13).
        tv.setSelectedRange(NSRange(location: 12, length: 0))
        XCTAssertEqual(model.caretUTF16, 12)
        XCTAssertEqual(model.caretByte, 13)
        model.pinAnchorAtCaret()
        XCTAssertEqual(model.anchor?.byteOffset, 13)
        try await waitUntil("bridge destination") { model.bridgeDestination != nil }
        let image = try BridgeClientTests.fixtureCapture().image
        let received = await model.submitCapture(image: image, captureId: "fixture-capture-1", instructions: "test")
        XCTAssertNotNil(received, model.captureNote ?? "")
        let converted = await model.convertCapture(captureId: "fixture-capture-1")
        let proposal = try XCTUnwrap(converted, model.captureNote ?? "")
        try await turn() // the approval is a separate event from the typing

        // Approve: durable first, then the hosted editor adopts it as one undo step.
        let outcome = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(outcome, .inserted(byteOffset: 13))
        let after = "Hello naïve \\fakecapture{fixture-capture-1}FlashTeX.\n"
        XCTAssertEqual(bridge.transactionTrace, ["ledger"], "durable before the editor changes")
        try await waitUntil("editor adoption") { probe.editApplied.count == 1 }
        XCTAssertEqual(tv.string, after)
        XCTAssertEqual(model.activeText, after)
        XCTAssertEqual(model.editorRevision, editedRevision + 1)
        XCTAssertEqual(undo.undoActionName, "Insert Capture")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 12, length: ("\\fakecapture{fixture-capture-1}" as NSString).length))
        try await waitUntil("confirmed") { model.bridgeCaptures.last?.state == .confirmed }
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "receipt", "confirmed"], "adoption sent the receipt, no document_edit")
        XCTAssertEqual(bridge.durable?.text, after)
        XCTAssertEqual(bridge.durable?.revision, model.editorRevision)
        XCTAssertNil(model.pendingEdit)
        XCTAssertNil(model.bridgeDestination, "the insertion invalidated the pinned target")
        try await turn()

        // ⌘Z: the whole capture comes out; the durable document follows as an ordinary edit.
        undo.undo()
        XCTAssertEqual(tv.string, before, "one undo step removes exactly the capture")
        XCTAssertEqual(model.activeText, before)
        try await waitUntil("durable undo") { bridge.durable?.text == before && bridge.durable?.revision == model.editorRevision }
        ledgerText = try await bridge.ledger!.status().document?.text
        XCTAssertEqual(ledgerText, before)
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "receipt", "confirmed"], "undo is not a second application")
        XCTAssertEqual(probe.editApplied.count, 1)

        // ⇧⌘Z: redo restores the capture text, again as an ordinary edit (no second receipt).
        undo.redo()
        XCTAssertEqual(tv.string, after)
        try await waitUntil("durable redo") { bridge.durable?.text == after && bridge.durable?.revision == model.editorRevision }
        ledgerText = try await bridge.ledger!.status().document?.text
        XCTAssertEqual(ledgerText, after)
        XCTAssertEqual(bridge.transactionTrace, ["ledger", "receipt", "confirmed"])
        XCTAssertEqual(probe.editApplied.count, 1)
        XCTAssertNil(bridge.ledgerError, bridge.ledgerError ?? "")

        // Undo twice: the capture, then the typed word; the durable text follows in order.
        undo.undo()
        XCTAssertEqual(tv.string, before)
        undo.undo()
        XCTAssertEqual(tv.string, "Hello FlashTeX.\n")
        XCTAssertEqual(model.activeText, "Hello FlashTeX.\n")
        try await waitUntil("durable double undo") { bridge.durable?.text == "Hello FlashTeX.\n" && bridge.durable?.revision == model.editorRevision }
        XCTAssertNil(bridge.ledgerError, bridge.ledgerError ?? "")

        // The tombstone survives the undo: approving again is a duplicate and inserts nothing.
        model.enqueue(proposal)
        let dup = await model.approveBridgeProposal(proposal, latex: proposal.latex)
        XCTAssertEqual(dup, .duplicate)
        XCTAssertNil(model.pendingEdit)
        try await turn()
        XCTAssertEqual(tv.string, "Hello FlashTeX.\n")
        XCTAssertEqual(probe.editApplied.count, 1)
        _ = store
        model.detachBridge()
    }

    // MARK: input methods (marked text)

    /// What an input method does, expressed as the `NSTextInputClient` calls
    /// AppKit forwards to the text view. A synthesized `NSEvent` cannot drive
    /// a real input source in a test process (an Option-e key event inserts
    /// its `characters` literally; no dead-key state, no IME candidate window),
    /// so the sequences are replayed at the client API, which is the same code
    /// path the view takes when the input context calls it.
    private static let noReplacement = NSRange(location: NSNotFound, length: 0)
    private func compose(_ tv: NSTextView, _ text: String) {
        tv.setMarkedText(text, selectedRange: NSRange(location: (text as NSString).length, length: 0), replacementRange: Self.noReplacement)
    }

    func testCompositionReachesTheModelOnlyWhenCommitted() async throws {
        let model = ShellModel()
        model.attachWorker(at: WorkerClientTests.python, arguments: [WorkerClientTests.fakeWorker.path])
        XCTAssertTrue(model.workerAttached)
        model.replaceProject(entryText: "ab\n")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        model.compile()
        try await waitUntil("first compile") { model.inFlightRevision == nil && model.result?.revision == model.editorRevision }
        TypingBench.shared.reset() // from here every compile result is recorded by revision
        tv.setSelectedRange(NSRange(location: 2, length: 0))
        try await turn()
        let rev = model.editorRevision
        let spokenBefore = co.announcements.count

        // Three composition steps (Japanese-style): the view shows the marked
        // text, the model holds the committed text, no revision, no compile.
        for (i, step) in ["か", "かん", "漢"].enumerated() {
            compose(tv, step)
            XCTAssertTrue(tv.hasMarkedText())
            XCTAssertEqual(tv.string, "ab\(step)\n")
            XCTAssertEqual(model.activeText, "ab\n", "step \(i) did not reach the model")
            XCTAssertEqual(model.editorRevision, rev, "step \(i) bumped no revision")
            XCTAssertEqual(model.caretUTF16, 2, "the caret the model hears is the composition start")
            XCTAssertEqual(model.caretByte, 2)
            XCTAssertEqual(model.caretLengthUTF16, 0)
            XCTAssertTrue(co.composing)
            XCTAssertGreaterThanOrEqual(co.compositionSteps, i + 1, "AppKit posts one or two selection changes per step")
            XCTAssertNotEqual(co.lastUserEditNs, 0, "composing counts as typing")
        }
        try await turn()
        XCTAssertEqual(co.announcements.count, spokenBefore, "composition steps are not announced")
        XCTAssertNil(model.inFlightRevision)
        XCTAssertTrue(TypingBench.shared.recorder.compilesMs.isEmpty, "no compile per composition step")

        // Commit: exactly one text change, one revision, one compile.
        tv.insertText("漢字", replacementRange: Self.noReplacement)
        XCTAssertFalse(tv.hasMarkedText())
        XCTAssertFalse(co.composing)
        XCTAssertEqual(tv.string, "ab漢字\n")
        XCTAssertEqual(model.activeText, "ab漢字\n")
        XCTAssertEqual(model.editorRevision, rev + 1)
        XCTAssertEqual(model.caretUTF16, 4)
        XCTAssertEqual(model.caretByte, 2 + "漢字".utf8.count)
        try await waitUntil("commit compiled") { model.inFlightRevision == nil && model.result?.revision == rev + 1 }
        XCTAssertEqual(Array(TypingBench.shared.recorder.compilesMs.keys), [rev + 1], "one compile, for the committed revision")
        try await turn()
        XCTAssertEqual(co.announcements.count, spokenBefore, "the commit is a typing step, not a caret move")

        // Navigation during a composition waits for it to end. (Without the
        // worker: the model drops `selection` when a compile result lands, so a
        // navigation still deferred when the commit's result arrives is
        // invalidated by the model, not applied late by the editor.)
        model.detachWorker()
        compose(tv, "x")
        model.selection = .init(path: "main.tex", nsRange: NSRange(location: 0, length: 2), token: 1)
        try await turn()
        XCTAssertTrue(tv.hasMarkedText(), "syncing the view did not disturb the composition")
        XCTAssertEqual(tv.string, "ab漢字x\n")
        XCTAssertNotEqual(tv.selectedRange(), NSRange(location: 0, length: 2))
        tv.insertText("x", replacementRange: Self.noReplacement)
        XCTAssertEqual(model.activeText, "ab漢字x\n")
        try await turn()
        XCTAssertEqual(co.deferredSelection?.token, 1, "deferred until the typing pause, not dropped")
        try await waitUntil("navigation after the composition", timeout: 3) { tv.selectedRange() == NSRange(location: 0, length: 2) }
        XCTAssertEqual(co.announcements.last, "Selected 2 characters, line 1 column 1 to 3")
    }

    func testCompositionCancelDropsTheStepsAndClosesTheCompletionList() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "x \\s")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let completing = try XCTUnwrap(tv as? CompletingTextView)
        let end = (tv.string as NSString).length
        tv.setSelectedRange(NSRange(location: end, length: 0))
        try await turn()
        let rev = model.editorRevision

        // The list is open; a composition starting under it closes it and its
        // pending scan can never reopen it while marked text exists.
        completing.requestCompletion()
        try await waitUntil("completion list") { completing.session != nil }
        // The vocabulary lane shows the argument shape in the label (computed
        // from the live vocabulary, not a hand-copied snapshot).
        XCTAssertEqual(completing.session?.items.first?.label, CompletionTestVocabulary.labels(forPrefix: "s").first)
        compose(tv, "か")
        completing.requestCompletion() // what the list's key path does after every keystroke
        try await waitUntil("list closed by the composition") { completing.session == nil }
        XCTAssertEqual(completing.lastCloseReason, .textChanged)
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertNil(completing.session, "no list opened mid-composition")
        XCTAssertTrue(tv.hasMarkedText())
        XCTAssertEqual(model.activeText, "x \\s")
        XCTAssertEqual(model.editorRevision, rev)

        // IME cancel (Esc in the candidate window): the marked text is removed
        // and unmarked; the buffer is back to the committed text, nothing was
        // pushed, no revision.
        compose(tv, "")
        tv.unmarkText()
        XCTAssertFalse(tv.hasMarkedText())
        XCTAssertEqual(tv.string, "x \\s")
        XCTAssertEqual(model.activeText, "x \\s")
        XCTAssertEqual(model.editorRevision, rev)
        try await turn()
        XCTAssertFalse(co.composing)
        XCTAssertNil(completing.session)
        XCTAssertEqual(model.caretUTF16, end)

        // `unmarkText` with marked text left commits it: one text change, one revision.
        compose(tv, "y")
        tv.unmarkText()
        XCTAssertFalse(tv.hasMarkedText())
        XCTAssertEqual(model.activeText, "x \\sy")
        XCTAssertEqual(model.editorRevision, rev + 1)
        XCTAssertEqual(model.caretUTF16, end + 1)

        // Dead key under the open list (Option-e then e on a US layout): the
        // input method marks "´", then replaces it with "é". The list closes,
        // nothing is swallowed, the model sees the accent once.
        tv.setSelectedRange(NSRange(location: end, length: 0))
        tv.insertText("", replacementRange: NSRange(location: end, length: 1)) // drop the "y"
        XCTAssertEqual(model.activeText, "x \\s")
        let rev2 = model.editorRevision
        completing.requestCompletion()
        try await waitUntil("completion list 2") { completing.session != nil }
        compose(tv, "´")
        XCTAssertEqual(tv.string, "x \\s´")
        try await waitUntil("list closed by the dead key") { completing.session == nil }
        XCTAssertEqual(model.activeText, "x \\s")
        XCTAssertEqual(model.editorRevision, rev2)
        tv.insertText("é", replacementRange: Self.noReplacement)
        XCTAssertFalse(tv.hasMarkedText())
        XCTAssertEqual(tv.string, "x \\sé", "the dead-key sequence was not swallowed")
        XCTAssertEqual(model.activeText, "x \\sé")
        XCTAssertEqual(model.editorRevision, rev2 + 1)
        XCTAssertEqual(model.caretUTF16, end + 1)
        XCTAssertEqual(model.caretByte, "x \\sé".utf8.count)
        try await turn()
        XCTAssertNil(completing.session, "the list does not reopen by itself after the commit")
    }

    func testCaretBytesAreExactAcrossComposedAndDecomposedCharacters() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)

        // e + combining acute (2 UTF-16 units, 3 bytes) is one user character.
        tv.insertText("e", replacementRange: NSRange(location: 0, length: 0))
        tv.insertText("\u{301}", replacementRange: NSRange(location: 1, length: 0))
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 2, length: 0))
        XCTAssertEqual(model.caretUTF16, 2)
        XCTAssertEqual(model.caretByte, 3)
        XCTAssertEqual(model.activeText.utf8.count, 3, "the decomposed form is kept byte for byte")
        XCTAssertTrue(model.activeText.sameBytes(as: "e\u{301}"))
        XCTAssertTrue(model.activeText == "é", "Swift == is canonical equivalence; the model compares bytes")
        XCTAssertFalse(model.activeText.sameBytes(as: "é"))
        XCTAssertEqual(SourceEditorView.lineColumn(text: model.activeText, utf16: 2)?.column, 2)
        XCTAssertEqual(model.activeText.nsRange(utf8Bytes: .init(path: "main.tex", startByte: 3, endByte: 3)), NSRange(location: 2, length: 0))

        // Precomposed é (1 unit, 2 bytes) after it.
        tv.insertText("é", replacementRange: NSRange(location: 2, length: 0))
        XCTAssertEqual(model.caretUTF16, 3)
        XCTAssertEqual(model.caretByte, 5)
        XCTAssertEqual(model.activeText.utf8.count, 5)
        XCTAssertEqual(SourceEditorView.lineColumn(text: model.activeText, utf16: 3)?.column, 3)
        XCTAssertEqual(model.activeText.nsRange(utf8Bytes: .init(path: "main.tex", startByte: 5, endByte: 5)), NSRange(location: 3, length: 0))
        XCTAssertEqual(model.activeText.nsRange(utf8Bytes: .init(path: "main.tex", startByte: 0, endByte: 3)), NSRange(location: 0, length: 2))

        // The same accent composed through the input method (dead key) lands identically.
        compose(tv, "´")
        XCTAssertEqual(model.caretUTF16, 3, "composition start while marked")
        tv.insertText("é", replacementRange: Self.noReplacement)
        XCTAssertEqual(model.caretUTF16, 4)
        XCTAssertEqual(model.caretByte, 7)
        XCTAssertTrue(model.activeText.sameBytes(as: "e\u{301}éé"))

        // Selecting the decomposed cluster announces one character; the byte range is exact.
        try await turn() // the typing event ends before the user selects
        tv.setSelectedRange(NSRange(location: 0, length: 2))
        try await turn()
        XCTAssertEqual(co.announcements.last, "Selected 1 character, line 1 column 1 to 2")
        XCTAssertEqual(model.caretLengthUTF16, 2)
        XCTAssertEqual(model.activeText.utf8ByteRange(of: NSRange(location: 0, length: 2))?.end, 3)
        // A mark or selection mapped from bytes 0..<3 covers the whole cluster, never half of it.
        XCTAssertEqual(model.activeText.clusterAlignedNSRange(utf8Start: 0, utf8End: 1), NSRange(location: 0, length: 2))
    }

    // MARK: delimiter pairs

    private func m(_ text: String, _ caret: Int) -> (Int, Int, Int, Int)? {
        SourceEditorView.BraceMatcher.match(in: text, caretUTF16: caret).map {
            ($0.open.location, $0.open.length, $0.close.location, $0.close.length)
        }
    }

    func testBraceMatcherIsExactAndSkipsEscapesCommentsAndVerb() {
        typealias BM = SourceEditorView.BraceMatcher
        let t = "a{b[c]d}e"
        XCTAssertTrue(m(t, 8)! == (1, 1, 7, 1), "after the closing brace")
        XCTAssertTrue(m(t, 1)! == (1, 1, 7, 1), "before the opening brace")
        XCTAssertTrue(m(t, 2)! == (1, 1, 7, 1), "after the opening brace")
        XCTAssertTrue(m(t, 5)! == (3, 1, 5, 1), "before the closing bracket")
        XCTAssertTrue(m(t, 4)! == (3, 1, 5, 1), "after the opening bracket")
        XCTAssertNil(m(t, 0)); XCTAssertNil(m(t, 9)) // no delimiter next to the caret
        XCTAssertTrue(m(t, 3)! == (3, 1, 5, 1), "the bracket after the caret counts")
        XCTAssertTrue(m("{{}}", 4)! == (0, 1, 3, 1), "depth counting")
        XCTAssertTrue(m("{{}}", 3)! == (1, 1, 2, 1))
        XCTAssertNil(m("{a", 1), "no closer")
        XCTAssertNil(m("a}", 2), "no opener")
        XCTAssertNil(m("[a}", 1), "kinds do not mix")
        // Escapes: \{ and \} are literal; \\{ is an escaped backslash then a real brace.
        XCTAssertNil(m("\\{x}", 4))
        XCTAssertNil(m("{x\\}", 4))
        XCTAssertTrue(m("\\\\{x}", 5)! == (2, 1, 4, 1))
        // Comments hide the rest of the line; the caret in a comment matches nothing.
        XCTAssertTrue(m("{a % }\n}", 8)! == (0, 1, 7, 1))
        XCTAssertNil(m("{a % }", 6))
        XCTAssertTrue(m("\\% {x}", 6)! == (3, 1, 5, 1), "an escaped percent is not a comment")
        // \verb arguments are not code.
        XCTAssertTrue(m("{\\verb|}|}", 10)! == (0, 1, 9, 1))
        XCTAssertTrue(m("{\\verb*|}|}", 11)! == (0, 1, 10, 1))
        XCTAssertNil(m("{\\verb|}|", 8), "inside the verb argument")
        // $…$ and $$…$$ by parity within the paragraph.
        XCTAssertTrue(m("a $x$ b", 3)! == (2, 1, 4, 1), "after the opening dollar")
        XCTAssertTrue(m("a $x$ b", 5)! == (2, 1, 4, 1), "after the closing dollar")
        XCTAssertTrue(m("a $x$ b", 4)! == (2, 1, 4, 1), "before the closing dollar")
        XCTAssertTrue(m("$$x$$", 2)! == (0, 2, 3, 2))
        XCTAssertTrue(m("$$x$$", 5)! == (0, 2, 3, 2))
        XCTAssertTrue(m("\\$a$b$", 6)! == (3, 1, 5, 1), "an escaped dollar does not count")
        XCTAssertTrue(m("$a\nb$", 5)! == (0, 1, 4, 1), "inline math continues over a line break")
        XCTAssertNil(m("$a\n\nb$", 1), "but never over a blank line")
        XCTAssertNil(m("$a\n\nb$", 6))
        XCTAssertTrue(m("$a$ $b$", 7)! == (4, 1, 6, 1), "the second span pairs with itself")
        // Multi-byte text before the pair keeps UTF-16 offsets exact.
        XCTAssertTrue(m("é{ü}", 4)! == (1, 1, 3, 1))
        XCTAssertTrue(m("👩‍💻{ü}", 8)! == (5, 1, 7, 1))
        XCTAssertNil(m("é{ü}", 2 + 1 + 100), "out of range")
        // Budget: an opener whose closer is beyond the byte budget is left unmatched, quickly.
        let far = "{" + String(repeating: "x\n", count: BM.budgetBytes) + "}"
        let t0 = Self.timed { XCTAssertNil(self.m(far, 1)) }
        XCTAssertLessThan(t0.cpu, 20, "bounded scan took \(t0.cpu) ms CPU")

        // Auto-close gate (the opener was just typed before the caret): code only,
        // not escaped, followed by nothing/space/closer.
        XCTAssertTrue(BM.autoCloseAllowed(in: "{", caretUTF16: 1))
        XCTAssertTrue(BM.autoCloseAllowed(in: "{ ", caretUTF16: 1))
        XCTAssertTrue(BM.autoCloseAllowed(in: "{}", caretUTF16: 1))
        XCTAssertFalse(BM.autoCloseAllowed(in: "{b", caretUTF16: 1), "not before a letter")
        XCTAssertFalse(BM.autoCloseAllowed(in: "\\{", caretUTF16: 2), "escaped")
        XCTAssertTrue(BM.autoCloseAllowed(in: "\\\\{", caretUTF16: 3), "after an escaped backslash")
        XCTAssertFalse(BM.autoCloseAllowed(in: "% {", caretUTF16: 3), "in a comment")
        XCTAssertFalse(BM.autoCloseAllowed(in: "\\verb|{|", caretUTF16: 7), "in a verb argument")
        XCTAssertTrue(BM.autoCloseAllowed(in: "é{", caretUTF16: 2))
        XCTAssertFalse(BM.autoCloseAllowed(in: "", caretUTF16: 0))
    }

    func testAutoCloseTypeOverAndBackspaceAreOrdinaryUndoSteps() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let undo = try XCTUnwrap(tv.undoManager)
        var spoken: [String] = []
        co.announce = { spoken.append($0) }
        XCTAssertEqual(co.parent.autoClosePairs, ["{"], "default: braces only")

        // `{` becomes `{|}` in one text change, one revision; the caret byte is exact.
        let rev = model.editorRevision
        tv.insertText("{", replacementRange: NSRange(location: 0, length: 0))
        XCTAssertEqual(tv.string, "{}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 1, length: 0))
        XCTAssertEqual(model.activeText, "{}")
        XCTAssertEqual(model.editorRevision, rev + 1, "opener and closer reach the model as one change")
        XCTAssertEqual(model.caretUTF16, 1); XCTAssertEqual(model.caretByte, 1)
        XCTAssertEqual(co.pendingClosers, [1])
        XCTAssertEqual(co.braceHighlight, .init(open: NSRange(location: 0, length: 1), close: NSRange(location: 1, length: 1)))
        try await turn()
        // ⌘Z after `{}` leaves nothing: the closer coalesced with the typed opener.
        undo.undo()
        XCTAssertEqual(tv.string, "")
        XCTAssertEqual(model.activeText, "")
        XCTAssertEqual(co.pendingClosers, [])
        undo.redo()
        XCTAssertEqual(tv.string, "{}")
        XCTAssertEqual(model.activeText, "{}")
        try await turn()

        /// Fresh buffer and undo stack (a programmatic range deletion would be
        /// folded into AppKit's typing coalescing and pollute the next phase).
        func reset() async throws {
            model.replaceProject(entryText: ""); try await waitUntil("reset") { tv.string == "" }; undo.removeAllActions()
            try await turn()
            XCTAssertEqual(model.activeText, "")
        }

        // Typing inside, then the closer types over the auto-inserted one: no text change, no revision.
        try await reset()
        tv.insertText("{", replacementRange: NSRange(location: 0, length: 0))
        tv.insertText("é", replacementRange: NSRange(location: 1, length: 0))
        XCTAssertEqual(tv.string, "{é}")
        XCTAssertEqual(co.pendingClosers, [2], "the closer shifted with the typing before it")
        let revBefore = model.editorRevision
        spoken = []
        tv.insertText("}", replacementRange: NSRange(location: 2, length: 0))
        XCTAssertEqual(tv.string, "{é}", "typed over, not duplicated")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 3, length: 0))
        XCTAssertEqual(model.editorRevision, revBefore, "a type-over changes no text")
        XCTAssertEqual(model.caretUTF16, 3); XCTAssertEqual(model.caretByte, 4)
        XCTAssertEqual(co.pendingClosers, [])
        XCTAssertEqual(spoken, ["matches line 1 column 1"], "a closer announces its opener")
        // A second `}` is a real character now.
        tv.insertText("}", replacementRange: NSRange(location: 3, length: 0))
        XCTAssertEqual(tv.string, "{é}}")
        XCTAssertEqual(model.editorRevision, revBefore + 1)
        try await turn()

        // Backspace inside an empty auto-closed pair removes both; ⌘Z brings the pair back.
        try await reset()
        tv.insertText("é", replacementRange: NSRange(location: 0, length: 0))
        try await turn()
        tv.insertText("{", replacementRange: NSRange(location: 1, length: 0))
        XCTAssertEqual(tv.string, "é{}")
        XCTAssertEqual(model.caretByte, 3)
        let revPair = model.editorRevision
        try await turn()
        tv.doCommand(by: #selector(NSResponder.deleteBackward(_:))) // the Delete key's path (interpretKeyEvents)
        XCTAssertEqual(tv.string, "é")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 1, length: 0))
        XCTAssertEqual(model.activeText, "é")
        XCTAssertEqual(model.editorRevision, revPair + 1, "the pair removal is one text change")
        XCTAssertEqual(co.pendingClosers, [])
        try await turn()
        undo.undo()
        XCTAssertEqual(tv.string, "é{}", "undo restores the pair as its own step")
        XCTAssertEqual(model.activeText, "é{}")
        XCTAssertEqual(tv.selectedRange(), NSRange(location: 1, length: 2))
        undo.undo()
        XCTAssertEqual(tv.string, "", "then the typing (AppKit coalesces é{} typed in a row across events)")
        XCTAssertEqual(model.activeText, "")
        try await turn()

        // Not auto-closed: a disabled opener, an escaped brace, a comment, a selection, before a letter.
        try await reset()
        tv.insertText("[", replacementRange: NSRange(location: 0, length: 0))
        XCTAssertEqual(tv.string, "[", "`[` is not in the default set")
        tv.insertText("\\", replacementRange: NSRange(location: 1, length: 0))
        tv.insertText("{", replacementRange: NSRange(location: 2, length: 0))
        XCTAssertEqual(tv.string, "[\\{", "escaped")
        tv.insertText(" % ", replacementRange: NSRange(location: 3, length: 0))
        tv.insertText("{", replacementRange: NSRange(location: 6, length: 0))
        XCTAssertEqual(tv.string, "[\\{ % {", "in a comment")
        tv.setSelectedRange(NSRange(location: 0, length: 1))
        tv.insertText("{", replacementRange: NSRange(location: 0, length: 1))
        XCTAssertEqual(tv.string, "{\\{ % {", "typing over a selection replaces it only")
        tv.setSelectedRange(NSRange(location: 1, length: 0))
        tv.insertText("{", replacementRange: NSRange(location: 1, length: 0))
        XCTAssertEqual(tv.string, "{{\\{ % {", "not before a non-space character")
        XCTAssertEqual(model.activeText, tv.string)

        // Never while marked text exists: an input-method commit of `{` is left alone.
        try await reset()
        compose(tv, "{")
        XCTAssertTrue(tv.hasMarkedText())
        tv.insertText("{", replacementRange: Self.noReplacement)
        XCTAssertEqual(tv.string, "{", "the composed brace gets no closer")
        XCTAssertEqual(model.activeText, "{")
        XCTAssertEqual(co.pendingClosers, [])
    }

    func testCaretMovesHighlightAndAnnounceTheMatchingDelimiter() async throws {
        let model = ShellModel()
        model.replaceProject(entryText: "a{b}\n$x$")
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        let lm = try XCTUnwrap(tv.layoutManager)
        var spoken: [String] = []
        co.announce = { spoken.append($0) }
        func highlighted(_ i: Int) -> Bool {
            lm.temporaryAttribute(SourceEditorView.Coordinator.highlightKey, atCharacterIndex: i, effectiveRange: nil) != nil
        }

        tv.setSelectedRange(NSRange(location: 4, length: 0)) // after `}`
        try await turn()
        XCTAssertEqual(spoken.last, "Line 1, column 5, matches line 1 column 2")
        XCTAssertTrue(highlighted(1)); XCTAssertTrue(highlighted(3)); XCTAssertFalse(highlighted(2))
        XCTAssertEqual(co.braceHighlight, .init(open: NSRange(location: 1, length: 1), close: NSRange(location: 3, length: 1)))

        tv.setSelectedRange(NSRange(location: 0, length: 0)) // no delimiter here
        try await turn()
        XCTAssertEqual(spoken.last, "Line 1, column 1")
        XCTAssertFalse(highlighted(1)); XCTAssertFalse(highlighted(3))
        XCTAssertNil(co.braceHighlight)

        tv.setSelectedRange(NSRange(location: 8, length: 0)) // after the closing `$`
        try await turn()
        XCTAssertEqual(spoken.last, "Line 2, column 4, matches line 2 column 1")
        XCTAssertTrue(highlighted(5)); XCTAssertTrue(highlighted(7))

        // A selection (length > 0) highlights nothing; a navigation announces the match too.
        tv.setSelectedRange(NSRange(location: 5, length: 3))
        try await turn()
        XCTAssertNil(co.braceHighlight)
        XCTAssertFalse(highlighted(5))
        model.selection = .init(path: "main.tex", nsRange: NSRange(location: 2, length: 0), token: 1)
        try await waitUntil("navigation") { tv.selectedRange() == NSRange(location: 2, length: 0) }
        XCTAssertEqual(spoken.last, "Line 1, column 3, matches line 1 column 4")
        XCTAssertTrue(highlighted(1)); XCTAssertTrue(highlighted(3))

        // Typing a closer announces its opener even though typing itself is silent.
        tv.setSelectedRange(NSRange(location: 4, length: 0))
        try await turn()
        spoken = []
        tv.insertText("[", replacementRange: NSRange(location: 4, length: 0))
        tv.insertText("]", replacementRange: NSRange(location: 5, length: 0))
        try await turn()
        XCTAssertEqual(spoken, ["matches line 1 column 5"])
        XCTAssertEqual(model.activeText, "a{b}[]\n$x$")
        XCTAssertTrue(highlighted(4)); XCTAssertTrue(highlighted(5))

        // A model text reset drops and recomputes the highlight.
        model.replaceProject(entryText: "{}")
        try await waitUntil("reset") { tv.string == "{}" }
        XCTAssertEqual(co.pendingClosers, [])
    }

    // MARK: large document keystrokes

    func testLargeDocumentKeystrokeRoundTripAndCaretBytesStayCorrect() async throws {
        let model = ShellModel()
        let seed = Self.largeDocument(bytes: 60_000)
        model.replaceProject(entryText: seed)
        let probe = Probe()
        let (window, tv) = try await host(model, probe: probe)
        defer { window.orderOut(nil) }
        let co = try XCTUnwrap(probe.coordinator)
        XCTAssertGreaterThanOrEqual(seed.utf8.count, 60_000)

        // Type 200 keystrokes before `\end{document}`: ASCII, 2-, 3- and 4-byte scalars, newlines.
        let script = Array(repeating: ["x", "é", "→", "👩‍💻", " ", "\n", "y", "ü"], count: 25).flatMap { $0 }
        XCTAssertEqual(script.count, 200)
        var caret = (seed as NSString).range(of: "\\end{document}", options: .backwards).location
        tv.setSelectedRange(NSRange(location: caret, length: 0))
        tv.scrollRangeToVisible(NSRange(location: caret, length: 0))
        // Steady state: the text up to the caret is laid out (the app's background
        // layout does this after a load; without it the first keystroke pays the
        // whole 60 KB layout, ~18 ms CPU, once).
        try XCTUnwrap(tv.layoutManager).ensureLayout(forCharacterRange: NSRange(location: 0, length: caret))
        try await turn()
        var roundTripsMs: [Double] = []
        var roundTripsCpuMs: [Double] = []
        var keystrokeCpuMs: [Double] = []
        var typed = ""
        let insertAtByte = SourceEditorView.caretByte(text: seed, utf16: caret)!
        // One warm-up keystroke: the first edit of a fresh text view pays one-time
        // AppKit setup (undo manager, input context, glyph caches; ~7 ms CPU here)
        // that no later keystroke pays and that the round trip does not include.
        tv.insertText("w", replacementRange: NSRange(location: caret, length: 0))
        caret += 1; typed += "w"
        try await turn()
        for key in script {
            TypingBench.shared.recorder.delegateReported(at: 0)
            // Whole keystroke (text storage edit + layout + delegate + binding + model) in CPU time.
            let whole = Self.timed { tv.insertText(key, replacementRange: NSRange(location: caret, length: 0)) }
            keystrokeCpuMs.append(whole.cpu)
            caret += (key as NSString).length
            typed += key
            let delegateNs = try XCTUnwrap(TypingBench.shared.recorder.pendingDelegateNs)
            XCTAssertNotEqual(delegateNs, 0, "the delegate stamped this keystroke")
            XCTAssertGreaterThanOrEqual(probe.bindingSetNs, delegateNs)
            roundTripsMs.append(Double(probe.bindingSetNs &- delegateNs) / 1e6)
            roundTripsCpuMs.append(Double(probe.bindingSetCpuNs &- co.lastUserEditCpuNs) / 1e6)
            // The model holds the edited text and a caret whose UTF-8 offset is exact.
            XCTAssertEqual(tv.selectedRange(), NSRange(location: caret, length: 0))
            XCTAssertEqual(model.caretUTF16, caret)
            XCTAssertEqual(model.caretLengthUTF16, 0)
            XCTAssertEqual(model.caretByte, insertAtByte + typed.utf8.count, "caret byte after typing \(typed.suffix(3).debugDescription)")
        }
        XCTAssertEqual(model.activeText.utf8.count, seed.utf8.count + typed.utf8.count)
        XCTAssertTrue(model.activeText.sameBytes(as: tv.string))
        let stats = LatencyStats(roundTripsMs), rtCpu = LatencyStats(roundTripsCpuMs), cpu = LatencyStats(keystrokeCpuMs)
        print("large-document keystrokes (60 KB, debug build): \(stats.count) textDidChange -> binding round trips, wall p50 \(stats.p50Ms!) ms, p99 \(stats.p99Ms!) ms, max \(stats.maxMs!) ms; round trip CPU p50 \(rtCpu.p50Ms!) ms, max \(rtCpu.maxMs!) ms; whole keystroke CPU p50 \(cpu.p50Ms!) ms, p99 \(cpu.p99Ms!) ms, max \(cpu.maxMs!) ms")
        // Each keystroke's round trip stays under 1 ms. A wall-clock miss counts
        // only when the round trip's own CPU time also exceeded the budget, so
        // preemption by other processes (other agents' builds) is not a failure.
        for (i, ms) in roundTripsMs.enumerated() {
            XCTAssertTrue(ms < 1.0 || roundTripsCpuMs[i] < 1.0,
                          "keystroke \(i) (\(script[i].debugDescription)) textDidChange -> binding took \(ms) ms wall, \(roundTripsCpuMs[i]) ms CPU (\(keystrokeCpuMs[i]) ms CPU for the whole keystroke)")
        }
        XCTAssertLessThan(stats.p50Ms!, 0.5, "median round trip")
        let slowest = keystrokeCpuMs.enumerated().sorted { $0.element > $1.element }.prefix(3)
        print("slowest whole keystrokes (CPU): " + slowest.map { "#\($0.offset) \(script[$0.offset].debugDescription) \($0.element) ms" }.joined(separator: ", "))
        // The last keystroke's caret maps back through the contract conversion.
        let byte = try XCTUnwrap(model.caretByte)
        XCTAssertEqual(model.activeText.nsRange(utf8Bytes: .init(path: "main.tex", startByte: byte, endByte: byte)), NSRange(location: caret, length: 0))
    }
}
