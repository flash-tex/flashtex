import AppKit
import HostedWindows
import SwiftUI
import XCTest
import FlashTeXDisplayListV3
import FlashTeXProtocol
@testable import FlashTeXMac

/// APP-MAIN-THREAD: a compile's Problems rows are made off the main thread
/// when there are many (`EngineV3ProblemsPipeline`), published on main as one
/// finished model (`ProblemsModel`), and never out of order.
@MainActor
final class EngineV3ProblemsPipelineTests: XCTestCase {
    private var env = EnvironmentOverride()

    override func setUp() async throws {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", FileManager.default.temporaryDirectory.appendingPathComponent("problems-pipeline-\(getpid())").path)
        env.set("FLASHTEX_HOST", "none") // the test feeds the session
    }

    override func tearDown() async throws { env.restore() }

    static let text = (1 ... 400).map { "Line \($0) of the text." }.joined(separator: "\n") + "\n"

    private func v3Model() -> ShellModel {
        let model = ShellModel()
        model.autoCompile = false
        model.replaceProject(entryText: Self.text, named: "main.tex")
        model.engineV3Enabled = true
        model.flushChrome() // the listing the groups are sorted by, before the first compile
        return model
    }

    /// `n` warnings on lines of main.tex, each message its own (`tag` tells two compiles apart).
    static func warnings(_ n: Int, tag: String) -> [DL3JSON] {
        (0 ..< n).map { i in
            .object(["severity": .string("warning"), "message": .string("Overfull \\hbox (\(i)pt too wide) \(tag)"),
                     "file": .string("main.tex"), "line": .int(Int64(i % 400 + 1))])
        }
    }

    /// A compile's frames: STARTED, the warnings, DONE.
    static func compile(_ id: Int, _ warnings: [DL3JSON]) -> [EngineV3Reader.Output] {
        [.started(.object(["id": .int(Int64(id)), "keep": .bool(true)]))] + warnings.map { .diagnostic($0) }
            + [.done(.object(["id": .int(Int64(id)), "status": .string("ok"), "pages": .int(1)]), compileID: id)]
    }

    private func waitUntil(_ what: String, timeout: TimeInterval = 30, _ cond: @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while !cond() {
            if Date() > deadline { XCTFail("timed out waiting for \(what)"); return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }

    /// A long document's rows are made off main: right after the DONE the
    /// main thread has not made them; once published they are what
    /// `EngineV3Session.problems` makes, grouped and counted with them.
    func testManyRowsAreMadeOffTheMainThreadAndPublishedFinished() async throws {
        let model = v3Model()
        defer { model.engineV3.stop() }
        let s = model.engineV3
        let warnings = Self.warnings(500, tag: "a")
        let runs = s.problemsPipeline.runs
        for out in Self.compile(1, warnings) { s.handle(out) }
        XCTAssertEqual(s.problemsInFlight, 1, "500 rows: made on the pipeline's queue")
        XCTAssertTrue(model.engineV3Diagnostics.isEmpty, "not published before they are made")
        try await waitUntil("the rows published") { !s.problemsPending }
        XCTAssertEqual(s.problemsPipeline.runs.offMain, runs.offMain + 1)
        XCTAssertEqual(s.problemsPipeline.runs.onMain, runs.onMain)
        let expected = EngineV3Session.problems(warnings, model: model, projectRoot: nil, texts: model.compiledDocuments)
        XCTAssertEqual(model.engineV3Diagnostics, expected)
        let published = model.engineV3Problems
        XCTAssertEqual(published.groups, EditorDiagnostics.groups(of: expected, documentOrder: published.documentOrder))
        XCTAssertEqual(published.counts.warnings, 500)
        XCTAssertEqual(published.spokenSummary, "500 warnings")
        // The same rows again: the same model, nothing published.
        let revision = model.engineV3MarksRevision
        for out in Self.compile(2, warnings) { s.handle(out) }
        try await waitUntil("the second compile's rows made") { !s.problemsPending }
        XCTAssertTrue(model.engineV3Problems === published, "unchanged rows publish nothing")
        XCTAssertEqual(model.engineV3MarksRevision, revision)
    }

    /// A few rows of a short document are made with the DONE, on main (the
    /// hop would cost more than the work), and are there when it returns.
    func testAFewRowsArePublishedWithTheDone() {
        let model = v3Model()
        defer { model.engineV3.stop() }
        let s = model.engineV3
        let runs = s.problemsPipeline.runs
        for out in Self.compile(1, Self.warnings(3, tag: "a")) { s.handle(out) }
        XCTAssertEqual(s.problemsInFlight, 0)
        XCTAssertEqual(model.engineV3Diagnostics.count, 3)
        XCTAssertEqual(s.problemsPipeline.runs.onMain, runs.onMain + 1)
    }

    /// Compiles answered faster than their rows are made (typing in a long
    /// book): one job runs, the later ones wait merged into one, and each
    /// finished job is published, so the rows follow the typing instead of
    /// waiting for it to stop; the last compile's rows are shown at the end,
    /// even when they are few enough to be made on main (they wait their turn).
    func testCompilesFasterThanTheirRowsPublishInOrderAndTheLastWins() async throws {
        let model = v3Model()
        defer { model.engineV3.stop() }
        let s = model.engineV3
        // A first compile sets the compiled texts (which also moves the marks' revision).
        for out in Self.compile(0, Self.warnings(1, tag: "first")) { s.handle(out) }
        let revision = model.engineV3MarksRevision
        for k in 1 ... 4 { for out in Self.compile(k, Self.warnings(800, tag: "c\(k)")) { s.handle(out) } }
        for out in Self.compile(5, Self.warnings(2, tag: "small")) { s.handle(out) }
        XCTAssertEqual(s.problemsInFlight, 1, "one job at a time")
        XCTAssertTrue(s.problemsPending)
        try await waitUntil("every job applied") { !s.problemsPending }
        XCTAssertEqual(model.engineV3Diagnostics.map(\.message), ["Overfull \\hbox (0pt too wide) small", "Overfull \\hbox (1pt too wide) small"])
        // The first compile's rows, then the merged rest's: two publications, in order.
        XCTAssertEqual(model.engineV3MarksRevision, revision + 2)
    }

    /// The Problems panel lists every group (no cap) and makes only the rows
    /// on screen: a new compile whose 3,000 groups all changed re-makes a
    /// screenful of rows, not 3,000, and the table still has 3,000 rows.
    func testThePanelListsEveryGroupAndMakesOnlyTheRowsOnScreen() async throws {
        let model = v3Model()
        defer { model.engineV3.stop() }
        model.problemsVisible = true
        func rows(_ tag: String) -> [RuntimeV1.Diagnostic] {
            (0 ..< 3000).map { i in
                RuntimeV1.Diagnostic(severity: i % 7 == 0 ? .error : .warning, message: "Overfull \\hbox (\(i)pt too wide) \(tag)",
                                     source: .init(path: "main.tex", startByte: (i * 37) % 9000, endByte: (i * 37) % 9000 + 4),
                                     recovery: nil, code: "tex/overfull-hbox")
            }
        }
        model.engineV3Diagnostics = rows("a")
        HostedWindowSupport.prepare()
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 900, height: 420), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let hosting = NSHostingView(rootView: ProblemsPanel().environment(model))
        window.contentView = hosting
        window.orderFrontRegardless()
        defer { window.orderOut(nil); window.contentView = nil }
        func table() -> NSTableView? { DiagnosticsPanelTests.descendants(hosting).compactMap { $0 as? NSTableView }.first }
        try await waitUntil("the table", timeout: 20) { table()?.numberOfRows == 3000 }
        guard let t = table() else { throw XCTSkip("SwiftUI did not back the List with an NSTableView in this host") }
        XCTAssertEqual(t.numberOfRows, 3000, "every group is a row")
        try await Task.sleep(nanoseconds: 300_000_000)
        ViewBodyProbe.start()
        defer { ViewBodyProbe.stop() }
        let t0 = MonotonicClock.nowNs()
        model.engineV3Diagnostics = rows("b") // every group changed
        model.flushChrome()
        try await waitUntil("the new rows on screen", timeout: 20) {
            (t.view(atColumn: 0, row: 0, makeIfNecessary: false).map { _ in true } ?? false) && model.problems === model.engineV3Problems
        }
        try await Task.sleep(nanoseconds: 300_000_000)
        let made = ViewBodyProbe.counts["DiagnosticRow"] ?? 0
        print("ProblemsList: \(made) rows made for 3000 changed groups in \(Double(MonotonicClock.nowNs() &- t0) / 1e6) ms")
        XCTAssertGreaterThan(made, 0, "the visible rows were made again")
        XCTAssertLessThanOrEqual(made, 200, "only the rows on screen are made (a 420 pt panel shows ~15)")
        XCTAssertEqual(t.numberOfRows, 3000)
    }

    /// The session stopped (another project) while a job was on the queue:
    /// its rows are never published.
    func testRowsOfAStoppedSessionAreDropped() async throws {
        let model = v3Model()
        let s = model.engineV3
        for out in Self.compile(1, Self.warnings(600, tag: "old")) { s.handle(out) }
        XCTAssertEqual(s.problemsInFlight, 1)
        s.stop()
        try await waitUntil("the job finished") { !s.problemsPending }
        XCTAssertTrue(model.engineV3Diagnostics.isEmpty, "a stopped session's rows: \(model.engineV3Diagnostics.count)")
    }
}
