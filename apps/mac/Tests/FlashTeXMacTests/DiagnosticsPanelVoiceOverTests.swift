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

    func testAnnouncerFlushBeforeTheIntervalKeepsThePendingSummary() {
        var a = DiagnosticsAnnouncer()
        let s: UInt64 = 1_000_000_000
        XCTAssertEqual(a.note(summary: "1 error", nowNs: 10 * s), "1 error")
        XCTAssertNil(a.note(summary: "2 errors", nowNs: 10 * s + s / 2))
        XCTAssertEqual(a.pending, "2 errors")
        // A timer armed for an earlier announcement fires early: nothing is spoken, the pending stays.
        XCTAssertNil(a.flush(nowNs: 11 * s))
        XCTAssertEqual(a.pending, "2 errors")
        XCTAssertEqual(a.lastSpoken, "1 error")
        XCTAssertEqual(a.delayNs(nowNs: 11 * s), s)
        // At the interval's end it is spoken; the interval restarts from then.
        XCTAssertEqual(a.flush(nowNs: 12 * s), "2 errors")
        XCTAssertEqual(a.lastSpokenNs, 12 * s)
        XCTAssertNil(a.pending)
        // A shortened interval (tests) throttles by that interval.
        var quick = DiagnosticsAnnouncer(intervalNs: s / 10)
        XCTAssertEqual(quick.note(summary: "1 error", nowNs: 0), "1 error")
        XCTAssertNil(quick.note(summary: "2 errors", nowNs: s / 20))
        XCTAssertNil(quick.flush(nowNs: s / 20 + 1))
        XCTAssertEqual(quick.flush(nowNs: s / 10), "2 errors")
    }

    // MARK: the shell

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("diagnostics-voiceover-\(getpid())")
    private var env = EnvironmentOverride()
    override func setUp() { OwnerStateGuard.install(); env.set("FLASHTEX_V3_CACHE", Self.cache.path) }
    override func tearDown() { env.restore(); try? FileManager.default.removeItem(at: Self.cache) }

    /// A shell whose engine session runs with no host (`FLASHTEX_HOST=none`):
    /// the announcement is made only while the session runs.
    private func shellModel() -> ShellModel {
        env.set("FLASHTEX_HOST", "none")
        let m = ShellModel()
        m.replaceProject(entryText: Self.text)
        m.engineV3Enabled = true
        return m
    }

    /// Applies a finished compile the way the engine's DONE does
    /// (EngineV3Session): its rows are published, then the count is noted.
    private func apply(_ diagnostics: [RuntimeV1.Diagnostic], to m: ShellModel) {
        m.engineV3Diagnostics = diagnostics
        m.noteCompileCompletedForVoiceOver()
    }

    func testShellAnnouncesChangedCountsOnlyWhenACompileCompletes() {
        let m = shellModel()
        defer { m.engineV3.stop() }
        XCTAssertEqual(m.diagnosticAnnouncements, [])
        // Typing (an in-flight revision) announces nothing: only an applied result does.
        m.updateActiveText(Self.text + "typed\n")
        XCTAssertEqual(m.diagnosticAnnouncements, [])
        // A clean first compile is silent.
        apply([], to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, [])
        // Problems appear: spoken.
        apply(Self.diagnostics(), to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented"])
        // The same counts again (another keystroke's compile): silent.
        apply(Self.diagnostics(), to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented"])
        // A change inside the interval waits for the throttle.
        apply(Array(Self.diagnostics().prefix(1)), to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented"])
        XCTAssertEqual(m.diagnosticsAnnouncer.pending, "1 error")
        XCTAssertNotNil(m.diagnosticsAnnouncementFlush, "a flush is scheduled for the end of the interval")
        // Driven early (the armed timer's deadline is still ahead): nothing is spoken, the timer stays.
        m.flushDiagnosticsAnnouncement(nowNs: MonotonicClock.nowNs())
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented"])
        XCTAssertEqual(m.diagnosticsAnnouncer.pending, "1 error")
        XCTAssertNotNil(m.diagnosticsAnnouncementFlush)
        m.flushDiagnosticsAnnouncement(nowNs: MonotonicClock.nowNs() + 10 * DiagnosticsAnnouncer.minimumIntervalNs)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented", "1 error"])
        XCTAssertNil(m.diagnosticsAnnouncer.pending)
        // Opening another project forgets the counts: its first clean compile is silent again.
        m.replaceProject(entryText: "plain\n")
        XCTAssertNil(m.diagnosticsAnnouncementFlush)
        apply([], to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["2 errors, 1 warning, 1 not implemented", "1 error"])
        XCTAssertEqual(m.diagnosticsAnnouncer.lastSpoken, "No problems")
    }

    /// A flush timer armed for one announcement that fires after a later,
    /// immediate one (the main thread was busy) must neither speak the
    /// newer pending summary early nor lose it: the throttle holds from the
    /// last announcement and the newest summary is what gets spoken.
    func testStaleFlushTimerNeverSpeaksEarlyAndTheNewestSummaryWins() async throws {
        let interval: UInt64 = 300_000_000
        let m = shellModel()
        defer { m.engineV3.stop() }
        m.diagnosticsAnnouncer.intervalNs = interval
        let one = Array(Self.diagnostics().prefix(1))
        let gapAndError = Array(Self.diagnostics().prefix(2))
        apply(one, to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error"])
        apply([], to: m)
        XCTAssertEqual(m.diagnosticsAnnouncer.pending, "No problems")
        XCTAssertNotNil(m.diagnosticsAnnouncementFlush, "T1 armed for the end of the interval")
        // Block the main thread past the interval: T1 is due but cannot run yet.
        Thread.sleep(forTimeInterval: Double(interval) / 1e9 + 0.05)
        let spokenAt = MonotonicClock.nowNs()
        apply(Self.diagnostics(), to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error", "2 errors, 1 warning, 1 not implemented"], "due: spoken at once")
        apply(gapAndError, to: m)
        XCTAssertEqual(m.diagnosticsAnnouncer.pending, "1 error, 1 not implemented")
        XCTAssertNotNil(m.diagnosticsAnnouncementFlush, "T1 is still queued; no second timer")
        // Let T1 run: it is early for the newest announcement, so it re-arms rather than speaking.
        let deadline = Date().addingTimeInterval(3)
        while Date() < deadline, m.diagnosticAnnouncements.count < 3 { try await Task.sleep(nanoseconds: 5_000_000) }
        let heardAt = MonotonicClock.nowNs()
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error", "2 errors, 1 warning, 1 not implemented", "1 error, 1 not implemented"],
                       "newest wins: the superseded \"No problems\" is never spoken")
        XCTAssertGreaterThanOrEqual(heardAt &- spokenAt, interval, "the throttle held from the last announcement")
        XCTAssertNil(m.diagnosticsAnnouncer.pending)
        // Cancelling (a new project) while a timer is armed: the timer does nothing when it fires.
        apply(one, to: m)
        XCTAssertEqual(m.diagnosticsAnnouncer.pending, "1 error")
        m.replaceProject(entryText: "plain\n")
        XCTAssertNil(m.diagnosticsAnnouncementFlush)
        try await Task.sleep(nanoseconds: interval + 100_000_000)
        XCTAssertEqual(m.diagnosticAnnouncements.count, 3)
        XCTAssertEqual(m.diagnosticsAnnouncer.lastSpoken, "No problems")
    }

    /// A compile-result fixture on disk, in the runtime v1 envelope `loadFixtures` decodes.
    private func fixture(named name: String, diagnostics: String) throws -> URL {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("DiagnosticsPanelVoiceOverTests-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let url = dir.appendingPathComponent(name)
        let json = """
        {"protocol_version":1,"id":"\(name)","type":"compile_result","payload":{"project_id":"fx","revision":1,"status":"ok","pages":[],"diagnostics":[\(diagnostics)],"pdf_path":null}}
        """
        try json.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    /// Loading a fixture replaces the project's documents without
    /// `replaceProject`, so it must reset the announcer the way
    /// `replaceProject` does: a timer armed for the old project stays silent,
    /// nothing is spoken for the load, and the next compile's first problem
    /// is spoken at once rather than throttled behind the old project's interval.
    func testLoadingAFixtureResetsTheAnnouncerAndSilencesThePendingTimer() async throws {
        let interval: UInt64 = 300_000_000
        let m = shellModel()
        defer { m.engineV3.stop() }
        m.diagnosticsAnnouncer.intervalNs = interval
        apply(Array(Self.diagnostics().prefix(1)), to: m)
        apply(Self.diagnostics(), to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error"])
        XCTAssertEqual(m.diagnosticsAnnouncer.pending, "2 errors, 1 warning, 1 not implemented")
        XCTAssertNotNil(m.diagnosticsAnnouncementFlush, "a timer is armed for the old project")
        let tokenBefore = m.diagnosticsFlushToken

        // A clean fixture replaces the project: the pending summary and the timer are gone, nothing is spoken.
        let clean = try fixture(named: "clean-result.json", diagnostics: "")
        m.loadFixtures(request: nil, result: clean)
        XCTAssertNil(m.loadError)
        XCTAssertEqual(m.fixtureURL, clean)
        XCTAssertNil(m.diagnosticsAnnouncementFlush)
        XCTAssertGreaterThan(m.diagnosticsFlushToken, tokenBefore, "the old timer's token is retired")
        XCTAssertNil(m.diagnosticsAnnouncer.pending)
        XCTAssertNil(m.diagnosticsAnnouncer.lastSpokenNs)
        XCTAssertEqual(m.diagnosticsAnnouncer.lastSpoken, "No problems")
        XCTAssertEqual(m.diagnosticsAnnouncer.intervalNs, interval, "the interval survives the reset")
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error"], "loading the fixture speaks nothing")
        // Past the old timer's deadline: it fired (or was cancelled) without speaking.
        try await Task.sleep(nanoseconds: interval + 100_000_000)
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error"], "the stale timer never speaks across projects")

        // The fixture's first compile with a problem: spoken at once, not held behind the old project's interval.
        apply([.init(severity: .warning, message: "Overfull line", source: nil, recovery: nil)], to: m)
        XCTAssertEqual(m.diagnosticAnnouncements, ["1 error", "1 warning"])
        XCTAssertNil(m.diagnosticsAnnouncementFlush)
        // And File > Open after a fixture resets the same way.
        m.replaceProject(entryText: "plain\n")
        XCTAssertEqual(m.diagnosticsAnnouncer.lastSpoken, "No problems")
        XCTAssertNil(m.diagnosticsAnnouncer.lastSpokenNs)
    }
}
