import AppKit
import HostedWindows
import Foundation
import SwiftUI
import XCTest
import FlashTeXDisplayListV3
import FlashTeXProtocol
@testable import FlashTeXMac

/// Lane ERROR-RECOVERY (owner, 2026-10-04): the preview is best effort. An
/// error TeX recovers from is a warning marked "pdfLaTeX would report an
/// error here" and the pages it made are shown; a fatal one (TeX stops)
/// stays an error with its cause, and the pane keeps the pages made before
/// the stop plus the last good pages after them, stale, never a blank pane.
/// The rule's tests are pure; the pane's need a built `flashtex-host` and
/// TeX Live (else skip) and run in a hosted window.
@MainActor
final class EngineV3BestEffortTests: XCTestCase {
    typealias Item = EngineV3ErrorPolicy.Item

    // MARK: the rule

    func testRecoveredErrorsAreWarningsUnderBestEffortOnly() {
        let items = [Item(error: true, fatal: false, line: 13), Item(error: false, fatal: false, line: 20), Item(error: true, fatal: false, line: 30)]
        XCTAssertEqual(EngineV3ErrorPolicy.keptErrors(items, mode: .bestEffort), [])
        XCTAssertEqual(EngineV3ErrorPolicy.keptErrors(items, mode: .strict), [0, 2])
    }

    /// ``File `x.tex' not found`` then "Emergency stop" and "Fatal error
    /// occurred", all at the `\input`'s line: the cause and the stops are
    /// errors; an undefined command on an earlier page stays a warning.
    func testAFatalStopKeepsItsCauseAsAnError() {
        let items = [
            Item(error: true, fatal: false, line: 13), // \foo on page 1: recovered
            Item(error: true, fatal: false, line: 672), // File `chapter-missing.tex' not found.
            Item(error: true, fatal: true, line: 672), // Emergency stop.
            Item(error: true, fatal: true, line: 672), // ==> Fatal error occurred
        ]
        XCTAssertEqual(EngineV3ErrorPolicy.keptErrors(items, mode: .bestEffort), [1, 2, 3])
        // "File ended while scanning use of \textbf" and its stop name no line.
        XCTAssertEqual(EngineV3ErrorPolicy.keptErrors([Item(error: true, fatal: false), Item(error: true, fatal: true)], mode: .bestEffort), [0, 1])
        // \end{document} deleted: the stop names no line, the earlier \foo is not its cause.
        XCTAssertEqual(EngineV3ErrorPolicy.keptErrors([Item(error: true, fatal: false, line: 13), Item(error: true, fatal: true)], mode: .bestEffort), [1])
    }

    func testFatalMessagesAreTheHostsRule() {
        for m in ["Emergency stop.", "==> Fatal error occurred, no output PDF file produced!", "TeX capacity exceeded, sorry [input stack size=10000].",
                  "! Emergency stop."] {
            XCTAssertTrue(EngineV3ErrorPolicy.isFatal(message: m), m)
        }
        for m in ["Undefined control sequence.", "File ended while scanning use of \\textbf .", "Missing $ inserted."] {
            XCTAssertFalse(EngineV3ErrorPolicy.isFatal(message: m), m)
        }
    }

    // MARK: the pane, with a host

    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-best-effort-tests-\(getpid())")
    private var env = EnvironmentOverride()
    private var storedZoom: Any?
    override func setUp() {
        OwnerStateGuard.install()
        env.set("FLASHTEX_V3_CACHE", Self.cache.path)
        storedZoom = UserDefaults.standard.object(forKey: PreviewZoom.storageKey)
    }
    override func tearDown() {
        env.restore()
        UserDefaults.standard.set(storedZoom, forKey: PreviewZoom.storageKey)
    }

    func waitUntil(_ what: String, timeout: TimeInterval = 120, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); throw EngineV3TestHost.HostNotReady(description: what) }
            try await Task.sleep(nanoseconds: 30_000_000)
        }
    }

    /// Four short pages; `%@P2` is on page 2.
    static let document = """
    \\documentclass{article}
    \\begin{document}
    \\pdfpageheight=150pt
    First page.\\newpage
    Second page.
    %@P2
    More of the second page.\\newpage
    Third page.\\newpage
    Fourth page.
    \\end{document}

    """

    /// The model, its pane in a hosted window, compiled; and the pages' hashes.
    func pane() async throws -> (ShellModel, EngineV3PagesView, NSWindow, [Int: [UInt8]]) {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.previewZoom = 0.5
        model.replaceProject(entryText: Self.document, named: "main.tex")
        model.engineV3Enabled = true
        model.autoCompile = true
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 560, height: 900), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        try await EngineV3TestHost.awaitReady(model.engineV3)
        let s = model.engineV3
        try await waitUntil("the compile") { s.statusNote.hasPrefix("ok") && s.pageCount == 4 && (0 ..< 4).allSatisfy { s.pages[$0] != nil } && !s.compiling }
        window.layoutIfNeeded()
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 0.5)
        pages.relayout()
        return (model, pages, window, hashes(s))
    }

    func hashes(_ s: EngineV3Session) -> [Int: [UInt8]] { s.pages.mapValues { Array($0.page.hash) } }

    func type(_ model: ShellModel, _ text: String) async throws {
        let s = model.engineV3
        let n = s.doneCount
        model.updateActiveText(text)
        try await waitUntil("the edit's compile") { s.doneCount > n && !s.compiling && s.lastDone?["status"]?.string != "cancelled" }
    }

    /// The owner's report: typing `\textbf{` leaves an unclosed argument, TeX
    /// reads to the end of the file and stops ("File ended while scanning use
    /// of \textbf"; pdflatex writes no PDF). The pane keeps page 1, which the
    /// run made before the stop, and the last good pages 2–4, stale; the
    /// cause is the pane's error. Closing the brace brings every page back.
    func testAFatalErrorKeepsTheLastGoodPagesStaleNotABlankPane() async throws {
        let (model, pages, window, good) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        let bad = Self.document.replacingOccurrences(of: "%@P2", with: "%@P2\nSome \\textbf{bold")
        try await type(model, bad)
        XCTAssertTrue(s.compileFatal, s.statusNote)
        XCTAssertEqual(s.lastDone?["pages"]?.int, 1, "TeX shipped page 1 before it stopped")
        XCTAssertEqual(s.pageCount, 4, "no page vanished: \(s.statusNote)")
        XCTAssertEqual(s.stale, [1, 2, 3], "the last good pages after the stop are stale")
        XCTAssertEqual(Set(s.pages.keys), [0, 1, 2, 3])
        XCTAssertEqual(hashes(s)[2], good[2], "page 3 is the last good one")
        XCTAssertTrue(s.statusNote.hasPrefix("stopped"), s.statusNote)
        XCTAssertTrue(s.firstError?.contains("File ended while scanning use of \\textbf") == true, s.firstError ?? "nil")
        XCTAssertEqual(model.engineV3ResultStatus, .failed)
        let errors = model.displayedDiagnostics.filter { $0.severity == .error }.map(\.message)
        XCTAssertTrue(errors.contains { $0.hasPrefix("File ended while scanning use of \\textbf") }, "\(errors)")
        // Gap 4: TeX names no place for it; the host puts it on the brace that never closes.
        let ended = try XCTUnwrap(model.displayedDiagnostics.first { $0.code == "tex/file-ended-while-scanning" })
        let at = try XCTUnwrap(ended.source, "placed (host diag.rs `locate_runaways`)")
        let brace = bad.utf8.distance(from: bad.startIndex, to: bad.range(of: "\\textbf{bold")!.lowerBound) + "\\textbf".utf8.count
        XCTAssertEqual(at.startByte, brace)
        XCTAssertEqual(bad.utf8Slice(at.startByte, at.endByte), "{")
        XCTAssertTrue(ended.notes?.first?.hasPrefix("The argument of \\textbf opens with {") == true, "\(ended.notes ?? [])")
        // The hosted pane draws all four: page 1 current, 2–4 dimmed as stale.
        window.layoutIfNeeded()
        pages.update(revision: s.layoutRevision, zoom: 0.5)
        pages.relayout()
        XCTAssertEqual(pages.heldPageView(0)?.axStale, false)
        for i in 1 ... 3 { XCTAssertEqual(pages.heldPageView(i)?.axStale, true, "page \(i + 1)") }

        // Typing on inside the argument: still fatal, the same pages kept.
        try await type(model, bad.replacingOccurrences(of: "{bold", with: "{bold te"))
        XCTAssertEqual(s.pageCount, 4)
        XCTAssertEqual(s.stale, [1, 2, 3])

        // The brace closed: every page current and as before but page 2.
        try await type(model, bad.replacingOccurrences(of: "{bold", with: "{bold}"))
        XCTAssertFalse(s.compileFatal)
        XCTAssertEqual(s.pageCount, 4)
        XCTAssertEqual(s.staleCount, 0)
        XCTAssertNil(s.firstError)
        XCTAssertNil(model.engineV3ResultStatus)
        XCTAssertEqual(hashes(s)[0], good[0])
        XCTAssertNotEqual(hashes(s)[1], good[1], "page 2 has the bold text")
        XCTAssertEqual(hashes(s)[3], good[3])
        for i in 0 ... 3 { XCTAssertEqual(pages.heldPageView(i)?.axStale, false, "page \(i + 1)") }
    }

    /// An undefined command: TeX recovers, so the pages are the run's, all
    /// current; the row is a warning marked as pdfLaTeX's error, on the
    /// command; the pane counts no error. Strict mode shows it as an error.
    func testARecoveredErrorIsAMarkedWarningAndEveryPageIsCurrent() async throws {
        let (model, _, window, good) = try await pane()
        defer { model.engineV3.stop(); window.contentView = nil }
        let s = model.engineV3
        let bad = Self.document.replacingOccurrences(of: "%@P2", with: "%@P2\nA \\foo command.")
        try await type(model, bad)
        XCTAssertFalse(s.compileFatal)
        XCTAssertEqual(s.pageCount, 4)
        XCTAssertEqual(s.staleCount, 0)
        XCTAssertEqual(s.errorCount, 0)
        XCTAssertEqual(s.warningCount, 1)
        XCTAssertNil(s.firstError)
        XCTAssertNil(model.engineV3ResultStatus)
        XCTAssertTrue(s.statusNote.hasPrefix("recovered"), s.statusNote)
        XCTAssertEqual(hashes(s)[0], good[0])
        let row = try XCTUnwrap(model.displayedDiagnostics.first { $0.code == "tex/undefined-control-sequence" })
        XCTAssertEqual(row.severity, .warning)
        XCTAssertEqual(row.message, "Undefined control sequence. (pdfLaTeX would report an error here)")
        let src = try XCTUnwrap(row.source)
        XCTAssertEqual(String(decoding: Array(bad.utf8)[src.startByte ..< src.endByte], as: UTF8.self), "\\foo")

        // Strict (Settings > Compile, opt-in): `-halt-on-error`. TeX stops at
        // \foo on page 2, an error; page 1 is current, the last good 2–4 stale.
        let n = s.doneCount
        model.strictTeXErrors = true // recompiles; the defaults are this test process's suite
        defer { model.strictTeXErrors = false }
        XCTAssertEqual(s.errorMode, .strict)
        XCTAssertTrue(s.hostHonoursHaltOnError, "this host lists `halt-on-error` in HELLO")
        XCTAssertFalse(s.strictModeIgnored)
        try await waitUntil("the strict compile") { s.doneCount > n && !s.compiling && s.lastDone?["status"]?.string != "cancelled" }
        XCTAssertTrue(s.compileFatal, s.statusNote)
        XCTAssertEqual(model.displayedDiagnostics.first { $0.code == "tex/undefined-control-sequence" }?.severity, .error)
        XCTAssertTrue(s.firstError?.contains("Undefined control sequence") == true, s.firstError ?? "nil")
        XCTAssertEqual(s.pageCount, 4)
        XCTAssertEqual(s.stale, [1, 2, 3])
    }

    /// A DONE without `pages` says nothing about them: stale pages stay
    /// stale (it used to clear every mark, making kept pages look current).
    func testADoneWithoutPagesKeepsTheStaleMarks() {
        let s = EngineV3Session()
        s.handle(.started(.object(["keep": .bool(true)])))
        s.handle(.pages(.object(["count": .int(3), "stale": .array([.array([.int(1), .int(2)])])])))
        XCTAssertEqual(s.pageCount, 3)
        XCTAssertEqual(s.stale, [1, 2])
        s.handle(.done(.object(["status": .string("error")]), compileID: 1))
        XCTAssertEqual(s.stale, [1, 2], "no `pages`: unknown, not none")
        XCTAssertEqual(s.pageCount, 3)
        s.handle(.done(.object(["status": .string("ok"), "pages": .int(3)]), compileID: 2))
        XCTAssertEqual(s.stale, [], "a DONE with its count: every page it names is current")
    }

    /// Strict mode with a host that does not list `halt-on-error`: errors
    /// are errors, TeX goes on, and Settings says so.
    func testStrictModeKnowsWhenTheHostIgnoresIt() {
        let s = EngineV3Session()
        XCTAssertFalse(s.strictModeIgnored, "no host: nothing to say")
        s.errorMode = .strict
        XCTAssertFalse(s.strictModeIgnored, "not connected yet")
        XCTAssertTrue(s.hostHonoursHaltOnError, "assumed until a HELLO says otherwise")
    }

    /// The request carries `halt_on_error` only in strict mode (protocol 3.x, `Job::halt`).
    func testStrictModeSendsHaltOnError() {
        var r = DL3CompileRequest(id: 1, root: "/r", main: "main.tex")
        XCTAssertNil(r.json["halt_on_error"])
        r.haltOnError = true
        XCTAssertEqual(r.json["halt_on_error"]?.bool, true)
    }
}
