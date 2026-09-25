import XCTest
import AppKit
import FlashTeXProtocol
import FlashTeXAccessibility
@testable import FlashTeXMac

/// The diagnostics panel as VoiceOver hears it (ux-diagnostics-panel-voiceover):
/// each row is one element with the severity and line in words, the list is
/// "Diagnostics" with a spoken count summary, and a finished compile whose
/// counts changed announces the new summary — throttled, and never for an
/// unchanged count or an in-flight request. SwiftUI materialises its
/// accessibility tree only for an assistive client (see
/// `PanelAccessibilityTests`), so the wording is pinned through the pure
/// rules the views call; the announcement is measured on the shell model.
/// Nothing here hosts a window or takes keyboard focus.
@MainActor
final class DiagnosticsPanelVoiceOverTests: XCTestCase {
    static let text = "line one\n\\foo here\nline three\n\\mathbb{R}\nlast\n"

    /// One error (line 2), one gap (line 4), one unsourced warning, plus a
    /// second `\foo` error on line 5 so the two form a group.
    static func diagnostics() -> [RuntimeV1.Diagnostic] {
        [
            .init(severity: .error, message: "Undefined control sequence \\foo",
                  source: .init(path: "main.tex", startByte: 9, endByte: 13), recovery: "ignored"),
            .init(severity: .warning, message: "\\mathbb is not supported in math mode",
                  source: .init(path: "main.tex", startByte: 30, endByte: 37), recovery: nil),
            .init(severity: .warning, message: "Overfull line", source: nil, recovery: nil),
            .init(severity: .error, message: "Undefined control sequence \\foo",
                  source: .init(path: "main.tex", startByte: 41, endByte: 45), recovery: "ignored"),
        ]
    }

    static func result(_ diagnostics: [RuntimeV1.Diagnostic], revision: Int) -> RuntimeV1.CompileResult {
        RuntimeV1.CompileResult(projectId: "p", revision: revision, status: .recovered, pages: [], diagnostics: diagnostics, pdfPath: nil)
    }

    // MARK: count summary

    func testSpokenSummaryLeavesOutEmptyBuckets() {
        XCTAssertEqual(EditorDiagnostics.spokenSummary((errors: 3, warnings: 1, gaps: 0)), "3 errors, 1 warning")
        XCTAssertEqual(EditorDiagnostics.spokenSummary((errors: 1, warnings: 0, gaps: 2)), "1 error, 2 not implemented")
        XCTAssertEqual(EditorDiagnostics.spokenSummary((errors: 0, warnings: 2, gaps: 0)), "2 warnings")
        XCTAssertEqual(EditorDiagnostics.spokenSummary((errors: 0, warnings: 0, gaps: 0)), "No problems")
        XCTAssertEqual(EditorDiagnostics.spokenSummary(Self.diagnostics()), "2 errors, 1 warning, 1 not implemented")
        XCTAssertEqual(DiagnosticsListView.listLabel, "Diagnostics")
    }

    // MARK: rows

    func testRowsSpeakSeverityMessageAndLine() throws {
        let diags = Self.diagnostics()
        let texts = ["main.tex": Self.text]
        let groups = EditorDiagnostics.groups(of: diags, documentOrder: ["main.tex"])
        XCTAssertEqual(groups.map(\.count), [2, 1, 1], "the two \\foo errors fold into one row")
        let grouped = try XCTUnwrap(groups.first { $0.count == 2 })
        let gap = try XCTUnwrap(groups.first { $0.first == 1 })
        let unsourced = try XCTUnwrap(groups.first { $0.first == 2 })

        let groupedRow = EditorDiagnostics.rowAccessibility(grouped, occurrence: 1, in: diags, status: .recovered, texts: texts)
        XCTAssertEqual(groupedRow.label, "Diagnostic 1 of 4: Error: Undefined control sequence \\foo, 2 places, 2 of 2, main.tex line 5")
        XCTAssertEqual(groupedRow.actions, [DiagnosticRowAccessibility.goToSourceAction])
        XCTAssertNil(groupedRow.hint)

        let gapRow = EditorDiagnostics.rowAccessibility(gap, occurrence: 0, in: diags, status: .recovered,
                                                        explanation: "FlashTeX does not typeset \\mathbb yet.", texts: texts)
        XCTAssertEqual(gapRow.label, "Diagnostic 2 of 4: Not implemented: \\mathbb is not supported in math mode, main.tex line 4",
                       "the puzzle glyph is spoken as words, and the line is in the label, not only in the dimmed trailing text")
        XCTAssertEqual(gapRow.value, "no provisional rendering; FlashTeX does not typeset \\mathbb yet.; main.tex bytes 30 to 37")

        let unsourcedRow = EditorDiagnostics.rowAccessibility(unsourced, occurrence: 0, in: diags, status: .recovered, texts: texts)
        XCTAssertEqual(unsourcedRow.label, "Diagnostic 3 of 4: Warning: Overfull line")
        XCTAssertEqual(unsourcedRow.hint, DiagnosticRowAccessibility.noSourceHint)
        XCTAssertEqual(unsourcedRow.actions, [])

        // Without the compiled text the location falls back to bytes, still spoken.
        let bytes = EditorDiagnostics.rowAccessibility(gap, occurrence: 0, in: diags, status: .ok)
        XCTAssertEqual(bytes.label, "Diagnostic 2 of 4: Not implemented: \\mathbb is not supported in math mode, main.tex bytes 30..<37")
    }

    // MARK: announcement throttle (pure)

    func testAnnouncerSpeaksChangesOnceAndThrottlesBursts() {
        var a = DiagnosticsAnnouncer()
        let s: UInt64 = 1_000_000_000
        // A clean first compile says nothing; the first problem is spoken at once.
        XCTAssertNil(a.note(summary: "No problems", nowNs: 10 * s))
        XCTAssertEqual(a.note(summary: "1 error", nowNs: 11 * s), "1 error")
        // The same count again is silent, even long after.
        XCTAssertNil(a.note(summary: "1 error", nowNs: 20 * s))
        XCTAssertNil(a.pending)
        // Two changes inside the interval: the newest waits, the older is dropped.
        XCTAssertEqual(a.note(summary: "2 errors", nowNs: 21 * s), "2 errors")
        XCTAssertNil(a.note(summary: "3 errors", nowNs: 21 * s + s / 2))
        XCTAssertNil(a.note(summary: "1 error, 1 warning", nowNs: 22 * s))
        XCTAssertEqual(a.pending, "1 error, 1 warning")
        XCTAssertEqual(a.delayNs(nowNs: 22 * s), s)
        XCTAssertEqual(a.flush(nowNs: 23 * s), "1 error, 1 warning")
        XCTAssertNil(a.pending)
        XCTAssertNil(a.flush(nowNs: 24 * s), "nothing pending")
        // A pending change that reverts to the spoken count is not spoken.
        XCTAssertEqual(a.note(summary: "No problems", nowNs: 30 * s), "No problems")
        XCTAssertNil(a.note(summary: "1 error", nowNs: 30 * s + s))
        XCTAssertNil(a.note(summary: "No problems", nowNs: 30 * s + s + s / 2))
        XCTAssertNil(a.pending, "back to the spoken summary: the pending change is withdrawn")
        XCTAssertNil(a.flush(nowNs: 40 * s))
        // After the interval a change is immediate again.
        XCTAssertEqual(a.note(summary: "1 warning", nowNs: 40 * s), "1 warning")
    }

    // MARK: the shell

    func testShellAnnouncesChangedCountsOnlyWhenACompileCompletes() {
        let m = ShellModel()
        m.replaceProject(entryText: Self.text)
        XCTAssertEqual(m.diagnosticAnnouncements, [])
        // Typing (an in-flight revision) announces nothing: only an applied result does.
        m.updateActiveText(Self.text + "typed\n")
        XCTAssertEqual(m.diagnosticAnnouncements, [])
        // A clean first compile is silent.
        m.result = Self.result([], revision: 1)
        XCTAssertEqual(m.diagnosticAnnouncements, [])
        // Problems appear: spoken.
        m.result = Self.result(Self.diagnostics(), revision: 2)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented"])
        // The same counts again (another keystroke's compile): silent.
        m.result = Self.result(Self.diagnostics(), revision: 3)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented"])
        // A change inside the interval waits for the throttle.
        m.result = Self.result(Array(Self.diagnostics().prefix(1)), revision: 4)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented"])
        XCTAssertEqual(m.diagnosticsAnnouncer.pending, "1 error")
        XCTAssertNotNil(m.diagnosticsAnnouncementFlush, "a flush is scheduled for the end of the interval")
        m.flushDiagnosticsAnnouncement(nowNs: MonotonicClock.nowNs() + 10 * DiagnosticsAnnouncer.minimumIntervalNs)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented", "1 error"])
        XCTAssertNil(m.diagnosticsAnnouncer.pending)
        // Opening another project forgets the counts: its first clean compile is silent again.
        m.replaceProject(entryText: "plain\n")
        XCTAssertNil(m.diagnosticsAnnouncementFlush)
        m.result = Self.result([], revision: 1)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented", "1 error"])
        XCTAssertEqual(m.diagnosticsAnnouncer.lastSpoken, "No problems")
    }
}
